//! C4 — the paid tier for the codex adapter: one real session, with a deliberate denial in it.
//!
//! **This costs money and it is never part of `task check`.** Two gates stand in front of it and
//! both are deliberate:
//!
//! 1. `#[ignore]`, so `cargo test --workspace` reports it as ignored rather than running it.
//! 2. `METAHARNESS_LIVE=1`, so even `cargo test -- --ignored` bills nothing by accident.
//!
//! A skipped test that reported `ok` would be the same defect as an audit with no verdict rows, so
//! a skip says so on stderr and names the gate.
//!
//! ```text
//! METAHARNESS_LIVE=1 cargo test -p metaharness --test live_codex -- --ignored --nocapture
//! ```
//!
//! # The three facts only this tier can establish
//!
//! Everything else about this seam is proven free at C2 and C3. These three are not, and each is
//! asserted **from the run's own record** rather than from the configuration that was written:
//!
//! | # | fact | where the evidence comes from |
//! |---|---|---|
//! | a | the hook **fired**, and received the call | the raw request the hook process published into this run's own channel, on disk, with the vendor's own field names in it |
//! | b | the deny reached the child **before the effect** | the session rollout: the refusal is in it, the command's output is not |
//! | c | the rollout reader consumed the **real** record | the opening event carries what the retained file's own first line carries |
//!
//! Fact (a) matters more here than it would on the Claude adapter, because on codex a
//! misconfigured hook is **silently ignored**: `[hooks]` in `config.toml` drops an unrecognised
//! event key without failing the config load, and a hook in a fresh `CODEX_HOME` needs a trust it
//! cannot have. Either way the run comes back looking exactly like one in which nothing forbidden
//! was attempted. That is the failure this test exists to make impossible to mistake for success.

use std::path::Path;
use std::time::{Duration, Instant};

use metaharness::protocol::{
    Command, Decision, DecisionCensus, DecisionMode, Event, HermeticMode, Kind,
};
use metaharness::{Input, Metaharness, Run};

/// The gate. Returns `false` when this run must not spend money.
fn live() -> bool {
    if std::env::var("METAHARNESS_LIVE").as_deref() == Ok("1") {
        return true;
    }
    eprintln!(
        "skipped: this test starts a real, paid codex session. Set METAHARNESS_LIVE=1 to run it — \
         it is deliberately not part of `task check`."
    );
    false
}

/// The wall-clock ceiling on the paid run.
///
/// `codex exec` has **no turn ceiling** — the adapter refuses `--max-turns` by name rather than
/// pretending to enforce one — so the only bound on what this test can spend is a clock and a
/// `halt`. Without it, a model that answers a denial by trying the same command again has an
/// unbounded budget, and this file is where that would be discovered.
const CEILING: Duration = Duration::from_secs(150);

/// The string the prompt asks the model to echo, and the thing that must never appear as output.
const MARKER: &str = "metaharness-cx-m2-marker";

/// Drive the run to its end, denying every call the seam presents, and report what was presented.
fn deny_everything(run: &mut Run) -> Vec<(String, String)> {
    let mut denied: Vec<(String, String)> = Vec::new();
    let started = Instant::now();
    let mut halted = false;
    loop {
        let line = match run.next_event() {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => panic!("the live run broke: {error}"),
        };
        if let Event::ToolRequested {
            call_id,
            name,
            input,
            decision_required: true,
            ..
        } = &line.event
        {
            eprintln!("live codex: the seam is holding {name} ({call_id}): {input}");
            denied.push((call_id.clone(), name.clone()));
            let call_id = call_id.clone();
            run.send(Command::ToolDecide {
                call_id,
                decision: Decision::Deny {
                    reason: "this step admits no shell, so the command did not run".to_string(),
                },
            })
            .expect("the decision is accepted");
        }
        if started.elapsed() > CEILING && !halted {
            halted = true;
            eprintln!("live codex: the ceiling was reached, halting");
            run.send(Command::Halt {
                reason: "the live test's wall-clock ceiling".to_string(),
            })
            .expect("halt is honoured");
        }
    }
    denied
}

