//! The seam: this adapter's record reader in, its hook response out.
//!
//! # Two sources, one stream, and why that is not a hedge
//!
//! Claude Code writes the record metaharness reads and the calls metaharness decides down **one**
//! pipe: the `tool_use` block arrives on stdout and the hook fires afterwards (row V23). Codex
//! does not. Its rich record is a **file** — `$CODEX_HOME/sessions/…/rollout-*.jsonl`, the one
//! that carries timestamps, durations and per-turn usage where `codex exec --json` stdout carries
//! none — and its hook is a separate process that speaks to metaharness over a directory pair. So
//! this seam is handed two kinds of line and tells them apart by the envelope:
//!
//! | line | source | what it becomes |
//! |---|---|---|
//! | a rollout record | the session file, tailed as it is written | whatever [`crate::RolloutReader`] maps it to — including `tool.requested` with [`Seam::None`], *a record of a call and not a call awaiting a decision* |
//! | a hook request | a `PreToolUse` process, blocked, holding the call | `tool.requested` with the seam that will carry the answer — **the decision-bearing one** |
//!
//! **One call appears twice, under two different ids, and metaharness does not pretend otherwise.**
//! The vendor's `PreToolUse` payload carries `tool_use_id` — its own per-call id, the same spelling
//! Claude Code uses (row V22) — so the live call is presented under a vendor id rather than a name
//! metaharness invented. But a driven run shows the two records of one call do **not** share it:
//! the hook received `tool_use_id: "exec-01ddc1c4-…"` while the rollout's output record for the
//! same command carried `call_id: "call_YeJLGplD8vsAFctXshaYdCKK"`. Different namespaces, one call.
//!
//! So the seam emits both and joins neither. Correlating them would mean matching on arguments and
//! timing, and a guard that guesses which call it is holding is worse than one that says it does
//! not know. What is available to a reader that wants the join is `turn_id`, which both sides
//! carry.
//!
//! What a **decision** is routed by is a third thing again: the hook process's own rendezvous name,
//! which is correct even for a payload that arrives with no id at all. The three are kept apart on
//! purpose, because collapsing any two of them would deliver a decision to the wrong blocked
//! process.

use std::collections::BTreeMap;

use metaharness_protocol::{
    Command, Decision, DecisionCensus, Emission, Event, HarnessSeam, HermeticAttestation, Seam,
    SeamFactory, TranscriptRef,
};
use serde_json::Value;

use crate::rollout::RolloutReader;
use crate::seam::{parse_hook_input, render_hook_response};

/// The envelope key that marks a line as a hook request rather than a rollout record.
///
/// metaharness's own name, in metaharness's own namespace: a rollout line is `{timestamp, type,
/// payload}` and could not carry this key by accident, and a line that somehow did would be read
/// as an unmapped shape and preserved as `opaque` rather than acted on.
const HOOK_ENVELOPE: &str = "metaharness.codex/1";

/// The value that key carries.
const HOOK_REQUEST: &str = "hook_request";

/// One blocked hook process, as a line the seam will read.
///
/// Published by this crate rather than assembled by the runner, because the envelope is a private
/// contract between the codex runner and the codex seam and a second party writing it would be a
/// second party who can get it wrong. `raw` is the hook's stdin **verbatim**: the program parses
/// no JSON, so whatever the vendor sent is what arrives here, valid or not.
#[must_use]
pub fn hook_request_line(key: &str, raw: &str) -> String {
    let input =
        serde_json::from_str::<Value>(raw).unwrap_or_else(|_| Value::String(raw.to_string()));
    serde_json::json!({
        HOOK_ENVELOPE: HOOK_REQUEST,
        "key": key,
        "input": input,
    })
    .to_string()
}

/// The codex seam: rollout records and live hook requests in, hook responses out.
pub struct CodexSeam {
    reader: RolloutReader,
    seam: Seam,
    /// Which hook process is holding which call: the metaharness `call_id` a decision will name,
    /// mapped to the rendezvous name that decision has to be written under.
    ///
    /// Held here and not in the runner because the runner never parses a payload — it carries the
    /// hook's stdin verbatim and the adapter decides what it means.
    routes: BTreeMap<String, String>,
}

impl std::fmt::Debug for CodexSeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexSeam").finish_non_exhaustive()
    }
}

