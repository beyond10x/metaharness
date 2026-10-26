//! The b10x caller seam through the public builder (atlas ADR 0104 point 3, design amendment a61).
//!
//! The loop lines are the released `b10x-harness 0.20.0`'s own record of one `--approve caller`
//! exchange against harness's deterministic emulator (`metaharness-b10x/fixtures/caller/`), fed to
//! the run by a scripted process that blocks where the real loop blocks: on stdin, after
//! `approval-required`, until a line for that call is written.

use metaharness::protocol::{
    DecidedBy, Decision, DecisionMode, Digest, Event, Frame, Handoff, Kind, NodeRef, Operation,
    OperationSet, RunSpec, Seam, StepRef, SubjectRule, SubjectScope, WorkflowRef,
};
use metaharness::{Input, Metaharness, Refusal, Run, ScriptStep, ScriptedLog, ScriptedRunner};

const DENY_RECORD: &str = include_str!("../../metaharness-b10x/fixtures/caller/deny.jsonl");
const CALL: &str = "call_b10x_001";

fn spec(decisions: DecisionMode) -> RunSpec {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.credentials = metaharness::protocol::CredentialSource::None;
    spec.decisions = decisions;
    spec.model = Some("b10x-emulated".to_owned());
    spec.model_endpoint = Some("http://fixture.invalid".to_owned());
    spec
}

fn frame(operations: &[Operation], subjects: SubjectScope) -> Frame {
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
        prior: Vec::new(),
        obligations: Vec::new(),
        reaching: Vec::new(),
        next: Vec::new(),
        handoff: Handoff::None,
        operations: OperationSet::of(operations.iter().cloned()),
        subjects,
        entities: None,
        digest: Digest::of(b""),
    }
}

const READS: [Operation; 3] = [Operation::FileRead, Operation::DirList, Operation::Search];

/// The recorded loop up to its question, a block until it is answered, then the rest. The lines
/// after the answer are the deny recording's; the run reads them whatever it answered, which is
/// what the disagreement check exists to catch.
fn script() -> Vec<ScriptStep> {
    let mut steps = Vec::new();
    for line in DENY_RECORD.lines() {
        steps.push(ScriptStep::line(line));
        if line.contains("\"approval-required\"") {
            steps.push(ScriptStep::awaiting(CALL));
        }
    }
    steps
}

fn start(
    builder: Metaharness,
    log: &ScriptedLog,
    root: &tempfile::TempDir,
) -> Result<Run, Refusal> {
    let cwd = root.path().join("work");
    std::fs::create_dir(&cwd).unwrap();
    let mut runner = ScriptedRunner::new(script(), log.clone());
    let mut seams = metaharness_b10x::B10xSeams::new(None, None);
    builder.with_cwd(cwd).start_with(
        Input::Prompt("write a note".to_owned()),
        &mut runner,
        &mut seams,
    )
}

fn decided(run: &Run) -> Vec<(Decision, DecidedBy, Seam)> {
    run.events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                decision,
                decided_by,
                seam,
                ..
            } => Some((decision.clone(), *decided_by, *seam)),
            _ => None,
        })
        .collect()
}

fn warnings(run: &Run) -> Vec<String> {
    run.events()
        .iter()
        .filter_map(|event| match event {
            Event::Warning { code, .. } => Some(code.clone()),
            _ => None,
        })
        .collect()
}

/// `frame`: the frame admits the reads and not `file.write`, so the write the loop asks about is
/// refused by the frame, the refusal is the line the loop reads, and the loop's own outcome agrees.
#[test]
fn a_frame_refuses_a_write_the_loop_asks_about_on_its_stdin() {
    let log = ScriptedLog::new();
    let root = tempfile::tempdir().unwrap();
    let mut run = start(
        Metaharness::from_spec(spec(DecisionMode::Frame))
            .with_frame(frame(&READS, SubjectScope::default())),
        &log,
        &root,
    )
    .expect("a frame admitting the reads starts");
    run.drain().expect("the run ends");

    let argv = log.launched().pop().expect("launched");
    let at = argv.iter().position(|w| w == "--approve-up-to").unwrap();
    assert_eq!(argv[at + 1], "low");
    assert!(
        argv.windows(2).any(|w| w == ["--approve", "caller"]),
        "{argv:?}"
    );

    let written = log.written();
    assert_eq!(written.len(), 1, "{written:?}");
    let answer: serde_json::Value = serde_json::from_str(&written[0]).unwrap();
    assert_eq!(answer["call_id"], CALL);
    assert_eq!(answer["decision"], "deny");
    assert!(
        answer["reason"].as_str().unwrap().contains("file_write"),
        "{answer}"
    );

    let decided = decided(&run);
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert!(matches!(decided[0].0, Decision::Deny { .. }));
    assert_eq!(decided[0].1, DecidedBy::Frame);
    assert_eq!(decided[0].2, Seam::ControlRequest);
    assert!(
        !warnings(&run)
            .iter()
            .any(|code| code == "approval-disagreed"),
        "{:?}",
        warnings(&run)
    );
}

