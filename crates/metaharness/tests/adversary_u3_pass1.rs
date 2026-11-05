//! Adversary cases, pass 1, for `story:actor-reaches-every-launch` (design amendment a74).
//!
//! The acceptance: a Codex run with `--actor agent:x` sets the actor where the Claude launch sets it
//! (`AEP_ACTOR` in the child's environment), or `check_spec` refuses it naming `--actor`; the same
//! for b10x. These cases plan launches as pure values and read the published CLI reference: no
//! vendor process, no credential, no network, no model.

use std::collections::BTreeMap;
use std::path::PathBuf;

use metaharness::check_spec;
use metaharness::protocol::{HermeticMode, RunSpec};
use metaharness_codex::{LaunchContext, plan_launch};

/// A synthetic world whose operator has exported an `AEP_ACTOR` of their own.
fn context() -> LaunchContext {
    LaunchContext {
        context_files: Vec::new(),
        scratch_root: PathBuf::from("/scratch/run-1"),
        cwd: PathBuf::from("/scratch/run-1/work"),
        credentials_file: Some(PathBuf::from("/operator/.codex/auth.json")),
        inherited_env: [
            ("HOME", "/operator"),
            ("USER", "operator"),
            ("LANG", "C.UTF-8"),
            ("AEP_ACTOR", "human:someone-else"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.to_owned()))
        .collect::<BTreeMap<_, _>>(),
        memory_ancestors: Vec::new(),
        inputs_digest: None,
        plugins: Vec::new(),
        loopback: None,
        config_layers: Vec::new(),
        dynamic_tools: Vec::new(),
    }
}

fn codex(actor: &str) -> RunSpec {
    let mut spec = RunSpec::from_document_str(r#"{"kind":"codex"}"#).expect("a document");
    spec.hermetic = HermeticMode::On;
    spec.prompt = Some("do the thing".to_owned());
    spec.actor = Some(actor.to_owned());
    spec
}

/// RED at 79f811c. `--actor ""` (or a document's `"actor": ""`, or whitespace) is admitted by
/// `check_spec` and the Codex launch sets `AEP_ACTOR` to the blank value. The subject parses
/// `AEP_ACTOR` as an actor and refuses every store write when it is not one
/// (`aep-cli/src/planning.rs`, `actor_from`), so the run starts and fails at its first write
/// instead of being refused before anything starts. Either answer satisfies this case: a refusal
/// naming `--actor`, or a child that carries no blank actor.
#[test]
fn a_blank_actor_is_refused_or_never_reaches_the_codex_child() {
    for blank in ["", "   "] {
        let spec = codex(blank);
        match check_spec(&spec) {
            Err(refusal) => assert!(
                refusal.to_string().contains("--actor"),
                "a blank actor refused, but not by name: {refusal}"
            ),
            Ok(()) => match plan_launch(&spec, &context()) {
                Err(refusal) => assert!(
                    refusal.to_string().contains("actor"),
                    "a blank actor refused by the launch, but not by name: {refusal}"
                ),
                Ok(plan) => assert!(
                    plan.env
                        .get("AEP_ACTOR")
                        .is_none_or(|value| !value.trim().is_empty()),
                    "story:actor-reaches-every-launch: --actor {blank:?} was admitted by \
                     check_spec and reached the Codex child as a blank AEP_ACTOR, which the \
                     subject refuses at every store write: {:?}",
                    plan.env.get("AEP_ACTOR")
                ),
            },
        }
    }
}

/// RED at 79f811c. `RunSpec::actor` promises "the adapter sets it on a constructed environment, and
/// the attestation records it as imposed" (`metaharness-protocol/src/spec.rs`), and the acceptance
/// names the attestation as a place the actor is observed. The Codex H3 row still says the child
/// environment is "an allowlist of 7 keys plus `PATH`, `TMPDIR` and `CODEX_HOME`" while the plan's
/// environment carries `AEP_ACTOR` beside them.
#[test]
fn the_codex_attestation_names_the_actor_it_set() {
    let plan = plan_launch(&codex("agent:x"), &context()).expect("plans");
    assert_eq!(
        plan.env.get("AEP_ACTOR").map(String::as_str),
        Some("agent:x"),
        "precondition: the declared actor reaches the child"
    );
    let named = plan
        .attestation
        .imposed
        .iter()
        .any(|control| control.how.contains("AEP_ACTOR"));
    assert!(
        named,
        "story:actor-reaches-every-launch: the child carries AEP_ACTOR and no imposed control \
         names it: {:#?}",
        plan.attestation.imposed
    );
}

/// RED at 79f811c. `website/docs/reference/cli.md` says `--actor` is "carried to the child as
/// `AEP_ACTOR`" without naming a kind, while `check_spec` now refuses it on b10x. The row beside it,
/// `--prompt-cache-ttl`, names the kinds that refuse it; this one sends a b10x caller to a refusal
/// the reference does not mention.
#[test]
fn the_cli_reference_says_b10x_refuses_actor() {
    let b10x = |extra: &str| {
        RunSpec::from_document_str(&format!(
            r#"{{"kind":"b10x","credentials":"none","decisions":"observe","model":"m","model-endpoint":"http://127.0.0.1:1"{extra}}}"#
        ))
        .expect("a run-spec document")
    };
    assert_eq!(
        check_spec(&b10x("")),
        Ok(()),
        "precondition: the bare b10x spec is admitted"
    );
    assert!(
        check_spec(&b10x(r#","actor":"agent:x""#))
            .is_err_and(|refusal| refusal.to_string().contains("--actor")),
        "precondition: b10x refuses --actor by name"
    );

    let reference =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../website/docs/reference/cli.md");
    let text = std::fs::read_to_string(&reference).expect("the CLI reference");
    let row = text
        .lines()
        .find(|line| line.starts_with("| `--actor"))
        .expect("the --actor row");
    assert!(
        row.contains("b10x"),
        "story:actor-reaches-every-launch: the CLI reference's --actor row does not say b10x \
         refuses it: {row}"
    );
}
