//! Source-backed, scoped observations. Content and requested configuration are never evidence.
use metaharness_protocol::{Event, FinalAnswer, ModelObservationScope, ObservedModel};
use serde_json::Value;
use std::collections::BTreeMap;

/// The `exit_code` Codex writes when it has no exit to report (amendment a47 § 5): a command it
/// could not start or rejected (`codex-rs/core/src/tools/events.rs:534-559` at `rust-v0.153.4`), or
/// one whose exit status it lost (`core/src/unified_exec/process_manager.rs:752`,
/// `core/src/unified_exec/async_watcher.rs:215`). It is never published as an exit.
const PLACEHOLDER_EXIT: i32 = -1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Family {
    Command,
    Patch,
    Other,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Evidence {
    is_error: bool,
    exit_code: Option<i32>,
}
#[derive(Debug)]
struct Call {
    family: Family,
    turn: u32,
    signature: Value,
    turn_id: Option<String>,
    evidence: Option<Evidence>,
    conflicted: bool,
}
#[derive(Debug, Default)]
pub(super) struct Observations {
    turn: u32,
    turn_id: Option<String>,
    models: Vec<ObservedModel>,
    calls: BTreeMap<String, Call>,
    /// Commands named by a `PreToolUse` request that carried the vendor's own id (amendment a47).
    /// A command run through the code-mode `exec` tool completes under that id, and no call record
    /// in the rollout carries it.
    hooked: BTreeMap<String, Call>,
}
fn reused(id: &str) -> Vec<Event> {
    vec![
        Event::ToolResult {
            call_id: id.to_owned(),
            is_error: None,
            exit_code: None,
            outcome_source: None,
            content: None,
            bytes: None,
            tool_use_result: None,
        },
        Event::Warning {
            code: "TOOL_OUTCOME_UNVERIFIED".to_owned(),
            message: "a call identifier was reused with another turn or request; its outcome is now unknown".to_owned(),
        },
    ]
}
/// Settle one completion against the call it was correlated with: the evidence it may publish, and
/// the warning it owes. A call once conflicted stays unknown.
fn settle(
    call: &mut Call,
    mismatch: bool,
    candidate: Option<Evidence>,
) -> (Option<Evidence>, Option<&'static str>) {
    let mut warning = None;
    if mismatch {
        call.conflicted = true;
        warning = Some("completion does not match the observed call family or turn");
    } else if candidate.is_none() {
        call.conflicted = true;
        warning = Some("completion lacks consistent supported outcome metadata");
    } else if call.evidence.is_some() && call.evidence != candidate {
        call.conflicted = true;
        warning = Some("contradictory structured tool outcomes");
    } else if !call.conflicted {
        call.evidence = candidate;
    }
    (if call.conflicted { None } else { call.evidence }, warning)
}
fn text(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .map(ToString::to_string)
}
pub(super) fn final_answer(complete: &Value, successful: bool) -> Option<FinalAnswer> {
    if !successful {
        return None;
    }
    Some(FinalAnswer {
        text: text(&complete["last_agent_message"])?,
        turn_id: text(&complete["turn_id"]),
    })
}
impl Observations {
    pub(super) fn turn(&mut self, turn: u32, payload: &Value) {
        self.turn = turn;
        self.turn_id = text(&payload["turn_id"]);
    }
    pub(super) fn model(&mut self, payload: &Value) {
        let observation = ObservedModel {
            turn: self.turn.max(1),
            turn_id: text(&payload["turn_id"]),
            model: text(&payload["model"]),
            scope: ModelObservationScope::TurnSelection,
        };
        if self.models.last() != Some(&observation) {
            self.models.push(observation);
        }
    }
    pub(super) fn models(&self) -> Option<Vec<ObservedModel>> {
        (!self.models.is_empty()).then(|| self.models.clone())
    }
    pub(super) fn call(&mut self, id: &str, name: &str, signature: &Value) -> Vec<Event> {
        if id.is_empty() {
            return Vec::new();
        }
        let family = match name {
            "exec" | "exec_command" | "shell" | "shell_command" => Family::Command,
            "apply_patch" => Family::Patch,
            _ => Family::Other,
        };
        if let Some(call) = self.calls.get_mut(id) {
            if call.family != family || call.turn != self.turn || call.signature != *signature {
                call.conflicted = true;
                return reused(id);
            }
        } else {
            self.calls.insert(
                id.to_owned(),
                Call {
                    family,
                    turn: self.turn,
                    signature: signature.clone(),
                    turn_id: self.turn_id.clone(),
                    evidence: None,
                    conflicted: false,
                },
            );
        }
        Vec::new()
    }
    /// A `PreToolUse` request for a shell command, under the vendor's `tool_use_id` and `turn_id`
    /// (amendment a47). Code mode mints that id for the command it runs
    /// (`codex-rs/core/src/tools/code_mode/mod.rs:374` at `rust-v0.153.4`), the hook receives it
    /// (`core/src/hook_runtime.rs:184-203`), and the command's `item_completed` carries it as
    /// `item.id`, with the same turn id (`core/src/tools/events.rs:563-586`,
    /// `core/src/session/mod.rs:2439-2464`). Only `Bash` is recorded: it is the one hook name a
    /// shell command takes (`core/src/tools/handlers/unified_exec/exec_command.rs:504-515`).
    pub(super) fn hooked(
        &mut self,
        id: &str,
        tool: &str,
        turn_id: Option<&str>,
        input: &Value,
    ) -> Vec<Event> {
        if id.is_empty() || tool != "Bash" {
            return Vec::new();
        }
        let turn_id = turn_id
            .filter(|t| !t.trim().is_empty())
            .map(ToString::to_string);
        if let Some(call) = self.hooked.get_mut(id) {
            if call.turn_id != turn_id || call.signature != *input {
                call.conflicted = true;
                return reused(id);
            }
        } else {
            self.hooked.insert(
                id.to_owned(),
                Call {
                    family: Family::Command,
                    turn: self.turn,
                    signature: input.clone(),
                    turn_id,
                    evidence: None,
                    conflicted: false,
                },
            );
        }
        Vec::new()
    }
    pub(super) fn output(&self, id: &str, content: Value) -> Event {
        let evidence = self
            .calls
            .get(id)
            .filter(|call| !call.conflicted)
            .and_then(|call| call.evidence);
        Event::ToolResult {
            call_id: id.to_owned(),
            is_error: evidence.map(|e| e.is_error),
            exit_code: evidence.and_then(|e| e.exit_code),
            outcome_source: evidence.map(|_| "retained_tool_completion".to_owned()),
            bytes: content.as_str().map(|s| s.len() as u64),
            content: Some(content),
            tool_use_result: None,
        }
    }
    pub(super) fn structured(&mut self, payload: &Value) -> Option<Vec<Event>> {
        let (item, id, family, legacy_patch) = match payload["type"].as_str()? {
            "patch_apply_end" => (
                payload,
                text(&payload["call_id"]).unwrap_or_default(),
                Family::Patch,
                true,
            ),
            "item_completed" => {
                let item = &payload["item"];
                let family = match item["type"].as_str()? {
                    "CommandExecution" => Family::Command,
                    "FileChange" => Family::Patch,
                    _ => return None,
                };
                (item, text(&item["id"]).unwrap_or_default(), family, false)
            }
            _ => return None,
        };
        let code = item["exit_code"]
            .as_i64()
            .and_then(|n| i32::try_from(n).ok());
        let status = item["status"].as_str();
        let candidate = match (family, status) {
            (Family::Command, Some("completed")) if code == Some(0) => Some(Evidence {
                is_error: false,
                exit_code: code,
            }),
            (Family::Command, Some("failed" | "declined")) if code != Some(0) => Some(Evidence {
                is_error: true,
                exit_code: code.filter(|&exit| exit != PLACEHOLDER_EXIT),
            }),
            (Family::Patch, Some("completed")) if !legacy_patch || item["success"] == true => {
                Some(Evidence {
                    is_error: false,
                    exit_code: None,
                })
            }
            (Family::Patch, Some("failed" | "declined"))
                if !legacy_patch || item["success"] == false =>
            {
                Some(Evidence {
                    is_error: true,
                    exit_code: None,
                })
            }
            _ => None,
        };
        let completion_turn = text(&payload["turn_id"]);
        let (evidence, warning) = if let Some(call) = self.calls.get_mut(&id) {
            let mismatch = call.family != family
                || call.turn != self.turn
                || matches!((&call.turn_id,&completion_turn),(Some(a),Some(b)) if a!=b);
            settle(call, mismatch, candidate)
        } else if let Some(call) = self.hooked.get_mut(&id) {
            // Amendment a47: the hook request and the completion are matched by the vendor's id
            // and the vendor's turn id, and the turn id is required on both sides. The rollout's
            // turn counter is not compared: the hook channel and the rollout tail are read
            // independently, so the counter says when the line was read, not which turn it is.
            let mismatch =
                call.family != family || call.turn_id.is_none() || call.turn_id != completion_turn;
            settle(call, mismatch, candidate)
        } else {
            (None, Some("completion has no matching observed call"))
        };
        let content = item
            .get("aggregated_output")
            .or_else(|| item.get("stdout"))
            .cloned();
        let mut events = vec![Event::ToolResult {
            call_id: id,
            is_error: evidence.map(|e| e.is_error),
            exit_code: evidence.and_then(|e| e.exit_code),
            outcome_source: evidence.map(|_| "retained_tool_completion".to_owned()),
            bytes: content
                .as_ref()
                .and_then(Value::as_str)
                .map(|s| s.len() as u64),
            content,
            tool_use_result: Some(item.clone()),
        }];
        if let Some(message) = warning {
            events.push(Event::Warning {
                code: "TOOL_OUTCOME_UNVERIFIED".to_owned(),
                message: message.to_owned(),
            });
        }
        Some(events)
    }
}