/// `ask`: the embedder's answer is the line the loop reads. Here it allows, and the recorded loop
/// (the deny recording) then says it denied — which the run reports as a disagreement rather than
/// recording the embedder's decision as having held.
#[test]
fn an_embedders_answer_reaches_the_loop_and_a_contrary_outcome_is_named() {
    let log = ScriptedLog::new();
    let root = tempfile::tempdir().unwrap();
    let mut run = start(Metaharness::from_spec(spec(DecisionMode::Ask)), &log, &root)
        .expect("ask needs no frame");
    loop {
        let line = run
            .next_event()
            .expect("polled")
            .expect("the question comes");
        if line.event.name() == "tool.requested" {
            break;
        }
    }
    run.send_line(format!(
        r#"{{"format":"metaharness.command/1","id":"d1","command":"tool.decide","call_id":"{CALL}","decision":{{"decision":"allow"}}}}"#
    ))
    .expect("answered");
    run.drain().expect("the run ends");

    assert_eq!(
        log.written(),
        vec![format!(r#"{{"call_id":"{CALL}","decision":"allow"}}"#)]
    );
    let decided = decided(&run);
    assert_eq!(decided.len(), 1, "{decided:?}");
    assert_eq!(decided[0].1, DecidedBy::Embedder);
    assert!(
        warnings(&run)
            .iter()
            .any(|code| code == "approval-disagreed"),
        "{:?}",
        warnings(&run)
    );
}

/// A subject rule is judged on a flat entry's own arguments: the frame admits writes, its scope
/// does not admit them on `note.md`, and the write the loop asks about is refused for where it
/// acts.
#[test]
fn a_subject_scope_holds_for_a_flat_write() {
    let mut operations = READS.to_vec();
    operations.push(Operation::FileWrite);
    let scope = SubjectScope {
        rules: vec![
            SubjectRule {
                subjects: vec!["file:note.md".to_owned()],
                operations: OperationSet::of(READS.iter().cloned()),
            },
            SubjectRule {
                subjects: vec!["**".to_owned()],
                operations: OperationSet::of(operations.iter().cloned()),
            },
        ],
    };
    let log = ScriptedLog::new();
    let root = tempfile::tempdir().unwrap();
    let mut run = start(
        Metaharness::from_spec(spec(DecisionMode::Frame)).with_frame(frame(&operations, scope)),
        &log,
        &root,
    )
    .expect("starts");
    run.drain().expect("the run ends");
    let decided = decided(&run);
    assert_eq!(decided.len(), 1, "{decided:?}");
    let Decision::Deny { reason } = &decided[0].0 else {
        panic!("the scope refuses the write: {decided:?}")
    };
    assert!(reason.contains("note.md"), "{reason}");
    let subjects: Vec<Vec<String>> = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolRequested { subjects, .. } => Some(subjects.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(subjects, vec![vec!["file:note.md".to_owned()]]);
}

/// The frames the caller seam cannot hold the loop to are refused before anything is spawned.
#[test]
fn a_frame_the_loop_would_not_be_held_to_is_refused_before_the_spawn() {
    let without_list = frame(
        &[Operation::FileRead, Operation::Search],
        SubjectScope::default(),
    );
    let narrowing = frame(
        &READS,
        SubjectScope {
            rules: vec![SubjectRule {
                subjects: vec!["file:secret/**".to_owned()],
                operations: OperationSet::of([Operation::FileRead]),
            }],
        },
    );
    for (builder, words) in [
        (
            Metaharness::from_spec(spec(DecisionMode::Frame)),
            "needs a frame",
        ),
        (
            Metaharness::from_spec(spec(DecisionMode::Frame)).with_frame(without_list),
            "dir.list",
        ),
        (
            Metaharness::from_spec(spec(DecisionMode::Ask)).with_frame(narrowing),
            "subject rule",
        ),
    ] {
        let log = ScriptedLog::new();
        let root = tempfile::tempdir().unwrap();
        let refusal = start(builder, &log, &root).expect_err("refused");
        assert!(refusal.to_string().contains(words), "{refusal}");
        assert_eq!(log.spawns(), 0, "a refused run spawns nothing");
    }
}

/// Observe is unchanged: the old ceiling, no caller, nothing written, every call on no seam.
#[test]
fn observe_launches_and_records_as_before() {
    let log = ScriptedLog::new();
    let root = tempfile::tempdir().unwrap();
    let mut run = start(
        Metaharness::from_spec(spec(DecisionMode::Observe)),
        &log,
        &root,
    )
    .expect("observe starts");
    let argv = log.launched().pop().expect("launched");
    let at = argv.iter().position(|w| w == "--approve-up-to").unwrap();
    assert_eq!(argv[at + 1], "high");
    assert!(!argv.iter().any(|w| w == "caller"), "{argv:?}");
    // The observe loop never asks, so the recorded question is answered by nobody; the scripted
    // child would wait for good, and the run is stopped where it waits.
    while let Ok(Some(_)) = run.next_event() {}
    assert!(log.written().is_empty(), "{:?}", log.written());
    assert_eq!(decided(&run).len(), 0, "{:?}", decided(&run));
    assert!(run.events().iter().all(|event| !matches!(
        event,
        Event::ToolRequested { seam, .. } if *seam != Seam::None
    )));
}
