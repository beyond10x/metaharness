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
//! So the seam emits both and does not join them. Correlating them would mean matching on
//! arguments and timing, and a guard that guesses which call it is holding is worse than one that
//! says it does not know.
//!
//! **The `exec-` id is not the code-mode call's, though: it is the command's, and the command's
//! completion carries it (amendment a47).** Code mode mints a fresh `exec-<uuid>` for each tool the
//! script calls (`codex-rs/core/src/tools/code_mode/mod.rs:374` at `rust-v0.153.4`); the hook
//! receives it as `tool_use_id` and the turn's id as `turn_id` (`core/src/hook_runtime.rs:184-203`);
//! the command's `item_completed` in the rollout carries the same id as `item.id` and the same turn
//! id (`core/src/tools/events.rs:563-586`, `core/src/session/mod.rs:2439-2464`). The same code at
//! `rust-v0.160.0` is `core/src/tools/code_mode/delegate.rs:323`, `hook_runtime.rs:188-208`,
//! `events.rs:581-612` and `session/mod.rs:2606-2645`; the hook's `cwd` there comes from the
//! step's local environment, falling back to the turn's (`hook_runtime.rs:248-252`). So the seam
//! records a `Bash` hook request under its `tool_use_id`, and a completion no rollout call record
//! carries is correlated with it when both name the same turn. That is the vendor's id and the vendor's turn,
//! never order, text or output. The outer `call_*` record stays unjoined, and a rollout read
//! without the hook requests, as the viewer reads one, cannot make this join.
//!
//! What a **decision** is routed by is a third thing again: the hook process's own rendezvous name,
//! which is correct even for a payload that arrives with no id at all. The three are kept apart on
//! purpose, because collapsing any two of them would deliver a decision to the wrong blocked
//! process.
//!
//! **The two sources are merged in one order, and the opening record is first in it — except for
//! a hook request read before the rollout's `session_meta`, which comes out ahead of it.** 0.153.4
//! does not produce one: it writes `session_meta` before any provider request, so before any call a
//! hook could hold. In every driven session here the opening record came out first, on 0.160.0 as
//! on 0.153.4 (six driver runs each, `docs/research/2026-10-23-codex-0.160.0-record-surface.md`).
//! The exception predates amendment a34. The reader
//! holds `session.started` until the first `turn_context` completes it (amendment a34), and a hook
//! can publish before the tail has read that record: the hook channel is read before the rollout.
//! A blocked hook never waits behind the hold. Its request releases the opening record at once,
//! without the personality the record had not yet stated — so H1b reads `unk` on that run — and
//! the call follows it onto the run loop's ordinary path.

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

/// The value [`HOOK_ENVELOPE`] carries on a client tool call from the app-server route (design
/// amendment a66).
const DYNAMIC_TOOL_CALL: &str = "dynamic_tool_call";

/// The key a decision line for a client tool call carries, in place of `call_id`.
const DYNAMIC_DECISION: &str = "dynamic_call";

/// One `item/tool/call` from the app-server, as a line the seam will read (design amendment a66).
///
/// `params` is the request's `params` as the vendor sent them: `{threadId, turnId, callId,
/// namespace, tool, arguments}` (`app-server-protocol/src/protocol/v2/item.rs:1657` at
/// `rust-v0.160.0`). `key` is the name the decision is routed back under.
#[must_use]
pub fn dynamic_tool_call_line(key: &str, params: &Value) -> String {
    serde_json::json!({
        HOOK_ENVELOPE: DYNAMIC_TOOL_CALL,
        "key": key,
        "call": params,
    })
    .to_string()
}

/// The key and the decision a decision line for a client tool call carries, or `None` when the
/// line is not one (design amendment a66).
///
/// The other half of [`CodexSeam`]'s decision line, published here so the runner that answers
/// the call reads the envelope this crate wrote and no other party spells it.
#[must_use]
pub fn dynamic_decision(line: &Value) -> Option<(String, Decision)> {
    let key = line.get(DYNAMIC_DECISION)?.as_str()?.to_string();
    let decision = serde_json::from_value(line.get("decision")?.clone()).ok()?;
    Some((key, decision))
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
    /// Which client tool call is waiting under which key, on the app-server route (a66): the
    /// vendor's `callId` mapped to the name the runner answers the JSON-RPC request under.
    dynamic: BTreeMap<String, String>,
}

impl std::fmt::Debug for CodexSeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexSeam").finish_non_exhaustive()
    }
}

