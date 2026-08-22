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
    assert_eq!(ADAPTER_ID, "claude");
    assert_eq!(PINNED_VERSIONS, ["2.1.239"]);
    assert_eq!(capabilities().versions_pinned, vec!["2.1.239".to_string()]);
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
#[test]
fn the_plans_attestation_reaches_the_opening_event() {
    let plan = plan_launch(&spec(), &context()).expect("the run plans");
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
        r#"{"type":"system","subtype":"init","claude_code_version":"2.1.239","tools":[],
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
    assert_eq!(outcomes.len(), 7);
    for outcome in &outcomes {
        assert!(outcome.passed, "{}: {}", outcome.id, outcome.detail);
        assert!(matches!(
            outcome.tier,
            ConformanceTier::C1 | ConformanceTier::C2
        ));
    }
}