/// Fact (a): every request a `PreToolUse` process published into this run's own channel, verbatim.
///
/// Read off the channel and **not** off the config that declared the hook. On this vendor a hook
/// that was never trusted, or declared under a key the deserializer does not recognise, is dropped
/// in silence — producing a run that looks exactly like one in which nothing was attempted.
fn hook_requests(scratch: &Path) -> Vec<String> {
    let requests = scratch.join("hook-channel").join("requests");
    let published: Vec<String> = std::fs::read_dir(&requests)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().extension().is_some_and(|kind| kind == "json"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .collect();
    assert!(
        !published.is_empty(),
        "(a) no PreToolUse hook process ever published a request into {}. The seam was never \
         consulted, so this run proves nothing about it — check hook trust and the [hooks] table \
         in the scratch config.toml before reading anything else here",
        requests.display()
    );
    for request in &published {
        eprintln!("live codex (a) the hook received: {request}");
    }
    assert!(
        published
            .iter()
            .any(|request| request.contains("tool_name")),
        "(a) the hook fired but its payload names no tool: {published:?}"
    );
    published
}

/// Fact (b): metaharness denied, and the vendor's own record agrees nothing ran.
///
/// Two halves, and the second is the one that matters. The **positive** half is that the deny
/// reached the child at all; the **negative** half is that the effect did not land. A test with
/// only the first would pass while the command ran anyway and the model was told off about it
/// afterwards — which is an audit, not a control (design § 7.2, last row).
fn assert_the_deny_preceded_the_effect(
    denied: &[(String, String)],
    census: &DecisionCensus,
    rollout: &str,
) {
    assert!(
        !denied.is_empty(),
        "(b) no call was ever presented for a decision — a run in which nothing forbidden was \
         attempted audits nothing, which is the failure this case exists to avoid"
    );
    assert!(census.denied >= 1, "(b) the census disagrees: {census:?}");
    eprintln!("live codex (b) census: {census:?}");

    assert!(
        rollout.contains("blocked by PreToolUse hook")
            || rollout.contains("this step admits no shell"),
        "(b) nothing in the session record says the call was refused, so this run does not show \
         the deny reaching the child at all"
    );
    // The command was `echo <marker>`, so its output would be the marker followed by a newline,
    // inside a tool-output record. The prompt itself carries the bare marker, which is why the
    // newline is part of the needle and why only output records are searched.
    let outputs: Vec<&str> = rollout
        .lines()
        .filter(|line| line.contains("call_output") || line.contains("exec_command_end"))
        .collect();
    for output in &outputs {
        eprintln!("live codex (b) the record's output for this call: {output}");
    }
    let landed = format!(r"{MARKER}\n");
    assert!(
        !outputs.iter().any(|line| line.contains(&landed)),
        "(b) the command's own output is in the session record, so the effect landed despite the \
         deny"
    );
}

/// Fact (c): the reader consumed the **real** record, end to end.
///
/// The retained file has to open with the vendor's own first line — `session_meta`, whose primacy
/// the binary enforces with its `session_configured_not_first_event` error — the opening event the
/// reader produced has to carry what that line carries, and the reader has to have reached the
/// terminal record rather than stopping somewhere in the middle. A reader that had consumed an
/// empty file, or one it made itself, fails all three.
///
/// # H9 is answered here, and it is **not** asserted equal to the pin
///
/// The design says a version outside the adapter's pin is a `warning`, and a refusal only under
/// `--strict-version` (§ 8.4 O1). An earlier draft of this test asserted equality instead, and the
/// first live run failed on it — which is how the split below was found rather than assumed. So
/// what is asserted is what the design actually promises: the record **answers** the version
/// question (never `unk`), and when the answer is off the pin the reader **says so** in its own
/// stream.
fn assert_the_reader_consumed_the_real_record(run: &Run, rollout: &str) {
    let first = rollout.lines().next().unwrap_or_default();
    assert!(
        first.contains("session_meta"),
        "(c) the retained record does not open with the vendor's own session_meta line: {first}"
    );
    let opening = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted {
                harness_version,
                session_id,
                cwd,
                ..
            } => Some((harness_version.clone(), session_id.clone(), cwd.clone())),
            _ => None,
        })
        .expect("(c) the reader never produced a session.started from the rollout");
    eprintln!("live codex (c) opening record: {opening:?}");

    // H9 is answered from the record, which is the row's whole point: a missing version is `unk`
    // and never a pass.
    let version = opening
        .0
        .expect("(c) H9: the record names no version at all, which is unk and never a pass");
    if !metaharness_codex::PINNED_VERSIONS.contains(&version.as_str()) {
        let warned = run.events().iter().any(
            |event| matches!(event, Event::Warning { code, .. } if code == "version_outside_pin"),
        );
        assert!(
            warned,
            "(c) the record says {version}, the adapter pins {:?}, and the reader passed it over \
             in silence — an off-pin read must be visible as such rather than as a change in the \
             agent's behaviour",
            metaharness_codex::PINNED_VERSIONS
        );
        eprintln!(
            "live codex (c) NOTE: the driven binary records {version} while `codex --version` \
             reports {:?}. Two installs on one machine; the reader warned, as it must.",
            metaharness_codex::PINNED_VERSIONS
        );
    }

    // The session id the reader emitted is in the bytes it read — not merely self-consistent.
    let session_id = opening.1.expect("(c) the record carries no session id");
    assert!(
        rollout.contains(&session_id),
        "(c) the session id the reader emitted is not in the bytes it read: {session_id}"
    );
    assert!(opening.2.is_some(), "(c) H7: the record carries no cwd");

    // End to end: the reader reached the vendor's terminal record, not just its opening one.
    // The record's own tail is printed first, so a failure here is diagnosable from this run
    // rather than from another paid one.
    for line in rollout.lines().rev().take(2) {
        eprintln!("live codex (c) record tail: {line}");
    }
    assert!(
        run.saw_terminal_record(),
        "(c) the reader never reached a terminal record, so it consumed the rollout only as far \
         as it got — which is exit 3, nobody found out, and not a proof of anything"
    );
    eprintln!(
        "live codex (c) {} rollout lines retained, {} events produced",
        rollout.lines().count(),
        run.events().len()
    );
}

