//! `metaharness project` — the `trace-ir/1` reader, and the two-column viewer it feeds.
//!
//! Decided by `docs/design/runs-side-by-side-v0.1.md` (P1–P4, V1–V3) and by amendment a15 to the
//! protocol design. Nothing here reaches a model, a network or a credential: every subject is a
//! committed file.
//!
//! The fixtures are the two recorded `evals/aep` runs, converted to `metaharness.event/1` by
//! `metaharness-claude/tests/recorded_runs.rs`. Reading a committed file is not running an eval;
//! see that file's note on invariant 5.

use std::path::PathBuf;

use clap::Parser as _;
use metaharness_cli::{Cli, execute};

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

fn evals_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evals/aep")
}

fn run_a() -> String {
    evals_dir()
        .join("runs/decomposer-clean.events.jsonl")
        .display()
        .to_string()
}

fn run_b() -> String {
    evals_dir()
        .join("runs/plan-reviewer-clean.events.jsonl")
        .display()
        .to_string()
}

fn project_to(out: &std::path::Path, events: &str) -> i32 {
    code_of(&[
        "metaharness",
        "project",
        events,
        "--out",
        &out.display().to_string(),
    ])
}

/// P1 — the verb no longer refuses, and what it writes is tagged.
#[test]
fn project_writes_a_trace_ir_document_and_exits_zero() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("run-a.trace-ir.json");
    assert_eq!(project_to(&out, &run_a()), 0);

    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).expect("the document is written"))
            .expect("the document is JSON");
    assert_eq!(document["format"], "trace-ir/1");
    assert_eq!(document["adapter"]["name"], "metaharness/project");
    assert!(
        document["events"]
            .as_array()
            .is_some_and(|events| events.len() == 16),
        "the sixteen event lines are sixteen nodes — fifteen, and the closing marker (a17)"
    );
}

/// P1 — byte-stable. The same input twice is the same bytes: no clock, no network, one order.
#[test]
fn the_same_event_stream_projects_to_the_same_bytes() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let first = scratch.path().join("first.json");
    let second = scratch.path().join("second.json");
    assert_eq!(project_to(&first, &run_a()), 0);
    assert_eq!(project_to(&second, &run_a()), 0);
    assert_eq!(
        std::fs::read(&first).expect("first"),
        std::fs::read(&second).expect("second"),
        "a projection that moved between two runs of itself is not evidence"
    );
}

/// P3 — `transcript_digest` names the **event stream's** bytes, and the vendor's own reference
/// travels beside it under its own name rather than pretending to be that digest.
#[test]
fn the_documents_digest_is_over_the_event_stream_and_the_vendor_reference_is_its_own_field() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("run-a.json");
    assert_eq!(project_to(&out, &run_a()), 0);
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).expect("written")).expect("JSON");

    let bytes = std::fs::read(run_a()).expect("the stream is committed");
    let expected = metaharness::protocol::Digest::of(&bytes);
    assert_eq!(document["transcript_digest"], expected.as_str());
    assert_eq!(document["metaharness"]["source"], "metaharness.event/1");
    assert!(
        document["metaharness"]["vendor_transcript"]["digest"].is_string(),
        "the vendor's own digest is carried, and it is not the one above"
    );
    assert_ne!(
        document["metaharness"]["vendor_transcript"]["digest"],
        document["transcript_digest"]
    );
}

/// P2 — a control-plane event is a node, and it says which kind it was.
#[test]
fn a_control_plane_event_is_written_as_an_unk_node_carrying_its_kind() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let stream = scratch.path().join("control.jsonl");
    std::fs::write(
        &stream,
        concat!(
            r#"{"format":"metaharness.event/1","seq":1,"run":"r","event":"warning","code":"COVERAGE_GAP","message":"a tool nothing covers"}"#,
            "\n",
            r#"{"format":"metaharness.event/1","seq":2,"run":"r","event":"turn.started","turn":1,"frame_digest":null}"#,
            "\n",
        ),
    )
    .expect("the stream is written");

    let out = scratch.path().join("control.json");
    assert_eq!(project_to(&out, &stream.display().to_string()), 0);
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).expect("written")).expect("JSON");
    let events = document["events"].as_array().expect("events");

    assert_eq!(events.len(), 2, "neither event was dropped");
    for (node, kind) in events.iter().zip(["warning", "turn.started"]) {
        assert_eq!(node["kind"]["event"], "unk");
        assert_eq!(node["kind"]["event_kind"], kind);
        assert_eq!(node["kind"]["reason"], "no trace-ir/1 family");
        assert_ne!(
            node["kind"]["event"], "opaque",
            "`opaque` means the vendor said something unreadable; this is not that"
        );
    }
    assert_eq!(document["metaharness"]["unk_kinds"]["warning"], 1);
    assert_eq!(document["metaharness"]["unk_kinds"]["turn.started"], 1);
}

