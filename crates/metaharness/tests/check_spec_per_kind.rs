//! `check_spec` refuses, before anything starts, a setting the spec's kind does not take
//! (story:run-spec-document, acceptance 4; design amendment a64).
//!
//! The reasons stay in the adapters (invariant 2): `check_spec` calls the adapter's own check and
//! maps its refusal the way the launch path does, so the words a caller reads are the launch's. The
//! launch keeps its check as well. Every start here hands in a scripted runner: no vendor process,
//! no credential, no network, no model.

use metaharness::protocol::{CredentialSource, Kind, RunSpec};
use metaharness::{
    Input, Metaharness, Refusal, ScriptedLog, ScriptedRunner, ScriptedSeams, check_spec,
};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

fn document(text: &str) -> RunSpec {
    RunSpec::from_document_str(text).expect("a run-spec document")
}

fn launch_words(refusal: &Refusal) -> &str {
    match refusal {
        Refusal::Launch { detail } => detail,
        other => panic!("expected the adapter's launch refusal, got {other:?}"),
    }
}

/// Acceptance 4: Codex has neither ceiling, and the spec says so before any launch is planned.
#[test]
fn codex_is_refused_a_turn_ceiling_and_a_spend_ceiling_by_check_spec() {
    for (text, flag) in [
        (r#"{"kind":"codex","max-turns":60}"#, "--max-turns"),
        (
            r#"{"kind":"codex","max-budget-usd":"5"}"#,
            "--max-budget-usd",
        ),
    ] {
        let spec = document(text);
        let refusal = check_spec(&spec).expect_err(flag);
        let words = launch_words(&refusal);
        assert!(words.contains(flag), "{words}");
        assert_eq!(
            words,
            metaharness_codex::unsupported_options(&spec)
                .expect_err(flag)
                .to_string(),
            "the launch's own words, mapped as the launch maps them"
        );
    }
}

/// Claude Code takes both, so the same settings pass there.
#[test]
fn claude_takes_both_ceilings() {
    let spec = document(r#"{"kind":"claude","max-turns":60,"max-budget-usd":"5"}"#);
    assert_eq!(check_spec(&spec), Ok(()));
}

/// The start a caller makes is refused with the same refusal, and nothing is spawned.
#[test]
fn a_codex_start_with_a_turn_ceiling_is_refused_before_the_spawn() {
    let spec = document(r#"{"kind":"codex","max-turns":60,"credentials":"none"}"#);
    let expected = check_spec(&spec).expect_err("refused");
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let Err(refusal) = Metaharness::from_spec(spec).start_with(
        Input::Prompt("write the failing test".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    ) else {
        panic!("the run started");
    };
    assert_eq!(refusal, expected);
    assert_eq!(log.spawns(), 0, "{refusal}");
    assert!(log.launched().is_empty(), "{refusal}");
}

/// A vendor arm pointed at a model endpoint must hold no operator credential. Both adapters refuse
/// it from the spec alone, so `check_spec` raises their refusal before the launch does.
#[test]
fn a_vendor_arm_pointed_at_an_endpoint_while_holding_a_credential_is_refused_by_check_spec() {
    for kind in [Kind::Claude, Kind::Codex] {
        for credentials in [CredentialSource::OperatorLogin, CredentialSource::ApiKey] {
            let mut spec = RunSpec::new(kind);
            spec.model_endpoint = Some("http://127.0.0.1:1".to_owned());
            spec.credentials = credentials;
            let refusal = check_spec(&spec).expect_err("refused");
            let words = launch_words(&refusal).to_owned();
            let adapter = match kind {
                Kind::Claude => metaharness_claude::guard_endpoint_credential(&spec)
                    .expect_err("claude refuses")
                    .to_string(),
                _ => metaharness_codex::guard_endpoint_credential(&spec)
                    .expect_err("codex refuses")
                    .to_string(),
            };
            assert_eq!(words, adapter, "{kind:?} {credentials:?}");
            assert!(words.contains("http://127.0.0.1:1"), "{words}");
        }
        for credentials in [CredentialSource::None, CredentialSource::Loopback] {
            let mut spec = RunSpec::new(kind);
            spec.model_endpoint = Some("http://127.0.0.1:1".to_owned());
            spec.credentials = credentials;
            assert_eq!(check_spec(&spec), Ok(()), "{kind:?} {credentials:?}");
        }
    }
    let mut b10x = RunSpec::new(Kind::B10x);
    b10x.model_endpoint = Some("http://127.0.0.1:1".to_owned());
    b10x.credentials = CredentialSource::ApiKey;
    assert_eq!(check_spec(&b10x), Ok(()), "the loop passes its key through");
}

/// The command line refuses `--substrate` beside `--substrate-embedded` (clap's
/// `conflicts_with`); a document does not pass through clap, so `check_spec` refuses the pair.
#[test]
fn a_socket_and_the_embedded_driver_together_are_refused_as_on_the_command_line() {
    let both =
        document(r#"{"kind":"b10x","substrate":"substrate.sock","substrate-embedded":true}"#);
    let refusal = check_spec(&both).expect_err("refused");
    let words = launch_words(&refusal);
    assert!(words.contains("--substrate "), "{words}");
    assert!(words.contains("--substrate-embedded"), "{words}");

    for alone in [
        r#"{"kind":"b10x","substrate":"substrate.sock"}"#,
        r#"{"kind":"b10x","substrate-embedded":true}"#,
    ] {
        assert_eq!(check_spec(&document(alone)), Ok(()), "{alone}");
    }
}
