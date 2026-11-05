//! Adversary cases, metaharness 0.29.0 wave, unit `story:catalogue-road-leftovers`, pass 1.
//!
//! The unit's CHANGELOG line and amendment a40's correction say that on a run serving a caller's
//! catalogue, "a call that resolves to no entry the run performs is no longer put to the embedder
//! under `--decisions ask`", "so the record can no longer hold an allow for a call the server
//! refused". `Run::outside_catalogue` decides "performs" from the harness mapping alone
//! (`CallerCatalogue::resolve_as`, through `harness_catalogue::harness_operation`). The server the
//! Codex app-server runner builds (`spawn_codex_app_server.rs`, `LocalOperations::unconfined(cwd,
//! Vec::new())`) publishes no `run` entry: harness says an empty program set "publishes no `run`
//! entry at all". So an entry whose contract name is `process.run` maps to a harness entry and is
//! performed by nothing in the run. These cases drive both halves: the server as the runner builds
//! it, and the run loop under `ask` with an embedder that allows whatever it is asked.
//!
//! Every start hands in a scripted runner: no vendor process, no credential, no network, no model.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    Command, CredentialSource, DecidedBy, Decision, DecisionMode, Event, Kind,
};
use metaharness::{Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner};
use metaharness_tools::{CallerCatalogue, Catalogue, EntryNaming, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../metaharness-tools/tests/fixtures/caller-catalogue.yaml")
}

/// The fixture moved to toolset `process` (the format names each operation `<toolset>.<op>`):
/// its read entry becomes `process.run`, published as `run_program`, an entry harness maps
/// (`process.run` → `run`); its stat entry becomes `process.stat`, which harness does not map.
/// Written to `dir`.
fn catalogue_with_a_run_entry(dir: &Path) -> PathBuf {
    let text = std::fs::read_to_string(fixture()).expect("the fixture reads");
    let text = text
        .replace("toolset: fs\n", "toolset: process\n")
        .replace("fs.", "process.")
        .replace("process.read", "process.run")
        .replace(
            "renderings: {claude: Read, codex: read_file, mcp: fs_read}",
            "renderings: {mcp: run_program}",
        );
    assert!(text.contains("name: process.run"), "the entry was replaced");
    assert!(
        text.contains("mcp: run_program"),
        "the rendering was replaced"
    );
    let path = dir.join("run-catalogue.yaml");
    std::fs::write(&path, text).expect("written");
    path
}

/// The Codex app-server stream: the opening listing the client tools, then one client call to
/// each of `calls`, then the turn's end.
fn codex_stream(offered: &[String], calls: &[(&str, &str, &str, Value)]) -> Vec<ScriptStep> {
    let meta = json!({"timestamp": "2026-10-29T10:00:00.000Z", "type": "session_meta",
        "payload": {"id": "th-fixture", "cli_version": "0.160.0", "cwd": "/w",
            "dynamic_tools": offered
                .iter()
                .map(|name| json!({"type": "function", "name": name, "description": "d",
                    "inputSchema": {"type": "object"}}))
                .collect::<Vec<_>>()}});
    let mut steps = vec![
        ScriptStep::line(meta.to_string()),
        ScriptStep::line(
            r#"{"timestamp":"2026-10-29T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"tu-1","model":"m","personality":"pragmatic"}}"#,
        ),
    ];
    for (key, call_id, tool, arguments) in calls {
        steps.push(ScriptStep::line(metaharness_codex::dynamic_tool_call_line(
            key,
            &json!({"threadId": "th-fixture", "turnId": "tu-1", "callId": call_id,
                "namespace": null, "tool": tool, "arguments": arguments}),
        )));
    }
    steps.push(ScriptStep::line(
        r#"{"timestamp":"2026-10-29T10:00:05.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"tu-1"}}"#,
    ));
    steps
}

/// The server the Codex app-server runner builds for a catalogue run.
fn runner_server(root: &Path, catalogue: CallerCatalogue) -> Server {
    Server::with_catalogue(
        Verbs::new(Catalogue::of(
            LocalOperations::unconfined(root, Vec::new()).expect("opens"),
        )),
        catalogue,
    )
}

/// Premise: the server the Codex runner builds performs nothing for a `process.run` entry.
/// Green: it states the condition the next case rests on.
#[test]
fn the_codex_runners_server_performs_nothing_for_a_process_run_entry() {
    let dir = tempfile::tempdir().expect("a directory");
    std::fs::write(dir.path().join("README.md"), "hello\n").expect("a file");
    let path = catalogue_with_a_run_entry(dir.path());
    let catalogue = CallerCatalogue::read(&path).expect("a process.run entry is a catalogue");
    assert!(
        matches!(
            catalogue.resolve_as("run_program", EntryNaming::Bare),
            metaharness_tools::Resolved::Operations(_)
        ),
        "the run loop counts the entry as performed"
    );
    let mut server = runner_server(dir.path(), catalogue);
    let answer = server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "run_program", "arguments": {"path": "README.md"}}}),
        )
        .expect("answered");
    eprintln!("the server's answer to run_program: {answer}");
    assert_eq!(
        answer["result"]["isError"], true,
        "the server performs nothing for the entry: {answer}"
    );
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(
        !text.contains("input schema does not admit"),
        "refused for the entry, not for its arguments: {text}"
    );
}