impl CodexSeam {
    /// A seam over the retained record and the attestation that goes into `session.started`.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation, seam: Seam) -> Self {
        Self {
            reader: RolloutReader::new(transcript, attestation),
            seam,
            routes: BTreeMap::new(),
        }
    }

    /// Every call this seam is holding, as `call_id` → the hook process's rendezvous name.
    ///
    /// Published so a live proof can assert **from the run's own record** that the hook fired and
    /// which call it was holding, rather than from the fact that a config file was written.
    #[must_use]
    pub fn routes(&self) -> &BTreeMap<String, String> {
        &self.routes
    }

    /// The `tool.requested` a blocked hook process owes, or `None` when the line is not one.
    fn hook_request(&mut self, line: &str) -> Option<Vec<Emission>> {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get(HOOK_ENVELOPE).and_then(Value::as_str) != Some(HOOK_REQUEST) {
            return None;
        }
        // The rendezvous name a decision travels on: a response written under it reaches exactly
        // the hook process that is holding this call, whatever the vendor's payload does or does
        // not carry.
        let key = value.get("key").and_then(Value::as_str)?.to_string();
        let raw = value.get("input").cloned().unwrap_or(Value::Null);
        let parsed = raw.as_str().map_or_else(
            || parse_hook_input(&raw.to_string()).ok(),
            |text| parse_hook_input(text).ok(),
        );
        // An input metaharness could not read is still a hook that is waiting, and it is still a
        // call. It is presented with **no tool name** — which no frame admits and no operation
        // renders to, so the frame denies it — and under the rendezvous name, because a call id
        // invented here would be a correlation nobody could check.
        let (call_id, name, input) = parsed.map_or_else(
            || (key.clone(), String::new(), raw.clone()),
            |parsed| {
                (
                    parsed.tool_use_id.unwrap_or_else(|| key.clone()),
                    parsed.tool_name.unwrap_or_default(),
                    parsed.tool_input,
                )
            },
        );
        self.routes.insert(call_id.clone(), key);
        Some(vec![Emission::untimed(Event::ToolRequested {
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
            decision_required: true,
            deadline_ms: None,
            seam: self.seam,
        })])
    }
}

impl HarnessSeam for CodexSeam {
    fn push_line(&mut self, line: &str) -> Vec<Emission> {
        if let Some(emissions) = self.hook_request(line) {
            return emissions;
        }
        self.reader.push_line(line)
    }

    fn finish(&mut self) -> Vec<Emission> {
        self.reader.finish()
    }

    fn set_census(&mut self, census: DecisionCensus) {
        self.reader.set_census(census);
    }

    fn decision_line(&self, call_id: &str, decision: &Decision) -> String {
        // The rendezvous name, or the call id itself when this seam is holding no such call. The
        // fallback is deliberate and it is not a guess: a decision for a call no hook published
        // reaches no hook process, parks, and is never delivered — which is the honest outcome for
        // a decision about a call nobody is holding.
        let key = self.routes.get(call_id).map_or(call_id, String::as_str);
        serde_json::json!({
            "call_id": key,
            "response": render_hook_response(decision),
        })
        .to_string()
    }

    fn control_line(&self, command: &Command) -> Option<String> {
        match command {
            // `tool.decide` reaches the child as a decision line, not a control line. The other
            // three are declared `Unverified` or refused in this adapter's capability set, so a
            // line here would be a control that appears to work and does not — which design § 7.1
            // forbids more strongly than it forbids an absent control.
            Command::ToolDecide { .. }
            | Command::FrameSet { .. }
            | Command::Steer { .. }
            | Command::MessageInject { .. }
            | Command::PermissionSet { .. } => None,
            // Delivered by terminating the child, exactly as on the Claude adapter: `turn/interrupt`
            // is verified present on the app-server surface (V14) and this adapter drives
            // `codex exec`, which has no such channel.
            Command::Interrupt { .. } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "interrupt" },
                })
                .to_string(),
            ),
            Command::Halt { .. } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "interrupt", "halt": true },
                })
                .to_string(),
            ),
        }
    }
}

/// The factory for [`CodexSeam`].
#[derive(Debug, Clone, Copy, Default)]
pub struct CodexSeams;

