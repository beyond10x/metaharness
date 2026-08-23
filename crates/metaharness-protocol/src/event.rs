//! What a run tells the outside world.
//!
//! Nineteen events in five groups plus one deviation, each named by the design's § 4.1 table.
//! Two properties of this module are load-bearing and neither is visible from a type signature:
//!
//! * **Payload fields are `Option` down to the leaves and serialize as `null` when absent.**
//!   Absence is the `unk` verdict. A reader that saw a missing key could not tell "the harness
//!   recorded nothing" from "this metaharness build does not emit it", and a bound that read a
//!   missing field as zero would report its blindest case as its best one (design § 2.1).
//! * **A record the adapter could not read becomes [`Event::Opaque`] and is never dropped**
//!   (design D4), because the failure that costs the most is a checker reporting "the tool was
//!   never called" when what happened is that it stopped being able to see tool calls.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::command::{CommandOutcome, Decision};
use crate::frame::{Digest, StepRef};
use crate::hermetic::HermeticAttestation;

/// Short codes for [`Event::Warning`], so an embedder can match on one without matching prose.
pub mod warning_code {
    /// The vendor binary's version is outside the adapter's pin (design § 8.4 O1).
    pub const VERSION_OFF_PIN: &str = "VERSION_OFF_PIN";
    /// A tool is offered that the control seam does not cover (design § 7.8).
    pub const COVERAGE_GAP: &str = "COVERAGE_GAP";
    /// The run asked for a control this adapter declares unverified rather than delivered.
    pub const UNVERIFIED_TIER: &str = "UNVERIFIED_TIER";
    /// An ambient input was found that metaharness reports and does not claim to have removed —
    /// git status is the named case (design § 8.1, H11's second half).
    pub const AMBIENT_INPUT: &str = "AMBIENT_INPUT";
}

/// Where a decision was taken.
///
/// The vendor mechanism, named, because "the seam decided" is not an audit if the seam could
/// have been any of four things with different guarantees (design § 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Seam {
    /// The offered set, fixed at launch. Cannot see an argument.
    Registration,
    /// A `PreToolUse` command hook the adapter installed, which blocks the call.
    Hook,
    /// A control request on the vendor's own bidirectional wire.
    ControlRequest,
    /// metaharness owns the tool and runs it, so the model never reaches an implementation.
    OwnedTool,
    /// No seam covered this call. Emitted rather than omitted, because a call nobody adjudicated
    /// must be visible as one.
    None,
}

/// Who took a decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DecidedBy {
    /// The embedder answered a `tool.decide` command.
    Embedder,
    /// The frame already said yes or no, and the adapter applied it without a round trip.
    Frame,
    /// metaharness's own deadline expired first, so metaharness denied (design § 7.7 rule 2).
    Deadline,
    /// The adapter refused for a reason of its own — an uncovered tool, a shadowed seam.
    Adapter,
}

/// How a step ended.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum StepOutcome {
    /// The step produced its handoff.
    Completed,
    /// The step ran and did not satisfy what it owed.
    Failed {
        /// Why, for the run report.
        reason: String,
    },
    /// Nobody found out. A crashed harness is not a failing run, and submitting a failing
    /// verdict for something that never ran fabricates an observation (design § 9.4).
    NoVerdict {
        /// What went missing.
        reason: String,
    },
}

/// One plugin the harness loaded.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginRef {
    /// The plugin's name as the harness reported it.
    pub name: Option<String>,
    /// Where it was loaded from.
    pub source: Option<String>,
    /// Its version, when the harness reported one.
    pub version: Option<String>,
}

/// One MCP server the session has.
///
/// A **list**, never a count, because a server the session cannot authenticate to still exists,
/// is still named, and is still a reach outside the sandbox; it exposes no tool, so a tool
/// inventory is identical with and without it (design § 8.1 H5).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerRef {
    /// The server's name, as the launch configuration named it.
    pub name: Option<String>,
    /// Its connection status, as the harness reported it.
    pub status: Option<String>,
}

/// The raw vendor transcript this run was read from.
///
/// Required by design § 8.4 O8: the projection's `transcript_digest` and `source_line`, the
/// § 4.4 cross-check and the § 9.4 auditor all read the bytes, and none of them works without
/// them retained.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TranscriptRef {
    /// Where the retained bytes are.
    pub path: Option<String>,
    /// The digest of those bytes.
    pub digest: Option<Digest>,
    /// How many bytes were retained.
    pub bytes: Option<u64>,
}

