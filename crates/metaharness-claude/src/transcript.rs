//! The `stream-json` transcript, read one line at a time.
//!
//! **Total by construction.** Every vendor record becomes at least one protocol event, and a
//! record this reader could not map becomes [`Event::Opaque`] carrying the vendor's own `type`,
//! its `subtype`, a digest of the raw line and its 1-based source line. Nothing is dropped
//! (design D4, § 8.4 O2). The rule that makes it total is one line of code rather than a promise:
//! a recognised record that produced no event gets an `Opaque` anyway, *"because an event that
//! produced nothing has vanished whatever the intention was"*.
//!
//! **Unknown fields are ignored in silence** (O3). A reader that refused a transcript for
//! carrying a new key is a reader that stops working on the next patch release, and it fails in
//! the worst available way.
//!
//! **No clock is read.** `at` is the timestamp the vendor recorded on that record, passed
//! through, or `None` — [`Emission::at`] and [`Emission::untimed`] are the only two ways to make
//! one, so an adapter with no vendor timestamp cannot invent one without the omission being
//! visible (design D2).

use metaharness_protocol::{
    DecisionCensus, Digest, Emission, Event, HermeticAttestation, McpServerRef, PermissionDenial,
    PluginRef, RateLimitInfo, Seam, TranscriptRef, Usage, warning_code,
};
use std::fmt::Write as _;

use serde_json::{Map, Value};

use crate::ADAPTER_ID;

/// One vendor record, already parsed.
type Record = Map<String, Value>;

/// The vendor's own words for a credential that aged out, read from the 2.1.239 binary.
///
/// **Detection is by the vendor's message text and is therefore weak.** It is matched
/// case-insensitively against the text-bearing fields of a record, it is unverified against a
/// real expiry, and it is recorded as such under **Q13**. A run's outcome must not depend on it
/// alone: the terminal record is still [`Event::SessionEnded`], and this event exists so an
/// embedder can tell *the credential aged out* from *the model failed* and retry deterministically
/// (design § 8.1 H6 as amended).
const AUTH_EXPIRY_PHRASES: [&str; 4] = [
    "session has expired",
    "session expired",
    "failed to refresh access token",
    "please run /login",
];

/// Reads a Claude Code `stream-json` transcript into protocol events.
#[derive(Debug, Clone)]
pub struct TranscriptReader {
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
    seam: Seam,
    census: DecisionCensus,
    inputs_digest: Option<Digest>,
    credential_source: Option<String>,
    line: u64,
    terminal_record_seen: bool,
}