/// Amendment a17 — the closing marker is a terminal node of its own, and the document says the
/// stream is whole. A checker reading this can tell an absence from a truncation, which is the one
/// thing that makes a negative expectation decidable.
#[test]
fn the_closing_marker_is_a_terminal_node_and_the_document_states_the_stream_is_complete() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("run-a.trace-ir.json");
    assert_eq!(project_to(&out, &run_a()), 0);
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).expect("written")).expect("JSON");

    let events = document["events"].as_array().expect("events");
    let last = events.last().expect("a last node");
    assert_eq!(last["kind"]["event"], "stream_closed");
    assert_ne!(
        last["kind"]["event"], "unk",
        "`unk` means the IR has no family for this; the marker is the node a check decides on"
    );
    assert_eq!(last["kind"]["events"], 15);
    assert_eq!(last["kind"]["reason"], "completed");
    assert_eq!(last["kind"]["run_id"], "decomposer-clean");

    assert_eq!(document["metaharness"]["stream_complete"], true);
    assert_eq!(document["metaharness"]["closed"]["events"], 15);
    assert_eq!(document["metaharness"]["closed"]["reason"], "completed");
    assert_eq!(document["metaharness"]["families"]["stream_closed"], 1);
    assert!(
        document["metaharness"]["unk_kinds"]["stream.closed"].is_null(),
        "the marker is not filed among the protocol-vocabulary gaps"
    );
}

/// A stream with no marker is not reported as whole. It is the reading amendment a17 exists to
/// end: *no marker* is *nobody closed this file*, never *nothing happened in it*.
#[test]
fn a_stream_with_no_closing_marker_is_not_complete() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let stream = scratch.path().join("cut-off.jsonl");
    std::fs::write(
        &stream,
        concat!(
            r#"{"format":"metaharness.event/1","seq":1,"run":"r","event":"text","text":"hello","request_id":null}"#,
            "
",
        ),
    )
    .expect("written");

    let out = scratch.path().join("cut-off.json");
    assert_eq!(project_to(&out, &stream.display().to_string()), 0);
    let document: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&out).expect("written")).expect("JSON");
    assert_eq!(document["metaharness"]["stream_complete"], false);
    assert!(document["metaharness"]["closed"].is_null());
}

/// A line this build cannot read is a refusal that names it, not a silently shorter document.
#[test]
fn a_stream_line_the_build_cannot_read_is_refused_by_name() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let stream = scratch.path().join("broken.jsonl");
    std::fs::write(
        &stream,
        "{\"format\":\"metaharness.event/1\",\"seq\":1,\"run\":\"r\",\"event\":\"not.an.event\"}\n",
    )
    .expect("written");
    assert_eq!(
        project_to(
            &scratch.path().join("out.json"),
            &stream.display().to_string()
        ),
        2
    );
}

/// A form this build does not write is refused rather than silently producing `trace-ir`.
#[test]
fn an_unknown_target_form_is_refused() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            &run_a(),
            "--to",
            "some-other-ir",
            "--out",
            &scratch.path().join("x.json").display().to_string(),
        ]),
        2
    );
}

// --- the viewer -------------------------------------------------------------------------------

fn render_html(out: &std::path::Path, runs: &[&str]) -> i32 {
    let mut argv = vec![
        "metaharness".to_string(),
        "project".to_string(),
        "--html".to_string(),
        out.display().to_string(),
    ];
    argv.extend(runs.iter().map(ToString::to_string));
    let argv: Vec<&str> = argv.iter().map(String::as_str).collect();
    code_of(&argv)
}

