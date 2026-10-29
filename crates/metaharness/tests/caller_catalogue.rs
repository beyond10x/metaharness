//! `story:caller-supplied-catalogue-on-every-kind` (design amendment a40): a caller's
//! `tool-contract/1` catalogue is read once before anything runs, carried on Claude and, since
//! amendment a66, on Codex over `codex app-server`, and refused by name on b10x.
//!
//! Every start here hands in a scripted runner: no vendor process, no credential, no network, no
//! model. The Claude run below reads the vendor's own stream-json shape through the adapter's real
//! seam, written by the test (decision D2).

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, Decision, DecisionMode, Digest, Event, EvidenceLine, Frame, Handoff, Kind,
    NodeRef, Operation, OperationSet, RunSpec, StepRef, SubjectScope, ToolSurface, WorkflowRef,
};
use metaharness::{
    CATALOGUE_MALFORMED, CATALOGUE_NAME_DUPLICATE, CATALOGUE_NEEDS_OWNED_SURFACE,
    CATALOGUE_NOT_CARRIED, CATALOGUE_NOT_TOOL_CONTRACT, CATALOGUE_UNREADABLE,
    CATALOGUE_WITH_ALLOW_PROGRAM, ClaudeSeams, Input, ManualClock, Metaharness, Refusal,
    ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams, check_spec, warning,
};
use metaharness_tools::{CallerCatalogue, Catalogue, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../metaharness-tools/tests/fixtures/caller-catalogue.yaml")
}

fn written(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, text).expect("a catalogue file");
    path
}

fn builder(kind: Kind) -> Metaharness {
    let builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    match kind {
        Kind::B10x => builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called"),
        Kind::Claude | Kind::Codex => builder,
    }
}

