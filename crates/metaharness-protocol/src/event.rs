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
    /// The vendor retried a request against its own API, and said so on the wire.
    ///
    /// Not [`Event::Opaque`] and not dropped. Dropped would lose the only account of why a run
    /// stalled — a recording on 2026-09-03 carried 10 of these before a `529 Overloaded` ended it
    /// — and opaque would make every absence row in a checker reading that stream `undecidable`,
    /// which is a transport hiccup deciding a plan question.
    pub const VENDOR_API_RETRY: &str = "VENDOR_API_RETRY";
    /// The vendor's set of live background tasks changed, and it restated the whole set.
    ///
    /// Here for the same reason as [`VENDOR_API_RETRY`], and against the same alternative. The
    /// vendor emits it on completion and kill as well as on start, and only the start is on the
    /// wire elsewhere as a `tool.result` — so dropping it lets a reader take absence for fact.
    pub const VENDOR_BACKGROUND_TASKS: &str = "VENDOR_BACKGROUND_TASKS";
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
    /// **The run is in observe mode**, so the call was allowed because every call is
    /// (amendment a10).
    ///
    /// Its own decider rather than [`DecidedBy::Adapter`] with an allow, because the two say
    /// different things to anybody counting: an adapter allow is a judgement about this call, and
    /// this is a run-wide posture that judged nothing. A census that folded them together would
    /// report a capture run as a run whose policy happened to permit everything.
    Observe,
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