/// Drive the run to its end, **allowing** every call the seam presents, and report what was
/// presented.
///
/// The mirror of [`deny_everything`], and the only thing that can settle the grant half of this
/// wire: everything up to the hook's stdout is proven free at C3, and whether 0.145.0 *honours* an
/// `allow` is a fact about the vendor. Its binary carries a literal that would refuse one
/// (quoted in `metaharness_codex::render_hook_response`), so a green run here is news and a red one
/// is more so.
fn allow_everything(run: &mut Run) -> Vec<(String, String)> {
    let mut allowed: Vec<(String, String)> = Vec::new();
    let started = Instant::now();
    let mut halted = false;
    loop {
        let line = match run.next_event() {
            Ok(Some(line)) => line,
            Ok(None) => break,
            Err(error) => panic!("the live run broke: {error}"),
        };
        if let Event::ToolRequested {
            call_id,
            name,
            input,
            decision_required: true,
            ..
        } = &line.event
        {
            eprintln!("live codex: the seam is holding {name} ({call_id}): {input}");
            allowed.push((call_id.clone(), name.clone()));
            let call_id = call_id.clone();
            run.send(Command::ToolDecide {
                call_id,
                decision: Decision::Allow,
            })
            .expect("the decision is accepted");
        }
        if started.elapsed() > CEILING && !halted {
            halted = true;
            eprintln!("live codex: the ceiling was reached, halting");
            run.send(Command::Halt {
                reason: "the live test's wall-clock ceiling".to_string(),
            })
            .expect("halt is honoured");
        }
    }
    allowed
}