fn refused_before_the_spawn(builder: Metaharness) -> Refusal {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let started = builder.start_with(
        Input::Prompt("write the failing test".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    );
    let Err(refusal) = started else {
        panic!("the run started");
    };
    assert_eq!(log.spawns(), 0, "{refusal}: refused before the spawn");
    assert!(log.launched().is_empty(), "{refusal}: nothing was launched");
    refusal
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

#[test]
fn the_declaration_on_the_builder_implies_the_owned_surface() {
    let spec = Metaharness::new(Kind::Claude)
        .with_catalogue(fixture())
        .spec()
        .clone();
    assert_eq!(spec.catalogue.as_deref(), Some(fixture().as_path()));
    assert_eq!(spec.tool_surface, ToolSurface::Owned);
    assert_eq!(check_spec(&spec), Ok(()));
}

#[test]
fn a_catalogue_with_the_native_surface_named_is_refused_rather_than_one_ignored() {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.catalogue = Some(fixture());
    spec.tool_surface = ToolSurface::Native;
    let refusal = check_spec(&spec).expect_err("the two declarations disagree");
    assert_eq!(refusal, Refusal::CatalogueNeedsOwnedSurface);
    assert!(
        refusal
            .to_string()
            .starts_with(&format!("{CATALOGUE_NEEDS_OWNED_SURFACE}: ")),
        "{refusal}"
    );

    let refusal = refused_before_the_spawn(
        builder(Kind::Claude)
            .with_catalogue(fixture())
            .with_tool_surface(ToolSurface::Native),
    );
    assert_eq!(refusal, Refusal::CatalogueNeedsOwnedSurface);
}

/// Only b10x still refuses a catalogue: Codex carries one over `codex app-server` since
/// amendment a66 (`story:codex-catalogue-on-app-server`).
#[test]
fn b10x_refuses_a_catalogue_by_name_before_the_spawn() {
    let kind = Kind::B10x;
    let refusal = refused_before_the_spawn(builder(kind).with_catalogue(fixture()));
    assert_eq!(
        refusal,
        Refusal::CatalogueNotCarried { kind },
        "story:codex-catalogue-on-app-server: b10x alone refuses a catalogue"
    );
    let words = refusal.to_string();
    assert!(
        words.starts_with(&format!("{CATALOGUE_NOT_CARRIED}: ")),
        "{words}"
    );
    assert!(words.contains("--catalogue"), "{words}");
    assert_eq!(
        refusal.code(),
        None,
        "a launch-condition code, not a protocol one"
    );
}

#[test]
fn the_kind_refusal_names_the_catalogue_and_not_the_surface_it_implied() {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.catalogue = Some(fixture());
    spec.tool_surface = ToolSurface::Owned;
    assert_eq!(
        check_spec(&spec),
        Err(Refusal::CatalogueNotCarried { kind: Kind::B10x }),
        "story:codex-catalogue-on-app-server: b10x alone refuses a catalogue"
    );

    // Codex admits the catalogue and the owned surface it implies, and still refuses the bare
    // owned surface, which would put the three verbs on a route nothing designed.
    spec.kind = Kind::Codex;
    assert_eq!(
        check_spec(&spec),
        Ok(()),
        "story:codex-catalogue-on-app-server: Codex carries a catalogue"
    );
    spec.catalogue = None;
    assert_eq!(
        check_spec(&spec),
        Err(Refusal::ToolSurfaceOwned { kind: Kind::Codex }),
        "story:codex-catalogue-on-app-server: the bare owned surface is still refused"
    );
}

/// The records a Codex app-server run hands its seam: the rollout's opening, which lists the
/// client tools, then each `item/tool/call` as the runner carries it, then the turn's end.
fn codex_app_server_stream(offered: &[String]) -> Vec<ScriptStep> {
    let meta = json!({"timestamp": "2026-10-29T10:00:00.000Z", "type": "session_meta",
        "payload": {"id": "th-fixture", "cli_version": "0.160.0", "cwd": "/w",
            "dynamic_tools": offered
                .iter()
                .map(|name| json!({"type": "function", "name": name, "description": "d",
                    "inputSchema": {"type": "object"}}))
                .collect::<Vec<_>>()}});
    let call = |key: &str, call_id: &str, tool: &str| {
        metaharness_codex::dynamic_tool_call_line(
            key,
            &json!({"threadId": "th-fixture", "turnId": "tu-1", "callId": call_id,
                "namespace": null, "tool": tool, "arguments": {"path": "README.md"}}),
        )
    };
    vec![
        ScriptStep::line(meta.to_string()),
        ScriptStep::line(
            r#"{"timestamp":"2026-10-29T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"tu-1","model":"m","personality":"pragmatic"}}"#,
        ),
        ScriptStep::line(call("rpc-1", "call-1", "fs_read")),
        // The rollout's record of the same call, under the same id: described, never decided.
        ScriptStep::line(
            r#"{"timestamp":"2026-10-29T10:00:02.000Z","type":"response_item","payload":{"type":"function_call","call_id":"call-1","name":"fs_read","arguments":"{\"path\":\"README.md\"}"}}"#,
        ),
        ScriptStep::line(call("rpc-2", "call-2", "fs_stat")),
        ScriptStep::line(call("rpc-3", "call-3", "shell")),
        ScriptStep::line(
            r#"{"timestamp":"2026-10-29T10:00:05.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"tu-1"}}"#,
        ),
    ]
}

/// One Codex run end to end through the production seam (amendment a66): the app-server launch,
/// the opening and the catalogue copy the runner reads, the offered tools from the record, and
/// each client call resolved by its bare name and decided by the frame.
#[test]
fn a_codex_run_carries_the_catalogue_over_the_app_server_and_denies_what_is_outside_it() {
    let caller = CallerCatalogue::read(&fixture()).expect("the fixture reads");
    let offered = caller.offered_tools_as(metaharness_tools::EntryNaming::Bare);
    assert_eq!(offered, ["fs_read", "fs_stat"]);

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(codex_app_server_stream(&offered), log.clone());
    let mut run = builder(Kind::Codex)
        .with_catalogue(fixture())
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting(OperationSet::of([Operation::FileRead])))
        .start_with_clock(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut metaharness::CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("story:codex-catalogue-on-app-server: a Codex run with a catalogue starts");
    run.drain().expect("drains");

    // The launch: the app-server and not exec, with the opening and the checked copy beside it.
    let argv = log.launched().pop().expect("one launch");
    assert_eq!(
        argv[1..],
        metaharness_codex::APP_SERVER_ARGS,
        "story:codex-catalogue-on-app-server: the app-server is launched"
    );
    let scratch = run.scratch_root().expect("a scratch root").to_path_buf();
    let directory = metaharness_codex::app_server_dir(&scratch);
    let copied = CallerCatalogue::read(&metaharness_codex::catalogue_copy_path(&directory))
        .expect("the scratch copy reads");
    assert_eq!(copied, caller);
    let opening: Value = serde_json::from_str(
        &std::fs::read_to_string(metaharness_codex::opening_path(&directory)).expect("written"),
    )
    .expect("JSON");
    assert_eq!(opening["thread_start"]["environments"], json!([]));
    let names: Vec<&str> = opening["thread_start"]["dynamicTools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names, offered);
    assert!(
        !scratch.join("hooks").exists(),
        "no hook program on this route"
    );

    // The record: the client tools, then the vendor tools the launch declares no config removes
    // (a66 § review corrections), and nothing the launch removed.
    let (offered_tools, _) = opened_with(run.events());
    let mut declared = offered.clone();
    declared.extend(metaharness_codex::APP_SERVER_VENDOR_TOOLS.map(str::to_owned));
    assert_eq!(
        offered_tools,
        Some(declared),
        "story:codex-catalogue-on-app-server: offered_tools lists the catalogue, then exec, wait \
         and request_user_input_async"
    );

    // Each call by its bare name: the entry the build performs resolves, the rest do not. The
    // rollout's record of the first call is described too, under the same id.
    assert_eq!(
        requested(run.events()),
        vec![
            (
                "fs_read".to_owned(),
                vec!["file.read".to_owned()],
                vec!["file:README.md".to_owned()]
            ),
            (
                "fs_read".to_owned(),
                vec!["file.read".to_owned()],
                vec!["file:README.md".to_owned()]
            ),
            ("fs_stat".to_owned(), Vec::new(), Vec::new()),
            ("shell".to_owned(), Vec::new(), Vec::new()),
        ]
    );
    let decisions = decided(run.events());
    assert_eq!(decisions.len(), 3, "{decisions:?}");
    assert_eq!(decisions[0], &Decision::Allow);
    for denied in &decisions[1..] {
        assert!(matches!(denied, Decision::Deny { .. }), "{decisions:?}");
    }

    // Each decision went to the runner under the key its answer is written for.
    let routed: Vec<(String, String)> = log
        .written()
        .iter()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter_map(|line| metaharness_codex::dynamic_decision(&line))
        .map(|(key, decision)| {
            let word = serde_json::to_value(&decision).expect("JSON")["decision"]
                .as_str()
                .expect("a word")
                .to_owned();
            (key, word)
        })
        .collect();
    assert_eq!(
        routed,
        [
            ("rpc-1".to_owned(), "allow".to_owned()),
            ("rpc-2".to_owned(), "deny".to_owned()),
            ("rpc-3".to_owned(), "deny".to_owned()),
        ]
    );
}

/// `--allow-program` names what the served `run` may start, and a server handed a catalogue
/// publishes no `run`: no entry is performed as one at the pinned harness. The pair is refused by
/// name, before the file is read, rather than accepted with the programs doing nothing.
#[test]
fn allow_program_beside_a_catalogue_is_refused_by_name_before_the_file_is_read_or_the_spawn() {
    let programs = vec!["/usr/bin/cargo".to_owned(), "/usr/bin/git".to_owned()];
    let mut spec = RunSpec::new(Kind::Claude);
    spec.catalogue = Some(fixture());
    spec.tool_surface = ToolSurface::Owned;
    spec.allow_program.clone_from(&programs);
    let refusal = check_spec(&spec).expect_err("the programs would do nothing");
    assert_eq!(
        refusal,
        Refusal::CatalogueWithAllowProgram {
            programs: programs.clone()
        }
    );
    let words = refusal.to_string();
    assert!(
        words.starts_with(&format!("{CATALOGUE_WITH_ALLOW_PROGRAM}: ")),
        "{words}"
    );
    assert!(
        words.contains("--allow-program /usr/bin/cargo, /usr/bin/git"),
        "{words}"
    );
    assert!(words.contains("--catalogue"), "{words}");
    assert_eq!(refusal.code(), None, "a launch-condition code");

    let base = tempfile::tempdir().expect("a scratch directory");
    let mut unread = builder(Kind::Claude)
        .with_catalogue(base.path().join("absent.yaml"))
        .spec()
        .clone();
    unread.allow_program.clone_from(&programs);
    let refusal = refused_before_the_spawn(
        Metaharness::from_spec(unread).with_credentials(CredentialSource::None),
    );
    assert_eq!(
        refusal,
        Refusal::CatalogueWithAllowProgram { programs },
        "refused before the absent file is read"
    );

    let mut without = RunSpec::new(Kind::Claude);
    without.tool_surface = ToolSurface::Owned;
    without.allow_program = vec!["/usr/bin/cargo".to_owned()];
    assert_eq!(check_spec(&without), Ok(()), "the programs alone still run");
}

#[test]
fn an_unreadable_catalogue_is_refused_by_name_before_the_spawn() {
    let dir = tempfile::tempdir().expect("a directory");
    let refusal = refused_before_the_spawn(
        builder(Kind::Claude).with_catalogue(dir.path().join("absent.yaml")),
    );
    let words = refusal.to_string();
    assert!(
        words.starts_with(&format!("{CATALOGUE_UNREADABLE}: ")),
        "{words}"
    );
    assert!(words.contains("absent.yaml"), "{words}");
}

#[test]
fn a_document_in_another_format_is_refused_by_name_before_the_spawn() {
    let dir = tempfile::tempdir().expect("a directory");
    let path = written(dir.path(), "ess.yaml", "format: ess/1\nentities: []\n");
    let refusal = refused_before_the_spawn(builder(Kind::Claude).with_catalogue(path));
    let words = refusal.to_string();
    assert!(
        words.starts_with(&format!("{CATALOGUE_NOT_TOOL_CONTRACT}: ")),
        "{words}"
    );
}

#[test]
fn a_malformed_catalogue_is_refused_by_name_before_the_spawn() {
    let dir = tempfile::tempdir().expect("a directory");
    for (name, text) in [
        (
            "broken.yaml",
            "format: tool-contract/1\noperations: [unclosed\n",
        ),
        (
            "empty.yaml",
            "format: tool-contract/1\ntoolset: fs\nversion: v1\nsummary: nothing\noperations: []\n",
        ),
    ] {
        let path = written(dir.path(), name, text);
        let refusal = refused_before_the_spawn(builder(Kind::Claude).with_catalogue(path));
        let words = refusal.to_string();
        assert!(
            words.starts_with(&format!("{CATALOGUE_MALFORMED}: ")),
            "{name}: {words}"
        );
    }
}

/// `story:codex-catalogue-live-follow-ups` (amendment a67): two entries under one published name
/// are refused on Claude and on Codex, before the spawn, when the catalogue is read. The
/// `tool-contract/1` checks refuse a rendering two operations share (`b10x-tool-contract` 0.5.0,
/// `validate.rs:195`), so the document is `catalogue-malformed` on every kind, and Codex's own
/// `catalogue-name-duplicate` is never reached from a catalogue file.
#[test]
fn two_entries_under_one_name_are_refused_on_claude_and_codex_before_the_spawn() {
    let dir = tempfile::tempdir().expect("a directory");
    let text = std::fs::read_to_string(fixture())
        .expect("the fixture")
        .replace("mcp: fs_stat", "mcp: fs_read");
    let path = written(dir.path(), "duplicate.yaml", &text);
    for kind in [Kind::Claude, Kind::Codex] {
        let refusal = refused_before_the_spawn(builder(kind).with_catalogue(&path));
        let words = refusal.to_string();
        assert!(
            words.starts_with(&format!("{CATALOGUE_MALFORMED}: ")),
            "{kind:?}: {words}"
        );
        assert!(
            words.contains("fs.stat: renderings: mcp name fs_read is also used by fs.read"),
            "{kind:?}: {words}"
        );
        assert!(
            !words.starts_with(&format!("{CATALOGUE_NAME_DUPLICATE}: ")),
            "{kind:?}: {words}"
        );
    }
}

#[test]
fn the_catalogue_is_read_before_the_frame_is_resolved() {
    let dir = tempfile::tempdir().expect("a directory");
    let refusal = refused_before_the_spawn(
        builder(Kind::Claude)
            .with_catalogue(dir.path().join("absent.yaml"))
            .with_decisions(DecisionMode::Frame)
            .with_frame_file(dir.path().join("no-frame.json")),
    );
    assert!(
        refusal.to_string().starts_with(CATALOGUE_UNREADABLE),
        "{refusal}"
    );
}

/// The vendor's own stream: an opening record listing `offered`, then each call and its result.
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

/// The word after `flag`, when `flag` is there.
fn after<'a>(words: &'a [String], flag: &str) -> Option<&'a str> {
    words
        .iter()
        .position(|word| word == flag)
        .and_then(|at| words.get(at + 1))
        .map(String::as_str)
}

/// The argument list the launch's MCP config gives the one server it names.
fn served_with(launch: &[String]) -> Vec<String> {
    let config_path = launch
        .iter()
        .position(|arg| arg == "--mcp-config")
        .map(|at| PathBuf::from(&launch[at + 1]))
        .expect("--mcp-config");
    let config: Value =
        serde_json::from_slice(&std::fs::read(&config_path).expect("the config was written"))
            .expect("JSON");
    let servers = config["mcpServers"].as_object().expect("servers");
    assert_eq!(servers.len(), 1);
    servers["metaharness"]["args"]
        .as_array()
        .expect("args")
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}

/// What `tools/list` answers, under the names the vendor gives them.
fn listed_by(server: &mut Server) -> Vec<String> {
    let answer = server
        .handle(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}))
        .expect("answered");
    answer["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .map(|tool| {
            format!(
                "mcp__metaharness__{}",
                tool["name"].as_str().expect("a name")
            )
        })
        .collect()
}