/// One tool the run **asked for** and the machine would not admit, with the reason.
///
/// A type of this crate's own, on invariant 1: `metaharness-protocol` depends on `clap`, `serde`,
/// `serde_json` and `sha2`, so the harness's own `Withheld` cannot be imported here however
/// identical the two shapes are. The wire is the contract between them, not a Rust type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WithheldTool {
    /// The tool the run declared and did not get, under the name the harness publishes it by.
    ///
    /// The entry's own name and never a surface verb's, for the reason an approval names the
    /// entry: a reader decides about `run`, never about `tool_invoke`.
    pub tool: String,
    /// The predicate that failed, as the machine stated it — the harness's words, passed through.
    ///
    /// Never rewritten here. A reason metaharness paraphrased would be a second description of a
    /// machine metaharness never probed.
    pub reason: String,
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
        /// **What the run could do**, in the neutral operation vocabulary.
        ///
        /// Beside `offered_tools` and answering a different question. That field says what the
        /// model was *offered*; this says what the run could *perform*, and the two stop agreeing
        /// the moment a surface publishes fewer tools than it has reach.
        ///
        /// Behind three verbs they come apart completely: every such run offers `tool_search`,
        /// `tool_describe`, `tool_invoke` and nothing else, while the catalogue behind them holds
        /// three entries on a machine that cannot confine a process and six on one that can. An
        /// attribution control asking *was there a writer to refuse* read the tool list, found no
        /// writer, and reported the run as having none — a verdict about a vocabulary rather than
        /// about the run.
        ///
        /// Filled for a vendor harness too, from its offered tools through the adapter's published
        /// rendering, so the question has one answer on every arm.
        ///
        /// [`None`] means the harness did not say, never that the run could do nothing.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        available_operations: Option<Vec<String>>,
        /// **What the run asked for and this machine would not admit**, each with the predicate
        /// that decided.
        ///
        /// The third question the opening record has to answer, and neither of the two above can.
        /// `offered_tools` says what the model was **offered**; `available_operations` says what
        /// the run could **do**; both describe a set that is *present*, and a tool the machine
        /// refused to admit is absent from every one of them. So a run that was denied execution
        /// and a run that never wanted it produce the identical record — which is not a gap in a
        /// report, it is a gap in the only evidence anybody has afterwards.
        ///
        /// On 2026-08-29 that cost weeks. A driven session whose only legal route was running a
        /// program was published a six-entry catalogue instead of seven — no error, no warning, no
        /// fact anywhere in the record — hand-wrote files instead, and the failure was read as a
        /// model failure. It was the machine's: the publication gate had dropped `run` because the
        /// capability facts it needs were absent, which is the gate working exactly as designed.
        /// What was missing was never a refusal — putting the tool back in front of the model is
        /// the thing publication exists to avoid — it was the **fact**, and this field is it.
        ///
        /// **[`None`] means the harness did not say, and never that nothing was withheld.** The
        /// two are different runs and only an adapter that watched the harness can tell them
        /// apart: a producer that writes this field states `[]` for a run that got everything it
        /// asked for, and one that has never heard of the field states nothing at all. An adapter
        /// that read silence as `[]` would be asserting *this machine admitted everything* about a
        /// vendor it never asked (invariant 3, amendment a4's rule: absence of evidence is not a
        /// property).
        ///
        /// Serialized as an explicit `null` rather than skipped, on § 2.1's rule and amendment
        /// a9's restatement of it — *an absent field is an explicit `null` and never a missing
        /// key, so a build that predates the amendment and a vendor that reports nothing stay
        /// distinguishable.* A missing key is precisely the silence this field exists to end, and
        /// it would be an odd field that reintroduced it one level up.
        #[serde(default)]
        withheld: Option<Vec<WithheldTool>>,
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
        /// What this call **is**, in the neutral operation vocabulary — `file.write`, `shell`.
        ///
        /// The field a reader of a finished run actually wants. `name` is the vendor's, and one
        /// act has a different one in every harness: `workspace_write`, `Write`, `apply_patch`,
        /// or `tool_invoke` with the entry inside its input. A consumer that selected on `name`
        /// was therefore written in one vendor's vocabulary and blind to the rest — which is what
        /// the evaluation corpus was, and why widening it kept making it worse.
        ///
        /// A **list**, because a rendering need not be injective: codex writes and edits through
        /// one `apply_patch`, so one call answers to two operations.
        ///
        /// Empty means one of three different things, and they are deliberately not distinguished
        /// here — no operation in the v0.1 vocabulary covers it, or the call is a question about
        /// the catalogue rather than an act, or the resolution was never attempted. What is *not*
        /// ambiguous is that an empty list is never evidence that nothing happened.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        operations: Vec<String>,
        /// **What** this call would touch: `file:src/lib.rs`, `proc:/usr/bin/python3`.
        ///
        /// `operations` says a write happened; this says which file. Both are needed and neither
        /// implies the other, and a consumer that had only the first could assert *this run wrote
        /// something* and never *this run wrote the test before the source*.
        ///
        /// The scheme-prefixed form is `harness_wire::Subject`'s, carried across rather than
        /// re-coined: `file:` for a path, `proc:` for a program a call would start.
        ///
        /// **The path is as the caller wrote it**, never canonicalised. A reader asking where a
        /// call was going has to see `../../etc/passwd` as the model sent it; the tidy answer is
        /// exactly wrong for the one call whose whole problem is where it pointed.
        ///
        /// Empty means the record does not say, never that the call touched nothing. Its absence
        /// is why a path-scoped expectation had to read a vendor's own argument name — `file_path`
        /// on Claude Code, and a level down under `arguments.path` on a three-verb surface — so
        /// every such row decided on one harness and reported `unk` for the rest.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        subjects: Vec<String>,
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

    /// Something worth saying that is not a step, a call or a turn.
    ///
    /// Mostly metaharness's own voice — an off-pin binary, an uncovered tool, an ambient input.
    /// Since 0.6.3 also the **vendor's**, for a record this adapter read perfectly and that no
    /// other event carries: `VENDOR_API_RETRY` and `VENDOR_BACKGROUND_TASKS` in [`warning_code`].
    /// The third case those two make is worth naming, because the first reading of this doc
    /// comment says they do not belong here: a record can be *read* and still have no home in the
    /// step/call/turn vocabulary, and the choice then is this or [`Event::Opaque`] — and `opaque`
    /// means *the vendor said something we could not read*, which would be false.
    ///
    /// So the distinction this draws is not who spoke. It is whether the adapter understood.
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

    /// The stream ends here, on purpose (amendment a17).
    ///
    /// **The completeness record, and the last line of every stream this driver writes.** Without
    /// it a stream that contains no `Bash` call and a stream that was cut off before the first one
    /// are the same bytes, so every *negative* expectation about a run — `nothing-was-moved`,
    /// `no-store-command-was-run` — is undecidable whatever the run actually did. Eight
    /// `aep observe trace check` reports ended `undecided` for exactly that reason on 2026-09-03. It is
    /// [`Event::Opaque`]'s rule (design D4) one level up: about the file rather than about a record
    /// inside it.
    ///
    /// It is **not** a second terminal record and it ends no run: [`Event::SessionEnded`] is still
    /// the terminal record, still carries every resource fact, and still comes first. This line
    /// carries the one thing that record cannot — *and then the file stopped*.
    #[serde(rename = "stream.closed")]
    StreamClosed {
        /// How many lines preceded this one.
        ///
        /// **Checked rather than believed.** A reader compares it against the lines it actually
        /// read, so a truncated stream that somehow kept its marker is `inconsistent` and never
        /// `complete` — see [`stream_completeness`]. A count a reader had to take on trust would
        /// add nothing a reader did not already have.
        events: u64,
        /// Why the stream ended.
        reason: CloseReason,
        /// Which run this closes.
        ///
        /// Also on the line itself, under `run` (design D2), and the duplication is deliberate: the
        /// marker's purpose is to be readable **on its own**, by something that seeks to the end of
        /// a file. Both are rendered from the same
        /// [`EventStream`](crate::EventStream), so they cannot disagree. It is spelled `run_id`
        /// rather than `run` because this payload is flattened into the line's own object and two
        /// keys of one name would be one key.
        run_id: String,
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
            Event::StreamClosed { .. } => "stream.closed",
        }
    }
}