/// V3 — one file, no server, no network, and the two runs are both in it.
#[test]
fn the_viewer_renders_two_runs_into_one_static_page() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("side-by-side.html");
    assert_eq!(render_html(&page, &[&run_a(), &run_b()]), 0);

    let html = std::fs::read_to_string(&page).expect("the page is written");
    assert!(html.starts_with("<!doctype html>"), "a whole document");
    assert!(html.contains("decomposer-clean") && html.contains("plan-reviewer-clean"));
    assert!(
        !html.contains("<script src=") && !html.contains("<link rel=\"stylesheet\""),
        "no external fetch: the page has to work from a file:// URL"
    );
    assert!(
        html.contains("aligned by tool-call index"),
        "V1 says which rule was used"
    );
}

/// V2 — a step in one column and not the other is a row, and it is marked as a divergence.
#[test]
fn a_call_present_in_one_run_and_absent_in_the_other_is_a_gap_row() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("side-by-side.html");
    assert_eq!(render_html(&page, &[&run_a(), &run_b()]), 0);
    let html = std::fs::read_to_string(&page).expect("written");

    // Six calls against five: the sixth row has one column and a gap in the other.
    assert!(html.contains("class=\"gap\""), "the gap is a rendered cell");
    assert!(
        html.contains("divergence"),
        "and the reader is told where it is"
    );
}

/// One run is a page too — a column with nothing beside it is still a reading surface.
#[test]
fn the_viewer_renders_one_run() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("one.html");
    assert_eq!(render_html(&page, &[&run_a()]), 0);
    let html = std::fs::read_to_string(&page).expect("written");
    assert!(html.contains("decomposer-clean"));
    assert!(!html.contains("plan-reviewer-clean"));
}

/// Three runs is a refusal: two columns is the decided shape, and a third would have to be
/// aligned against something this design does not decide.
#[test]
fn a_third_run_is_refused_rather_than_dropped() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    assert_eq!(
        render_html(
            &scratch.path().join("three.html"),
            &[&run_a(), &run_b(), &run_a()]
        ),
        2
    );
}

/// V3 — deterministic bytes, which is what lets the fixture below be a check rather than a
/// screenshot somebody looked at once.
#[test]
fn the_page_is_the_same_bytes_twice() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let first = scratch.path().join("a.html");
    let second = scratch.path().join("b.html");
    assert_eq!(render_html(&first, &[&run_a(), &run_b()]), 0);
    assert_eq!(render_html(&second, &[&run_a(), &run_b()]), 0);
    assert_eq!(
        std::fs::read(&first).expect("a"),
        std::fs::read(&second).expect("b")
    );
}

// --- the committed fixtures --------------------------------------------------------------------

/// The two recorded runs render as the fixture, and the fixture is bytes.
#[test]
fn the_committed_fixtures_are_what_this_build_produces() {
    let scratch = tempfile::tempdir().expect("a scratch directory");

    for (stream, committed) in [
        (run_a(), "runs/decomposer-clean.trace-ir.json"),
        (run_b(), "runs/plan-reviewer-clean.trace-ir.json"),
    ] {
        let out = scratch.path().join("document.json");
        assert_eq!(project_to(&out, &stream), 0);
        assert_eq!(
            std::fs::read_to_string(&out).expect("written"),
            std::fs::read_to_string(evals_dir().join(committed)).expect("committed"),
            "{committed} moved; regenerate it deliberately and read the diff"
        );
    }

    let page = scratch.path().join("page.html");
    assert_eq!(render_html(&page, &[&run_a(), &run_b()]), 0);
    assert_eq!(
        std::fs::read_to_string(&page).expect("written"),
        std::fs::read_to_string(evals_dir().join("runs/side-by-side.html")).expect("committed"),
        "runs/side-by-side.html moved; regenerate it deliberately and read the diff"
    );
}

/// Rewrite the committed fixtures. `#[ignore]`d because it writes into the source tree:
///
/// ```console
/// cargo test -p metaharness-cli --test project regenerate -- --ignored
/// ```
#[test]
#[ignore = "writes evals/aep/runs/*.trace-ir.json and side-by-side.html; run after a deliberate change, then read the diff"]
fn regenerate_the_committed_fixtures() {
    for (stream, committed) in [
        (run_a(), "runs/decomposer-clean.trace-ir.json"),
        (run_b(), "runs/plan-reviewer-clean.trace-ir.json"),
    ] {
        assert_eq!(project_to(&evals_dir().join(committed), &stream), 0);
    }
    assert_eq!(
        render_html(
            &evals_dir().join("runs/side-by-side.html"),
            &[&run_a(), &run_b()]
        ),
        0
    );
}