impl TranscriptReader {
    /// A reader over these retained bytes, carrying this attestation into `session.started`.
    ///
    /// The attestation is metaharness's own claim and **not** evidence; it sits in the opening
    /// event beside the vendor's own record so a reader can notice when the two disagree
    /// (design § 8.3).
    ///
    /// The seam starts as [`Seam::None`] — *"no seam covered this call"* — because a reader that
    /// claimed the hook by default would claim a seam nobody installed. [`Self::with_seam`] is
    /// how the launch's seam is stamped.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation) -> Self {
        Self {
            transcript,
            attestation,
            seam: Seam::None,
            census: DecisionCensus::default(),
            inputs_digest: None,
            credential_source: None,
            line: 0,
            terminal_record_seen: false,
        }
    }

    /// The seam that will carry decisions, stamped onto every `tool.requested` this reader emits.
    #[must_use]
    pub fn with_seam(mut self, seam: Seam) -> Self {
        self.seam = seam;
        self
    }

    /// The digest of the copied input tree, stamped onto `session.started` — the evidence for
    /// H10, which is the row that stops governing documents moving under a run.
    ///
    /// It is the same value the launch was planned with
    /// ([`crate::LaunchContext::inputs_digest`]), and the caller wires the two together: a
    /// digest that reached the attestation and not the event would leave H10 asserted in
    /// metaharness's own claim and unreadable in the run's record, which is the one place § 8.3
    /// says the evidence lives.
    #[must_use]
    pub fn with_inputs_digest(mut self, digest: Digest) -> Self {
        self.inputs_digest = Some(digest);
        self
    }

    /// The census the reader stamps onto `session.ended`; the caller owns the counting.
    ///
    /// Set it before the vendor's terminal record arrives: `session.ended` is emitted the moment
    /// that record is pushed, and a census that landed afterwards would be a census nothing
    /// carried.
    pub fn set_census(&mut self, census: DecisionCensus) {
        self.census = census;
    }

    /// Whether the vendor's terminal record has been read.
    ///
    /// `false` at end of stream is design § 9.4's *"nobody found out"* — exit `3`, not exit `1`.
    /// A crashed harness is not a failing run, and this reader will not invent a terminal record
    /// to make the two look alike.
    #[must_use]
    pub fn saw_terminal_record(&self) -> bool {
        self.terminal_record_seen
    }

    /// One transcript line in, zero or more events out. 1-based line numbers.
    pub fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.line += 1;
        let source_line = self.line;
        if line.trim().is_empty() {
            // Not a record: no `type` to name, no bytes to preserve, and an `opaque` for it would
            // be an event about a line break.
            return Vec::new();
        }
        let Ok(Value::Object(record)) = serde_json::from_str::<Value>(line) else {
            return vec![Emission::untimed(opaque(line, None, None, source_line))];
        };

        let at = str_field(&record, "timestamp");
        if control_plane(&record) {
            // Recognised and owed nothing: see [`control_plane`]. Not `opaque`, because an opaque
            // record is one this reader could not name, and it turns every absence row `unk`.
            return Vec::new();
        }
        let mut events = self.auth_expiry(&record, source_line);
        events.extend(self.map_record(&record, source_line));
        if events.is_empty() {
            events.push(opaque(
                line,
                str_field(&record, "type"),
                str_field(&record, "subtype"),
                source_line,
            ));
        }
        events
            .into_iter()
            .map(|event| match &at {
                Some(timestamp) => Emission::at(timestamp.clone(), event),
                None => Emission::untimed(event),
            })
            .collect()
    }

    /// Anything owed at end of stream.
    ///
    /// Nothing, always, and the empty vector is the point: every record was converted as it
    /// arrived, and *"the harness died without producing a record"* is a verdict about the run
    /// (design § 9.4, exit `3`) rather than an event this reader may invent. Ask
    /// [`Self::saw_terminal_record`] for that instead.
    #[allow(
        clippy::unused_self,
        reason = "the contract is a reader method, not a free function"
    )]
    pub fn finish(&mut self) -> Vec<Emission> {
        Vec::new()
    }

    /// Which events one recognised record becomes. An empty answer means `opaque`.
    ///
    /// The vendor's own bookkeeping records are answered before this is asked, by
    /// [`control_plane`], and never reach it.
    fn map_record(&mut self, record: &Record, source_line: u64) -> Vec<Event> {
        match str_field(record, "type").as_deref() {
            Some("system") => match str_field(record, "subtype").as_deref() {
                Some("init") => vec![self.session_started(record)],
                Some("thinking_tokens") => thinking_estimate(record),
                Some("api_retry") => vec![api_retry(record)],
                Some("background_tasks_changed") => vec![background_tasks(record)],
                _ => Vec::new(),
            },
            Some("assistant") => self.assistant(record, source_line),
            Some("user") => user(record, source_line),
            Some("rate_limit_event") => rate_limit(record),
            Some("result") => {
                self.terminal_record_seen = true;
                vec![self.session_ended(record)]
            }
            _ => Vec::new(),
        }
    }

    /// The opening record: the only place that can distinguish *offered* from *called*.
    fn session_started(&mut self, record: &Record) -> Event {
        let credential_source = str_field(record, "apiKeySource");
        self.credential_source.clone_from(&credential_source);
        Event::SessionStarted {
            // Left to the run loop, which holds this adapter's published rendering: turning a
            // vendor's tool list into operations is that table's job and an adapter that kept its
            // own copy would be a second owner of one rule (design § 8.4 O6).
            available_operations: None,
            // The vendor states no such thing: Claude Code's opening record lists the tools it
            // has and never a tool it wanted and could not have. `None` is *it did not say*, and
            // an empty list here would be this adapter asserting the machine admitted everything.
            withheld: None,
            adapter: ADAPTER_ID.to_string(),
            adapter_class: "harness".to_string(),
            harness_version: str_field(record, "claude_code_version"),
            session_id: str_field(record, "session_id"),
            model: str_field(record, "model"),
            permission_mode: str_field(record, "permissionMode"),
            credential_source,
            output_style: str_field(record, "output_style"),
            cwd: str_field(record, "cwd"),
            offered_tools: str_list(record, "tools"),
            slash_commands: str_list(record, "slash_commands"),
            skills: str_list(record, "skills"),
            agents: str_list(record, "agents"),
            plugins: plugins(record),
            mcp_servers: mcp_servers(record),
            inputs_digest: self.inputs_digest.clone(),
            transcript: self.transcript.clone(),
            hermetic: self.attestation.clone(),
        }
    }

    /// The terminal record: the source of every resource fact.
    fn session_ended(&self, record: &Record) -> Event {
        Event::SessionEnded {
            is_error: record.get("is_error").and_then(Value::as_bool),
            subtype: str_field(record, "subtype"),
            stop_reason: str_field(record, "stop_reason"),
            terminal_reason: str_field(record, "terminal_reason"),
            api_error_status: str_field(record, "api_error_status"),
            num_turns: u64_field(record, "num_turns"),
            duration_ms: u64_field(record, "duration_ms"),
            duration_api_ms: u64_field(record, "duration_api_ms"),
            ttft_ms: u64_field(record, "ttft_ms"),
            time_to_request_ms: u64_field(record, "time_to_request_ms"),
            total_cost_usd: record.get("total_cost_usd").and_then(Value::as_f64),
            // Passed through, and metaharness never adds to it: its own per-call denial audit is
            // `tool.decided`, and mixing the two would guarantee a disagreement (finding F10).
            permission_denials: permission_denials(record),
            subagents_spawned: record
                .get("subagent_stats")
                .and_then(|stats| stats.get("spawned"))
                .and_then(Value::as_u64),
            usage: record.get("usage").map(usage),
            model_usage: model_usage(record),
            census: self.census.clone(),
        }
    }

    /// One assistant record: its content blocks in order, then one `usage` event.
    fn assistant(&self, record: &Record, source_line: u64) -> Vec<Event> {
        let Some(message) = record.get("message").and_then(Value::as_object) else {
            return Vec::new();
        };
        let request_id = str_field(record, "request_id");
        let mut events = Vec::new();
        for block in blocks(message) {
            events.push(self.content_block(block, request_id.as_ref(), source_line));
        }
        if let Some(reported) = message.get("usage") {
            events.push(Event::Usage {
                request_id,
                model: str_field(message, "model"),
                usage: usage(reported),
            });
        }
        events
    }

    /// One content block. An unrecognised block is `opaque` rather than skipped: a block that
    /// produced nothing has vanished as surely as a record that did.
    fn content_block(&self, block: &Value, request_id: Option<&String>, source_line: u64) -> Event {
        let kind = block
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match kind {
            "text" => Event::Text {
                text: text_of(block, "text"),
                request_id: request_id.cloned(),
            },
            "thinking" => Event::Thinking {
                text: text_of(block, "thinking"),
                request_id: request_id.cloned(),
            },
            "tool_use" => Event::ToolRequested {
                // Left empty here on purpose, exactly as `operations` is: what a call touches is
                // resolved by whoever holds the run's published rendering, and an adapter that
                // answered for itself would be a second owner of one rule (design § 8.4 O6).
                subjects: Vec::new(),
                // Left empty here on purpose: the resolution needs the adapter\'s *published*
                // rendering, which the loop holds and an adapter must not (design § 8.4 O6).
                operations: Vec::new(),
                call_id: block
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                name: block
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                input: block.get("input").cloned().unwrap_or(Value::Null),
                // The record this reader is holding is a call the vendor already dispatched, so
                // nothing about it can block. The *blocking* `tool.requested` is the seam's, and
                // its `deadline_ms` is armed when the embedder is handed it (design § 7.7 rule 5).
                decision_required: false,
                deadline_ms: None,
                seam: self.seam,
            },
            _ => opaque_value(block, Some("assistant"), Some(kind), source_line),
        }
    }

    /// The credential aged out mid-run, in the vendor's own words.
    fn auth_expiry(&self, record: &Record, source_line: u64) -> Vec<Event> {
        match auth_expiry_detail(record) {
            Some(detail) => vec![Event::AuthExpired {
                credential_source: self.credential_source.clone(),
                detail: Some(detail),
                source_line: Some(source_line),
            }],
            None => Vec::new(),
        }
    }
}

