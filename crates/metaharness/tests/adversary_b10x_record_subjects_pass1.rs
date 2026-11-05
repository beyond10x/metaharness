//! Adversary pass 1, wave of 2026-11-05, unit `story:b10x-record-keeps-the-loops-subjects`.
//!
//! The unit reads a b10x run on the surface its loop's `started` line published, and adds a rule
//! beyond its acceptance (design amendment a75 § 3, `Run::resolve`): on a run that published the
//! three verbs, a bare entry name "is a tool the loop did not publish" and names no operation and no
//! subject.
//!
//! The pinned loop does the opposite. At harness `3eca074` a bare entry name on the verbs surface
//! is a **route**, not a refusal: `Verbs::invoked_entry`
//! (`crates/harness-catalogue/src/verbs.rs:100-124`) answers `file_read {"path": …}` as the entry
//! itself, `Verbs::operation` and `Verbs::subjects_of` read through it, the loop performs the call
//! under the entry's own approval and says so with `unpublished-tool-routed`
//! (`crates/harness-loop/src/lib.rs:4320-4349`). So the loop's own `tool-requested` line for such a
//! call carries `"operation"` and `"subjects"` exactly as the flat line does, and the call runs.
//!
//! These cases feed the run that line, under the `started` publication `--surface verbs` writes,
//! through the production b10x seam. No vendor process, no credential, no network.

use metaharness::protocol::{
    CredentialSource, DecidedBy, Decision, DecisionMode, Digest, Event, Frame, Handoff, Kind,
    NodeRef, Operation, OperationSet, RunSpec, StepRef, SubjectRule, SubjectScope, WorkflowRef,
};
use metaharness::{Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner};

const GOLDEN: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");
const DENY_RECORD: &str = include_str!("../../metaharness-b10x/fixtures/caller/deny.jsonl");
const CALL: &str = "call_b10x_001";

const GOLDEN_PUBLICATION: &str = r#""published_tools":["file_read","dir_list","search","find"]"#;
const CALLER_PUBLICATION: &str =
    r#""published_tools":["file_read","dir_list","search","find","file_write","file_edit"]"#;
/// What `--surface verbs` publishes at the pin (`crates/harness-cli/src/lib.rs`, `Surface::Verbs`).
const VERBS_PUBLICATION: &str =
    r#""published_tools":["tool_search","tool_describe","tool_invoke"]"#;

/// The warning the pinned loop writes when it routes a bare entry name on the verbs surface.
fn routed_warning(entry: &str) -> String {
    format!(
        r#"{{"kind":"warning","code":"unpublished-tool-routed","message":"the model called `{entry}` directly; this run publishes it behind a verb, so the call was routed to that entry under the same gate"}}"#
    )
}

/// `record` with its `started` line publishing the verbs, and the routed warning written right
/// after its one `tool-requested` line, which is left as recorded: the verbs loop writes the same
/// `operation` and `subjects` for a routed bare name as the flat loop writes for the entry.
fn on_the_verbs_surface(record: &str, flat: &str, entry: &str) -> Vec<String> {
    assert!(record.contains(flat), "the record publishes the flat surface");
    record
        .replacen(flat, VERBS_PUBLICATION, 1)
        .lines()
        .flat_map(|line| {
            if line.contains(r#""kind":"tool-requested""#) {
                vec![line.to_owned(), routed_warning(entry)]
            } else {
                vec![line.to_owned()]
            }
        })
        .collect()
}

/// Observe: a bare `file_read` the verbs loop routed and performed is recorded as the read it was.
///
/// The record says `file.read` on `file:README.md` (the loop's own line, `operation` and
/// `subjects`), and the call completed. `Run::resolve` answers no operation for it, so the run's
/// `tool.requested` says a call happened that touched a file and was no operation at all.
#[test]
fn a_bare_entry_name_the_verbs_loop_routed_is_recorded_as_the_operation_it_performed() {
    let record = on_the_verbs_surface(GOLDEN, GOLDEN_PUBLICATION, "file_read");
    let mut runner = ScriptedRunner::of_lines(record, ScriptedLog::new());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut run = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("b10x-emulated")
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run plans on b10x");
    run.drain().expect("pumps");
    let recorded = run
        .events()
        .iter()
        .find_map(|event| match event {
            Event::ToolRequested {
                operations,
                subjects,
                ..
            } => Some((operations.clone(), subjects.clone())),
            _ => None,
        })
        .expect("the loop's call is recorded");
    assert_eq!(
        recorded,
        (
            vec!["file.read".to_owned()],
            vec!["file:README.md".to_owned()]
        ),
        "the verbs loop routed `file_read` to its entry and performed it (harness 3eca074 \
         `Verbs::invoked_entry`); the record names the operation it was"
    );
}

