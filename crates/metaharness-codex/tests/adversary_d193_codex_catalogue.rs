//! Adversary cases, wave D193 pass 1, unit `story:codex-catalogue-on-app-server` (design
//! amendment a66).
//!
//! The record lines below are the shapes the 0.160.0 binary wrote in the unit's own live run
//! (`docs/research/2026-10-29-codex-0.160.0-app-server-catalogue.md` § 1-2): a code-mode model
//! calls the catalogue from inside an `exec` cell, which the rollout records as a
//! `custom_tool_call` whose script is in `input`. Paths and ids are invented.

use metaharness_codex::{
    AppServerOpening, AppServerSession, CodexSeam, DynamicTool, SessionAction,
};
use metaharness_protocol::{
    Event, HarnessSeam, HermeticAttestation, HermeticMode, Seam, TranscriptRef,
};
use serde_json::{Value, json};

const META: &str = r#"{"timestamp":"2026-10-29T10:00:00.000Z","type":"session_meta","payload":{"id":"01a1-fixture","cli_version":"0.160.0","cwd":"/scratch/work","dynamic_tools":[{"type":"function","name":"fs_read","description":"Read a file.","inputSchema":{"type":"object"}},{"type":"function","name":"fs_stat","description":"Describe a path.","inputSchema":{"type":"object"}}]}}"#;
const TURN: &str = r#"{"timestamp":"2026-10-29T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"t1","model":"a-model","personality":"pragmatic"}}"#;
/// The code cell as the live rollout recorded it: `custom_tool_call`, name `exec`, the script in
/// `input` and no `arguments` key.
const CELL: &str = r#"{"timestamp":"2026-10-29T10:00:02.000Z","type":"response_item","payload":{"type":"custom_tool_call","status":"completed","call_id":"call_cell","name":"exec","input":"const r = await tools.fs_read({path:\"note.txt\"});text(r);\n"}}"#;
const SCRIPT: &str = "const r = await tools.fs_read({path:\"note.txt\"});text(r);\n";

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

fn events(lines: &[&str]) -> Vec<Event> {
    let mut seam = owned_seam();
    let mut out: Vec<Event> = Vec::new();
    for line in lines {
        out.extend(
            seam.push_line(line)
                .into_iter()
                .map(|emission| emission.event),
        );
    }
    out.extend(seam.finish().into_iter().map(|emission| emission.event));
    out
}

/// Invariant 3 and the `session.started.offered_tools` contract ("every tool the model was
/// offered"): on the app-server route the field is filled from `dynamic_tools`, while the same
/// record shows the model calling `exec`, a tool the field says it was never offered. A record
/// that says a tool was called and was not offered contradicts itself; the field should be absent
/// (as on every other Codex record, which lists no built-in) or name what was called.
#[test]
fn every_tool_the_record_shows_called_is_among_the_offered_tools() {
    let events = events(&[META, TURN, CELL]);
    let offered = events
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted { offered_tools, .. } => Some(offered_tools.clone()),
            _ => None,
        })
        .expect("session.started");
    let called: Vec<String> = events
        .iter()
        .filter_map(|event| match event {
            Event::ToolRequested { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(called, ["exec"], "the cell is recorded as a call");
    if let Some(offered) = offered {
        let unoffered: Vec<&String> = called
            .iter()
            .filter(|name| !offered.contains(name))
            .collect();
        assert!(
            unoffered.is_empty(),
            "session.started.offered_tools = {offered:?} claims the whole offer, and the same \
             record shows the model calling {unoffered:?}"
        );
    }
}

/// A code-mode cell is the only record of what the model ran, and every nested catalogue call
/// comes out of it. The rollout carries the script in `input`; the reader reads `arguments` only
/// and emits `input: ""`, which is what the live run's stream showed (seq 11).
#[test]
fn a_code_mode_cell_keeps_its_script_in_the_tool_requested_input() {
    let events = events(&[META, TURN, CELL]);
    let input = events
        .iter()
        .find_map(|event| match event {
            Event::ToolRequested { name, input, .. } if name == "exec" => Some(input.clone()),
            _ => None,
        })
        .expect("the cell is recorded");
    assert_eq!(
        input,
        Value::String(SCRIPT.to_owned()),
        "the cell's script was dropped from the record"
    );
}

fn opened_session() -> AppServerSession {
    let opening = AppServerOpening::new(
        std::path::Path::new("/scratch/work"),
        None,
        &[DynamicTool {
            name: "fs_read".to_owned(),
            description: "Read a file.".to_owned(),
            input_schema: json!({"type": "object"}),
        }],
        "read the readme",
    );
    let mut session = AppServerSession::new(opening);
    let _ = session.start();
    let _ = session.receive(&json!({"id": 1, "result": {"userAgent": "x"}}));
    let _ = session.receive(&json!({"id": 2, "result": {"thread": {"id": "th-main"}}}));
    assert_eq!(session.thread_id(), Some("th-main"));
    session
}

/// `turn/completed` closes the child's stdin, and the session takes the first one it sees from any
/// thread. A turn completing on another thread (a spawned agent's, which the live run's model was
/// offered `spawn_agent` for) would close stdin while this run's turn is still waiting on client
/// tool calls, which could then never be answered.
#[test]
fn a_turn_completed_on_another_thread_does_not_close_the_input() {
    let mut session = opened_session();
    let other = json!({"method": "turn/completed", "params": {"threadId": "th-other",
        "turn": {"id": "tu-other", "status": "completed"}}});
    assert_eq!(
        session.receive(&other),
        Vec::<SessionAction>::new(),
        "another thread's turn ended this run's input"
    );
    let own = json!({"method": "turn/completed", "params": {"threadId": "th-main",
        "turn": {"id": "tu-1", "status": "completed"}}});
    assert_eq!(session.receive(&own), [SessionAction::CloseInput]);
}