/// Why a run's stream ended (amendment a17).
///
/// Read from the run's own record, never guessed. The one word this workspace has read out of a
/// real terminal record for a budget stop is `budget-exhausted`, which the b10x loop writes; a
/// vendor's word for the same thing that nobody here has read is **not** guessed at, and such a run
/// closes [`CloseReason::Completed`] or [`CloseReason::Error`] on the terminal record's own
/// `is_error` (invariant 3: absence of evidence is not a property).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CloseReason {
    /// The harness ended its own stream and its terminal record reports a finished run.
    Completed,
    /// The terminal record's own word says a budget or turn ceiling stopped it.
    Budget,
    /// The child was killed and no steering command asked for it.
    ///
    /// **No path in this build's run loop reaches this**, and it is in the vocabulary rather than
    /// left out because the alternative is a later producer inventing a sixth word, which is a wire
    /// change under design D3 rather than an addition. It is reachable today from outside the loop:
    /// [`EventStream::close`](crate::EventStream::close) is public, so an embedder that kills a
    /// child itself closes the stream with the word for what it did.
    Killed,
    /// The terminal record reports an error, or there is no terminal record at all.
    ///
    /// The second case is the one worth naming: *the stream is complete and the run is not*. Those
    /// are two different facts and they live in two different fields — this one, and
    /// `session.ended`'s absence.
    Error,
    /// A `halt` steering command ended the run.
    ///
    /// Distinct from [`CloseReason::Killed`] although metaharness kills the child on both: the
    /// reader who has to act on this needs to know **who** ended the run, and here it was the
    /// embedder.
    SteerHalt,
}

impl CloseReason {
    /// The reason's word on the wire.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            CloseReason::Completed => "completed",
            CloseReason::Budget => "budget",
            CloseReason::Killed => "killed",
            CloseReason::Error => "error",
            CloseReason::SteerHalt => "steer-halt",
        }
    }
}