/// Tokens, as the vendor reported them — and, where the vendor priced them, what they cost.
///
/// Never computed here: costs are read from what the vendor said, and a number metaharness
/// derived would be a second figure that can disagree with the invoice (design § 4.1).
///
/// `Eq` is deliberately not derived, on the same rule [`RateLimitInfo`] carries: `cost_usd` is the
/// vendor's own float, and two runs are compared by what they recorded rather than by an equality
/// this crate invented for a fraction.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    /// Input tokens.
    pub input_tokens: Option<u64>,
    /// Output tokens.
    pub output_tokens: Option<u64>,
    /// Tokens read from the prompt cache.
    pub cache_read_input_tokens: Option<u64>,
    /// Tokens written to the prompt cache.
    pub cache_creation_input_tokens: Option<u64>,
    /// The service tier the vendor billed at.
    pub service_tier: Option<String>,
    /// The **billed** thinking tokens, where the vendor breaks them out of the output figure
    /// (amendment a9).
    ///
    /// The invoice's number, never [`Event::ThinkingEstimate`]'s, which is a mid-stream guess
    /// wearing a similar name: an adapter that filled this from the estimate would be reporting a
    /// guess where a reader expects a charge.
    pub thinking_tokens: Option<u64>,
    /// How many per-iteration usage records the vendor's own record held (amendment a9).
    ///
    /// A **length read off the vendor's list**, not a counter metaharness kept — and the one
    /// number in this struct nobody billed, which is why it is carried as a count rather than as
    /// an array whose per-iteration figures would be a second copy of the totals beside it.
    /// `Some(0)` is the vendor saying *none*; [`None`] is the vendor not saying.
    pub iterations: Option<u64>,
    /// The speed tier the account was served at, in the vendor's own word (amendment a9).
    ///
    /// Beside `service_tier` rather than folded into it: a vendor that reports both reports two
    /// different facts, and a reader asserting on one must not be answered with the other.
    pub speed: Option<String>,
    /// What these tokens cost in US dollars, **as the vendor priced them** (amendment a9).
    ///
    /// Filled only where the vendor prices this slice of a run. Claude Code prices its per-model
    /// split and nothing else, so this is present under `session.ended`'s `model_usage` and absent
    /// in the aggregate and in every per-request `usage` event — the run's own figure is
    /// `session.ended.total_cost_usd` and stays there.
    ///
    /// Never derived from the token counts beside it. A cost metaharness multiplied out is a
    /// number nobody billed, and it would disagree with the invoice the first time a rate moved
    /// (design § 4.1, D4).
    pub cost_usd: Option<f64>,
}

/// One entry of the vendor's own terminal denial list, passed through unchanged.
///
/// metaharness never adds to it. Its own per-call denial audit is [`Event::ToolDecided`], and
/// mixing the two would guarantee a disagreement between the two counts (design D6, finding F10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PermissionDenial {
    /// The tool that was refused.
    pub tool_name: Option<String>,
    /// The call it was refused for.
    pub tool_use_id: Option<String>,
    /// The input it was refused with.
    pub tool_input: Option<Value>,
}

/// What metaharness's own seam did, counted.
///
/// Always printed, because a report that hides "0 denials" reads as clean when it may mean
/// nothing was ever attempted (design § 9.4).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DecisionCensus {
    /// Calls allowed.
    pub allowed: u32,
    /// Calls denied.
    pub denied: u32,
    /// Calls whose input was replaced.
    pub replaced: u32,
    /// Calls metaharness adjudicated **not at all**, leaving the vendor's own permission
    /// pipeline to decide (amendment a3). Counted separately from `allowed` because "we let it
    /// through" and "we claimed nothing" are different facts about who was in control, and a
    /// census that folded them together would report an unadjudicated run as a permissive one.
    pub abstained: u32,
    /// The same totals split by the seam that carried them.
    pub by_seam: BTreeMap<String, u32>,
    /// The same totals split by who decided.
    pub by_decider: BTreeMap<String, u32>,
}

/// A rate-limit window, as the vendor reported it.
///
/// `Eq` is deliberately not derived: `utilization` is the vendor's own float and two runs are
/// compared by what they recorded, never by an equality this crate invented for a fraction.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// The vendor's own status word.
    pub status: Option<String>,
    /// Which window this is.
    pub window: Option<String>,
    /// When it resets, as the vendor recorded it.
    pub resets_at: Option<i64>,
    /// How much of it is used, as the vendor computed it.
    pub utilization: Option<f64>,
    /// Whether the run is being paid for out of overage — a fact about money nothing else
    /// carries (design § 4.1).
    pub using_overage: Option<bool>,
}