impl CodexSeam {
    /// A seam over the retained record and the attestation that goes into `session.started`.
    ///
    /// Under [`Seam::OwnedTool`], the app-server route (design amendment a66), the reader takes the
    /// opening record's `dynamic_tools` for the offered tools: that launch removed every built-in
    /// tool a config key reaches, so the client tools are the catalogue the model calls. A model
    /// catalogue can still add code mode's `exec` and `wait` and `request_user_input_async`, which
    /// the rollout does not list, so the offered tools name them after the catalogue from the
    /// launch's declaration ([`crate::APP_SERVER_VENDOR_TOOLS`], a66 § review corrections). Under
    /// any other seam the reader keeps `offered_tools` absent, as a rollout lists no built-in
    /// tools.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation, seam: Seam) -> Self {
        let reader = RolloutReader::new(transcript, attestation);
        Self {
            reader: if seam == Seam::OwnedTool {
                reader.on_the_app_server_route()
            } else {
                reader
            },
            seam,
            routes: BTreeMap::new(),
            dynamic: BTreeMap::new(),
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

    /// Every client tool call this seam is holding on the app-server route, as the vendor's
    /// `callId` → the key its answer is routed under (design amendment a66).
    #[must_use]
    pub fn dynamic_routes(&self) -> &BTreeMap<String, String> {
        &self.dynamic
    }

    /// The `tool.requested` a client tool call owes, or `None` when the line is not one (design
    /// amendment a66 § 4).
    ///
    /// Presented under the vendor's `callId`, which the rollout's `function_call` record for the
    /// same call carries too (`core/src/tools/handlers/dynamic.rs:174-205` passes the model's id
    /// through), and decision-bearing: the call waits for the answer metaharness writes.
    fn dynamic_tool_call(&mut self, line: &str) -> Option<Vec<Emission>> {
        let value = serde_json::from_str::<Value>(line).ok()?;
        if value.get(HOOK_ENVELOPE).and_then(Value::as_str) != Some(DYNAMIC_TOOL_CALL) {
            return None;
        }
        let key = value.get("key").and_then(Value::as_str)?.to_string();
        let call = value.get("call").cloned().unwrap_or(Value::Null);
        // The vendor's `callId`, unless this seam already holds a call under it: a reused id is
        // presented under its own key, so a decision for the earlier call is never routed to this
        // one (a66 § review corrections). The runner's keys are unique within the session.
        let call_id = call["callId"]
            .as_str()
            .filter(|id| !id.is_empty() && !self.dynamic.contains_key(*id))
            .map_or_else(|| key.clone(), ToString::to_string);
        self.dynamic.insert(call_id.clone(), key);
        Some(vec![Emission::untimed(Event::ToolRequested {
            // Resolved by the run, which holds the catalogue (design § 8.4 O6).
            subjects: Vec::new(),
            operations: Vec::new(),
            call_id,
            name: crate::app_server::dynamic_tool_name(&call),
            input: call
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
            decision_required: true,
            deadline_ms: None,
            seam: self.seam,
        })])
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
        // Amendment a47: the vendor's own id names the command's completion too. Recorded only
        // when the payload carried it; the rendezvous name below is metaharness's and no vendor
        // record can carry it.
        let invalidated = parsed
            .as_ref()
            .and_then(|parsed| {
                parsed.tool_use_id.as_deref().map(|id| {
                    self.reader.observe_hook_request(
                        id,
                        parsed.tool_name.as_deref().unwrap_or_default(),
                        parsed.turn_id.as_deref(),
                        &parsed.tool_input,
                    )
                })
            })
            .unwrap_or_default();
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
        let mut emissions = vec![Emission::untimed(Event::ToolRequested {
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
        })];
        emissions.extend(invalidated.into_iter().map(Emission::untimed));
        Some(emissions)
    }
}

impl HarnessSeam for CodexSeam {
    fn push_line(&mut self, line: &str) -> Vec<Emission> {
        if let Some(emissions) = self
            .dynamic_tool_call(line)
            .or_else(|| self.hook_request(line))
        {
            // A blocked hook never waits behind the held opening record (amendment a34): its
            // request releases the record at once, as it stands, and the call follows it onto the
            // run loop's ordinary path — admitted, decided or deadlined, abandoned on a halt.
            let mut released = self.reader.release_opening_record();
            released.extend(emissions);
            return released;
        }
        self.reader.push_line(line)
    }

    fn finish(&mut self) -> Vec<Emission> {
        // Never a call: the reader holds only the preamble's records, and a hook request is never
        // held. `Run::wind_up` puts these straight into the outbox, without admitting them.
        self.reader.finish()
    }