/// The records the vendor writes about its **own bookkeeping**, which carry no fact any
/// expectation reads and are recognised so that they are not `opaque`.
///
/// Four shapes, all of them the lifecycle of a sub-agent task — `system/task_started`,
/// `system/task_progress`, `system/task_notification`, `system/task_updated` — plus a top-level
/// `tool_progress` heartbeat while a tool runs. The call each one narrates is already on the wire:
/// the `Agent` call is a `tool.requested`, and what the sub-agent produced arrives in that call's
/// `tool.result`, so these add a second account of the same thing and nothing more.
///
/// **The test each candidate has to pass** is whether the fact is stated elsewhere in the same
/// stream, and it is a test a shape can fail. 0.6.2 added `system/background_tasks_changed` here on
/// the same reading and was wrong: the vendor's schema calls it *"every live background task after
/// the change"* with REPLACE semantics, emitted on completion and kill as well as start, and only
/// the start has a `tool.result` of its own. It is read by `background_tasks` instead. A list whose
/// entries are not checked against the vendor's own schema is a list that reintroduces the failure
/// it exists to prevent.
///
/// Named one by one — a record this list does not name still goes `opaque` — so that a shape a
/// later release adds is met the way D4 requires and not waved through with these.
///
/// Why it matters that they are recognised rather than left opaque: one recorded run on 2026-09-03
/// carried 183 of them, and every `tool.absent` row in the checker that read it came back `unk`
/// with *"183 events the adapter could not read"*. The checker was right to, and the fix belongs
/// here, where the record's shape is known.
fn control_plane(record: &Record) -> bool {
    match str_field(record, "type").as_deref() {
        Some("tool_progress") => true,
        Some("system") => matches!(
            str_field(record, "subtype").as_deref(),
            Some("task_started" | "task_progress" | "task_notification" | "task_updated")
        ),
        _ => false,
    }
}

/// The harness's live estimate, never the billed figure.
fn thinking_estimate(record: &Record) -> Vec<Event> {
    let Some(estimate) = u64_field(record, "estimated_tokens") else {
        return Vec::new();
    };
    vec![Event::ThinkingEstimate {
        estimate,
        delta: record.get("estimated_tokens_delta").and_then(Value::as_i64),
    }]
}

/// The vendor retried a request against its own API and said so.
///
/// A [`Event::Warning`] and not a drop, and not `opaque`. Dropped it would lose the only account of
/// why a run stalled: on 2026-09-03 a recording carried 10 of these in 210 seconds before a
/// `529 Overloaded` ended the session at turn 1, and without them the stream is a gap. Left
/// `opaque` it would decide plan questions it knows nothing about — the recording before that one
/// had a gate row come back `undecidable` over two records of a different bookkeeping shape.
///
/// `warning` is control plane on the consuming side too, so the fact reaches a reader without
/// standing between the checker and a row.
fn api_retry(record: &Record) -> Event {
    let mut message = "the vendor retried its API call".to_owned();
    if let Some(status) = u64_field(record, "error_status") {
        let _ = write!(message, " after HTTP {status}");
    }
    if let Some(reason) = retry_reason(record) {
        let _ = write!(message, ": {reason}");
    }
    if let Some(attempt) = u64_field(record, "attempt") {
        let _ = write!(message, " (attempt {attempt}");
        if let Some(of) = u64_field(record, "max_retries") {
            let _ = write!(message, " of {of}");
        }
        if let Some(delay) = u64_field(record, "retry_delay_ms") {
            let _ = write!(message, ", after {delay}ms");
        }
        message.push(')');
    }
    Event::Warning {
        code: warning_code::VENDOR_API_RETRY.to_owned(),
        message,
    }
}