/// The unit's acceptance, driven on the entry the premise above shows nothing performs: under
/// `--decisions ask`, a call to it must not be put to the embedder, and the record must not hold
/// an allow for it. Red: `outside_catalogue` reads the harness mapping, not what the run's server
/// publishes, so the call is asked, the embedder's allow is recorded, and the server refuses.
#[test]
fn under_ask_an_entry_nothing_in_the_codex_run_performs_is_not_put_to_the_embedder() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = catalogue_with_a_run_entry(dir.path());
    let caller = CallerCatalogue::read(&path).expect("reads");
    let offered = caller.offered_tools_as(EntryNaming::Bare);
    assert_eq!(offered, ["run_program", "fs_stat"]);

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        codex_stream(
            &offered,
            &[
                ("rpc-1", "call-1", "fs_stat", json!({"path": "README.md"})),
                (
                    "rpc-2",
                    "call-2",
                    "run_program",
                    json!({"path": "README.md"}),
                ),
            ],
        ),
        log.clone(),
    );
    let mut run = Metaharness::new(Kind::Codex)
        .with_credentials(CredentialSource::None)
        .with_catalogue(path)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut metaharness::CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");

    let mut asked = Vec::new();
    while let Some(line) = run.next_event().expect("reads") {
        if let Event::ToolRequested {
            call_id,
            name,
            decision_required: true,
            ..
        } = line.event
        {
            asked.push(name);
            run.send(Command::ToolDecide {
                call_id,
                decision: Decision::Allow,
            })
            .expect("answered");
        }
    }
    let allowed: Vec<String> = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                call_id,
                decision: Decision::Allow,
                ..
            } => Some(call_id.clone()),
            _ => None,
        })
        .collect();
    assert_eq!(
        asked,
        Vec::<String>::new(),
        "only an entry the run performs is put to the embedder; `run_program` is performed by \
         nothing (the runner's server publishes no `run`), and the record now holds allowed for \
         {allowed:?}"
    );
    let denied_by_adapter = run.events().iter().any(|event| {
        matches!(
            event,
            Event::ToolDecided {
                call_id,
                decision: Decision::Deny { .. },
                decided_by: DecidedBy::Adapter,
                ..
            } if call_id == "call-2"
        )
    });
    assert!(
        denied_by_adapter,
        "the adapter denies the call it cannot perform"
    );
}

/// The unit's wording claim on the server's side: a refusal for an entry the catalogue publishes
/// and nothing performs names the catalogue, as the server's `fs.stat`-shaped refusal does
/// ("is in this run's catalogue as … and nothing in this run performs it"). Red: for a mapped
/// entry nothing performs, the refusal comes from the verb layer underneath, names harness's
/// `run`, and lists harness entry names the caller's catalogue never published.
#[test]
fn the_servers_refusal_for_an_entry_nothing_performs_names_the_catalogue() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = catalogue_with_a_run_entry(dir.path());
    let catalogue = CallerCatalogue::read(&path).expect("reads");
    let mut server = runner_server(dir.path(), catalogue);
    let answer = server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "run_program", "arguments": {"path": "README.md"}}}),
        )
        .expect("answered");
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text.contains("this run's catalogue"), "{text}");
    assert!(text.contains("run_program"), "{text}");
    assert!(
        !text.contains("file_read"),
        "a harness entry the catalogue never published is not offered: {text}"
    );
}

/// A call outside the catalogue that the vendor presents twice, byte for byte, under `ask`: the
/// record keeps both presentations and the census one deny. Green: it holds the census half of the
/// brief's interaction check on the kind that reaches the branch.
#[test]
fn under_ask_a_repeated_call_outside_the_catalogue_is_decided_once() {
    let caller = CallerCatalogue::read(&fixture()).expect("reads");
    let offered = caller.offered_tools_as(EntryNaming::Bare);
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(
        codex_stream(
            &offered,
            &[
                ("rpc-1", "call-1", "fs_stat", json!({"path": "README.md"})),
                ("rpc-1", "call-1", "fs_stat", json!({"path": "README.md"})),
            ],
        ),
        log.clone(),
    );
    let mut run = Metaharness::new(Kind::Codex)
        .with_credentials(CredentialSource::None)
        .with_catalogue(fixture())
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut metaharness::CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");
    run.drain().expect("drains");
    let requested = run
        .events()
        .iter()
        .filter(
            |event| matches!(event, Event::ToolRequested { call_id, .. } if call_id == "call-1"),
        )
        .count();
    let decided: Vec<&Decision> = run
        .events()
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided {
                call_id, decision, ..
            } if call_id == "call-1" => Some(decision),
            _ => None,
        })
        .collect();
    assert!(requested >= 1, "the call is in the record");
    assert_eq!(decided.len(), 1, "decided once: {decided:?}");
    assert!(matches!(decided[0], Decision::Deny { .. }), "{decided:?}");
}