/// Each `tool.requested`: the name, its operations and its subjects.
fn requested(events: &[Event]) -> Vec<(String, Vec<String>, Vec<String>)> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ToolRequested {
                name,
                operations,
                subjects,
                ..
            } => Some((name.clone(), operations.clone(), subjects.clone())),
            _ => None,
        })
        .collect()
}

/// What `session.started` says was offered: the vendor's tool names and the operations.
fn opened_with(events: &[Event]) -> (Option<Vec<String>>, Option<Vec<String>>) {
    events
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted {
                offered_tools,
                available_operations,
                ..
            } => Some((offered_tools.clone(), available_operations.clone())),
            _ => None,
        })
        .expect("session.started")
}

/// Asked for an entry nothing performs, a verb and a vendor tool, the server does none of them.
fn performs_none(mcp: &mut Server) {
    for (name, arguments) in [
        ("fs_stat", json!({"path": "README.md"})),
        (
            "tool_invoke",
            json!({"name": "file_read", "arguments": {"path": "README.md"}}),
        ),
        ("Read", json!({"file_path": "README.md"})),
    ] {
        let answer = mcp
            .handle(&json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
                "params": {"name": name, "arguments": arguments}}))
            .expect("answered");
        assert_eq!(answer["result"]["isError"], true, "{name}");
        let text = answer["result"]["content"][0]["text"]
            .as_str()
            .expect("text");
        assert!(text.contains("nothing was done"), "{name}: {text}");
        assert!(!text.contains("hello"), "{name}: {text}");
    }
}