impl SeamFactory for CodexSeams {
    fn build(
        &mut self,
        transcript: TranscriptRef,
        attestation: HermeticAttestation,
        seam: Seam,
    ) -> Box<dyn HarnessSeam> {
        Box::new(CodexSeam::new(transcript, attestation, seam))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::HermeticMode;

    fn seam() -> CodexSeam {
        CodexSeam::new(
            TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            HermeticAttestation::none(HermeticMode::Off),
            Seam::Hook,
        )
    }

    /// The line a blocked hook process becomes: a call the seam is holding, stamped with the seam
    /// that will carry the answer.
    #[test]
    fn a_hook_request_becomes_the_decision_bearing_call() {
        let mut seam = seam();
        let raw = r#"{"hook_event_name":"PreToolUse","tool_name":"shell","tool_use_id":"call-7","session_id":"s-1","turn_id":"t1","cwd":"/scratch/work","permission_mode":"default","tool_input":{"command":["bash","-lc","echo hi"]}}"#;
        let emitted = seam.push_line(&hook_request_line("abcd1234", raw));
        assert_eq!(emitted.len(), 1);
        match &emitted[0].event {
            Event::ToolRequested {
                call_id,
                name,
                input,
                seam,
                ..
            } => {
                // The **vendor's** hook id, not the rendezvous name. It does not join to the
                // rollout's own `call_id` for the same call — driven, they differ — and this
                // adapter claims no such join.
                assert_eq!(call_id, "call-7");
                assert_eq!(name, "shell");
                assert_eq!(input["command"][0], "bash");
                assert_eq!(*seam, Seam::Hook);
            }
            other => panic!("{other:?}"),
        }
        // …and the decision still travels on the rendezvous name, which is what reaches the
        // process that is actually blocked.
        assert_eq!(
            seam.routes().get("call-7").map(String::as_str),
            Some("abcd1234")
        );
        let line = seam.decision_line("call-7", &Decision::Allow);
        let value: Value = serde_json::from_str(&line).expect("parses");
        assert_eq!(value["call_id"], "abcd1234");
    }

    /// A payload with no `tool_use_id` is presented under the rendezvous name rather than under an
    /// id metaharness made up, and the decision still reaches the process that is holding it.
    #[test]
    fn a_payload_without_the_vendors_id_falls_back_to_the_rendezvous_name() {
        let mut seam = seam();
        let emitted = seam.push_line(&hook_request_line("k9", r#"{"tool_name":"shell"}"#));
        match &emitted[0].event {
            Event::ToolRequested { call_id, .. } => assert_eq!(call_id, "k9"),
            other => panic!("{other:?}"),
        }
        let line = seam.decision_line("k9", &Decision::Allow);
        assert!(line.contains(r#""call_id":"k9""#), "{line}");
    }

    /// A rollout record is a **record**: it is read by the reader and it carries [`Seam::None`],
    /// so nothing downstream mistakes a post-hoc line for a call anybody is holding open.
    #[test]
    fn a_rollout_record_is_read_as_a_record_and_never_as_a_live_call() {
        let mut seam = seam();
        seam.push_line(
            r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"s","cli_version":"0.145.0","cwd":"/scratch/work"}}"#,
        );
        let emitted = seam.push_line(
            r#"{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{"type":"custom_tool_call","call_id":"call-1","name":"exec","arguments":"{\"command\":\"ls\"}"}}"#,
        );
        assert_eq!(emitted.len(), 1);
        match &emitted[0].event {
            Event::ToolRequested {
                call_id,
                seam,
                decision_required,
                ..
            } => {
                assert_eq!(call_id, "call-1");
                assert_eq!(*seam, Seam::None);
                assert!(!decision_required);
            }
            other => panic!("{other:?}"),
        }
    }

    /// A hook whose stdin metaharness cannot read is still a hook that is waiting. It is presented
    /// with no tool name — which no frame admits — rather than dropped or guessed at.
    #[test]
    fn an_unreadable_hook_input_is_still_presented_and_names_no_tool() {
        let mut seam = seam();
        let emitted = seam.push_line(&hook_request_line("k1", "not json at all"));
        match &emitted[0].event {
            Event::ToolRequested { call_id, name, .. } => {
                assert_eq!(call_id, "k1");
                assert!(name.is_empty());
            }
            other => panic!("{other:?}"),
        }
    }

    /// A deny always carries a reason, because 0.145.0 rejects one that does not: the binary's own
    /// string is *"`PreToolUse` hook returned `permissionDecision:deny` without a non-empty
    /// `permissionDecisionReason`"*, and a rejected hook response is a guard that stopped guarding.
    #[test]
    fn a_deny_carries_the_non_empty_reason_this_vendor_requires() {
        let seam = seam();
        let line = seam.decision_line(
            "abcd1234",
            &Decision::Deny {
                reason: "this step admits no shell".to_string(),
            },
        );
        let value: Value = serde_json::from_str(&line).expect("parses");
        let output = &value["response"]["hookSpecificOutput"];
        assert_eq!(output["permissionDecision"], "deny");
        assert_eq!(
            output["permissionDecisionReason"],
            "this step admits no shell"
        );
    }

    /// A control this adapter has not driven reaches the child by no line at all, so it cannot
    /// appear to work: the capability set refuses it at run start instead.
    #[test]
    fn an_undriven_control_produces_no_line() {
        let seam = seam();
        assert!(
            seam.control_line(&Command::Steer {
                text: "x".to_string()
            })
            .is_none()
        );
        assert!(
            seam.control_line(&Command::MessageInject {
                text: "x".to_string()
            })
            .is_none()
        );
        assert!(
            seam.control_line(&Command::Halt {
                reason: String::new()
            })
            .is_some()
        );
    }
}
