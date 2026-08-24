//! The rollout reader: Codex's session record, mapped onto the protocol's events.
//!
//! The adapter's input is the **session rollout** —
//! `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ISO8601>-<uuid7>.jsonl` — not `codex exec --json`
//! stdout, which carries no timestamps, no durations and no cost (research record, verified
//! against 2,437 local files on codex-cli 0.145.0). Every rollout line is
//! `{timestamp, type, payload}`; `session_meta` is first, enforced by the vendor's own
//! `session_configured_not_first_event` error.
//!
//! **No stability guarantee is documented for this format**, and drift is observable inside one
//! install (April files spell a shell call `exec_command_begin`, August files
//! `custom_tool_call`). So this reader version-gates on `session_meta.cli_version` — a version
//! outside the pin is a `warning`, never a refusal mid-read — and every record it cannot map
//! becomes [`Event::Opaque`] with the vendor's own type words, a digest and its 1-based line.
//! Nothing is dropped (design D4).
//!
//! # What the rollout has, of amendment a9's four fields
//!
//! One of the four is really in this vendor's record and three are not. Named here rather than
//! left to be discovered by a reader wondering why a driven Codex run answers `unk` where a
//! driven Claude run answers:
//!
//! | field | this vendor |
//! |---|---|
//! | `usage.thinking_tokens` | **carried** — `token_count.info.total_token_usage.reasoning_output_tokens`, the same quantity under the vendor's own name |
//! | `usage.iterations` | **absent.** A `token_count` payload has no per-iteration list to take a length of |
//! | `usage.speed` | **absent.** No speed tier is reported, as no service tier is |
//! | `usage.cost_usd` | **absent.** The rollout prices nothing, and there is no per-model split to price; `session.ended.total_cost_usd` is absent for the same reason |
//! | `tool.result.tool_use_result` | **absent.** A `*_call_output` payload carries `call_id`, `output` and turn metadata, and nothing that answers what a per-tool result record answers |
//!
//! Each absence is an `unk` in a checker's verdict and never a pass. Filling one of them from a
//! neighbouring field — the `output` array as a result record, a cost multiplied out of tokens —
//! would turn *this vendor does not say* into *this vendor says fine*, which is the failure the
//! whole protocol is arranged against.

use metaharness_protocol::{
    Digest, Emission, Event, HermeticAttestation, RateLimitInfo, TranscriptRef, Usage,
};
use serde_json::Value;

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// Reads one rollout line at a time and emits protocol events.
///
/// Mirrors the Claude adapter's `TranscriptReader` obligations: it spawns nothing, reads no
/// clock (every timestamp is the vendor's, passed through), and drops nothing.
#[derive(Debug)]
pub struct RolloutReader {
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
    line_number: u64,
    saw_meta: bool,
    saw_task_complete: bool,
    turn: u32,
    /// The last `task_complete` payload, which is the closest thing a rollout has to a terminal
    /// record; [`RolloutReader::finish`] builds `session.ended` from it.
    last_complete: Option<(Option<String>, Value)>,
    /// The last `token_count` usage, folded into `session.ended` because the rollout never
    /// carries cost and carries usage per turn rather than per session.
    last_usage: Option<Usage>,
    /// What metaharness's own seam decided, handed in rather than computed here: the census
    /// counts what *metaharness* did and no vendor record can see it (design D6, finding F10).
    census: metaharness_protocol::DecisionCensus,
}