    fn set_census(&mut self, census: DecisionCensus) {
        self.reader.set_census(census);
    }

    fn decision_line(&self, call_id: &str, decision: &Decision) -> String {
        // A client tool call on the app-server route (a66) is answered by the runner, which
        // performs the call or refuses it as the decision says; the decision travels whole.
        if let Some(key) = self.dynamic.get(call_id) {
            return serde_json::json!({
                DYNAMIC_DECISION: key,
                "decision": decision,
            })
            .to_string();
        }
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

    // Synthetic Codex 0.153.4 terminal shapes reported in #11. These tests
    // exercise the reader without claiming a new verified compatibility pin.
    mod codex_terminal_regression {
        use super::*;

        const FAILURE: &str = r#"{"type":"event_msg","payload":{"type":"task_complete","error":{"message":"synthetic unsupported model"}}}"#;
        const SUCCESS: &str = r#"{"type":"event_msg","payload":{"type":"task_complete","error":null,"last_agent_message":"done"}}"#;
        const PARTIAL: &str = r#"{"type":"event_msg","payload":{"type":"agent_message","message":"starting the work"}}"#;

        fn terminal_error(lines: &[&str]) -> Option<bool> {
            let mut seam = seam();
            for line in lines {
                seam.push_line(line);
            }
            let emitted = seam.finish();
            assert_eq!(emitted.len(), 1, "one terminal event is owed");
            match &emitted[0].event {
                Event::SessionEnded { is_error, .. } => *is_error,
                other => panic!("expected session.ended, got {other:?}"),
            }
        }

        #[test]
        fn explicit_error_is_normalized_as_failure() {
            assert_eq!(terminal_error(&[FAILURE]), Some(true));
        }

        #[test]
        fn actual_success_is_normalized_as_success() {
            assert_eq!(terminal_error(&[SUCCESS]), Some(false));
        }

        #[test]
        fn preliminary_text_does_not_hide_a_later_failure() {
            assert_eq!(terminal_error(&[PARTIAL, FAILURE]), Some(true));
        }

        #[test]
        fn missing_terminal_evidence_does_not_invent_success() {
            let mut seam = seam();
            seam.push_line(PARTIAL);
            assert_eq!(seam.finish(), []);
        }

        #[test]
        fn missing_error_field_remains_unknown() {
            assert_eq!(
                terminal_error(&[r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#]),
                None
            );
        }

        #[test]
        fn legacy_final_message_is_positive_success_evidence() {
            assert_eq!(
                terminal_error(&[
                    r#"{"type":"event_msg","payload":{"type":"task_complete","last_agent_message":"done"}}"#
                ]),
                Some(false)
            );
        }

        #[test]
        fn malformed_or_empty_final_message_is_not_success() {
            for message in [
                serde_json::json!(false),
                serde_json::json!([]),
                serde_json::json!(""),
                serde_json::json!(null),
            ] {
                let line = serde_json::json!({"type":"event_msg","payload":{"type":"task_complete","last_agent_message":message}}).to_string();
                assert_eq!(terminal_error(&[&line]), None);
            }
        }

        #[test]
        fn every_non_null_error_overrides_a_final_message() {
            for error in [
                serde_json::json!({"message":"failed"}),
                serde_json::json!("failed"),
                serde_json::json!(false),
                serde_json::json!([]),
            ] {
                let line = serde_json::json!({"type":"event_msg","payload":{"type":"task_complete","error":error,"last_agent_message":"done"}}).to_string();
                assert_eq!(terminal_error(&[&line]), Some(true));
            }
        }

        #[test]
        fn failure_survives_a_later_ambiguous_or_successful_completion() {
            for later in [
                r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#,
                SUCCESS,
            ] {
                assert_eq!(terminal_error(&[FAILURE, later]), Some(true));
            }
        }

        #[test]
        fn dangling_new_turn_invalidates_prior_success() {
            let mut seam = seam();
            seam.push_line(SUCCESS);
            seam.push_line(
                r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"new"}}"#,
            );
            assert!(seam.finish().is_empty(), "the new turn never completed");
        }

        #[test]
        fn failure_survives_a_dangling_new_turn() {
            assert_eq!(
                terminal_error(&[
                    FAILURE,
                    r#"{"type":"event_msg","payload":{"type":"task_started","turn_id":"new"}}"#
                ]),
                Some(true)
            );
        }
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
                // rollout's own `call_id` for a code-mode call — driven, they differ — and this
                // adapter claims no such join. The command's completion carries it (a47).
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

    /// A hook can publish before the tail has read the `turn_context` that completes the opening
    /// record: the hook channel is read before the rollout. A blocked hook must never wait behind
    /// the hold (amendment a34), so its request releases the opening record at once — without the
    /// personality, which the record had not yet stated, so H1b reads `unk` — and the call goes on
    /// the run loop's ordinary path in the same breath. The `turn_context` that arrives afterwards
    /// completes nothing: the opening record has already gone out.
    #[test]
    fn a_hook_request_during_the_hold_releases_the_opening_record_at_once() {
        let mut seam = seam();
        assert_eq!(
            seam.push_line(
                r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"s","cli_version":"0.153.4","cwd":"/scratch/work"}}"#,
            ),
            []
        );
        let raw = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"call-7","tool_input":{"command":"ls"}}"#;
        let emitted = seam.push_line(&hook_request_line("k7", raw));
        let names: Vec<&str> = emitted.iter().map(|line| line.event.name()).collect();
        assert_eq!(names, ["session.started", "tool.requested"]);
        match &emitted[0].event {
            Event::SessionStarted { output_style, .. } => assert_eq!(*output_style, None),
            other => panic!("{other:?}"),
        }
        match &emitted[1].event {
            Event::ToolRequested {
                call_id,
                decision_required,
                seam,
                ..
            } => {
                assert_eq!(call_id, "call-7");
                assert!(decision_required);
                assert_eq!(*seam, Seam::Hook);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(seam.routes().get("call-7").map(String::as_str), Some("k7"));
        let later = seam.push_line(
            r#"{"timestamp":"2026-08-22T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"t1","personality":"pragmatic"}}"#,
        );
        let names: Vec<&str> = later.iter().map(|line| line.event.name()).collect();
        assert_eq!(names, ["opaque"], "the opening record went out once");
    }

    /// `Run::wind_up` puts what `finish` returns straight into the outbox, without `admit`, so
    /// `finish` must never return a call. Held records are only the preamble's — the opening
    /// record, its version warning, `turn.started`, `opaque` — and a hook request is never held.
    #[test]
    fn finish_never_returns_a_call() {
        let mut seam = seam();
        for line in [
            r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"s","cli_version":"0.999.0","cwd":"/scratch/work"}}"#,
            r#"{"timestamp":"2026-08-22T10:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#,
            r#"{"timestamp":"2026-08-22T10:00:00.002Z","type":"response_item","payload":{"type":"message","role":"developer","content":[]}}"#,
            r#"{"timestamp":"2026-08-22T10:00:00.003Z","type":"world_state","payload":{"full":true,"state":{}}}"#,
        ] {
            assert_eq!(seam.push_line(line), [], "held: {line}");
        }
        let emitted = seam.finish();
        let names: Vec<&str> = emitted.iter().map(|line| line.event.name()).collect();
        assert_eq!(
            names,
            [
                "session.started",
                "warning",
                "turn.started",
                "opaque",
                "opaque"
            ]
        );

