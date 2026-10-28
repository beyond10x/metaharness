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

/// RED at 9109f336. `max-budget-usd` on b10x: `check_spec` admitted it and `b10x_launch` reads
/// nothing of it, so the loop was launched with exactly the argv it got without the ceiling. A spend
/// ceiling that is accepted and dropped is "a cap that exists on paper"
/// (`metaharness-aep/src/eval.rs:212-217`). Rewritten by the coordinator after pass 1: the
/// evidence half compared two started runs, and the capped one no longer starts; now the start
/// itself must refuse it by name, and the uncapped run must still start.
#[test]
fn b10x_is_refused_a_spend_ceiling_its_launch_drops() {
    let capped = b10x(r#","max-budget-usd":"5""#);
    let started = launched(capped.clone());
    assert!(
        started
            .as_ref()
            .is_err_and(|refusal| refusal.contains("--max-budget-usd")),
        "story:run-spec-document: a b10x start with a spend ceiling is refused by name: {started:?}"
    );
    launched(b10x("")).expect("story:run-spec-document: the uncapped b10x run still starts");
    let refusal = check_spec(&capped);
    assert!(
        refusal
            .as_ref()
            .is_err_and(|refusal| refusal.to_string().contains("--max-budget-usd")),
        "story:run-spec-document: check_spec admits a b10x spend ceiling the launch drops: {refusal:?}"
    );
}
/// RED at 9109f336. Each pair is a key the kind's launch reads nowhere (by `grep` over
/// `crates/*/src` at 9109f336: `spec.actor` is read only by the Claude launch, `spec.hooks` and
/// `spec.driver` only by `b10x_launch`, `spec.effort` only by the Claude and Codex launches). The
/// story's outcome says `check_spec` refuses such a key before anything starts; it admitted every
/// one of them. Rewritten by the coordinator after pass 1: `actor` on Codex and b10x moves to its
/// own story, because `aep drive` passes `--actor` to Codex today and a refusal would break it.
#[test]
fn every_key_a_kinds_launch_reads_nowhere_is_refused_by_check_spec() {
    let cases = [
        (r#"{"kind":"codex","hooks":"hooks.toml"}"#, "hooks"),
        (r#"{"kind":"codex","driver":"bin/aep"}"#, "driver"),
        (r#"{"kind":"claude","hooks":"hooks.toml"}"#, "hooks"),
        (r#"{"kind":"claude","driver":"bin/aep"}"#, "driver"),
    ];
    let mut admitted = Vec::new();
    for (text, key) in cases {
        let spec = RunSpec::from_document_str(text).expect("a run-spec document");
        if !check_spec(&spec)
            .is_err_and(|refusal| refusal.to_string().contains(&format!("--{key}")))
        {
            admitted.push(format!("{text} ({key})"));
        }
    }
    if !check_spec(&b10x(r#","effort":"high""#))
        .is_err_and(|refusal| refusal.to_string().contains("--effort"))
    {
        admitted.push("b10x effort".to_owned());
    }
    assert!(
        admitted.is_empty(),
        "story:run-spec-document: check_spec admits a key the kind drops in silence:\n{}",
        admitted.join("\n")
    );
    for spec in [
        RunSpec::from_document_str(r#"{"kind":"codex","actor":"agent:x"}"#).expect("a document"),
        b10x(r#","actor":"agent:x""#),
    ] {
        assert_eq!(
            check_spec(&spec),
            Ok(()),
            "story:actor-reaches-every-launch: --actor stays admitted on {} until that story \
             decides whether the launch carries it or check_spec refuses it",
            spec.kind.as_str()
        );
    }
}

/// Green probe at 9109f336: an empty path read from a document reached `check_spec` and passed it,
/// where the command line refuses the flag. Rewritten by the coordinator after pass 1: the
/// document now refuses the empty path itself, so nothing empty reaches `check_spec`.
#[test]
fn an_empty_retain_dir_from_a_document_is_refused_before_check_spec() {
    let read = RunSpec::from_document_str(r#"{"kind":"claude","retain-dir":""}"#);
    assert!(
        matches!(&read, Err(metaharness::protocol::RunSpecDocumentError::BadValue { key, .. }) if key == "retain-dir"),
        "story:run-spec-document: an empty retain-dir is refused by the document as the command \
         line refuses it: {read:?}"
    );
}