impl RolloutReader {
    /// A reader for one rollout, carrying what the launch knew.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation) -> Self {
        Self {
            transcript,
            attestation,
            line_number: 0,
            saw_meta: false,
            saw_task_complete: false,
            turn: 0,
            last_complete: None,
            last_usage: None,
            census: metaharness_protocol::DecisionCensus::default(),
        }
    }

    /// Whether a `task_complete` has been seen — the rollout's nearest thing to a terminal
    /// record.
    #[must_use]
    pub fn saw_terminal_record(&self) -> bool {
        self.saw_task_complete
    }

    /// Hand the terminal record metaharness's own decision census before it is emitted.
    ///
    /// Set rather than derived, for the reason design D6 gives: the census counts what
    /// metaharness decided, and a rollout that recorded a call the seam denied cannot say who
    /// denied it or why.
    pub fn set_census(&mut self, census: metaharness_protocol::DecisionCensus) {
        self.census = census;
    }

    /// One rollout line in, zero or more events out.
    pub fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.line_number += 1;
        if line.trim().is_empty() {
            return Vec::new();
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            // Not JSON at all: preserved, never dropped and never fatal — the vendor's format
            // carries no stability guarantee, so an unreadable line is a fact about the record.
            return vec![Emission::untimed(self.opaque(None, None, line))];
        };
        let at = record["timestamp"].as_str().map(ToString::to_string);
        let record_type = record["type"].as_str().unwrap_or_default().to_string();
        let payload = &record["payload"];

        let events = match record_type.as_str() {
            "session_meta" => self.session_meta(payload, line),
            "response_item" => self.response_item(payload, line),
            "event_msg" => self.event_msg(payload, line),
            _ => vec![self.opaque(Some(&record_type), payload["type"].as_str(), line)],
        };
        events
            .into_iter()
            .map(|event| match &at {
                Some(timestamp) => Emission::at(timestamp.clone(), event),
                None => Emission::untimed(event),
            })
            .collect()
    }

    /// The events owed after the last line.
    ///
    /// A rollout is append-only and has no closing record; `task_complete` is per turn. So the
    /// terminal `session.ended` is built here, from the last `task_complete` and the last
    /// `token_count` — with `is_error` left absent rather than guessed, because a rollout that
    /// simply stops does not say why.
    pub fn finish(&mut self) -> Vec<Emission> {
        let Some((at, complete)) = self.last_complete.take() else {
            return Vec::new();
        };
        let event = Event::SessionEnded {
            is_error: None,
            subtype: None,
            stop_reason: None,
            terminal_reason: None,
            api_error_status: None,
            num_turns: Some(u64::from(self.turn)),
            duration_ms: complete["duration_ms"].as_u64(),
            duration_api_ms: None,
            ttft_ms: complete["time_to_first_token_ms"].as_u64(),
            time_to_request_ms: None,
            // Never emitted by the vendor: zero cost keys across 2,437 local files. Absent is
            // the honest value; a caller that wants money derives it from tokens and says so.
            total_cost_usd: None,
            permission_denials: None,
            subagents_spawned: None,
            usage: self.last_usage.take(),
            model_usage: None,
            census: self.census.clone(),
        };
        match at {
            Some(timestamp) => vec![Emission::at(timestamp, event)],
            None => vec![Emission::untimed(event)],
        }
    }

    fn session_meta(&mut self, payload: &Value, line: &str) -> Vec<Event> {
        if self.saw_meta {
            // A second session_meta is a shape this pin does not know.
            return vec![self.opaque(Some("session_meta"), None, line)];
        }
        self.saw_meta = true;
        let version = payload["cli_version"].as_str().map(ToString::to_string);
        let mut events = vec![Event::SessionStarted {
            adapter: ADAPTER_ID.to_string(),
            adapter_class: "harness".to_string(),
            harness_version: version.clone(),
            session_id: payload["session_id"]
                .as_str()
                .or_else(|| payload["id"].as_str())
                .map(ToString::to_string),
            // The model is a per-turn fact in a rollout (`turn_context.model`), not a session
            // one; absent here rather than copied from a turn that has not happened yet.
            model: None,
            permission_mode: None,
            credential_source: None,
            output_style: None,
            cwd: payload["cwd"].as_str().map(ToString::to_string),
            offered_tools: None,
            slash_commands: None,
            skills: None,
            agents: None,
            // A rollout lists no plugins and no MCP servers. `None` is `unk`, never zero: the
            // hermetic floor must not read the vendor's silence as an empty surface.
            plugins: None,
            mcp_servers: None,
            inputs_digest: None,
            transcript: self.transcript.clone(),
            hermetic: self.attestation.clone(),
        }];
        if let Some(version) = version
            && !PINNED_VERSIONS.contains(&version.as_str())
        {
            events.push(Event::Warning {
                code: "version_outside_pin".to_string(),
                message: format!(
                    "the rollout was written by codex-cli {version} and this adapter is \
                         pinned to {}; the format has no documented stability guarantee, so \
                         unmapped shapes become opaque rather than errors",
                    PINNED_VERSIONS.join(", ")
                ),
            });
        }
        events
    }

    fn response_item(&mut self, payload: &Value, line: &str) -> Vec<Event> {
        let item_type = payload["type"].as_str().unwrap_or_default();
        match item_type {
            "function_call" | "custom_tool_call" => {
                let call_id = payload["call_id"].as_str().unwrap_or_default().to_string();
                let name = payload["name"].as_str().unwrap_or_default().to_string();
                // `arguments` is a JSON-encoded string on the wire; decoded when it parses,
                // carried verbatim when it does not — never dropped either way.
                let raw = payload["arguments"].as_str().unwrap_or_default();
                let input = serde_json::from_str::<Value>(raw)
                    .unwrap_or_else(|_| Value::String(raw.to_string()));
                vec![Event::ToolRequested {
                // Left empty here on purpose, exactly as `operations` is: what a call touches is
                // resolved by whoever holds the run's published rendering, and an adapter that
                // answered for itself would be a second owner of one rule (design § 8.4 O6).
                subjects: Vec::new(),
                    // Left empty here on purpose: the resolution needs the adapter\'s *published*
                    // rendering, which the loop holds and an adapter must not (design § 8.4 O6).
                    operations: Vec::new(),
                    call_id,
                    name,
                    input,
                    // A rollout is read after the fact: nothing is blocked on this event, and
                    // saying so is what keeps a post-hoc record from impersonating a control.
                    decision_required: false,
                    deadline_ms: None,
                    seam: metaharness_protocol::Seam::None,
                }]
            }
            "function_call_output" | "custom_tool_call_output" => {
                let content = payload["output"].clone();
                let bytes = content.as_str().map(|text| text.len() as u64);
                vec![Event::ToolResult {
                    call_id: payload["call_id"].as_str().unwrap_or_default().to_string(),
                    // The rollout's output envelope carries no error flag this pin has
                    // verified; absent, never false.
                    is_error: None,
                    content: Some(content),
                    bytes,
                    // **This vendor writes no per-tool result record** (amendment a9). A
                    // `*_call_output` payload carries `call_id`, `output` and the turn metadata
                    // passthrough, and nothing that answers the question Claude Code's
                    // `tool_use_result` answers — no per-tool `success`, no `commandName`. So an
                    // expectation reading those fields is `unk` against a driven Codex run, which
                    // is the honest answer; folding the `output` array in here to fill the field
                    // would make it a pass.
                    tool_use_result: None,
                }]
            }
            "message" => match payload["role"].as_str() {
                Some("assistant") => vec![Event::Text {
                    text: text_of(payload),
                    request_id: None,
                }],
                _ => vec![self.opaque(Some("response_item"), Some(item_type), line)],
            },
            "reasoning" => vec![Event::Thinking {
                text: text_of(payload),
                request_id: None,
            }],
            _ => vec![self.opaque(Some("response_item"), Some(item_type), line)],
        }
    }

    fn event_msg(&mut self, payload: &Value, line: &str) -> Vec<Event> {
        let message_type = payload["type"].as_str().unwrap_or_default();
        match message_type {
            "task_started" => {
                self.turn += 1;
                vec![Event::TurnStarted {
                    turn: self.turn,
                    frame_digest: None,
                }]
            }
            "task_complete" => {
                self.saw_task_complete = true;
                self.last_complete = Some((None, payload.clone()));
                vec![Event::TurnEnded {
                    turn: self.turn.max(1),
                    stop_reason: None,
                }]
            }
            "token_count" => {
                let mut events = Vec::new();
                let totals = &payload["info"]["total_token_usage"];
                if totals.is_object() {
                    let usage = Usage {
                        input_tokens: totals["input_tokens"].as_u64(),
                        output_tokens: totals["output_tokens"].as_u64(),
                        cache_read_input_tokens: totals["cached_input_tokens"].as_u64(),
                        cache_creation_input_tokens: totals["cache_write_input_tokens"].as_u64(),
                        service_tier: None,
                        // The vendor's own name for the same quantity: reasoning tokens billed
                        // out of the output figure. Mapped rather than left absent, because a
                        // different spelling of a fact the record really carries is exactly what
                        // an adapter is for (amendment a9).
                        thinking_tokens: totals["reasoning_output_tokens"].as_u64(),
                        // **Three figures this vendor does not report** (amendment a9). A
                        // `token_count` payload carries `total_token_usage`, `last_token_usage`
                        // and `model_context_window`: no per-iteration list to take a length of,
                        // and no speed tier. `service_tier` was already absent for the same
                        // reason. Absent is the record's answer; a zero would be ours.
                        iterations: None,
                        speed: None,
                        // The rollout prices nothing — zero cost keys across 2,437 local files,
                        // and no per-model split to hang a cost on. `session.ended` carries the
                        // same absence for `total_cost_usd` and for the same reason.
                        cost_usd: None,
                    };
                    self.last_usage = Some(usage.clone());
                    events.push(Event::Usage {
                        request_id: None,
                        model: None,
                        usage,
                    });
                }
                let limits = &payload["rate_limits"];
                if limits.is_object() {
                    events.push(Event::RateLimit {
                        info: RateLimitInfo {
                            status: limits["rate_limit_reached_type"]
                                .as_str()
                                .map(ToString::to_string),
                            window: limits["limit_name"]
                                .as_str()
                                .or_else(|| limits["limit_id"].as_str())
                                .map(ToString::to_string),
                            resets_at: None,
                            utilization: limits["primary"]["used_percent"].as_f64(),
                            using_overage: None,
                        },
                    });
                }
                if events.is_empty() {
                    events.push(self.opaque(Some("event_msg"), Some(message_type), line));
                }
                events
            }
            "agent_message" => vec![Event::Text {
                text: text_of(payload),
                request_id: None,
            }],
            "agent_reasoning" | "agent_reasoning_raw_content" => vec![Event::Thinking {
                text: text_of(payload),
                request_id: None,
            }],
            _ => vec![self.opaque(Some("event_msg"), Some(message_type), line)],
        }
    }

    fn opaque(&self, vendor_type: Option<&str>, vendor_subtype: Option<&str>, line: &str) -> Event {
        Event::Opaque {
            vendor_type: vendor_type.map(ToString::to_string),
            vendor_subtype: vendor_subtype.map(ToString::to_string),
            digest: Digest::of(line.as_bytes()),
            source_line: Some(self.line_number),
        }
    }
}

/// The text of a message-ish payload, wherever this pin has seen it live.
fn text_of(payload: &Value) -> String {
    if let Some(text) = payload["message"].as_str() {
        return text.to_string();
    }
    if let Some(text) = payload["text"].as_str() {
        return text.to_string();
    }
    if let Some(parts) = payload["content"].as_array() {
        let joined: Vec<&str> = parts
            .iter()
            .filter_map(|part| part["text"].as_str())
            .collect();
        if !joined.is_empty() {
            return joined.join("");
        }
    }
    String::new()
}
