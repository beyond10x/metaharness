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
//! | `tool.result.tool_use_result` | **source-dependent (a22).** Raw `*_call_output` content supplies none. Supported retained structured completions preserve their item; legacy command output alone remains unverified. |
//!
//! Each absence is an `unk` in a checker's verdict and never a pass. Filling one of them from a
//! neighbouring field — the `output` array as a result record, a cost multiplied out of tokens —
//! would turn *this vendor does not say* into *this vendor says fine*, which is the failure the
//! whole protocol is arranged against.
//!
//! # The opening record, completed from the first turn (amendment a34)
//!
//! | `session.started` field | this vendor |
//! |---|---|
//! | `output_style` | **carried, one record late** — the personality the first `turn_context` records, which is the session's record of the setting and not proof of what the model was told. A hook request that arrives first releases the opening record without it. The reader holds the opening record from `session_meta` until that record and completes it; the hold covers only the preamble records the pinned binary writes in between, so the stream order and every timestamp are unchanged |
//! | `plugins`, `mcp_servers`, `credential_source` | **absent** — no record of 0.153.4 carries them; [`RECORD_SILENCES`] says so in the floor's own rows |

use metaharness_protocol::{
    Digest, Emission, Event, HermeticAttestation, HermeticRow, RateLimitInfo, TranscriptRef, Usage,
};
use serde_json::Value;

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// Why the opening record this reader produces is silent on three gating rows, for the floor to
/// print beside each `unk` (amendment a34).
///
/// Each is a statement about the pinned binary's record, observed on a credential-free 0.153.4 run
/// (`docs/research/2026-10-21-codex-0.153.4-record-surface.md`): `session_meta`, `turn_context`
/// and `world_state` carry no plugin list, no credential source and no MCP server list. Each row
/// stays `unk`, because what the launch imposed is metaharness's own claim and deciding a record
/// row from it is open under invariant 3 (decision D1 = B keeps amendment a13 as written).
pub const RECORD_SILENCES: [(HermeticRow, &str); 3] = [
    (
        HermeticRow::H1a,
        "codex-cli 0.153.4's rollout lists no plugins, and the scratch CODEX_HOME is not shown to \
         be the whole plugin surface (the apps and remote_plugin features are on by default); \
         deciding H1a from what the launch imposed is open under invariant 3",
    ),
    (
        HermeticRow::H4,
        "codex-cli 0.153.4's rollout does not record which credential the session used; deciding \
         H4 from the environment the launch built is open under invariant 3",
    ),
    (
        HermeticRow::H5,
        "codex-cli 0.153.4's rollout lists no MCP servers; deciding H5 from the empty \
         [mcp_servers] the launch wrote is open under invariant 3",
    ),
];

/// Reads one rollout line at a time and emits protocol events.
///
/// Mirrors the Claude adapter's `TranscriptReader` obligations: it spawns nothing, reads no
/// clock (every timestamp is the vendor's, passed through), and drops nothing.
#[derive(Debug)]
pub struct RolloutReader {
    observations: crate::observations::Observations,
    transcript: TranscriptRef,
    attestation: HermeticAttestation,
    line_number: u64,
    saw_meta: bool,
    saw_task_complete: bool,
    turn: u32,
    /// The last `task_complete` payload, which is the closest thing a rollout has to a terminal
    /// record; [`RolloutReader::finish`] builds `session.ended` from it.
    last_complete: Option<(Option<String>, Value)>,
    /// An explicit failure belongs to the whole driven session. Later text or an
    /// ambiguous completion cannot turn it into a successful run.
    terminal_failure: bool,
    /// The last `token_count` usage, folded into `session.ended` because the rollout never
    /// carries cost and carries usage per turn rather than per session.
    last_usage: Option<Usage>,
    /// What metaharness's own seam decided, handed in rather than computed here: the census
    /// counts what *metaharness* did and no vendor record can see it (design D6, finding F10).
    census: metaharness_protocol::DecisionCensus,
    /// The opening record and every emission after it, each with the line it was read from, held
    /// from `session_meta` until the record that completes the opening record (amendment a34).
    ///
    /// The recorded output style is a per-turn fact in a rollout — `turn_context.personality` —
    /// and the first `turn_context` arrives after `session_meta`, so the opening record is held
    /// until it and completed from it. The hold covers only the records 0.153.4 writes between
    /// the two (see [`is_preamble`]); any other record releases it with the field still absent,
    /// so a record that drifted cannot hold the stream open, and [`RolloutReader::finish`]
    /// releases whatever is still held.
    held: Option<Vec<(u64, Emission)>>,
}

