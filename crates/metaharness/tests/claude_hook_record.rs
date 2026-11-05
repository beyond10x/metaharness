//! What the Claude seam's hook program does with a call reaches the run's record (design amendment
//! a41 § 11; `story:claude-hook-refusals-reach-the-record`,
//! `story:claude-seam-abstain-is-recorded-as-vendor-decided`).
//!
//! Every run below goes through the builder's real Claude start, so the hook program is the one
//! the adapter renders and installs, and through the real spawn runner and the real Claude seam.
//! Only the vendor is fake: a `/bin/sh` script that prints stream-json and runs that installed
//! program the way the vendor does, with the hook input on its stdin. No vendor binary, no model,
//! no network and no credential.
//!
//! The program refuses a call on its own in three cases: it cannot reach its channel, metaharness
//! does not answer inside its budget, or the answer is not a hook decision. Each case below makes
//! the program take that path, with a frame that admits the call, so metaharness's own verdict is
//! `allow`, and the record must still say `deny`, with the program's cause, and never `allow`.
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
use metaharness_claude::{HOOK_WAIT_SECONDS, HookChannelPaths, hook_program_path, shell_quote};
use serde_json::{Value, json};

/// The call the fake vendor makes, and the id the hook input carries (row V22).
const CALL: &str = "toolu_hook_record_000000000001";

/// What the fake vendor does around its one call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Vendor {
    /// The call, then the hook, as the vendor orders them (row V23); metaharness answers.
    Answered,
    /// The call, then the request directory is gone, then the hook: the program cannot publish.
    NoChannel,
    /// The hook first, on a budget shortened to three tenths of a second, then the call: the
    /// program gives up before metaharness has seen the call, which is what a run loop that was
    /// not polled in time looks like from the program's side.
    Unanswered,
    /// The hook first, answered by somebody other than metaharness with plain text, then the call.
    Unreadable,
    /// The call, and the hook is never run.
    NoHook,
    /// No call at all.
    Silent,
}

/// The real spawn runner, starting the fake vendor in place of `claude`.
struct FakeClaude {
    vendor: Vendor,
    inner: SpawnRunner,
}

