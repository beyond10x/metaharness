//! Adversary pass 1 against design amendment a41 § 11 (`story:claude-hook-refusals-reach-the-record`,
//! `story:claude-seam-abstain-is-recorded-as-vendor-decided`).
//!
//! Same rig as `claude_hook_record.rs`: the builder's real Claude start, the real installed hook
//! program, the real spawn runner and the real Claude seam, with a `/bin/sh` fake vendor. No
//! vendor binary, no model, no network and no credential.
//!
//! 1. A decision the program never reports on is now held until the run winds up. The design's
//!    own words (§ 6.2, "Every mode emits `tool.decided`. The census in `session.ended` counts them
//!    all") and a41 § 11 ("a call's `tool.decided` still precedes its `tool.result`") both break on
//!    that path, which § 11 itself lists as handled (`HOOK_OUTCOME_UNSEEN`).
//! 2. `outcomes/` is written by whoever can write the run's scratch tree. An outcome file naming a
//!    call, dropped there while metaharness holds that call's decision, replaces the decision in
//!    the record with a deny, while the real program goes on to forward metaharness's allow and
//!    the call runs.
#![cfg(unix)]

use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecidedBy, Decision, DecisionMode, Digest, Event, EvidenceLine, Frame,
    Handoff, Kind, NodeRef, Operation, OperationSet, StepRef, SubjectScope, WorkflowRef,
};
use metaharness::{
    ClaudeSeams, HarnessProcess, Input, LaunchPlanView, Metaharness, ProcessRunner, Run,
    SpawnRunner,
};
use metaharness_claude::{HookChannelPaths, hook_program_path, shell_quote};
use serde_json::{Value, json};

const CALL: &str = "toolu_adversary_hook_rec_00000001";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vendor {
    /// The call and its result, and the hook is never run: what the vendor does for a call it
    /// answers before `PreToolUse`, or a hook it killed at its own timeout.
    NoHook,
    /// The call; then, while metaharness holds the decision, an outcome file naming the call is
    /// dropped into `outcomes/` by something other than the program; then the real hook runs.
    ForgedOutcome,
}

struct FakeClaude {
    vendor: Vendor,
    inner: SpawnRunner,
}

fn say(line: &Value) -> String {
    format!("printf '%s\\n' {}\n", shell_quote(&line.to_string()))
}

fn quoted(path: &Path) -> String {
    shell_quote(path.to_str().expect("a UTF-8 scratch path"))
}

fn hook_input() -> Value {
    json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Bash",
        "tool_input": {"command": "ls"},
        "session_id": "s-1",
        "tool_use_id": CALL,
    })
}

fn vendor_script(vendor: Vendor, scratch: &Path, channel: &HookChannelPaths) -> String {
    let version = metaharness_claude::PINNED_VERSIONS[0];
    let init = json!({
        "type": "system", "subtype": "init", "session_id": "s-1",
        "claude_code_version": version, "cwd": "/w", "apiKeySource": "none",
        "output_style": "default", "plugins": [], "mcp_servers": [], "tools": ["Bash"],
    });
    let call = json!({
        "type": "assistant",
        "message": {"id": "msg-1", "role": "assistant", "content": [
            {"type": "tool_use", "id": CALL, "name": "Bash", "input": {"command": "ls"}}
        ]},
    });
    let result = json!({
        "type": "user",
        "message": {"role": "user", "content": [
            {"type": "tool_result", "tool_use_id": CALL, "is_error": false,
             "content": "fake vendor result"}
        ]},
    });
    let end = json!({
        "type": "result", "subtype": "success", "is_error": false, "num_turns": 1,
        "total_cost_usd": 0,
    });
    let hook = quoted(&hook_program_path(scratch));
    let out = quoted(&scratch.join("hook-stdout"));
    let input = shell_quote(&hook_input().to_string());
    let outcomes = quoted(&channel.outcomes);

    let mut script = String::from("#!/bin/sh\nset -u\n");
    script.push_str(&say(&init));
    script.push_str(&say(&call));
    match vendor {
        Vendor::NoHook => {}
        Vendor::ForgedOutcome => {
            // Long enough for metaharness to read the call and hold its decision.
            script.push_str("sleep 0.5\n");
            writeln!(
                script,
                "printf '%s' {input} > {outcomes}/.forging\n\
                 mv -f {outcomes}/.forging {outcomes}/forged.no-channel"
            )
            .expect("a String takes a write");
            // Long enough for metaharness to read the forged outcome.
            script.push_str("sleep 0.5\n");
            writeln!(script, "printf '%s' {input} | {hook} > {out} 2>/dev/null")
                .expect("a String takes a write");
        }
    }
    script.push_str(&say(&result));
    script.push_str(&say(&end));
    script
}

