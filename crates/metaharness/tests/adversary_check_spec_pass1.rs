//! Adversary cases, pass 1, for `story:run-spec-document` (wave D170, unit 1; design amendment a64).
//!
//! The story's outcome: `check_spec` refuses, before anything starts, a key the spec's harness kind
//! does not take, so a setting is never dropped in silence. These cases read a document per kind
//! with a key whose launch reads nothing, over the scripted runner: no vendor process, no
//! credential, no network, no model.

use metaharness::protocol::RunSpec;
use metaharness::{Input, Metaharness, ScriptedLog, ScriptedRunner, ScriptedSeams, check_spec};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// A b10x document that starts over the scripted runner, plus `extra`.
fn b10x(extra: &str) -> RunSpec {
    let text = format!(
        r#"{{"kind":"b10x","credentials":"none","decisions":"observe","model":"m","model-endpoint":"http://127.0.0.1:1"{extra}}}"#
    );
    RunSpec::from_document_str(&text).expect("a run-spec document")
}

/// The argv the scripted runner was handed for this spec, or the refusal's words. The value after
/// `--workspace` is a fresh scratch directory per start, so it is replaced by a placeholder.
fn launched(spec: RunSpec) -> Result<Vec<String>, String> {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    match Metaharness::from_spec(spec).start_with(
        Input::Prompt("go".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    ) {
        Ok(_) => {
            let mut argv = log.launched().concat();
            if let Some(at) = argv.iter().position(|word| word == "--workspace")
                && let Some(value) = argv.get_mut(at + 1)
            {
                "<scratch>".clone_into(value);
            }
            Ok(argv)
        }
        Err(refusal) => Err(refusal.to_string()),
    }
}

/// RED. `max-budget-usd` on b10x: `check_spec` admits it and `b10x_launch` reads nothing of it, so
/// the loop is launched with exactly the argv it gets without the ceiling. A spend ceiling that is
/// accepted and dropped is "a cap that exists on paper" (`metaharness-aep/src/eval.rs:212-217`).
#[test]
fn b10x_is_refused_a_spend_ceiling_its_launch_drops() {
    let capped = b10x(r#","max-budget-usd":"5""#);
    let uncapped = b10x("");
    let with = launched(capped.clone()).expect("the capped run starts");
    let without = launched(uncapped).expect("the uncapped run starts");
    assert_eq!(
        with, without,
        "evidence: the ceiling reaches no argument of the loop"
    );
    let refusal = check_spec(&capped);
    assert!(
        refusal
            .as_ref()
            .is_err_and(|refusal| refusal.to_string().contains("--max-budget-usd")),
        "check_spec admits a b10x spend ceiling the launch drops: {refusal:?}"
    );
}

/// RED. Each pair is a key the kind's launch reads nowhere (by `grep` over `crates/*/src` at
/// 9109f336: `spec.actor` is read only by the Claude launch, `spec.hooks` and `spec.driver` only
/// by `b10x_launch`, `spec.effort` only by the Claude and Codex launches). The story's outcome says
/// `check_spec` refuses such a key before anything starts; it admits every one of them.
#[test]
fn every_key_a_kinds_launch_reads_nowhere_is_refused_by_check_spec() {
    let cases = [
        (r#"{"kind":"codex","actor":"agent:x"}"#, "actor"),
        (r#"{"kind":"codex","hooks":"hooks.toml"}"#, "hooks"),
        (r#"{"kind":"codex","driver":"bin/aep"}"#, "driver"),
        (r#"{"kind":"claude","hooks":"hooks.toml"}"#, "hooks"),
        (r#"{"kind":"claude","driver":"bin/aep"}"#, "driver"),
    ];
    let mut admitted = Vec::new();
    for (text, key) in cases {
        let spec = RunSpec::from_document_str(text).expect("a run-spec document");
        if check_spec(&spec).is_ok() {
            admitted.push(format!("{text} ({key})"));
        }
    }
    for (extra, key) in [
        (r#","actor":"agent:x""#, "actor"),
        (r#","effort":"high""#, "effort"),
    ] {
        if check_spec(&b10x(extra)).is_ok() {
            admitted.push(format!("b10x {extra} ({key})"));
        }
    }
    assert!(
        admitted.is_empty(),
        "check_spec admits a key the kind drops in silence:\n{}",
        admitted.join("\n")
    );
}

/// Green probe: an empty path read from a document reaches `check_spec` and passes it, where the
/// command line would have refused the flag (see the protocol crate's pass-1 case).
#[test]
fn an_empty_retain_dir_from_a_document_passes_check_spec() {
    let spec = RunSpec::from_document_str(r#"{"kind":"claude","retain-dir":""}"#)
        .expect("the document admits it");
    assert_eq!(spec.retain_dir.as_deref(), Some(std::path::Path::new("")));
    assert_eq!(check_spec(&spec), Ok(()));
}
