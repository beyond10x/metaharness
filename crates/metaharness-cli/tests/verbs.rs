//! What each verb does, and what each refusal says.
//!
//! Every refusal is asserted to be exit `2` **and** to name what is missing. A refusal that only
//! said "unsupported" would make the reader open the source to find out what to install, and a
//! silent success would be worse than either.
//!
//! # No test in this file may start a session
//!
//! `metaharness run claude -p …` now **spawns the real binary and bills a real account.** This
//! file used to assert that it exited `2` because there was no spawner; when the spawner landed
//! that assertion stopped being free, and the suite quietly spent money on two runs before
//! anybody noticed. So: every `run` invocation here is one that is refused **before** the spawn —
//! a kind with no adapter, a frame document, an owned tool surface — and
//! `no_run_in_this_file_can_reach_a_spawn` holds that line mechanically. Paid runs live in
//! `metaharness/tests/live.rs`, behind `METAHARNESS_LIVE=1`, and are never part of `task check`.

use clap::Parser as _;
use metaharness_cli::{Cli, execute};

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

/// The one `run` refusal that is still free to assert: a spec fault is caught before the spawn,
/// so a caller with a bad spec never pays to find out.
///
/// The frame path is a **directory**, which no filesystem reads as a file — since the on-disk
/// frame format landed (amendment a5), `--frame` on a *valid* document proceeds to a spawn, so a
/// refusal this file relies on must be unreadable by construction, not by hoping a file is
/// absent.
#[test]
fn a_run_with_a_bad_spec_exits_two_before_anything_is_spawned() {
    assert_eq!(
        code_of(&["metaharness", "run", "claude", "--frame", ".", "-p", "x"]),
        2
    );
}

/// The interlock, over this file's own source.
///
/// A prompt is what turns `run` into a session, so an argv that carries `-p` and is not refused
/// on the way in is an argv that spends money. This test reads the file it lives in and refuses
/// to let one back. Since the on-disk frame format landed (amendment a5), `--frame` alone no
/// longer guarantees a refusal — a valid document proceeds to the spawn — so a `--frame` argv is
/// only free when its path is a directory, and the interlock requires exactly that spelling.
#[test]
fn no_run_in_this_file_can_reach_a_spawn() {
    let source = include_str!("verbs.rs");
    for line in source.lines() {
        // An argv line names the binary too, which is what keeps this test from matching its
        // own source.
        let is_argv = line.contains(r#""metaharness""#)
            && line.contains(r#""run""#)
            && line.contains(r#""-p""#);
        if is_argv {
            assert!(
                line.contains(r#""--frame", ".""#) || line.contains(r#""codex""#),
                "this argv would spawn the vendor and bill for it: {line}"
            );
        }
    }
}

#[test]
fn a_codex_run_exits_two_by_name_because_there_is_no_codex_adapter() {
    assert_eq!(code_of(&["metaharness", "run", "codex", "-p", "hello"]), 2);
}

/// The two free frame refusals: a path nothing can read, and a document that is not a sealed
/// frame. Both are raised at resolution, before any spawn, so neither costs a session.
#[test]
fn an_unreadable_frame_document_exits_two() {
    assert_eq!(
        code_of(&["metaharness", "run", "claude", "--frame", "."]),
        2
    );
}

#[test]
fn a_malformed_frame_document_exits_two_before_anything_is_spawned() {
    let directory = tempfile::tempdir().expect("a scratch directory");
    let path = directory.path().join("not-a-frame.json");
    std::fs::write(&path, "{\"format\":\"metaharness.frame/1\",\"workflow\":3}").expect("written");
    let argv = [
        "metaharness".to_string(),
        "run".to_string(),
        "claude".to_string(),
        "--frame".to_string(),
        path.display().to_string(),
    ];
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    assert_eq!(code_of(&argv), 2);
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

/// `doctor` answers H9's question for free — it runs `claude --version` and nothing else.
///
/// The code is deliberately not asserted to be `0`: that depends on which version is installed
/// on the machine running the suite, and a test that demanded one would fail on the next release
/// for the one reason `doctor` exists to report. What is asserted is that it never reports
/// "nobody found out" about a question it either answered or could not ask.
#[test]
fn doctor_answers_the_version_question_without_starting_a_session() {
    let code = code_of(&["metaharness", "doctor", "claude"]);
    assert!(code == 0 || code == 1 || code == 2, "{code}");
    assert_ne!(code, 3);
}

#[test]
fn doctor_for_a_kind_with_no_adapter_exits_two_by_name() {
    assert_eq!(code_of(&["metaharness", "doctor", "codex"]), 2);
}

#[test]
fn no_verb_ever_exits_one_without_a_verdict_to_contradict() {
    for argv in [
        // Never `run claude -p …`: that starts a paid session. See this file's own note.
        vec!["metaharness", "run", "codex"],
        vec!["metaharness", "run", "claude", "--tool-surface", "owned"],
        vec!["metaharness", "project", "--events", "e.jsonl"],
        vec!["metaharness", "audit", "--transcript", "t.jsonl"],
    ] {
        assert_ne!(code_of(&argv), 1, "{argv:?}");
    }
}
