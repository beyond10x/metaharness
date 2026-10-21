//! Adversary cases for `story:caller-supplied-catalogue-on-every-kind` (wave 5, unit 4).
//!
//! A run serving a caller's catalogue publishes no verbs (amendment a40), so a call named like the
//! catalogue questions (`tool_search`, `tool_describe`) is a tool nobody offered. The frame must
//! deny it like any other uncovered call. The only line that keeps it from the "listing is not an
//! act" admission is `self.publishes_verbs() &&` in `Run::decide` together with
//! `&& self.catalogue.is_none()` in `publishes_verbs`; the shipped end-to-end case calls
//! `tool_invoke`, `fs_stat` and `Read` and never a catalogue question, so dropping either guard
//! stays green there.
//!
//! Every start here hands in a scripted runner: no vendor process, no credential, no network, no
//! model.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, Decision, DecisionMode, Digest, Event, EvidenceLine, Frame, Handoff, Kind,
    NodeRef, Operation, OperationSet, StepRef, SubjectScope, WorkflowRef,
};
use metaharness::{
    ClaudeSeams, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner,
};
use metaharness_tools::CallerCatalogue;
use serde_json::{Value, json};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../metaharness-tools/tests/fixtures/caller-catalogue.yaml")
}

fn frame_admitting(operations: OperationSet) -> Frame {
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
            source: Some("docs/specs/one.md".into()),
        }],
        obligations: Vec::new(),
        reaching: Vec::new(),
        next: Vec::new(),
        handoff: Handoff::None,
        operations,
        subjects: SubjectScope::default(),
        entities: None,
        digest: Digest::of(b""),
    }
}

fn claude_stream(offered: &[String], calls: &[(&str, &str, Value)]) -> Vec<ScriptStep> {
    let init = json!({
        "type": "system",
        "subtype": "init",
        "version": "2.1.305",
        "cwd": "/w",
        "tools": offered,
        "mcp_servers": [{"name": "metaharness", "status": "connected"}],
    })
    .to_string();
    let mut script = vec![ScriptStep::line(init)];
    for (id, name, input) in calls {
        script.push(ScriptStep::line(
            json!({"type": "assistant", "message": {"content": [
                {"type": "tool_use", "id": id, "name": name, "input": input}
            ]}})
            .to_string(),
        ));
        script.push(ScriptStep::line(
            json!({"type": "user", "message": {"content": [
                {"type": "tool_result", "tool_use_id": id, "is_error": false}
            ]}})
            .to_string(),
        ));
    }
    script.push(ScriptStep::line(
        r#"{"type":"result","subtype":"success","is_error":false}"#,
    ));
    script
}

fn decided(events: &[Event]) -> Vec<Decision> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided { decision, .. } => Some(decision.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn under_a_catalogue_a_call_named_like_a_catalogue_question_is_denied_by_the_frame() {
    let caller = CallerCatalogue::read(&fixture()).expect("the fixture reads");
    let offered = caller.offered_tools();
    let calls = [
        (
            "toolu_1",
            "mcp__metaharness__tool_search",
            json!({"query": "read"}),
        ),
        (
            "toolu_2",
            "mcp__metaharness__tool_describe",
            json!({"name": "file_read"}),
        ),
    ];

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(claude_stream(&offered, &calls), log.clone());
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_catalogue(fixture())
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting(OperationSet::of([Operation::FileRead])))
        .start_with_clock(
            Input::Prompt("list the tools".to_owned()),
            &mut runner,
            &mut ClaudeSeams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");
    run.drain().expect("drains");

    let decisions = decided(run.events());
    assert_eq!(decisions.len(), 2, "{decisions:?}");
    for decision in &decisions {
        assert!(
            matches!(decision, Decision::Deny { .. }),
            "a tool the catalogue did not publish was decided {decision:?}: {decisions:?}"
        );
    }
}