impl ProcessRunner for FakeClaude {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, plan: &LaunchPlanView) -> io::Result<Box<dyn HarnessProcess>> {
        let channel = HookChannelPaths::at_root(plan.decision_channel);
        let scratch = plan
            .decision_channel
            .parent()
            .expect("the channel sits under the scratch root")
            .to_path_buf();
        let script = scratch.join("fake-claude.sh");
        std::fs::write(&script, vendor_script(self.vendor, &scratch, &channel))?;
        let args = vec![script.display().to_string()];
        let mut env = plan.env.clone();
        env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
        self.inner.start(&LaunchPlanView {
            program: "/bin/sh",
            args: &args,
            env: &env,
            ..*plan
        })
    }
}

fn frame_admitting_shell() -> Frame {
    Frame {
        workflow: WorkflowRef {
            id: "development/default".into(),
            version: "1".into(),
        },
        node: NodeRef {
            id: "implement".into(),
        },
        step: StepRef {
            workflow: "development/default".into(),
            state: "implement".into(),
            index: 1,
            attempt: 1,
        },
        prior: vec![EvidenceLine {
            text: "the specification is approved".into(),
            source: None,
        }],
        obligations: Vec::new(),
        reaching: Vec::new(),
        next: Vec::new(),
        handoff: Handoff::None,
        operations: OperationSet::of([Operation::Shell]),
        subjects: SubjectScope::default(),
        entities: None,
        digest: Digest::of(b""),
    }
}

struct Driven {
    run: Run,
    hook_stdout: String,
}

fn drive(vendor: Vendor) -> Driven {
    let builder = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting_shell());
    let mut runner = FakeClaude {
        vendor,
        inner: SpawnRunner::new(),
    };
    let mut run = builder
        .start_with(
            Input::Prompt("run ls".to_owned()),
            &mut runner,
            &mut ClaudeSeams,
        )
        .unwrap_or_else(|refusal| panic!("the run starts: {refusal}"));
    run.drain().expect("the run drains");
    let scratch: PathBuf = run.scratch_root().expect("a scratch root").to_path_buf();
    let hook_stdout = std::fs::read_to_string(scratch.join("hook-stdout")).unwrap_or_default();
    Driven { run, hook_stdout }
}

fn decisions(run: &Run) -> Vec<(Decision, DecidedBy)> {
    run.events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                call_id,
                decision,
                decided_by,
                ..
            } if call_id == CALL => Some((decision.clone(), *decided_by)),
            _ => None,
        })
        .collect()
}

fn kinds(run: &Run) -> Vec<&'static str> {
    run.events().iter().map(Event::name).collect()
}

/// Design § 6.2: "Every mode emits `tool.decided`. The census in `session.ended` counts them
/// all." A decision the hook program never reported on is recorded (a41 § 11's
/// `HOOK_OUTCOME_UNSEEN` row), so the census the terminal record carries must count it.
#[test]
fn the_census_in_session_ended_counts_a_decision_no_hook_program_reported_on() {
    let driven = drive(Vendor::NoHook);
    let run = &driven.run;
    assert_eq!(
        decisions(run),
        vec![(Decision::Allow, DecidedBy::Frame)],
        "precondition: the call's allow is recorded"
    );
    let census = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::SessionEnded { census, .. } => Some(census.clone()),
            _ => None,
        })
        .expect("a session.ended");
    assert_eq!(
        census.allowed,
        1,
        "session.ended's census omits the allow its own record holds: {census:?}; events: {:?}",
        kinds(run)
    );
}

/// a41 § 11: "So a call's `tool.decided` still precedes its `tool.result`." On the path the same
/// section names `HOOK_OUTCOME_UNSEEN`, the decision is held until wind-up and lands after the
/// call's result and after `session.ended`.
#[test]
fn a_decision_no_hook_program_reported_on_still_precedes_the_calls_result() {
    let driven = drive(Vendor::NoHook);
    let events = driven.run.events();
    let decided = events
        .iter()
        .position(|e| matches!(e, Event::ToolDecided { call_id, .. } if call_id == CALL))
        .expect("a tool.decided for the call");
    let result = events
        .iter()
        .position(|e| matches!(e, Event::ToolResult { call_id, .. } if call_id == CALL))
        .expect("a tool.result for the call");
    let ended = events
        .iter()
        .position(|e| matches!(e, Event::SessionEnded { .. }))
        .expect("a session.ended");
    assert!(
        decided < result && decided < ended,
        "tool.decided at {decided}, tool.result at {result}, session.ended at {ended}: {:?}",
        kinds(&driven.run)
    );
}

/// The program forwarded metaharness's allow and the call ran. An outcome file in `outcomes/`
/// that the program did not write must not turn the record into a deny for that call: the record
/// would then hide a call that ran, the opposite of the defect § 11 fixes.
#[test]
fn an_outcome_the_program_did_not_write_does_not_record_a_deny_for_a_call_it_forwarded() {
    let driven = drive(Vendor::ForgedOutcome);
    assert!(
        driven
            .hook_stdout
            .contains(r#""permissionDecision":"allow""#),
        "precondition: the real program forwarded the allow, so the call ran: {:?}",
        driven.hook_stdout
    );
    assert_eq!(
        decisions(&driven.run),
        vec![(Decision::Allow, DecidedBy::Frame)],
        "the record says what the vendor was not told; events: {:?}",
        kinds(&driven.run)
    );
}