/// Live run: the deliberate **grant** — does an `allow` metaharness rendered actually let the call
/// run on codex?
///
/// This is the one claim the free tiers cannot reach, and the repository labels it as outstanding
/// everywhere it is mentioned until this test is spent. Three things have to be true, and the
/// third is the one that distinguishes "the vendor honoured the allow" from "the vendor ignored the
/// hook entirely":
///
/// | # | fact | evidence |
/// |---|---|---|
/// | a | the hook **fired** and received the call | the raw request published into this run's own channel |
/// | b | the call **ran** | the marker's own output is in the session rollout |
/// | c | the run **decided** it | the census counts an allow, and no denial appears in the record |
///
/// Fact (c) matters because a run in which the hook was never consulted also produces the marker:
/// that is the silent failure this vendor makes easy (an unrecognised `[hooks]` key is dropped
/// without failing the config load), and it would otherwise read exactly like success.
#[test]
#[ignore = "starts a real, paid codex session; run with METAHARNESS_LIVE=1 and --ignored"]
fn an_allowed_shell_call_runs_and_the_codex_record_shows_its_output() {
    if !live() {
        return;
    }
    let prompt = format!(
        "Run exactly this shell command and show me its output: echo {MARKER}. Use your shell \
         tool, do not answer from memory. Run it once and then stop."
    );
    let mut run = Metaharness::new(Kind::Codex)
        .with_hermetic(HermeticMode::On)
        .with_decisions(DecisionMode::Ask)
        .start(Input::Prompt(prompt))
        .expect("the run starts");

    let scratch = run
        .scratch_root()
        .expect("a live run owns a scratch root")
        .to_path_buf();

    let allowed = allow_everything(&mut run);
    let rollout = std::fs::read_to_string(scratch.join("rollout.jsonl")).unwrap_or_default();

    // (a) the seam was really consulted — read off the channel, never off the config.
    let published = hook_requests(&scratch);

    // (c) and metaharness is what admitted the call.
    assert!(
        !allowed.is_empty(),
        "(c) no call was ever presented for a decision, so this run says nothing about the grant \
         half: the marker below could have been produced by a run with no seam at all"
    );
    let census = run.census();
    assert!(
        census.allowed >= 1,
        "(c) the census counts no allow: {census:?}"
    );
    eprintln!("live codex (c) census: {census:?}");

    // (b) the effect landed. The command was `echo <marker>`, so its output is the marker followed
    // by a newline, inside a tool-output record — the exact needle the deny vector asserts is
    // **absent**, asserted present here. One wire, both polarities, one binary.
    let outputs: Vec<&str> = rollout
        .lines()
        .filter(|line| line.contains("call_output") || line.contains("exec_command_end"))
        .collect();
    for output in &outputs {
        eprintln!("live codex (b) the record's output for this call: {output}");
    }
    let landed = format!(r"{MARKER}\n");
    assert!(
        outputs.iter().any(|line| line.contains(&landed)),
        "(b) the allowed command's own output is not in the session record: either the vendor \
         discarded the allow — the reading the 0.145.0 literal \"PreToolUse hook returned \
         unsupported permissionDecision:allow\" would support — or the model never ran it. Read \
         the rollout before concluding either"
    );
    assert!(
        !rollout.contains("blocked by PreToolUse hook"),
        "(b) the record says a call was blocked, so this run did not exercise the grant half"
    );

    assert_the_reader_consumed_the_real_record(&run, &rollout);
    eprintln!(
        "live codex: {} calls allowed, {} hook requests published — the grant half of the wire is \
         driven, and every label that called it undriven can move",
        allowed.len(),
        published.len()
    );
}

/// Live run: the deliberate denial — does the seam really stop a call on codex?
#[test]
#[ignore = "starts a real, paid codex session; run with METAHARNESS_LIVE=1 and --ignored"]
fn a_denied_shell_call_never_runs_and_the_codex_record_says_so() {
    if !live() {
        return;
    }
    let prompt = format!(
        "Run exactly this shell command and show me its output: echo {MARKER}. Use your shell \
         tool, do not answer from memory. If the command is blocked, stop immediately and say it \
         was blocked — do not retry it and do not try another way."
    );
    let mut run = Metaharness::new(Kind::Codex)
        .with_hermetic(HermeticMode::On)
        // The embedder decides, and its policy is one line: nothing is admitted. A frame would say
        // the same thing; a policy says it without depending on the operation rendering, which is
        // the one thing about this vendor's tool names that is still unverified.
        .with_decisions(DecisionMode::Ask)
        .start(Input::Prompt(prompt))
        .expect("the run starts");

    let scratch = run
        .scratch_root()
        .expect("a live run owns a scratch root")
        .to_path_buf();

    let denied = deny_everything(&mut run);
    let rollout = std::fs::read_to_string(scratch.join("rollout.jsonl")).unwrap_or_default();

    let published = hook_requests(&scratch);
    assert_the_deny_preceded_the_effect(&denied, run.census(), &rollout);
    assert_the_reader_consumed_the_real_record(&run, &rollout);

    // What the run cost, in the only currency this vendor reports: tokens. Cost is **never**
    // emitted by codex — zero cost keys across 2,437 local rollouts — so nothing here invents one.
    for event in run.events() {
        if let Event::SessionEnded { usage, .. } = event {
            eprintln!("live codex: usage {usage:?} (this vendor emits no cost; derive it)");
        }
    }
    eprintln!(
        "live codex: {} calls presented, {} hook requests published",
        denied.len(),
        published.len()
    );
}