fn spec(decisions: DecisionMode) -> RunSpec {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.credentials = CredentialSource::None;
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

/// The caller `deny` recording on the verbs surface, blocking on stdin after its question as the
/// real loop does. Its call is a bare `file_write {"path": "note.md", …}`, which the verbs loop
/// routes and asks about under the entry's name (`approval-required` names `file_write`).
fn verbs_caller_script() -> Vec<ScriptStep> {
    let mut steps = Vec::new();
    for line in on_the_verbs_surface(DENY_RECORD, CALLER_PUBLICATION, "file_write") {
        let asks = line.contains("\"approval-required\"");
        steps.push(ScriptStep::line(line));
        if asks {
            steps.push(ScriptStep::awaiting(CALL));
        }
    }
    steps
}

/// The decisions a frame-mode run took over the verbs caller record, and its warnings.
fn frame_decides(frame: Frame) -> (Vec<Decision>, Vec<String>) {
    let log = ScriptedLog::new();
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("work");
    std::fs::create_dir(&cwd).unwrap();
    let mut runner = ScriptedRunner::new(verbs_caller_script(), log.clone());
    let mut seams = metaharness_b10x::B10xSeams::new(None, None);
    let mut run = Metaharness::from_spec(spec(DecisionMode::Frame))
        .with_frame(frame)
        .with_cwd(cwd)
        .start_with(
            Input::Prompt("write a note".to_owned()),
            &mut runner,
            &mut seams,
        )
        .expect("a frame admitting the reads starts");
    run.drain().expect("the run ends");
    let decided = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                decision,
                decided_by: DecidedBy::Frame,
                ..
            } => Some(decision.clone()),
            _ => None,
        })
        .collect();
    let warnings = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::Warning { code, .. } => Some(code.clone()),
            _ => None,
        })
        .collect();
    (decided, warnings)
}

/// Frame: a routed bare `file_write` that the frame admits is allowed, not refused as a tool
/// outside the vocabulary. The loop asked about `file_write`; the frame admits `file.write`.
#[test]
fn a_frame_admits_a_routed_write_on_a_verbs_b10x_run() {
    let mut operations = READS.to_vec();
    operations.push(Operation::FileWrite);
    let (decided, warnings) = frame_decides(frame(&operations, SubjectScope::default()));
    assert_eq!(
        decided,
        vec![Decision::Allow],
        "the loop asked about `file_write`, which this frame admits; warnings: {warnings:?}"
    );
    assert!(
        !warnings.iter().any(|code| code == "UNCOVERED_TOOL"),
        "{warnings:?}"
    );
}

/// Frame: the same routed write is judged on where it acts. A scope that admits writes everywhere
/// but on `note.md` refuses it for `note.md`, as it does on the flat surface
/// (`b10x_caller_seam.rs`, `a_subject_scope_holds_for_a_flat_write`). A fix that only restored the
/// operation would leave the subject empty, and an empty subject is never judged on where it acts.
#[test]
fn a_subject_scope_holds_for_a_routed_write_on_a_verbs_b10x_run() {
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
    let (decided, warnings) = frame_decides(frame(&operations, scope));
    assert_eq!(decided.len(), 1, "{decided:?} {warnings:?}");
    let Decision::Deny { reason } = &decided[0] else {
        panic!("the scope refuses the write: {decided:?}")
    };
    assert!(
        reason.contains("note.md"),
        "refused for where it acts, not for being unknown: {reason}"
    );
}
