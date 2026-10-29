//! Security review, wave D193, `story:codex-catalogue-on-app-server` (design amendment a66).
//!
//! Integrity conformance checks of what the app-server route says about itself against what its
//! own amendment and the live run at 0.160.0 found (a66 § 6,
//! `docs/research/2026-10-29-codex-0.160.0-app-server-catalogue.md` § 2): a code-mode model is
//! offered `exec`, `wait` and `request_user_input_async` beside the catalogue, none of them a client
//! tool, and the record shows the model calling `exec`.
//!
//! Every value here is synthetic. No vendor binary, no credential, no network, no model.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use metaharness_codex::{
    CodexSeam, DynamicTool, LaunchContext, dynamic_tool_call_line, plan_launch,
};
use metaharness_protocol::{
    CredentialSource, DecisionMode, Event, HarnessSeam, HermeticAttestation, HermeticMode,
    HermeticRow, Kind, RunSpec, Seam, ToolSurface, TranscriptRef,
};
use serde_json::json;

fn catalogue_spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Codex);
    spec.hermetic = HermeticMode::On;
    spec.credentials = CredentialSource::None;
    spec.decisions = DecisionMode::Frame;
    spec.model = Some("fixture-model".to_owned());
    spec.model_endpoint = Some("http://127.0.0.1:9".to_owned());
    spec.prompt = Some("read note.txt".to_owned());
    spec.catalogue = Some(PathBuf::from("/operator/session-catalogue.yaml"));
    spec.tool_surface = ToolSurface::Owned;
    spec
}

fn catalogue_context(root: &Path) -> LaunchContext {
    let schema = json!({
        "type": "object",
        "properties": {"path": {"type": "string"}},
        "required": ["path"],
        "additionalProperties": false,
    });
    LaunchContext {
        context_files: Vec::new(),
        scratch_root: root.to_path_buf(),
        cwd: root.join("work"),
        credentials_file: None,
        inherited_env: BTreeMap::from([
            ("HOME".to_owned(), "/operator".to_owned()),
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
        ]),
        memory_ancestors: Vec::new(),
        inputs_digest: None,
        plugins: Vec::new(),
        loopback: None,
        config_layers: Vec::new(),
        dynamic_tools: vec![
            DynamicTool {
                name: "fs_read".to_owned(),
                description: "Read a file.".to_owned(),
                input_schema: schema.clone(),
            },
            DynamicTool {
                name: "fs_stat".to_owned(),
                description: "Describe a path.".to_owned(),
                input_schema: schema,
            },
        ],
    }
}

/// H8 is an *imposed* control, so its sentence is a claim about every tool the run offers. The
/// same attestation's `ambient_inputs` says a code-mode model is offered `exec` and `wait`, which
/// are not client tools and reach no `item/tool/call` of their own. Both cannot be true; a66 § 6
/// and the live run say the second is.
#[test]
fn the_h8_row_does_not_claim_every_offered_tool_is_a_client_tool_beside_exec_and_wait() {
    let root = Path::new("/scratch/run-1");
    let plan = plan_launch(&catalogue_spec(), &catalogue_context(root)).expect("the run plans");
    let attestation = &plan.attestation;

    let names_code_mode = attestation
        .ambient_inputs
        .iter()
        .any(|input| input.contains("`exec` and `wait`"));
    assert!(
        names_code_mode,
        "precondition: the catalogue route's ambient inputs name code mode's tools: {:?}",
        attestation.ambient_inputs
    );

    let h8 = attestation
        .imposed
        .iter()
        .find(|control| control.row == HermeticRow::H8)
        .expect("H8 is imposed on the catalogue route");
    assert!(
        !h8.how.contains("every tool offered is a client tool"),
        "story:codex-catalogue-on-app-server: H8 claims {:?} while the same attestation says a \
         code-mode model is also offered `exec`, `wait` and `request_user_input_async`, which are \
         vendor tools no item/tool/call decides (a66 § 6)",
        h8.how
    );
}

const META: &str = r#"{"timestamp":"2026-10-29T10:00:00.000Z","type":"session_meta","payload":{"id":"01a1-fixture","cli_version":"0.160.0","cwd":"/scratch/run-1/work","dynamic_tools":[{"type":"function","name":"fs_read","description":"Read a file.","inputSchema":{"type":"object"}},{"type":"function","name":"fs_stat","description":"Describe a path.","inputSchema":{"type":"object"}}]}}"#;
const TURN: &str = r#"{"timestamp":"2026-10-29T10:00:01.000Z","type":"turn_context","payload":{"turn_id":"tu-1","model":"fixture-model","personality":"pragmatic"}}"#;
/// A code-mode cell, recorded as the 0.160.0 rollout records one: a `custom_tool_call` named
/// `exec` under the model's id (research note § 1, seq 11). Synthetic values.
const EXEC_CELL: &str = r#"{"timestamp":"2026-10-29T10:00:02.000Z","type":"response_item","payload":{"type":"custom_tool_call","status":"completed","call_id":"call-cell-1","name":"exec","input":"const r = await tools.fs_read({path: 'note.txt'}); text(r);"}}"#;

/// `session.started.offered_tools` is "every tool the model was offered"
/// (`metaharness-protocol/src/event.rs`, `Event::SessionStarted`). A run whose record shows the
/// model calling a tool that list omits has a list that is not that, however it was derived.
/// Invariant 3: tool restriction is asserted from the run's own record, never from the config
/// switches the launch wrote.
#[test]
fn every_tool_the_record_shows_called_is_among_the_offered_tools_when_they_are_stated() {
    let mut seam = CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::OwnedTool,
    );
    let nested = json!({"threadId": "th-1", "turnId": "tu-1",
        "callId": "exec-0b6f2a1e-0000-4000-8000-000000000001", "namespace": null,
        "tool": "fs_read", "arguments": {"path": "note.txt"}});
    let mut emitted = Vec::new();
    for line in [META, TURN, EXEC_CELL] {
        emitted.extend(seam.push_line(line));
    }
    emitted.extend(seam.push_line(&dynamic_tool_call_line("rpc-0", &nested)));

    let offered = emitted
        .iter()
        .find_map(|emission| match &emission.event {
            Event::SessionStarted { offered_tools, .. } => Some(offered_tools.clone()),
            _ => None,
        })
        .expect("session.started");
    let called: Vec<String> = emitted
        .iter()
        .filter_map(|emission| match &emission.event {
            Event::ToolRequested { name, .. } => Some(name.clone()),
            _ => None,
        })
        .collect();
    assert!(
        called.iter().any(|name| name == "exec"),
        "precondition: the record shows the exec cell: {called:?}"
    );

    if let Some(offered) = offered {
        let unoffered: Vec<&String> = called
            .iter()
            .filter(|name| !offered.contains(name))
            .collect();
        assert!(
            unoffered.is_empty(),
            "story:codex-catalogue-on-app-server: session.started.offered_tools is {offered:?}, \
             and the same record shows the model calling {unoffered:?}; a stated offer that \
             omits a called tool is an offer read from configuration rather than from the record"
        );
    }
}