        let mut called = seam_with_hook_during_hold();
        let names: Vec<&str> = called
            .finish()
            .iter()
            .map(|line| line.event.name())
            .collect();
        assert!(!names.contains(&"tool.requested"), "{names:?}");
    }

    /// A seam whose hook request arrived during the hold, with nothing after it.
    fn seam_with_hook_during_hold() -> CodexSeam {
        let mut seam = seam();
        seam.push_line(
            r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"s","cli_version":"0.153.4","cwd":"/scratch/work"}}"#,
        );
        let emitted = seam.push_line(&hook_request_line(
            "k1",
            r#"{"tool_name":"Bash","tool_use_id":"call-1","tool_input":{"command":"ls"}}"#,
        ));
        assert_eq!(
            emitted.len(),
            2,
            "the hook released the hold and was emitted"
        );
        seam
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
        // The call is the first record that is not the preamble's, so it also releases the held
        // opening record and its off-pin warning ahead of itself (amendment a34).
        let names: Vec<&str> = emitted.iter().map(|line| line.event.name()).collect();
        assert_eq!(names, ["session.started", "warning", "tool.requested"]);
        match &emitted[2].event {
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
                assert_eq!(name, "");
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

    fn owned_seam() -> CodexSeam {
        CodexSeam::new(
            TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            HermeticAttestation::none(HermeticMode::Off),
            Seam::OwnedTool,
        )
    }

    const DYNAMIC_META: &str = r#"{"timestamp":"2026-10-29T10:00:00.000Z","type":"session_meta","payload":{"id":"01a1-fixture","cli_version":"0.160.0","cwd":"/scratch/work","dynamic_tools":[{"type":"function","name":"fs_read","description":"Read a file.","inputSchema":{"type":"object"}},{"type":"function","name":"fs_stat","description":"Describe a path.","inputSchema":{"type":"object"}}]}}"#;
    const DYNAMIC_TURN: &str = r#"{"timestamp":"2026-10-29T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"t1","model":"a-model","personality":"pragmatic"}}"#;

    fn offered(seam: &mut CodexSeam) -> Option<Vec<String>> {
        let mut emitted = seam.push_line(DYNAMIC_META);
        emitted.extend(seam.push_line(DYNAMIC_TURN));
        emitted
            .iter()
            .find_map(|emission| match &emission.event {
                Event::SessionStarted { offered_tools, .. } => Some(offered_tools.clone()),
                _ => None,
            })
            .expect("session.started")
    }

    /// Amendment a66 § 5: on the app-server route the opening record's `dynamic_tools` are the
    /// offered tools, because that launch removed every built-in a config key reaches, followed by
    /// the vendor tools the launch declares no config removes (a66 § review corrections).
    #[test]
    fn on_the_owned_tool_seam_the_dynamic_tools_are_the_offered_tools() {
        assert_eq!(
            offered(&mut owned_seam()),
            Some(
                [
                    "fs_read",
                    "fs_stat",
                    "exec",
                    "wait",
                    "request_user_input_async"
                ]
                .map(str::to_owned)
                .to_vec()
            ),
            "story:codex-catalogue-on-app-server: the catalogue, then the declared vendor tools"
        );
    }

    /// a66 § review corrections: a `callId` the seam already holds is presented under the
    /// runner's key, so a decision for the first call is never routed to the second.
    #[test]
    fn a_reused_call_id_is_presented_under_its_own_key() {
        let mut seam = owned_seam();
        let params = |path: &str| {
            serde_json::json!({"threadId": "th-1", "turnId": "tu-1", "callId": "call-7",
                "namespace": null, "tool": "fs_read", "arguments": {"path": path}})
        };
        let ids: Vec<String> = [
            dynamic_tool_call_line("rpc-1", &params("README.md")),
            dynamic_tool_call_line("rpc-2", &params("secret.txt")),
        ]
        .iter()
        .flat_map(|line| seam.push_line(line))
        .filter_map(|emission| match emission.event {
            Event::ToolRequested { call_id, .. } => Some(call_id),
            _ => None,
        })
        .collect();
        assert_eq!(ids, ["call-7", "rpc-2"]);
        assert_eq!(seam.dynamic_routes()["call-7"], "rpc-1");
        assert_eq!(seam.dynamic_routes()["rpc-2"], "rpc-2");
    }

    /// Under the hook seam the same record says nothing about the built-ins the exec route
    /// offers, so the field stays absent.
    #[test]
    fn on_the_hook_seam_dynamic_tools_do_not_become_the_offered_tools() {
        assert_eq!(offered(&mut seam()), None);
    }

    /// Amendment a66 § 4: a client tool call is decision-bearing, under the vendor's `callId`,
    /// and the decision line names the key the runner answers it under.
    #[test]
    fn a_client_tool_call_is_held_under_its_call_id_and_answered_under_its_key() {
        let mut seam = owned_seam();
        let params = serde_json::json!({"threadId": "th-1", "turnId": "tu-1", "callId": "call-7",
            "namespace": null, "tool": "fs_read", "arguments": {"path": "README.md"}});
        let emitted = seam.push_line(&dynamic_tool_call_line("rpc-0", &params));
        let [emission] = emitted.as_slice() else {
            panic!("{emitted:?}");
        };
        let Event::ToolRequested {
            call_id,
            name,
            input,
            decision_required,
            seam: carried,
            ..
        } = &emission.event
        else {
            panic!("{emission:?}");
        };
        assert_eq!(call_id, "call-7");
        assert_eq!(name, "fs_read");
        assert_eq!(input, &serde_json::json!({"path": "README.md"}));
        assert!(decision_required);
        assert_eq!(*carried, Seam::OwnedTool);
        assert_eq!(seam.dynamic_routes()["call-7"], "rpc-0");

        let deny = Decision::Deny {
            reason: "outside the frame".to_owned(),
        };
        let line: Value = serde_json::from_str(&seam.decision_line("call-7", &deny)).expect("JSON");
        assert_eq!(dynamic_decision(&line), Some(("rpc-0".to_owned(), deny)));
        let hook: Value = serde_json::from_str(&seam.decision_line("call-other", &Decision::Allow))
            .expect("JSON");
        assert_eq!(dynamic_decision(&hook), None, "a call no client tool holds");
    }
}