/// Each `tool.decided`'s decision.
fn decided(events: &[Event]) -> Vec<&Decision> {
    events
        .iter()
        .filter_map(|event| match event {
            Event::ToolDecided { decision, .. } => Some(decision),
            _ => None,
        })
        .collect()
}

/// One Claude run end to end: the launch, the materialised scratch copy, the server built from it,
/// and the vendor's own opening record read back through the adapter's seam.
#[test]
fn a_claude_run_offers_the_catalogue_and_nothing_else_and_denies_what_is_outside_it() {
    let caller = CallerCatalogue::read(&fixture()).expect("the fixture reads");
    let offered = caller.offered_tools();
    let calls = [
        (
            "toolu_1",
            "mcp__metaharness__fs_read",
            json!({"path": "README.md"}),
        ),
        (
            "toolu_2",
            "mcp__metaharness__fs_stat",
            json!({"path": "README.md"}),
        ),
        (
            "toolu_3",
            "mcp__metaharness__tool_invoke",
            json!({"name": "file_read", "arguments": {"path": "README.md"}}),
        ),
        ("toolu_4", "Read", json!({"file_path": "README.md"})),
    ];

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(claude_stream(&offered, &calls), log.clone());
    let mut run = builder(Kind::Claude)
        .with_catalogue(fixture())
        .with_decisions(DecisionMode::Frame)
        .with_frame(frame_admitting(OperationSet::of([Operation::FileRead])))
        .start_with_clock(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut ClaudeSeams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");
    run.drain().expect("drains");

    // The launch: built-ins removed, one server, and that server handed the scratch copy.
    let scratch = run.scratch_root().expect("a scratch root").to_path_buf();
    let argv = log.launched().pop().expect("one launch");
    assert_eq!(after(&argv, "--tools"), Some(""));
    let copy = scratch.join("catalogue.json");
    assert_eq!(
        after(&served_with(&argv), "--catalogue"),
        Some(copy.to_str().expect("UTF-8"))
    );

    // The copy is what was checked, and it serves the catalogue and nothing else.
    let copied = CallerCatalogue::read(&copy).expect("the scratch copy reads");
    assert_eq!(copied, caller);
    let tree = tempfile::tempdir().expect("a tree");
    std::fs::write(tree.path().join("README.md"), "hello\n").expect("a file");
    let mut mcp = Server::with_catalogue(
        Verbs::new(Catalogue::of(
            LocalOperations::unconfined(tree.path(), Vec::new()).expect("opens"),
        )),
        copied,
    );
    assert_eq!(listed_by(&mut mcp), offered);

    // The record: the vendor's own list, equal to the catalogue's names.
    let (offered_tools, available_operations) = opened_with(run.events());
    assert_eq!(offered_tools, Some(offered.clone()));
    assert_eq!(
        available_operations, None,
        "the owned surface leaves it absent"
    );

    // What each call was, and what the frame made of it.
    assert_eq!(
        requested(run.events()),
        vec![
            (
                "mcp__metaharness__fs_read".to_owned(),
                vec!["file.read".to_owned()],
                vec!["file:README.md".to_owned()]
            ),
            (
                "mcp__metaharness__fs_stat".to_owned(),
                Vec::new(),
                Vec::new()
            ),
            (
                "mcp__metaharness__tool_invoke".to_owned(),
                Vec::new(),
                Vec::new()
            ),
            ("Read".to_owned(), Vec::new(), Vec::new()),
        ]
    );
    let decisions = decided(run.events());
    assert_eq!(decisions.len(), 4, "{decisions:?}");
    assert_eq!(decisions[0], &Decision::Allow);
    for denied in &decisions[1..] {
        assert!(matches!(denied, Decision::Deny { .. }), "{decisions:?}");
    }
    assert!(
        run.events().iter().any(|event| matches!(
            event,
            Event::Warning { code, .. } if code == warning::UNCOVERED_TOOL
        )),
        "an uncovered call is named"
    );

    // And the server, asked for the same names, performs none of the three.
    performs_none(&mut mcp);
}

/// A caller's document may carry, escaped, characters YAML admits only escaped (DEL, a C1
/// control, a noncharacter). The parent reads it; the scratch copy the run writes must read back
/// as the same catalogue, or the server would refuse it and the run would start with no tools.
#[test]
fn a_catalogue_holding_characters_yaml_admits_only_escaped_reaches_the_server_as_checked() {
    let text = std::fs::read_to_string(fixture())
        .expect("the fixture reads")
        .replace(
            "    summary: Read a window of a file as numbered lines.",
            "    summary: \"Read a window\\x7F\\x92\\uFFFE of a file as numbered lines.\"",
        );
    let dir = tempfile::tempdir().expect("a directory");
    let path = written(dir.path(), "escaped.yaml", &text);
    let checked = CallerCatalogue::read(&path).expect("the parent reads it");
    assert!(
        checked.entries()[0]
            .description
            .contains("window\u{7f}\u{92}\u{fffe} of"),
        "the characters are in the catalogue"
    );

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let run = builder(Kind::Claude)
        .with_catalogue(path)
        .start_with(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        )
        .expect("the run starts");
    let copy = run
        .scratch_root()
        .expect("a scratch root")
        .join("catalogue.json");
    let served = CallerCatalogue::read(&copy).expect("the server reads the scratch copy");
    assert_eq!(served, checked);
}

#[test]
fn without_a_catalogue_the_owned_launch_is_unchanged() {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let run = builder(Kind::Claude)
        .with_tool_surface(ToolSurface::Owned)
        .start_with(
            Input::Prompt("read the readme".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        )
        .expect("the run starts");
    let scratch = run.scratch_root().expect("a scratch root").to_path_buf();
    assert!(!scratch.join("catalogue.json").exists());
    let argv = log.launched().pop().expect("one launch");
    let config_path = argv
        .iter()
        .position(|arg| arg == "--mcp-config")
        .map(|at| PathBuf::from(&argv[at + 1]))
        .expect("--mcp-config");
    let config = std::fs::read_to_string(config_path).expect("the config was written");
    assert!(!config.contains("--catalogue"), "{config}");
}