impl FakeClaude {
    fn new(vendor: Vendor) -> Self {
        Self {
            vendor,
            inner: SpawnRunner::new(),
        }
    }
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
            {"type": "tool_result", "tool_use_id": CALL, "is_error": vendor != Vendor::Answered,
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
    let run_hook = format!("printf '%s' {input} | {hook} > {out} 2>/dev/null\n");
    let requests = quoted(&channel.requests);
    let responses = quoted(&channel.responses);

    let mut script = String::from("#!/bin/sh\nset -u\n");
    script.push_str(&say(&init));
    match vendor {
        Vendor::Answered => {
            script.push_str(&say(&call));
            script.push_str(&run_hook);
        }
        Vendor::NoChannel => {
            script.push_str(&say(&call));
            writeln!(script, "rm -rf {requests}").expect("a String takes a write");
            script.push_str(&run_hook);
        }
        Vendor::Unanswered => {
            script.push_str(&run_hook);
            script.push_str(&say(&call));
        }
        Vendor::Unreadable => {
            write!(
                script,
                "printf '%s' {input} | {hook} > {out} 2>/dev/null &\n\
                 held=$!\n\
                 n=0\n\
                 while [ \"$n\" -lt 200 ]; do\n\
                 \x20   for request in {requests}/*.json; do\n\
                 \x20       if [ -e \"$request\" ]; then\n\
                 \x20           name=${{request##*/}}\n\
                 \x20           printf 'the call is fine\\n' > {responses}/.foreign.\"$name\"\n\
                 \x20           mv -f {responses}/.foreign.\"$name\" {responses}/\"$name\"\n\
                 \x20           n=200\n\
                 \x20       fi\n\
                 \x20   done\n\
                 \x20   if [ \"$n\" -lt 200 ]; then sleep 0.05; fi\n\
                 \x20   n=$((n + 1))\n\
                 done\n\
                 wait \"$held\"\n"
            )
            .expect("a String takes a write");
            script.push_str(&say(&call));
        }
        Vendor::NoHook => script.push_str(&say(&call)),
        Vendor::Silent => {}
    }
    if vendor != Vendor::Silent {
        script.push_str(&say(&result));
    }
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
        if self.vendor == Vendor::Unanswered {
            let hook = hook_program_path(&scratch);
            let program = std::fs::read_to_string(&hook)?;
            let budget = format!("\nTICKS={}\n", HOOK_WAIT_SECONDS * 10);
            assert!(program.contains(&budget), "the installed program's budget");
            std::fs::write(&hook, program.replace(&budget, "\nTICKS=3\n"))?;
        }
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

/// One drained run, with what the hook program printed to the vendor.
struct Driven {
    run: Run,
    hook_stdout: String,
}

fn drive(vendor: Vendor, framed: bool) -> Driven {
    let mut builder = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Frame);
    if framed {
        builder = builder.with_frame(frame_admitting_shell());
    }
    let mut runner = FakeClaude::new(vendor);
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

/// Every `tool.decided` the record holds for [`CALL`].
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

/// The message of every warning with this code.
fn warnings(run: &Run, code: &str) -> Vec<String> {
    run.events()
        .iter()
        .filter_map(|event| match event {
            Event::Warning { code: c, message } if c == code => Some(message.clone()),
            _ => None,
        })
        .collect()
}

/// The program refused the call itself: what it printed is a deny carrying `phrase`, and the
/// record holds exactly one decision for the call, that deny, with the program's own reason, and
/// a `HOOK_PROGRAM_REFUSED` warning naming the call and `cause`.
fn assert_program_refused(driven: &Driven, phrase: &str, cause: &str) {
    let run = &driven.run;
    assert!(
        driven
            .hook_stdout
            .contains(r#""permissionDecision":"deny""#)
            && driven.hook_stdout.contains(phrase),
        "precondition: the program itself refused ({cause}): {:?}",
        driven.hook_stdout
    );
    let decided = decisions(run);
    assert!(
        !decided
            .iter()
            .any(|(decision, _)| *decision == Decision::Allow),
        "{cause}: the record says allow for a call the hook program refused: {decided:?}"
    );
    assert_eq!(
        decided.len(),
        1,
        "{cause}: one decision for the call: {decided:?}"
    );
    let (decision, by) = &decided[0];
    let Decision::Deny { reason } = decision else {
        panic!("{cause}: the call's decision is a deny: {decision:?}");
    };
    assert!(
        reason.contains(phrase),
        "{cause}: the deny names the program's cause: {reason}"
    );
    assert_eq!(*by, DecidedBy::Adapter, "{cause}");
    let refused = warnings(run, "HOOK_PROGRAM_REFUSED");
    assert_eq!(refused.len(), 1, "{cause}: {:?}", run.events());
    assert!(
        refused[0].contains(CALL) && refused[0].contains(cause),
        "{cause}: {}",
        refused[0]
    );
}

#[test]
fn a_call_the_program_cannot_publish_is_recorded_as_its_deny_and_never_as_allow() {
    let driven = drive(Vendor::NoChannel, true);
    assert_program_refused(
        &driven,
        "could not be reached over its decision channel",
        "no-channel",
    );
}

#[test]
fn a_call_the_programs_backstop_refuses_is_recorded_as_its_deny_and_never_as_allow() {
    let driven = drive(Vendor::Unanswered, true);
    assert_program_refused(&driven, "did not answer this call within", "unanswered");
}

#[test]
fn a_call_whose_answer_the_program_cannot_read_is_recorded_as_its_deny_and_never_as_allow() {
    let driven = drive(Vendor::Unreadable, true);
    assert_program_refused(&driven, "not a hook decision", "unreadable");
}

/// The road nothing went wrong on is unchanged: the allow metaharness wrote is the allow the
/// program forwarded, recorded once, by the frame.
#[test]
fn an_allow_the_program_forwarded_is_recorded_as_the_frames_allow() {
    let driven = drive(Vendor::Answered, true);
    assert!(
        driven
            .hook_stdout
            .contains(r#""permissionDecision":"allow""#),
        "{:?}",
        driven.hook_stdout
    );
    assert_eq!(
        decisions(&driven.run),
        vec![(Decision::Allow, DecidedBy::Frame)]
    );
    assert_eq!(
        warnings(&driven.run, "HOOK_PROGRAM_REFUSED"),
        Vec::<String>::new()
    );
    assert_eq!(
        warnings(&driven.run, "HOOK_OUTCOME_UNSEEN"),
        Vec::<String>::new()
    );
}

/// A decision the program never said it delivered is still recorded, with a warning naming the
/// call, rather than presented as what the vendor was told.
#[test]
fn a_decision_no_hook_program_reported_on_is_recorded_with_a_warning_naming_the_call() {
    let driven = drive(Vendor::NoHook, true);
    assert_eq!(
        decisions(&driven.run),
        vec![(Decision::Allow, DecidedBy::Frame)]
    );
    let unseen = warnings(&driven.run, "HOOK_OUTCOME_UNSEEN");
    assert_eq!(unseen.len(), 1, "{:?}", driven.run.events());
    assert!(unseen[0].contains(CALL), "{}", unseen[0]);
}

/// An abstain is an empty answer, and the vendor's own permission pipeline decides the call. The
/// record names that pipeline as the call's decider, beside metaharness's own `abstain`.
#[test]
fn an_abstained_call_is_recorded_as_decided_by_the_vendor_pipeline() {
    let driven = drive(Vendor::Answered, false);
    assert!(
        driven.hook_stdout.is_empty(),
        "precondition: abstain prints nothing: {:?}",
        driven.hook_stdout
    );
    assert_eq!(
        decisions(&driven.run),
        vec![(Decision::Abstain, DecidedBy::Adapter)]
    );
    let vendor = warnings(&driven.run, "VENDOR_PIPELINE_DECIDES");
    assert_eq!(vendor.len(), 1, "{:?}", driven.run.events());
    assert!(
        vendor[0].contains(CALL) && vendor[0].contains("permission pipeline"),
        "{}",
        vendor[0]
    );
}

/// `--decisions frame` with no frame abstains on every call, and says so once, at the opening
/// record, whether or not a call is ever made.
#[test]
fn a_run_that_abstains_on_every_call_warns_once_at_start() {
    for vendor in [Vendor::Silent, Vendor::Answered] {
        let driven = drive(vendor, false);
        let events = driven.run.events();
        let at = |wanted: &dyn Fn(&Event) -> bool| events.iter().position(wanted);
        let started = at(&|e| matches!(e, Event::SessionStarted { .. })).expect("opened");
        let warned: Vec<usize> = events
            .iter()
            .enumerate()
            .filter(
                |(_, e)| matches!(e, Event::Warning { code, .. } if code == "NO_FRAME_IN_FORCE"),
            )
            .map(|(index, _)| index)
            .collect();
        assert_eq!(warned.len(), 1, "{vendor:?}: once: {events:?}");
        assert!(warned[0] > started, "{vendor:?}: after the opening record");
        if let Some(first_call) = at(&|e| matches!(e, Event::ToolRequested { .. })) {
            assert!(warned[0] < first_call, "{vendor:?}: before any call");
        }
        let Event::Warning { message, .. } = &events[warned[0]] else {
            unreachable!()
        };
        assert!(
            message.contains("every call") && message.contains("permission pipeline"),
            "{vendor:?}: {message}"
        );
    }
}
