//! Free end-to-end vectors for the injected process-envelope boundary.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use metaharness::protocol::{
    CredentialChannel, Digest, EnvelopeAssessment, HermeticMode, MountAccess, NetworkPolicy,
    ProcessBounds, ProcessEnvelopeMeasurement, ProcessEnvelopeRequest, StagedExecutable,
};
use metaharness::{
    EnvelopeStartError, LaunchPlanView, ScriptedEnvelope, ScriptedLog, ScriptedRunner,
    start_in_envelope,
};

fn request() -> ProcessEnvelopeRequest {
    ProcessEnvelopeRequest {
        runtime_roots: vec!["/runtime".into()],
        workspace_root: "/workspace".into(),
        writable_subtrees: vec!["/workspace/out".into()],
        scratch_root: "/state".into(),
        executables: vec![StagedExecutable {
            digest: Digest::of(b"harness"),
            mounted_path: "/runtime/harness".into(),
        }],
        environment: BTreeMap::from([("PATH".into(), "/runtime".into())]),
        credential_channel: CredentialChannel::None,
        network: NetworkPolicy::None,
        bounds: ProcessBounds {
            processes: 2,
            wall_time_ms: 10_000,
            output_bytes: 10_000,
        },
    }
}

fn measurement() -> ProcessEnvelopeMeasurement {
    ProcessEnvelopeMeasurement {
        mounts: BTreeMap::from([
            ("/runtime".into(), MountAccess::ReadOnly),
            ("/state".into(), MountAccess::ReadWrite),
            ("/workspace".into(), MountAccess::ReadOnly),
            ("/workspace/out".into(), MountAccess::ReadWrite),
        ]),
        writable_paths: BTreeSet::from(["/state".into(), "/workspace/out".into()]),
        environment_keys: BTreeSet::from(["PATH".into()]),
        executable_digests: BTreeMap::from([("/runtime/harness".into(), Digest::of(b"harness"))]),
        network: NetworkPolicy::None,
        bounds: ProcessBounds {
            processes: 2,
            wall_time_ms: 10_000,
            output_bytes: 10_000,
        },
        cwd: "/workspace".into(),
    }
}

fn plan<'a>(args: &'a [String], env: &'a BTreeMap<String, String>) -> LaunchPlanView<'a> {
    LaunchPlanView {
        program: "/runtime/harness",
        args,
        env,
        cwd: Path::new("/workspace"),
        credential_copies: &[],
        decision_channel: Path::new("/state/decisions"),
        transcript: Path::new("/state/transcript.jsonl"),
    }
}

#[test]
fn the_scripted_port_receives_the_exact_seal_and_matching_evidence_runs() {
    let sealed = request().seal();
    let log = ScriptedLog::new();
    let mut port =
        ScriptedEnvelope::new(ScriptedRunner::of_lines(["done"], log), Some(measurement()));
    let args = Vec::new();
    let env = BTreeMap::new();
    let started = start_in_envelope(&mut port, &sealed, &plan(&args, &env), HermeticMode::Strict)
        .expect("matching evidence admits the child");

    assert_eq!(started.assessment, EnvelopeAssessment::Matched);
    assert_eq!(port.digests(), [sealed.digest().clone()]);
}

#[test]
fn strict_mode_kills_a_child_with_a_wider_write_surface() {
    let sealed = request().seal();
    let mut wider = measurement();
    wider.writable_paths.insert("/workspace".into());
    let log = ScriptedLog::new();
    let mut port = ScriptedEnvelope::new(
        ScriptedRunner::of_lines(["must not run"], log.clone()),
        Some(wider),
    );
    let args = Vec::new();
    let env = BTreeMap::new();
    let Err(error) =
        start_in_envelope(&mut port, &sealed, &plan(&args, &env), HermeticMode::Strict)
    else {
        panic!("strict mode must refuse a mismatch");
    };

    assert!(matches!(error, EnvelopeStartError::StrictEvidence { .. }));
    assert!(log.killed(), "the wider child is stopped before admission");
    assert!(error.to_string().contains("writable_paths"));
}

#[test]
fn withheld_evidence_stays_unknown_outside_strict_mode() {
    let sealed = request().seal();
    let log = ScriptedLog::new();
    let mut port = ScriptedEnvelope::new(ScriptedRunner::of_lines(["done"], log), None);
    let args = Vec::new();
    let env = BTreeMap::new();
    let started = start_in_envelope(&mut port, &sealed, &plan(&args, &env), HermeticMode::On)
        .expect("non-strict mode retains unknown evidence");

    assert_eq!(started.assessment, EnvelopeAssessment::Withheld);
}