impl RolloutReader {
    /// A reader for one rollout, carrying what the launch knew.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation) -> Self {
        Self {
            observations: crate::observations::Observations::default(),
            transcript,
            attestation,
            line_number: 0,
            saw_meta: false,
            saw_task_complete: false,
            turn: 0,
            last_complete: None,
            terminal_failure: false,
            last_usage: None,
            census: metaharness_protocol::DecisionCensus::default(),
            held: None,
        }
    }

    /// Whether the opening record is being held for the record that completes it (amendment
    /// a34). A seam that emits events of its own — the codex hook channel — releases it first, with
    /// [`RolloutReader::release_opening_record`], so `session.started` stays the first event.
    #[must_use]
    pub fn holding_opening_record(&self) -> bool {
        self.held.is_some()
    }

    /// Release whatever is held, the opening record first and without the field the record never
    /// supplied. Empty when nothing is held.
    pub fn release_opening_record(&mut self) -> Vec<Emission> {
        self.held
            .take()
            .unwrap_or_default()
            .into_iter()
            .map(|(_, emission)| emission)
            .collect()
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
    ///
    /// Zero while the opening record is held for the record that completes it (amendment a34);
    /// the release then carries everything held, in order.
    pub fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.push_line_sourced(line)
            .into_iter()
            .map(|(_, emission)| emission)
            .collect()
    }

    /// [`RolloutReader::push_line`], each emission paired with the 1-based rollout line it was
    /// read from — which, for an emission released from the hold, is an earlier line than this
    /// one. A replay that cites record lines (`metaharness project --from codex-rollout`,
    /// amendment a28) reads them from here rather than from the line that released them.
    pub fn push_line_sourced(&mut self, line: &str) -> Vec<(u64, Emission)> {
        self.line_number += 1;
        if line.trim().is_empty() {
            return Vec::new();
        }
        let Ok(record) = serde_json::from_str::<Value>(line) else {
            // Not JSON at all: preserved, never dropped and never fatal — the vendor's format
            // carries no stability guarantee, so an unreadable line is a fact about the record.
            let emitted = vec![Emission::untimed(self.opaque(None, None, line))];
            return self.hold_or_release(Opening::Releases, emitted);
        };
        let at = record["timestamp"].as_str().map(ToString::to_string);
        let record_type = record["type"].as_str().unwrap_or_default().to_string();
        let payload = &record["payload"];
        let opens = record_type == "session_meta" && !self.saw_meta;

        let events = match record_type.as_str() {
            "session_meta" => self.session_meta(payload, line),
            "response_item" => self.response_item(payload, line),
            "event_msg" => self.event_msg(payload, line),
            "turn_context" if payload.is_object() => {
                self.observations.model(payload);
                // Preserve the rest of this record rather than silently claiming to map it all.
                vec![self.opaque(Some("turn_context"), None, line)]
            }
            _ => vec![self.opaque(Some(&record_type), payload["type"].as_str(), line)],
        };
        let emitted: Vec<Emission> = events
            .into_iter()
            .map(|event| match &at {
                Some(timestamp) => Emission::at(timestamp.clone(), event),
                None => Emission::untimed(event),
            })
            .collect();
        let effect = if opens {
            Opening::Opens
        } else if record_type == "turn_context" && payload.is_object() {
            // The personality the session recorded, verbatim, or nothing: a
            // `turn_context` without one completes the opening record with the field absent.
            Opening::Completes(payload["personality"].as_str().map(ToString::to_string))
        } else if is_preamble(&record_type, payload)
            && emitted.iter().all(|emission: &Emission| {
                matches!(
                    &emission.event,
                    Event::TurnStarted { .. } | Event::Opaque { .. }
                )
            })
        {
            // Held only when what the line produced is a turn boundary or an opaque record, so the
            // hold — and therefore what `finish` can release — never holds a call, a result or
            // text, whatever a later mapping makes of a preamble record.
            Opening::Held
        } else {
            Opening::Releases
        };
        self.hold_or_release(effect, emitted)
    }

    /// Apply one line's effect on the held opening record to the emissions the line produced.
    fn hold_or_release(&mut self, effect: Opening, emitted: Vec<Emission>) -> Vec<(u64, Emission)> {
        let line = self.line_number;
        let tagged = emitted.into_iter().map(|emission| (line, emission));
        let Some(mut held) = self.held.take() else {
            if matches!(effect, Opening::Opens) {
                self.held = Some(tagged.collect());
                return Vec::new();
            }
            return tagged.collect();
        };
        match effect {
            Opening::Completes(output_style) => {
                if let Some((
                    _,
                    Emission {
                        event:
                            Event::SessionStarted {
                                output_style: style,
                                ..
                            },
                        ..
                    },
                )) = held.first_mut()
                {
                    *style = output_style;
                }
            }
            Opening::Opens | Opening::Held => {
                held.extend(tagged);
                self.held = Some(held);
                return Vec::new();
            }
            Opening::Releases => {}
        }
        held.extend(tagged);
        held
    }

    /// The events owed after the last line.
    ///
    /// A rollout is append-only and has no closing record; `task_complete` is per turn. So the
    /// terminal `session.ended` is built here, from the last `task_complete` and the last
    /// `token_count`. Explicit errors take precedence; a legacy final message is
    /// positive success evidence. A bare completion remains unknown.
    ///
    /// An opening record still held is released first, without the field the record never
    /// supplied (amendment a34).
    pub fn finish(&mut self) -> Vec<Emission> {
        self.finish_sourced()
            .into_iter()
            .map(|(_, emission)| emission)
            .collect()
    }

    /// [`RolloutReader::finish`], each emission paired with the rollout line it was read from; the
    /// terminal record, which no line carries, cites the last line read.
    pub fn finish_sourced(&mut self) -> Vec<(u64, Emission)> {
        let mut owed = self.held.take().unwrap_or_default();
        owed.extend(
            self.terminal()
                .into_iter()
                .map(|emission| (self.line_number, emission)),
        );
        owed
    }

    /// The terminal `session.ended`, when a `task_complete` was seen.
    fn terminal(&mut self) -> Vec<Emission> {
        let Some((at, complete)) = self.last_complete.take() else {
            return Vec::new();
        };
        let event = Event::SessionEnded {
            final_answer: crate::observations::final_answer(
                &complete,
                completion_error(&complete) == Some(false),
            ),
            observed_models: self.observations.models(),
            is_error: completion_error(&complete),
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
            // Left to the run loop, which holds this adapter's published rendering: turning a
            // vendor's tool list into operations is that table's job and an adapter that kept its
            // own copy would be a second owner of one rule (design § 8.4 O6).
            available_operations: None,
            // A rollout's `session_meta` names no tool the run wanted and did not get; the vendor
            // has no such record. `None` is *it did not say*, never *nothing was withheld*.
            withheld: None,
            context_files: Vec::new(),
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
            // `session_meta` does not say where the credential came from (RECORD_SILENCES).
            credential_source: None,
            // Completed from the first `turn_context.personality` before this record is emitted
            // (amendment a34); absent if the record names none.
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
                let mut events = self.observations.call(&call_id, &name, payload);
                events.push(Event::ToolRequested {
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
                });
                events
            }
            "function_call_output" | "custom_tool_call_output" => {
                vec![self.observations.output(
                    payload["call_id"].as_str().unwrap_or_default(),
                    payload["output"].clone(),
                )]
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
        if let Some(events) = self.observations.structured(payload) {
            return events;
        }
        let message_type = payload["type"].as_str().unwrap_or_default();
        match message_type {
            "task_started" => {
                self.turn += 1;
                self.observations.turn(self.turn, payload);
                // A new unfinished turn invalidates an earlier success, but never
                // erases a failure already observed in this session.
                if !self.terminal_failure {
                    self.last_complete = None;
                }
                vec![Event::TurnStarted {
                    turn: self.turn,
                    frame_digest: None,
                }]
            }
            "task_complete" => {
                self.saw_task_complete = true;
                if !self.terminal_failure {
                    self.terminal_failure = completion_error(payload) == Some(true);
                    self.last_complete = Some((None, payload.clone()));
                }
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

/// What one rollout line does to the held opening record (amendment a34).
enum Opening {
    /// The first `session_meta`: the opening record is produced and held.
    Opens,
    /// A `turn_context`: the opening record takes its output style — the personality the record
    /// names, or none — and is released with everything held behind it.
    Completes(Option<String>),
    /// One of the records the pinned binary writes between the two: held behind the opening record.
    Held,
    /// Anything else: the opening record is released as it stands.
    Releases,
}

/// Whether a record is one the pinned binary writes between `session_meta` and the first
/// `turn_context`, which the hold may keep behind the opening record (amendment a34).
///
/// 0.153.4 writes `task_started`, the developer and user context messages and `world_state`
/// there, observed on a credential-free native run; the 0.144.0 golden record writes the same
/// kinds. Nothing that is the model's — an assistant message, a call, its output, usage — is a
/// preamble record, so a record without a `turn_context` releases the opening record at the first
/// such line rather than holding the stream behind it.
fn is_preamble(record_type: &str, payload: &Value) -> bool {
    match record_type {
        "world_state" => true,
        "event_msg" => payload["type"].as_str() == Some("task_started"),
        "response_item" => {
            payload["type"].as_str() == Some("message")
                && payload["role"].as_str() != Some("assistant")
        }
        _ => false,
    }
}

/// The distinction between an absent field and explicit null matters: older
/// records report success through their final message instead. Non-null errors
/// fail closed even if their payload shape is unfamiliar.
fn completion_error(complete: &Value) -> Option<bool> {
    match complete.get("error") {
        Some(Value::Null) => Some(false),
        Some(_) => Some(true),
        None => complete["last_agent_message"]
            .as_str()
            .filter(|message| !message.trim().is_empty())
            .map(|_| false),
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