/// Something a run tells the outside world.
///
/// The wire name is the `event` field, not the Rust spelling. Unknown fields on a known event
/// are ignored in silence; an unknown event **name** is a named refusal, because this wire is an
/// authored schema and a misspelling here is a mistake the author wants to be told about
/// (design D2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum Event {
    /// The opening record: where a class of defect is visible before a turn is spent, and the
    /// only place that can distinguish *offered* from *called*.
    ///
    /// The field set is the IR's rather than a shorter one of our own, because a field
    /// metaharness omits is an expectation kind that becomes undecidable (design § 4.1, F11).
    #[serde(rename = "session.started")]
    SessionStarted {
        /// Which adapter is driving, and of which class.
        adapter: String,
        /// The adapter's class — a harness adapter never silently becomes a direct API call
        /// (design § 8.4 O5).
        adapter_class: String,
        /// The vendor version actually observed, against which the adapter's pin is checked.
        harness_version: Option<String>,
        /// The vendor's own session id.
        session_id: Option<String>,
        /// The model the vendor resolved to.
        model: Option<String>,
        /// The permission posture the session is in.
        permission_mode: Option<String>,
        /// Where the credential came from — the evidence for H4.
        credential_source: Option<String>,
        /// The output style in force — the evidence for H1b.
        output_style: Option<String>,
        /// The working directory — the evidence for H7.
        cwd: Option<String>,
        /// Every tool the model was offered. Offered is not called, and only this record can
        /// tell them apart.
        offered_tools: Option<Vec<String>>,
        /// Slash commands the session has.
        slash_commands: Option<Vec<String>>,
        /// Skills the session has.
        skills: Option<Vec<String>>,
        /// Subagents the session has.
        agents: Option<Vec<String>>,
        /// Plugins the session loaded — the evidence for H1a.
        plugins: Option<Vec<PluginRef>>,
        /// MCP servers, as a list — the evidence for H5. `None` is `unk`, never zero.
        mcp_servers: Option<Vec<McpServerRef>>,
        /// The digest of the copied input tree — the evidence for H10.
        inputs_digest: Option<Digest>,
        /// The retained raw transcript (design § 8.4 O8).
        transcript: TranscriptRef,
        /// What metaharness imposed and what it could not. **Not evidence** — it is
        /// metaharness's own claim about its own actions, and it sits beside the vendor's record
        /// so a reader can notice when the two disagree (design § 8.3).
        hermetic: HermeticAttestation,
    },

    /// The terminal record: the source of every resource fact.
    #[serde(rename = "session.ended")]
    SessionEnded {
        /// Whether the vendor called it an error.
        is_error: Option<bool>,
        /// The vendor's own result subtype.
        subtype: Option<String>,
        /// Why the model stopped.
        stop_reason: Option<String>,
        /// Why the session ended.
        terminal_reason: Option<String>,
        /// The API error status, when there was one.
        api_error_status: Option<String>,
        /// How many turns were spent.
        num_turns: Option<u64>,
        /// Wall-clock duration, as the vendor recorded it.
        duration_ms: Option<u64>,
        /// API duration, as the vendor recorded it.
        duration_api_ms: Option<u64>,
        /// Time to first token, as the vendor recorded it.
        ttft_ms: Option<u64>,
        /// Time to the first request, as the vendor recorded it.
        time_to_request_ms: Option<u64>,
        /// What the vendor says it cost.
        total_cost_usd: Option<f64>,
        /// The vendor's own denial list, passed through and never added to.
        permission_denials: Option<Vec<PermissionDenial>>,
        /// How many subagents were spawned, as the vendor counted them.
        subagents_spawned: Option<u64>,
        /// Tokens, as the vendor reported them.
        usage: Option<Usage>,
        /// The same, split per model.
        model_usage: Option<BTreeMap<String, Usage>>,
        /// What metaharness's own seam did. Distinct from `permission_denials` on purpose.
        census: DecisionCensus,
    },

    /// The embedder's unit of work opened.
    #[serde(rename = "step.entered")]
    StepEntered {
        /// Which step.
        step: StepRef,
        /// The frame in force, cited rather than repeated.
        frame_digest: Option<Digest>,
    },

    /// The embedder's unit of work closed.
    #[serde(rename = "step.left")]
    StepLeft {
        /// Which step.
        step: StepRef,
        /// How it ended.
        outcome: StepOutcome,
    },

    /// The vendor's unit of work opened.
    #[serde(rename = "turn.started")]
    TurnStarted {
        /// Which turn, counting from 1.
        turn: u32,
        /// The frame in force.
        frame_digest: Option<Digest>,
    },

    /// The vendor's unit of work closed.
    #[serde(rename = "turn.ended")]
    TurnEnded {
        /// Which turn.
        turn: u32,
        /// Why it stopped.
        stop_reason: Option<String>,
    },

    /// What the model said to the operator.
    #[serde(rename = "text")]
    Text {
        /// The text.
        text: String,
        /// Which request produced it.
        request_id: Option<String>,
    },

    /// What the model reasoned.
    ///
    /// Kept separate from [`Event::Text`] because an assertion about what the model *said* must
    /// not match its reasoning (design § 4.1).
    #[serde(rename = "thinking")]
    Thinking {
        /// The reasoning text.
        text: String,
        /// Which request produced it.
        request_id: Option<String>,
    },

    /// The harness's live estimate of thinking tokens — a mid-stream guess, never the invoice.
    #[serde(rename = "thinking.estimate")]
    ThinkingEstimate {
        /// The running estimate.
        estimate: u64,
        /// How much it moved.
        delta: Option<i64>,
    },

    /// Text the *harness* put in the conversation.
    ///
    /// Recorded, and deliberately given no expectation kind: a matcher over injected text is a
    /// wording assertion in a structural costume (design § 4.1).
    #[serde(rename = "injection")]
    Injection {
        /// The injected text.
        text: String,
        /// Where it came from — a loaded skill, an injected frame.
        origin: Option<String>,
    },

    /// A tool call, **before** the decision and before any effect.
    #[serde(rename = "tool.requested")]
    ToolRequested {
        /// Correlates this request to its decision and its result.
        call_id: String,
        /// The vendor's tool name.
        name: String,
        /// The input as presented.
        input: Value,
        /// Whether the run is blocked here waiting for an embedder decision.
        decision_required: bool,
        /// The budget for deciding, armed at delivery to the embedder rather than at the
        /// vendor's request, so an embedder cannot be timed out by its own queue (design § 7.7
        /// rule 5).
        deadline_ms: Option<u64>,
        /// Which seam will carry the decision.
        seam: Seam,
    },

    /// The denial record. A first-class event, not a log file.
    ///
    /// Three events rather than two because a denial has no result and a decision is not a
    /// result; folding the decision into the result could not express "this was refused and
    /// nothing ran" without inventing a fake result (design § 4.1).
    #[serde(rename = "tool.decided")]
    ToolDecided {
        /// The call this decides.
        call_id: String,
        /// What was decided.
        decision: Decision,
        /// Who decided it.
        decided_by: DecidedBy,
        /// Which seam carried it.
        seam: Seam,
        /// How long the decision took, when the adapter observed both ends.
        latency_ms: Option<u64>,
    },

    /// What the tool returned.
    #[serde(rename = "tool.result")]
    ToolResult {
        /// The call this answers.
        call_id: String,
        /// Whether the vendor called it an error.
        is_error: Option<bool>,
        /// The result content, as the vendor recorded it.
        content: Option<Value>,
        /// How many bytes of content there were.
        bytes: Option<u64>,
        /// The vendor's own per-tool result record, verbatim, where the vendor writes one beside
        /// the content (amendment a9).
        ///
        /// Claude Code's `tool_use_result` sibling is the case this exists for: `Skill` records
        /// `commandName` and `success`, `Bash` records `stdout`, `stderr` and `interrupted`,
        /// `Edit` records `filePath` and `userModified`. Carried as the vendor's own JSON rather
        /// than as a field set of ours, because the shape belongs to the *tool* — enumerating it
        /// here would mean a tool nobody has heard of yet reports into a record with no room for
        /// it, which is § 4.1's failure in miniature.
        ///
        /// [`None`] where the vendor writes no such sibling; a Codex rollout's tool output has
        /// none. Never synthesised from `content`, which is what the model was told and a
        /// different record answering a different question — an adapter that filled one from the
        /// other would let `skill.completed` pass on evidence nobody produced.
        tool_use_result: Option<Value>,
    },

    /// Tokens for one request or turn.
    #[serde(rename = "usage")]
    Usage {
        /// Which request.
        request_id: Option<String>,
        /// Which model.
        model: Option<String>,
        /// The figures, as the vendor reported them.
        usage: Usage,
    },

    /// A rate-limit window.
    #[serde(rename = "rate_limit")]
    RateLimit {
        /// The window, as the vendor reported it.
        info: RateLimitInfo,
    },

    /// The answer to exactly one command.
    ///
    /// Every command produces one of these: a command that can be silently ignored is a control
    /// surface that cannot be tested (design D9).
    #[serde(rename = "command.result")]
    CommandResult {
        /// The command's id.
        id: String,
        /// What happened to it.
        outcome: CommandOutcome,
    },

    /// metaharness has something to say.
    ///
    /// Distinct from [`Event::Opaque`], which means *the vendor said something we could not
    /// read*.
    #[serde(rename = "warning")]
    Warning {
        /// A short stable code — see [`warning_code`].
        code: String,
        /// What it means, for a person.
        message: String,
    },

    /// The vendor said something the adapter could not map.
    ///
    /// Mandatory and unconditional (design D4). An adapter that recognised a record's envelope
    /// and read nothing out of it emits this too, because an event that produced nothing has
    /// vanished whatever the intention was.
    #[serde(rename = "opaque")]
    Opaque {
        /// The vendor's own `type`.
        vendor_type: Option<String>,
        /// The vendor's own `subtype`.
        vendor_subtype: Option<String>,
        /// The digest of the raw record, so it is citable without being reproduced.
        digest: Digest,
        /// Which 1-based line of the retained transcript it was.
        source_line: Option<u64>,
    },

    /// The credential the run was launched with stopped working while the run was in flight.
    ///
    /// **This event is a deviation from design v0.1 and is recorded as one** — see Q13 in
    /// `docs/design/metaharness-protocol-v0.1.md` § 12 and the amendment to § 8.1 H6. It exists
    /// because an operator-login token is a snapshot with a lifetime: a governed run observed on
    /// 2026-08-22 died an hour in with the vendor reporting an expired OAuth session that could
    /// not be refreshed, and a copied file cannot refresh itself.
    ///
    /// It is not the `error` channel § 4.3 refuses. It does not end the run and it is not a
    /// second terminal record: the run still ends with [`Event::SessionEnded`]. It exists so an
    /// embedder can tell "the credential aged out" from "the model failed" without matching on
    /// vendor prose, and retry deterministically.
    #[serde(rename = "auth.expired")]
    AuthExpired {
        /// Where the credential came from, so a retry knows what to refresh.
        credential_source: Option<String>,
        /// The vendor's own words, passed through — metaharness does not paraphrase a diagnosis
        /// it did not make.
        detail: Option<String>,
        /// Which 1-based line of the retained transcript said so.
        source_line: Option<u64>,
    },
}