/// The vendor's own words for why it retried, out of a field whose type the schema does not fix.
///
/// `error` is an object in the 2.1.259 schema, not a string, and the useful part of it is
/// `message`. Both shapes are read because a field a schema declares as an object today is one a
/// patch release can flatten, and a reader that took only the object would report a retry with no
/// reason on the release that flattened it.
fn retry_reason(record: &Record) -> Option<String> {
    match record.get("error") {
        Some(Value::String(text)) if !text.is_empty() => Some(text.clone()),
        Some(Value::Object(error)) => error
            .get("message")
            .and_then(Value::as_str)
            .filter(|text| !text.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}

/// The full set of live background tasks, restated because one of them changed.
///
/// **Not dropped, and this is where 0.6.2 was wrong.** That release put
/// `system/background_tasks_changed` in [`control_plane`] on the reading that it is a second telling
/// of a call already on the wire. The vendor's own 2.1.259 schema says otherwise: the payload is
/// *"every live background task after the change"* with **REPLACE semantics**, emitted *"whenever
/// membership changes (start, completion, kill, a foreground agent being backgrounded) or an
/// entry's `ambient` flag flips"*.
///
/// Only the start is on the wire elsewhere. The `Bash` call that launched a background task gets
/// its `tool.result` when the shell hands back an id, not when the task ends — so a completion, a
/// kill and an `ambient` flip are stated **here and nowhere else**, and dropping them let a checker
/// read absence as fact. That is the failure D4 exists to prevent, arrived at by a list that was
/// meant to prevent it.
///
/// A [`Event::Warning`] for the same reason [`api_retry`] is one: the fact reaches a reader, and
/// `warning` is control plane on the consuming side so it does not stand between a checker and a
/// row.
fn background_tasks(record: &Record) -> Event {
    let tasks = record.get("tasks").and_then(Value::as_array);
    let count = tasks.map_or(0, Vec::len);
    let named: Vec<String> = tasks
        .map(|tasks| {
            tasks
                .iter()
                .filter_map(|task| task.get("task_id").and_then(Value::as_str))
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let mut message = format!("the vendor's live background task set changed: {count} live");
    if !named.is_empty() {
        let _ = write!(message, " ({})", named.join(", "));
    }
    Event::Warning {
        code: warning_code::VENDOR_BACKGROUND_TASKS.to_owned(),
        message,
    }
}

/// A rate-limit window: a billing guard, because *this run must not have been paid for out of
/// overage* is a fact about money nothing else carries.
fn rate_limit(record: &Record) -> Vec<Event> {
    let Some(info) = record.get("rate_limit_info").and_then(Value::as_object) else {
        return Vec::new();
    };
    vec![Event::RateLimit {
        info: RateLimitInfo {
            status: str_field(info, "status"),
            window: str_field(info, "rateLimitType"),
            resets_at: info.get("resetsAt").and_then(Value::as_i64),
            utilization: info.get("utilization").and_then(Value::as_f64),
            using_overage: info.get("isUsingOverage").and_then(Value::as_bool),
        },
    }]
}

/// One user record: tool results, and text the *harness* put in the conversation.
fn user(record: &Record, source_line: u64) -> Vec<Event> {
    let Some(message) = record.get("message").and_then(Value::as_object) else {
        return Vec::new();
    };
    let origin = if record
        .get("isSynthetic")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        "synthetic"
    } else {
        "user"
    };
    // The per-tool result record is a sibling of the *record*, not of the block, so it is read
    // here and handed down. Claude Code at 2.1.240 writes one `tool_result` block per user
    // record; where a record carried more, the sibling is carried onto each rather than dropped,
    // which is what the `stream-json` reader in `AEP` does with the same bytes.
    // Two readers of one run disagreeing about it would be worse than either answer.
    let tool_use_result = record
        .get("tool_use_result")
        .filter(|value| !value.is_null());
    match message.get("content") {
        Some(Value::String(text)) => vec![Event::Injection {
            text: text.clone(),
            origin: Some(origin.to_string()),
        }],
        Some(Value::Array(items)) => items
            .iter()
            .map(|block| user_block(block, origin, tool_use_result, source_line))
            .collect(),
        _ => Vec::new(),
    }
}

fn user_block(
    block: &Value,
    origin: &str,
    tool_use_result: Option<&Value>,
    source_line: u64,
) -> Event {
    let kind = block
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    match kind {
        "tool_result" => {
            let content = block.get("content").cloned();
            Event::ToolResult {
                call_id: block
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                is_error: block.get("is_error").and_then(Value::as_bool),
                bytes: content.as_ref().map(byte_count),
                content,
                tool_use_result: tool_use_result.cloned(),
            }
        }
        "text" => Event::Injection {
            text: text_of(block, "text"),
            origin: Some(origin.to_string()),
        },
        _ => opaque_value(block, Some("user"), Some(kind), source_line),
    }
}

/// How many bytes of content the vendor recorded.
///
/// A string is its own UTF-8 length; anything else is the length of the JSON the vendor wrote.
/// Counted rather than estimated, because a byte count is the one tool-result fact that does not
/// depend on reading the content.
fn byte_count(content: &Value) -> u64 {
    let len = match content {
        Value::String(text) => text.len(),
        other => serde_json::to_string(other).map_or(0, |json| json.len()),
    };
    len as u64
}

/// The vendor's own terminal denial list, passed through unchanged.
fn permission_denials(record: &Record) -> Option<Vec<PermissionDenial>> {
    let entries = record.get("permission_denials")?.as_array()?;
    Some(
        entries
            .iter()
            .map(|entry| PermissionDenial {
                tool_name: entry
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                tool_use_id: entry
                    .get("tool_use_id")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                tool_input: entry.get("tool_input").cloned(),
            })
            .collect(),
    )
}

/// Tokens, as the vendor reported them, never computed.
///
/// Three of the figures arrive only on the terminal record's `usage` and not on a per-request one
/// — 2.1.240 writes `output_tokens_details`, `iterations` and `speed` in `result.usage` and none
/// of them in an assistant message's — so a `usage` event carries them absent. That is the record
/// being honest about which question it can answer, not a gap in this reader (amendment a9).
fn usage(value: &Value) -> Usage {
    Usage {
        input_tokens: value.get("input_tokens").and_then(Value::as_u64),
        output_tokens: value.get("output_tokens").and_then(Value::as_u64),
        cache_read_input_tokens: value.get("cache_read_input_tokens").and_then(Value::as_u64),
        cache_creation_input_tokens: value
            .get("cache_creation_input_tokens")
            .and_then(Value::as_u64),
        service_tier: value
            .get("service_tier")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        // The API's own breakdown, which is the billed figure. The `system`/`thinking_tokens`
        // record this adapter also reads is an *estimate* and goes to `thinking.estimate`; the
        // two must never be filled from each other.
        thinking_tokens: value
            .get("output_tokens_details")
            .and_then(|details| details.get("thinking_tokens"))
            .and_then(Value::as_u64),
        // The length of the vendor's own array. An absent array is `None` and an empty one is
        // `Some(0)`, because *the record did not say* and *the record says none* are different
        // answers all the way to a verdict.
        iterations: value
            .get("iterations")
            .and_then(Value::as_array)
            .map(|records| records.len() as u64),
        speed: value
            .get("speed")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        // The vendor prices a model, not a `usage` block: `total_cost_usd` is the run's and
        // `modelUsage[…].costUSD` is the model's. Neither belongs here, and multiplying the
        // tokens out would invent a third figure that can disagree with both.
        cost_usd: None,
    }
}

/// The per-model split, whose keys the vendor spells differently from the per-request ones.
///
/// `service_tier`, `thinking_tokens`, `iterations` and `speed` have no counterpart here, so they
/// stay `None` rather than being filled from the aggregate — a figure carried over from another
/// record is a figure this model never reported.
///
/// `costUSD` is the one place Claude Code prices a slice of a run smaller than the whole, and it
/// is the reason a cost scoped to one model can be asserted at all (amendment a9). It is read,
/// never derived: a per-model cost this reader multiplied out of tokens and a rate card would be
/// a number nobody billed.
fn model_usage(record: &Record) -> Option<std::collections::BTreeMap<String, Usage>> {
    let entries = record.get("modelUsage")?.as_object()?;
    Some(
        entries
            .iter()
            .map(|(model, value)| {
                (
                    model.clone(),
                    Usage {
                        input_tokens: value.get("inputTokens").and_then(Value::as_u64),
                        output_tokens: value.get("outputTokens").and_then(Value::as_u64),
                        cache_read_input_tokens: value
                            .get("cacheReadInputTokens")
                            .and_then(Value::as_u64),
                        cache_creation_input_tokens: value
                            .get("cacheCreationInputTokens")
                            .and_then(Value::as_u64),
                        service_tier: None,
                        thinking_tokens: None,
                        iterations: None,
                        speed: None,
                        cost_usd: value.get("costUSD").and_then(Value::as_f64),
                    },
                )
            })
            .collect(),
    )
}

/// The plugins the harness loaded — the evidence for H1a.
///
/// The vendor also records each plugin's `path`; [`PluginRef`] has no field for it, so it is
/// ignored in silence like any other unrecognised field (design § 8.4 O3).
fn plugins(record: &Record) -> Option<Vec<PluginRef>> {
    let entries = record.get("plugins")?.as_array()?;
    Some(
        entries
            .iter()
            .map(|entry| PluginRef {
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                source: entry
                    .get("source")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                version: entry
                    .get("version")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            })
            .collect(),
    )
}

/// The MCP servers, as a **list** — the evidence for H5.
///
/// A missing key is `None`, which is `unk`, and **never zero**: a bound that read a missing field
/// as zero would report its blindest case as its best one (design § 2.1). An empty array is a
/// real zero and is kept as one.
fn mcp_servers(record: &Record) -> Option<Vec<McpServerRef>> {
    let entries = record.get("mcp_servers")?.as_array()?;
    Some(
        entries
            .iter()
            .map(|entry| McpServerRef {
                name: entry
                    .get("name")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                status: entry
                    .get("status")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
            })
            .collect(),
    )
}

/// The vendor's own words, where one of [`AUTH_EXPIRY_PHRASES`] appears in them.
fn auth_expiry_detail(record: &Record) -> Option<String> {
    candidate_texts(record).into_iter().find(|text| {
        let lowered = text.to_lowercase();
        AUTH_EXPIRY_PHRASES
            .iter()
            .any(|phrase| lowered.contains(phrase))
    })
}

/// The text-bearing fields a diagnosis could be written in.
fn candidate_texts(record: &Record) -> Vec<String> {
    let mut texts = Vec::new();
    for key in ["result", "error"] {
        if let Some(text) = str_field(record, key) {
            texts.push(text);
        }
    }
    if let Some(message) = record.get("error").and_then(|error| error.get("message"))
        && let Some(text) = message.as_str()
    {
        texts.push(text.to_string());
    }
    if let Some(message) = record.get("message").and_then(Value::as_object) {
        for block in blocks(message) {
            for key in ["text", "content"] {
                if let Some(text) = block.get(key).and_then(Value::as_str) {
                    texts.push(text.to_string());
                }
            }
        }
    }
    texts
}

/// The content blocks of a message, or none.
fn blocks(message: &Record) -> &[Value] {
    message
        .get("content")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn opaque(
    raw: &str,
    vendor_type: Option<String>,
    vendor_subtype: Option<String>,
    line: u64,
) -> Event {
    Event::Opaque {
        vendor_type,
        vendor_subtype,
        digest: Digest::of(raw.as_bytes()),
        source_line: Some(line),
    }
}

fn opaque_value(
    value: &Value,
    vendor_type: Option<&str>,
    vendor_subtype: Option<&str>,
    line: u64,
) -> Event {
    let raw = serde_json::to_string(value).unwrap_or_default();
    opaque(
        &raw,
        vendor_type.map(ToString::to_string),
        vendor_subtype
            .filter(|kind| !kind.is_empty())
            .map(ToString::to_string),
        line,
    )
}

fn str_field(record: &Record, key: &str) -> Option<String> {
    record
        .get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn u64_field(record: &Record, key: &str) -> Option<u64> {
    record.get(key).and_then(Value::as_u64)
}

fn str_list(record: &Record, key: &str) -> Option<Vec<String>> {
    let entries = record.get(key)?.as_array()?;
    Some(
        entries
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
    )
}

fn text_of(block: &Value, key: &str) -> String {
    block
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::HermeticMode;

    fn new_reader() -> TranscriptReader {
        TranscriptReader::new(
            TranscriptRef {
                path: Some("/scratch/run-1/transcript.jsonl".to_string()),
                digest: Some(Digest::of(b"transcript")),
                bytes: Some(10),
            },
            HermeticAttestation::none(HermeticMode::Strict),
        )
        .with_seam(Seam::Hook)
    }

    fn only(events: Vec<Emission>) -> Event {
        assert_eq!(events.len(), 1, "expected exactly one event");
        events.into_iter().next().expect("one event").event
    }

    /// The failure that costs the most is a checker reporting "the tool was never called" when
    /// what happened is that it stopped being able to see tool calls (design D4).
    #[test]
    fn an_unknown_record_becomes_opaque_and_is_never_dropped() {
        let mut reader = new_reader();
        let event = only(reader.push_line(r#"{"type":"invented_by_a_later_release","x":1}"#));
        let Event::Opaque {
            vendor_type,
            vendor_subtype,
            source_line,
            ..
        } = event
        else {
            panic!("expected opaque");
        };
        assert_eq!(vendor_type.as_deref(), Some("invented_by_a_later_release"));
        assert_eq!(vendor_subtype, None);
        assert_eq!(source_line, Some(1));
    }

    /// A record whose envelope was recognised and whose body was not is `opaque` too, because an
    /// event that produced nothing has vanished whatever the intention was.
    #[test]
    fn a_recognised_envelope_with_an_unreadable_body_is_opaque_too() {
        let mut reader = new_reader();
        let event = only(reader.push_line(r#"{"type":"assistant","session_id":"s"}"#));
        assert!(matches!(event, Event::Opaque { .. }));
        let event = only(reader.push_line(r#"{"type":"system","subtype":"unheard_of"}"#));
        let Event::Opaque { vendor_subtype, .. } = event else {
            panic!("expected opaque");
        };
        assert_eq!(vendor_subtype.as_deref(), Some("unheard_of"));
    }

    /// Claude Code 2.1.259's sub-agent task lifecycle and tool-progress heartbeats are the vendor's
    /// own bookkeeping: recognised, owed no event, and **not** opaque — one recorded run carried 183
    /// of them and every absence row read over it came back `unk`.
    #[test]
    fn the_vendors_bookkeeping_records_are_recognised_and_emit_nothing() {
        let mut reader = new_reader();
        for line in [
            r#"{"type":"system","subtype":"task_started","task_id":"t1","description":"scope it"}"#,
            r#"{"type":"system","subtype":"task_progress","task_id":"t1","usage":{"total_tokens":10}}"#,
            r#"{"type":"system","subtype":"task_notification","task_id":"t1","status":"completed"}"#,
            r#"{"type":"system","subtype":"task_updated","task_id":"t1","description":"renamed"}"#,
            r#"{"type":"tool_progress","tool_use_id":"toolu_1","elapsed_time_seconds":3}"#,
        ] {
            assert!(
                reader.push_line(line).is_empty(),
                "a bookkeeping record emits nothing and is not opaque: {line}"
            );
        }
        // The list is closed: a subtype it does not name is still met the way D4 requires.
        let event = only(reader.push_line(r#"{"type":"system","subtype":"task_invented_later"}"#));
        assert!(matches!(event, Event::Opaque { .. }));
    }

    #[test]
    fn a_background_task_set_is_a_warning_and_is_not_in_the_drop_list() {
        // 0.6.2 dropped this shape and 0.6.4 stopped: the vendor's own 2.1.259 schema calls the
        // payload "every live background task after the change" with REPLACE semantics, emitted on
        // completion and kill as well as start. Only the start has a `tool.result` of its own, so a
        // drop lets a checker read absence as fact.
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"system","subtype":"background_tasks_changed","tasks":[
                {"task_id":"bg_1","task_type":"bash","description":"cargo build"},
                {"task_id":"bg_2","task_type":"bash","description":"npm test","ambient":true}],
                "uuid":"u","session_id":"s"}"#,
        ));
        let Event::Warning { code, message } = event else {
            panic!("expected a warning, not a drop");
        };
        assert_eq!(code, warning_code::VENDOR_BACKGROUND_TASKS);
        assert!(message.contains("2 live"), "{message}");
        assert!(message.contains("bg_1, bg_2"), "{message}");

        // The set emptying is the completion or kill nothing else in the stream states.
        let emptied = only(
            reader
                .push_line(r#"{"type":"system","subtype":"background_tasks_changed","tasks":[]}"#),
        );
        let Event::Warning { message, .. } = emptied else {
            panic!("expected a warning");
        };
        assert!(message.contains("0 live"), "{message}");
    }

    #[test]
    fn a_vendor_api_retry_is_a_warning_and_neither_dropped_nor_opaque() {
        // The third shape this adapter met on a recording, and the one that is not bookkeeping: no
        // other record says the vendor retried, so dropping it would leave a stall unexplained.
        // `warning` is control plane on the consuming side, so keeping it decides no row either.
        // The field names are the vendor's own, read off the 2.1.259 binary's schema:
        // `attempt`, `max_retries`, `retry_delay_ms`, `error_status` and an `error` **object**.
        // 0.6.3 guessed `delayMs` and a string `error`, so every real retry reported no reason and
        // no backoff — a reader that invents a shape and a test that invents the same one agree
        // with each other and with nothing else.
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"system","subtype":"api_retry","attempt":2,"max_retries":10,
                "retry_delay_ms":1500,"error_status":529,
                "error":{"message":"Overloaded"},"uuid":"u","session_id":"s"}"#,
        ));
        let Event::Warning { code, message } = event else {
            panic!("expected a warning");
        };
        assert_eq!(code, warning_code::VENDOR_API_RETRY);
        assert!(message.contains("HTTP 529"), "{message}");
        assert!(message.contains("Overloaded"), "{message}");
        assert!(message.contains("attempt 2 of 10"), "{message}");
        assert!(message.contains("1500ms"), "{message}");

        // A patch release that flattens `error` to a string is still read.
        let flat = only(reader.push_line(
            r#"{"type":"system","subtype":"api_retry","attempt":1,"error":"Overloaded"}"#,
        ));
        let Event::Warning { message, .. } = flat else {
            panic!("expected a warning");
        };
        assert!(message.contains("Overloaded"), "{message}");

        // A record stating none of them is still a warning, not an opaque line.
        let bare = only(reader.push_line(r#"{"type":"system","subtype":"api_retry"}"#));
        assert!(matches!(bare, Event::Warning { .. }));
    }

    #[test]
    fn a_line_that_is_not_json_is_opaque_with_the_digest_of_its_bytes() {
        let mut reader = new_reader();
        let event = only(reader.push_line("this is not json"));
        let Event::Opaque { digest, .. } = event else {
            panic!("expected opaque");
        };
        assert_eq!(digest, Digest::of(b"this is not json"));
    }

    #[test]
    fn source_lines_are_one_based_and_count_blank_lines_too() {
        let mut reader = new_reader();
        assert!(reader.push_line("   ").is_empty());
        let event = only(reader.push_line(r#"{"type":"nope"}"#));
        let Event::Opaque { source_line, .. } = event else {
            panic!("expected opaque");
        };
        assert_eq!(source_line, Some(2));
    }

    #[test]
    fn an_unknown_field_on_a_known_record_is_ignored_in_silence() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":40,
                "estimated_tokens_delta":-2,"a_field_from_the_future":true}"#,
        ));
        assert_eq!(
            event,
            Event::ThinkingEstimate {
                estimate: 40,
                delta: Some(-2)
            }
        );
    }

    /// A server the session cannot authenticate to still exists, is still named, and is still a
    /// reach outside the sandbox — so H5 reads a list (design § 8.1).
    #[test]
    fn a_missing_mcp_list_is_unk_and_an_empty_one_is_a_real_zero() {
        let mut reader = new_reader();
        let event = only(reader.push_line(r#"{"type":"system","subtype":"init","tools":[]}"#));
        let Event::SessionStarted { mcp_servers, .. } = event else {
            panic!("expected session.started");
        };
        assert_eq!(mcp_servers, None);

        let mut reader = new_reader();
        let event = only(
            reader.push_line(r#"{"type":"system","subtype":"init","mcp_servers":[],"tools":[]}"#),
        );
        let Event::SessionStarted { mcp_servers, .. } = event else {
            panic!("expected session.started");
        };
        assert_eq!(mcp_servers, Some(Vec::new()));
    }

    #[test]
    fn the_opening_record_carries_the_attestation_and_the_inputs_digest() {
        let mut reader = new_reader().with_inputs_digest(Digest::of(b"inputs"));
        let event = only(reader.push_line(
            r#"{"type":"system","subtype":"init","cwd":"/scratch/run-1/work",
                "session_id":"s-1","tools":["Read"],"mcp_servers":[],"model":"a-model",
                "permissionMode":"default","slash_commands":[],"apiKeySource":"none",
                "claude_code_version":"2.1.240","output_style":"default","agents":[],
                "skills":[],"plugins":[]}"#,
        ));
        let Event::SessionStarted {
            adapter,
            harness_version,
            credential_source,
            output_style,
            offered_tools,
            inputs_digest,
            transcript,
            ..
        } = event
        else {
            panic!("expected session.started");
        };
        assert_eq!(adapter, ADAPTER_ID);
        assert_eq!(harness_version.as_deref(), Some("2.1.240"));
        assert_eq!(credential_source.as_deref(), Some("none"));
        assert_eq!(output_style.as_deref(), Some("default"));
        assert_eq!(offered_tools, Some(vec!["Read".to_string()]));
        assert_eq!(inputs_digest, Some(Digest::of(b"inputs")));
        assert_eq!(transcript.bytes, Some(10));
    }

    #[test]
    fn an_assistant_record_becomes_its_blocks_in_order_and_then_one_usage_event() {
        let mut reader = new_reader();
        let events = reader.push_line(
            r#"{"type":"assistant","timestamp":"2026-08-22T10:00:00Z","request_id":"req-1",
                "message":{"model":"a-model","role":"assistant","content":[
                  {"type":"thinking","thinking":"weighing it"},
                  {"type":"text","text":"here goes"},
                  {"type":"tool_use","id":"call-1","name":"Bash","input":{"command":"ls"}}],
                 "usage":{"input_tokens":10,"output_tokens":3,"service_tier":"standard"}}}"#,
        );
        let names: Vec<&str> = events.iter().map(|e| e.event.name()).collect();
        assert_eq!(names, ["thinking", "text", "tool.requested", "usage"]);
        assert!(
            events
                .iter()
                .all(|e| e.at.as_deref() == Some("2026-08-22T10:00:00Z"))
        );
        let Event::ToolRequested {
            call_id,
            name,
            decision_required,
            seam,
            ..
        } = &events[2].event
        else {
            panic!("expected tool.requested");
        };
        assert_eq!(call_id, "call-1");
        assert_eq!(name, "Bash");
        assert!(!decision_required);
        assert_eq!(*seam, Seam::Hook);
    }

    /// A content block nobody could read is preserved for the same reason a record is.
    #[test]
    fn an_unknown_content_block_is_opaque_rather_than_skipped() {
        let mut reader = new_reader();
        let events = reader.push_line(
            r#"{"type":"assistant","message":{"content":[{"type":"hologram","payload":1}]}}"#,
        );
        assert_eq!(events.len(), 1);
        let Event::Opaque { vendor_subtype, .. } = &events[0].event else {
            panic!("expected opaque");
        };
        assert_eq!(vendor_subtype.as_deref(), Some("hologram"));
    }

    #[test]
    fn a_user_record_carrying_a_tool_result_becomes_one() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"user","message":{"role":"user","content":[
                {"type":"tool_result","tool_use_id":"call-1","content":"four","is_error":false}]}}"#,
        ));
        assert_eq!(
            event,
            Event::ToolResult {
                call_id: "call-1".to_string(),
                is_error: Some(false),
                content: Some(Value::String("four".to_string())),
                bytes: Some(4),
                tool_use_result: None,
            }
        );
    }

    /// The vendor's per-tool result record is a sibling of the record and is carried verbatim.
    ///
    /// This is the field `skill.completed` reads: `commandName` names the skill and `success`
    /// says whether it finished, and while the seam dropped the sibling the strongest single
    /// claim a checker can make about a step was undecidable for a driven run (amendment a9).
    #[test]
    fn the_vendors_per_tool_result_record_rides_on_the_result_verbatim() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"user","tool_use_result":{"commandName":"verify","success":true},
                "message":{"role":"user","content":[
                  {"type":"tool_result","tool_use_id":"call-1","content":"ran","is_error":false}]}}"#,
        ));
        let Event::ToolResult {
            tool_use_result, ..
        } = event
        else {
            panic!("expected tool.result");
        };
        let recorded = tool_use_result.expect("the sibling is carried");
        assert_eq!(recorded["commandName"], Value::String("verify".to_string()));
        assert_eq!(recorded["success"], Value::Bool(true));
    }

    /// A record without the sibling leaves the field absent, and a `null` one is absence too. A
    /// reader that turned either into an empty object would be offering a result record the
    /// vendor never wrote.
    #[test]
    fn a_result_with_no_sibling_carries_none_and_a_null_sibling_is_absence() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"user","tool_use_result":null,"message":{"role":"user","content":[
                {"type":"tool_result","tool_use_id":"call-1","content":"ran"}]}}"#,
        ));
        let Event::ToolResult {
            tool_use_result, ..
        } = event
        else {
            panic!("expected tool.result");
        };
        assert_eq!(tool_use_result, None);
    }

    /// Text the *harness* put in the conversation is `injection` and not `text`: an assertion
    /// about what the model said must not match what the harness typed for it.
    #[test]
    fn a_synthetic_user_record_is_an_injection_and_says_where_it_came_from() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"user","isSynthetic":true,"message":{"role":"user","content":"the frame"}}"#,
        ));
        assert_eq!(
            event,
            Event::Injection {
                text: "the frame".to_string(),
                origin: Some("synthetic".to_string()),
            }
        );
    }

    #[test]
    fn a_rate_limit_record_carries_the_overage_flag_a_billing_guard_needs() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed",
                "rateLimitType":"five_hour","resetsAt":1770000000,"utilization":0.5,
                "isUsingOverage":false}}"#,
        ));
        let Event::RateLimit { info } = event else {
            panic!("expected rate_limit");
        };
        assert_eq!(info.window.as_deref(), Some("five_hour"));
        assert_eq!(info.using_overage, Some(false));
    }

    #[test]
    fn the_terminal_record_passes_the_vendors_denial_list_through_unchanged() {
        let mut reader = new_reader();
        reader.set_census(DecisionCensus {
            allowed: 2,
            denied: 1,
            ..DecisionCensus::default()
        });
        let event = only(reader.push_line(
            r#"{"type":"result","subtype":"success","is_error":false,"num_turns":3,
                "duration_ms":1200,"duration_api_ms":900,"ttft_ms":300,"time_to_request_ms":50,
                "total_cost_usd":0.01,"subagent_stats":{"spawned":0},
                "permission_denials":[{"tool_name":"Write","tool_use_id":"call-9",
                  "tool_input":{"file_path":"/x"}}],
                "usage":{"input_tokens":10,"output_tokens":2},
                "modelUsage":{"a-model":{"inputTokens":10,"outputTokens":2}}}"#,
        ));
        let Event::SessionEnded {
            permission_denials,
            census,
            subagents_spawned,
            ttft_ms,
            model_usage,
            ..
        } = event
        else {
            panic!("expected session.ended");
        };
        let denials = permission_denials.expect("the list is passed through");
        assert_eq!(denials.len(), 1);
        assert_eq!(denials[0].tool_name.as_deref(), Some("Write"));
        assert_eq!(census.denied, 1);
        assert_eq!(subagents_spawned, Some(0));
        assert_eq!(ttft_ms, Some(300));
        assert_eq!(
            model_usage.expect("per-model split")["a-model"].input_tokens,
            Some(10)
        );
        assert!(reader.saw_terminal_record());
    }

    /// The terminal record's three extra usage figures and the per-model price, all read and none
    /// derived (amendment a9).
    ///
    /// `iterations` is the **length** of the vendor's array, `thinking_tokens` comes from the
    /// API's `output_tokens_details` breakdown and never from the mid-stream estimate, and the
    /// dollar figure sits on the model the vendor priced — not on the aggregate, where the vendor
    /// prices nothing.
    #[test]
    fn the_terminal_record_carries_the_billed_thinking_tokens_the_iteration_count_and_the_speed() {
        let mut reader = new_reader();
        let event = only(reader.push_line(
            r#"{"type":"result","subtype":"success","is_error":false,"total_cost_usd":0.24,
                "usage":{"input_tokens":4,"output_tokens":96,"cache_read_input_tokens":22308,
                  "cache_creation_input_tokens":22421,"service_tier":"standard","speed":"standard",
                  "output_tokens_details":{"thinking_tokens":48},
                  "iterations":[{"input_tokens":2},{"input_tokens":2}]},
                "modelUsage":{"a-model":{"inputTokens":4,"outputTokens":96,"costUSD":0.237784}}}"#,
        ));
        let Event::SessionEnded {
            usage, model_usage, ..
        } = event
        else {
            panic!("expected session.ended");
        };
        let usage = usage.expect("the aggregate");
        assert_eq!(usage.thinking_tokens, Some(48));
        assert_eq!(usage.iterations, Some(2));
        assert_eq!(usage.speed.as_deref(), Some("standard"));
        assert_eq!(
            usage.cost_usd, None,
            "the vendor prices a model and a run, never a usage block"
        );
        let per_model = model_usage.expect("the per-model split");
        assert_eq!(per_model["a-model"].cost_usd, Some(0.237_784));
        assert_eq!(
            per_model["a-model"].thinking_tokens, None,
            "a figure carried over from another record is one this model never reported"
        );
    }

    /// A per-request `usage` event answers what the vendor wrote on that message, which at 2.1.240
    /// is four token figures and a tier. The three terminal-only figures stay absent rather than
    /// being back-filled from the run's own record.
    #[test]
    fn a_per_request_usage_event_leaves_the_terminal_only_figures_absent() {
        let mut reader = new_reader();
        let events = reader.push_line(
            r#"{"type":"assistant","request_id":"req-1","message":{"model":"a-model",
                "content":[{"type":"text","text":"hi"}],
                "usage":{"input_tokens":2,"output_tokens":20,"service_tier":"standard"}}}"#,
        );
        let Event::Usage { usage, .. } = &events[1].event else {
            panic!("expected usage");
        };
        assert_eq!(usage.service_tier.as_deref(), Some("standard"));
        assert_eq!(
            (usage.thinking_tokens, usage.iterations, usage.speed.clone()),
            (None, None, None)
        );
    }

    /// An empty iteration list is the vendor saying *none*; a missing one is the vendor not
    /// saying. Collapsing the two would make a blind record indistinguishable from a quiet run.
    #[test]
    fn an_empty_iteration_list_is_zero_and_a_missing_one_is_absence() {
        let mut reader = new_reader();
        let event = only(
            reader.push_line(r#"{"type":"result","subtype":"success","usage":{"iterations":[]}}"#),
        );
        let Event::SessionEnded { usage, .. } = event else {
            panic!("expected session.ended");
        };
        assert_eq!(usage.expect("the aggregate").iterations, Some(0));

        let mut reader = new_reader();
        let event = only(reader.push_line(r#"{"type":"result","subtype":"success","usage":{}}"#));
        let Event::SessionEnded { usage, .. } = event else {
            panic!("expected session.ended");
        };
        assert_eq!(usage.expect("the aggregate").iterations, None);
    }

    /// The census must be set before the terminal record arrives, and a run that never reached
    /// one is *"nobody found out"* rather than a failure (design § 9.4).
    #[test]
    fn a_stream_with_no_terminal_record_says_so_and_invents_nothing_at_finish() {
        let mut reader = new_reader();
        reader.push_line(r#"{"type":"system","subtype":"init","tools":[]}"#);
        assert!(!reader.saw_terminal_record());
        assert!(reader.finish().is_empty());
    }

    /// The vendor's words, passed through and never paraphrased. Detection is weak by
    /// construction and is labelled so at [`AUTH_EXPIRY_PHRASES`] (Q13).
    #[test]
    fn an_expired_credential_is_reported_beside_the_terminal_record_and_not_instead_of_it() {
        let mut reader = new_reader();
        reader.push_line(r#"{"type":"system","subtype":"init","apiKeySource":"none","tools":[]}"#);
        let events = reader.push_line(
            r#"{"type":"result","subtype":"error_during_execution","is_error":true,
                "result":"Your session has expired. Please run /login to sign in again."}"#,
        );
        let names: Vec<&str> = events.iter().map(|e| e.event.name()).collect();
        assert_eq!(names, ["auth.expired", "session.ended"]);
        let Event::AuthExpired {
            credential_source,
            detail,
            source_line,
        } = &events[0].event
        else {
            panic!("expected auth.expired");
        };
        assert_eq!(credential_source.as_deref(), Some("none"));
        assert_eq!(
            detail.as_deref(),
            Some("Your session has expired. Please run /login to sign in again.")
        );
        assert_eq!(*source_line, Some(2));
    }

    #[test]
    fn a_healthy_terminal_record_reports_no_auth_expiry() {
        let mut reader = new_reader();
        let events = reader.push_line(r#"{"type":"result","subtype":"success","result":"done"}"#);
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].event.name(), "session.ended");
    }

    /// Nothing here reads a clock: a record with no timestamp produces events with none.
    #[test]
    fn a_record_without_a_timestamp_produces_events_without_one() {
        let mut reader = new_reader();
        let events = reader.push_line(r#"{"type":"result","subtype":"success"}"#);
        assert_eq!(events[0].at, None);
    }
}
