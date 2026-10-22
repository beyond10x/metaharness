//! Amendment a50 through the production builder, the b10x seam and the floor: a run whose key is
//! named by a secret (`story:b10x-api-key-from-keychain`, D29 and D30).
//!
//! The loop record is the adapter's golden `loop.jsonl` with its `started` line's
//! `credential_source` set to the string under test. No vendor process, no credential, no store:
//! the secret name is the fixture `llm/opencode-go-api-key` and only ever travels as an argv word.
#![cfg(unix)]

use metaharness::protocol::{
    CredentialSource, DecisionMode, Event, HermeticAttestation, HermeticMode, HermeticRow, Kind,
    RunSpec, Seam, SeamFactory as _, TranscriptRef, Verdict,
};
use metaharness::{
    FloorInputs, Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner, hermetic_floor,
};
use serde_json::{Value, json};

const FIXTURE_SECRET: &str = "llm/opencode-go-api-key";
const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

fn recorded_loop_saying(credential_source: &str) -> Vec<String> {
    B10X_RECORDED_LOOP
        .lines()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut value) if value["kind"] == "started" => {
                value["credential_source"] = Value::String(credential_source.to_owned());
                value.to_string()
            }
            _ => line.to_owned(),
        })
        .collect()
}

/// A secret-named b10x run through the production builder over the recorded loop, drained.
fn secret_run(credential_source: &str) -> (metaharness::Run, ScriptedLog) {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines(recorded_loop_saying(credential_source), log.clone());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut run = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::On)
        .with_credentials(CredentialSource::ApiKey)
        .with_api_key_secret(FIXTURE_SECRET)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("b10x-emulated")
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("a secret-named run plans on b10x");
    run.drain().unwrap();
    (run, log)
}

fn row(run: &metaharness::Run, which: HermeticRow) -> (Verdict, String) {
    let rows = run.hermetic_floor();
    let found = rows
        .iter()
        .find(|row| row.row == which)
        .expect("every row is evaluated");
    (found.verdict, found.detail.clone())
}

fn attestation(run: &metaharness::Run) -> HermeticAttestation {
    run.events()
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted { hermetic, .. } => Some(hermetic.clone()),
            _ => None,
        })
        .expect("the run opened")
}

/// The argv the scripted runner was handed carries the name, and no other key route.
#[test]
fn the_launched_argv_names_the_secret_and_no_variable() {
    let (_run, log) = secret_run(metaharness_b10x::SECRET_KEY_SOURCE);
    let launched = log.launched();
    assert_eq!(launched.len(), 1, "{launched:?}");
    let argv = &launched[0];
    let at = argv
        .iter()
        .position(|word| word == "--api-key-secret")
        .unwrap_or_else(|| panic!("no secret flag: {argv:?}"));
    assert_eq!(argv[at + 1], FIXTURE_SECRET);
    assert!(
        !argv
            .iter()
            .any(|word| word.starts_with("--api-key-env") || word.starts_with("--api-key-file")),
        "{argv:?}"
    );
}

/// D29: H3's imposed sentence says which two variables the secret route adds to the constructed
/// environment, so the attestation states what the child was given.
#[test]
fn h3_names_the_session_bus_variables_on_the_secret_route() {
    let (run, _) = secret_run(metaharness_b10x::SECRET_KEY_SOURCE);
    let attested = attestation(&run);
    let h3 = attested
        .imposed
        .iter()
        .find(|control| control.row == HermeticRow::H3)
        .expect("H3 is imposed");
    for variable in metaharness_b10x::SECRET_SERVICE_SESSION_BUS_VARIABLES {
        assert!(h3.how.contains(variable), "{}", h3.how);
    }
    assert!(!h3.how.contains("OPENAI_API_KEY"), "{}", h3.how);
}

/// D30: H4 reads `ok` on exactly the pinned loop's string for this route.
#[test]
fn h4_is_ok_on_exactly_the_loops_secret_source_string() {
    let (run, _) = secret_run("api-key:secrets");
    let (verdict, detail) = row(&run, HermeticRow::H4);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(detail.contains("api-key:secrets"), "{detail}");
}

/// D30: any other string is a gap, the environment route's own included: the key reached the loop
/// by a road the run did not declare.
#[test]
fn h4_is_a_gap_on_any_other_source_string() {
    for other in [
        "api-key:environment",
        "api-key:file",
        "none",
        "oauth:file",
        "API-KEY:SECRETS",
        "api-key:secrets ",
    ] {
        let (run, _) = secret_run(other);
        let (verdict, detail) = row(&run, HermeticRow::H4);
        assert_eq!(verdict, Verdict::Gap, "{other}: {detail}");
        assert!(detail.contains("api-key:secrets"), "{other}: {detail}");
    }
}

/// The floor's own inputs, for a kind whose launch hands no source string for the route.
fn floor_h4(spec: &RunSpec, observed: &str, secret_key_source: Option<&str>) -> (Verdict, String) {
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut seam = seams.build(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::On),
        Seam::None,
    );
    let line = json!({"kind":"started","model":"m","published_tools":[],"credential_source":observed,"cwd":"/scratch/run/work"});
    let events: Vec<_> = seam
        .push_line(&line.to_string())
        .into_iter()
        .map(|emission| emission.event)
        .collect();
    let pins = vec![metaharness_b10x::PINNED_VERSIONS[0].to_owned()];
    let rows = hermetic_floor(
        &events,
        &FloorInputs {
            spec,
            pinned_versions: &pins,
            planned_cwd: Some("/scratch/run/work"),
            declared_plugins: &[],
            builtin_plugins: &[],
            endpoint_key_posture: None,
            secret_key_source,
            planned_mcp_servers: &[],
            planned_output_style: None,
            record_silences: &[],
            memory_canaries: None,
        },
    );
    let found = rows
        .iter()
        .find(|row| row.row == HermeticRow::H4)
        .expect("H4 is evaluated");
    (found.verdict, found.detail.clone())
}

/// D30: a kind that states no source string for a secret-named key cannot be checked, so H4 is
/// `unk` and says why, rather than `ok` by the looser "names a key" reading.
#[test]
fn h4_is_unk_when_the_kind_states_no_secret_source_string() {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.credentials = CredentialSource::ApiKey;
    spec.api_key_secret = Some(FIXTURE_SECRET.to_owned());
    let (verdict, detail) = floor_h4(&spec, "api-key:secrets", None);
    assert_eq!(verdict, Verdict::Unk, "{detail}");
    assert!(detail.contains("--api-key-secret"), "{detail}");
}

/// A run that names no secret reads H4 as it did before a50: the environment route's string is a
/// declared key, and the secret string is not consulted.
#[test]
fn h4_without_a_secret_is_unchanged() {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.credentials = CredentialSource::ApiKey;
    let (verdict, detail) = floor_h4(
        &spec,
        "api-key:environment",
        Some(metaharness_b10x::SECRET_KEY_SOURCE),
    );
    assert_eq!(verdict, Verdict::Ok, "{detail}");
}