/// An event with the timestamp the vendor recorded for it, or none.
///
/// A pair rather than a field on [`Event`] so that "metaharness never measures" is enforced by
/// the type: an adapter that has no vendor timestamp has no way to invent one here without the
/// omission being visible (design D2).
#[derive(Debug, Clone, PartialEq)]
pub struct Emission {
    /// The vendor's recorded timestamp, passed through verbatim, or `None`.
    pub at: Option<String>,
    /// The event.
    pub event: Event,
}

impl Emission {
    /// An event the vendor recorded no timestamp for.
    #[must_use]
    pub fn untimed(event: Event) -> Self {
        Self { at: None, event }
    }

    /// An event with the timestamp the vendor recorded, verbatim.
    #[must_use]
    pub fn at(timestamp: impl Into<String>, event: Event) -> Self {
        Self {
            at: Some(timestamp.into()),
            event,
        }
    }
}

impl Event {
    /// The event's wire name.
    ///
    /// A method rather than a `match` at each call site, because the wire names are the thing
    /// two crates must agree about.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Event::SessionStarted { .. } => "session.started",
            Event::SessionEnded { .. } => "session.ended",
            Event::StepEntered { .. } => "step.entered",
            Event::StepLeft { .. } => "step.left",
            Event::TurnStarted { .. } => "turn.started",
            Event::TurnEnded { .. } => "turn.ended",
            Event::Text { .. } => "text",
            Event::Thinking { .. } => "thinking",
            Event::ThinkingEstimate { .. } => "thinking.estimate",
            Event::Injection { .. } => "injection",
            Event::ToolRequested { .. } => "tool.requested",
            Event::ToolDecided { .. } => "tool.decided",
            Event::ToolResult { .. } => "tool.result",
            Event::Usage { .. } => "usage",
            Event::RateLimit { .. } => "rate_limit",
            Event::CommandResult { .. } => "command.result",
            Event::Warning { .. } => "warning",
            Event::Opaque { .. } => "opaque",
            Event::AuthExpired { .. } => "auth.expired",
        }
    }
}
