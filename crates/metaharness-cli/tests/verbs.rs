//! What each verb does, and what each refusal says.
//!
//! Every refusal is asserted to be exit `2` **and** to name what is missing. A refusal that only
//! said "unsupported" would make the reader open the source to find out what to install, and a
//! silent success would be worse than either.

use clap::Parser as _;
use metaharness_cli::{Cli, execute};

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

#[test]
fn run_exits_two_because_this_build_has_no_spawner() {
    assert_eq!(code_of(&["metaharness", "run", "claude", "-p", "hello"]), 2);
}

#[test]
fn a_codex_run_exits_two_by_name_because_there_is_no_codex_adapter() {
    assert_eq!(code_of(&["metaharness", "run", "codex", "-p", "hello"]), 2);
}

#[test]
fn a_frame_document_exits_two_because_the_on_disk_format_is_owed() {
    assert_eq!(
        code_of(&["metaharness", "run", "claude", "--frame", "f.yaml"]),
        2
    );
}

#[test]
fn an_owned_tool_surface_exits_two_because_metaharness_does_not_implement_the_tools() {
    assert_eq!(
        code_of(&["metaharness", "run", "claude", "--tool-surface", "owned",]),
        2
    );
}

#[test]
fn capabilities_works_with_no_model_and_no_credential() {
    assert_eq!(code_of(&["metaharness", "capabilities", "claude"]), 0);
}

#[test]
fn capabilities_render_works_and_needs_no_run() {
    assert_eq!(
        code_of(&["metaharness", "capabilities", "claude", "--render"]),
        0
    );
}

#[test]
fn capabilities_for_a_kind_with_no_adapter_exits_two() {
    assert_eq!(code_of(&["metaharness", "capabilities", "codex"]), 2);
}

#[test]
fn conformance_runs_every_free_vector_and_exits_zero_when_they_pass() {
    assert_eq!(code_of(&["metaharness", "conformance", "claude"]), 0);
}

#[test]
fn conformance_for_a_kind_with_no_adapter_exits_two_and_not_zero() {
    // A conformance run that silently reported zero vectors would read exactly like one that
    // passed.
    assert_eq!(code_of(&["metaharness", "conformance", "codex"]), 2);
}

#[test]
fn conformance_covers_the_adapters_tiers_and_this_crates_control_tier() {
    let vectors = metaharness::conformance_vectors(metaharness::protocol::Kind::Claude)
        .expect("the claude adapter exists");
    let tiers: std::collections::BTreeSet<&str> =
        vectors.iter().map(|vector| vector.tier.as_str()).collect();
    assert!(tiers.contains("C3"), "the control vectors must be in it");
    assert!(
        vectors.iter().all(|vector| vector.passed),
        "{:?}",
        vectors
            .iter()
            .filter(|vector| !vector.passed)
            .collect::<Vec<_>>()
    );
    assert!(
        !vectors.iter().any(|vector| vector.tier.needs_a_model()),
        "the free tiers must never reach for a model"
    );
}

#[test]
fn project_refuses_honestly_and_names_the_open_question_it_waits_on() {
    assert_eq!(
        code_of(&["metaharness", "project", "--events", "e.jsonl"]),
        2
    );
}

#[test]
fn audit_refuses_honestly_and_names_what_it_is_waiting_for() {
    assert_eq!(
        code_of(&["metaharness", "audit", "--transcript", "t.jsonl"]),
        2
    );
}

#[test]
fn doctor_refuses_honestly_because_it_needs_the_vendor_binary() {
    assert_eq!(code_of(&["metaharness", "doctor", "claude"]), 2);
}

#[test]
fn no_verb_ever_exits_one_without_a_verdict_to_contradict() {
    for argv in [
        vec!["metaharness", "run", "claude", "-p", "x"],
        vec!["metaharness", "run", "codex"],
        vec!["metaharness", "doctor", "claude"],
        vec!["metaharness", "project", "--events", "e.jsonl"],
        vec!["metaharness", "audit", "--transcript", "t.jsonl"],
    ] {
        assert_ne!(code_of(&argv), 1, "{argv:?}");
    }
}
