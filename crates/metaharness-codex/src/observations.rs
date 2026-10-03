//! Source-backed, scoped observations. Content and requested configuration are never evidence.
use metaharness_protocol::{Event, FinalAnswer, ModelObservationScope, ObservedModel};
use serde_json::Value;
use std::collections::BTreeMap;

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
                return vec![
                    Event::ToolResult {call_id:id.to_owned(),is_error:None,exit_code:None,outcome_source:None,content:None,bytes:None,tool_use_result:None},
                    Event::Warning {code:"TOOL_OUTCOME_UNVERIFIED".to_owned(),message:"a call identifier was reused with another turn or request; its outcome is now unknown".to_owned()},
                ];
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
                exit_code: code,
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
        let mut warning = None;
        let evidence = if let Some(call) = self.calls.get_mut(&id) {
            let completion_turn = text(&payload["turn_id"]);
            if call.family != family
                || call.turn != self.turn
                || matches!((&call.turn_id,&completion_turn),(Some(a),Some(b)) if a!=b)
            {
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
            if call.conflicted { None } else { call.evidence }
        } else {
            warning = Some("completion has no matching observed call");
            None
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