/// What a stream says about its own completeness.
///
/// Three answers and no fourth, because the two failures are different questions to a reader: a
/// stream with no marker was **cut off** (or was written by a build older than amendment a17), and
/// a stream whose marker does not add up was **written by something that got its own count wrong**.
/// Folding them together would send the wrong person looking.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamCompleteness {
    /// A marker is present, is the last event, and counts exactly the events before it.
    Complete {
        /// How many events preceded the marker.
        events: u64,
        /// Why the stream ended.
        reason: CloseReason,
    },
    /// A marker is present and something about it does not hold.
    Inconsistent {
        /// What did not hold, in the words a reader needs to act on it.
        detail: String,
        /// The reason the marker stated, which is still the producer's own claim.
        reason: CloseReason,
    },
    /// **No marker at all.** The stream was cut off, or its producer never promised to close it.
    /// Never read as *complete*: that is the reading amendment a17 exists to end.
    Truncated,
}

impl StreamCompleteness {
    /// Whether this stream closed itself and the marker adds up.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        matches!(self, StreamCompleteness::Complete { .. })
    }

    /// How many events preceded the marker, where there is one that adds up.
    #[must_use]
    pub fn events(&self) -> Option<u64> {
        match self {
            StreamCompleteness::Complete { events, .. } => Some(*events),
            _ => None,
        }
    }

    /// The reason the marker stated, where there is a marker.
    #[must_use]
    pub fn reason(&self) -> Option<CloseReason> {
        match self {
            StreamCompleteness::Complete { reason, .. }
            | StreamCompleteness::Inconsistent { reason, .. } => Some(*reason),
            StreamCompleteness::Truncated => None,
        }
    }

    /// One line a report prints, saying which of the three this is.
    #[must_use]
    pub fn render(&self) -> String {
        match self {
            StreamCompleteness::Complete { events, reason } => format!(
                "stream: complete — {events} event(s) before the marker, reason {}",
                reason.as_str()
            ),
            StreamCompleteness::Inconsistent { detail, reason } => format!(
                "stream: INCONSISTENT — a closing marker says {} and {detail}",
                reason.as_str()
            ),
            StreamCompleteness::Truncated => "stream: TRUNCATED — no `stream.closed` marker, so \
                                              nothing in it can tell an absence from a cut-off \
                                              file"
                .to_string(),
        }
    }
}

/// Read a stream's own account of whether it is whole.
///
/// **Verified, never restated.** The marker's count is compared against the events actually read
/// and its position against the end of the stream, so `Complete` is a fact about these bytes rather
/// than a field somebody wrote. Takes an iterator so one implementation serves both a `&[Event]`
/// and a stream of framed lines — two readers of one rule cannot disagree if there is only one.
pub fn stream_completeness<'a>(events: impl IntoIterator<Item = &'a Event>) -> StreamCompleteness {
    let mut total: u64 = 0;
    let mut markers: Vec<(u64, u64, CloseReason)> = Vec::new();
    for event in events {
        total += 1;
        if let Event::StreamClosed {
            events: stated,
            reason,
            ..
        } = event
        {
            markers.push((total, *stated, *reason));
        }
    }

    let Some(&(position, stated, reason)) = markers.first() else {
        return StreamCompleteness::Truncated;
    };
    if markers.len() > 1 {
        return StreamCompleteness::Inconsistent {
            detail: format!("there are {} of them; a stream ends once", markers.len()),
            reason,
        };
    }
    if position != total {
        return StreamCompleteness::Inconsistent {
            detail: format!(
                "it is event {position} of {total}; the marker is the last line or the stream was \
                 appended to afterwards"
            ),
            reason,
        };
    }
    if stated != position - 1 {
        return StreamCompleteness::Inconsistent {
            detail: format!(
                "it counts {stated} preceding event(s) and there are {}",
                position - 1
            ),
            reason,
        };
    }
    StreamCompleteness::Complete {
        events: stated,
        reason,
    }
}
