//! The adapter as an embedder sees it: only the public surface, from outside the crate.
//!
//! These are the assertions that would catch a rename or a retype before the crate that depends
//! on this one does, which is the whole reason the surface is small and named in one place.

use std::collections::BTreeMap;
use std::path::PathBuf;

use metaharness_claude::{
    ADAPTER_ID, HOOK_TIMEOUT_SECONDS, LaunchContext, LaunchRefusal, PINNED_VERSIONS,
    TranscriptReader, capabilities, conformance_vectors, parse_hook_input, plan_launch,
    render_hook_response, render_operation,
};
use metaharness_protocol::{
    ConformanceTier, Decision, DecisionCensus, DecisionMode, Digest, Event, HermeticMode, Kind,
    Operation, RefusalCode, RunSpec, Seam, ToolSurface, TranscriptRef,
};

fn context() -> LaunchContext {
    LaunchContext {
        scratch_root: PathBuf::from("/scratch/run-9"),
        cwd: PathBuf::from("/scratch/run-9/work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([("HOME".to_string(), "/operator".to_string())]),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"inputs")),
        plugins: Vec::new(),
        marketplace_plugins: Vec::new(),
        loopback: None,
        tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
    }
}

fn spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("go".to_string());
    spec
}

#[test]
fn the_adapter_names_itself_and_its_pin() {
    // **The one deliberate literal.** Every other assertion about the pin reads `PINNED_VERSIONS`,
    // so a move shows up here — in a diff a reviewer reads — and nowhere else. Moved
    // 2.1.240 → 2.1.241 on 2026-08-24 and 2.1.241 → 2.1.259 on 2026-09-03; see `PINNED_VERSIONS`'
    // own note for what was re-read on the new binary and what was carried over unverified.
    assert_eq!(ADAPTER_ID, "claude");
    assert_eq!(PINNED_VERSIONS, ["2.1.259"]);
    assert_eq!(
        capabilities().versions_pinned,
        PINNED_VERSIONS.map(ToString::to_string).to_vec(),
        "what the adapter publishes and what it pins cannot disagree"
    );
}

#[test]
fn the_operation_rendering_is_a_value_an_embedder_can_assert_on_without_a_run() {
    assert_eq!(render_operation(&Operation::FileWrite), Some("Write"));
    assert_eq!(
        render_operation(&Operation::McpCall {
            server: "s".to_string(),
            tool: "t".to_string()
        }),
        None
    );
    assert_eq!(capabilities().rendering["shell"], Some("Bash".to_string()));
}

#[test]
fn a_hook_response_is_the_vendors_own_shape() {
    let response = render_hook_response(&Decision::Deny {
        reason: "this step admits no shell".to_string(),
    });
    assert_eq!(
        response["hookSpecificOutput"]["hookEventName"],
        serde_json::json!("PreToolUse")
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"],
        serde_json::json!("deny")
    );
}

#[test]
fn the_hook_input_parses_and_a_malformed_one_is_refused_by_name() {
    let parsed = parse_hook_input(r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#)
        .expect("the hook input parses");
    assert_eq!(parsed.tool_name.as_deref(), Some("Bash"));
    assert_eq!(
        parse_hook_input("{").expect_err("refused").code,
        RefusalCode::Malformed
    );
}

/// The plan is a value: the argv, the environment, the copy list, the settings and the hook are
/// all readable before any process exists (design § 8.4 O7).
#[test]
fn a_plan_is_readable_in_full_before_anything_is_spawned() {
    let plan = plan_launch(&spec(), &context()).expect("the run plans");
    assert_eq!(plan.program, ADAPTER_ID);
    assert!(plan.args.contains(&"--strict-mcp-config".to_string()));
    assert_eq!(
        plan.config_home,
        PathBuf::from("/scratch/run-9/claude-home")
    );
    assert_eq!(plan.credential_copies.len(), 1);
    assert_eq!(
        plan.hook["hooks"][0]["timeout"],
        serde_json::json!(HOOK_TIMEOUT_SECONDS)
    );
    assert!(plan.hook["hooks"][0].get("async").is_none());
    assert_eq!(plan.attestation.mode, HermeticMode::Strict);
}

#[test]
fn a_shadowed_run_is_refused_with_the_protocols_own_code() {
    let mut spec = spec();
    spec.decisions = DecisionMode::Ask;
    spec.tool_surface = ToolSurface::Owned;
    let refusal = plan_launch(&spec, &context()).expect_err("refused");
    assert_eq!(refusal.code(), Some(RefusalCode::Shadowed));
    assert!(matches!(refusal, LaunchRefusal::Shadowed { .. }));
    // It is an error, so an embedder can put it anywhere an error goes.
    let _: &dyn std::error::Error = &refusal;
}

/// The attestation metaharness built at launch is the one the opening event carries, so a reader
/// sees the intent beside the vendor's outcome (design § 8.3).
///
/// Driven with the two fields amendment a10 added set to **non-default values**, because a
/// whole-struct comparison over a default-valued attestation would pass even if the mode and the
/// installed plugins were dropped on the way to the record — and those two are exactly what an
/// eval's arm column is read from.
#[test]
fn the_plans_attestation_reaches_the_opening_event() {
    let mut spec = spec();
    spec.decisions = DecisionMode::Observe;
    spec.plugin_dir
        .push(PathBuf::from("/operator/integrations/claude-code"));
    let mut context = context();
    context.plugins.push(metaharness_protocol::PluginTree {
        source: PathBuf::from("/operator/integrations/claude-code"),
        content: metaharness_protocol::PluginContent::Files {
            count: 1,
            digest: Digest::of(b"one file"),
        },
    });
    let plan = plan_launch(&spec, &context).expect("the run plans");
    assert_eq!(plan.attestation.decisions, DecisionMode::Observe);
    assert_eq!(plan.attestation.installed_plugins.len(), 1);
    let mut reader = TranscriptReader::new(
        TranscriptRef {
            path: Some("/scratch/run-9/transcript.jsonl".to_string()),
            digest: Some(Digest::of(b"bytes")),
            bytes: Some(5),
        },
        plan.attestation.clone(),
    )
    .with_seam(Seam::Hook);
    reader.set_census(DecisionCensus::default());
    let events = reader.push_line(
        r#"{"type":"system","subtype":"init","claude_code_version":"2.1.240","tools":[],
            "mcp_servers":[]}"#,
    );
    let Event::SessionStarted { hermetic, .. } = &events[0].event else {
        panic!("expected session.started");
    };
    assert_eq!(*hermetic, plan.attestation);
    assert!(reader.finish().is_empty());
}

#[test]
fn every_conformance_vector_passes_without_a_model_a_network_or_a_credential() {
    let outcomes = conformance_vectors();
    // Five recorded launch expectations, the three computed launch vectors that carry observe
    // mode, plugin injection (a10, crossing #4) and the pinned marketplace placement (a16), three
    // synthesised replays, the two golden recorded-wire vectors (CT-2) and the version pair
    // (CT-3).
    assert_eq!(outcomes.len(), 14);
    for outcome in &outcomes {
        assert!(outcome.passed, "{}: {}", outcome.id, outcome.detail);
        assert!(matches!(
            outcome.tier,
            ConformanceTier::C1 | ConformanceTier::C2
        ));
    }
}
