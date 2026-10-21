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

// --- vendor records, through the production readers (amendment a28) ---------------------------

/// The three golden vendor records, as `(--from value, crate, record, the adapter's expected
/// stream)`. Each record is real vendor bytes (`fixtures/golden/README.md` in each crate) and each
/// expected stream is what that crate's own reader owes for it, held byte for byte by its C2
/// golden vector. Reading a committed file is not a paid run (invariant 5).
const VENDOR_RECORDS: [(&str, &str, &str, &str); 3] = [
    (
        "claude-code-stream-json",
        "metaharness-claude",
        "transcript.jsonl",
        "transcript.expected.jsonl",
    ),
    (
        "codex-rollout",
        "metaharness-codex",
        "rollout.jsonl",
        "rollout.expected.jsonl",
    ),
    (
        "harness-json",
        "metaharness-b10x",
        "loop.jsonl",
        "loop.expected.jsonl",
    ),
];

fn golden(crate_name: &str, file: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_name)
        .join("fixtures/golden")
        .join(file)
        .display()
        .to_string()
}

fn project_from(out: &std::path::Path, from: &str, record: &str) -> i32 {
    code_of(&[
        "metaharness",
        "project",
        "--from",
        from,
        record,
        "--out",
        &out.display().to_string(),
    ])
}

fn read_document(path: &std::path::Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("the document is written"))
        .expect("the document is JSON")
}

/// A document's family census without the closing marker. The expected streams carry none — they
/// are a reader's emissions, not a run — and a replayed record is closed by nobody (a28), so the
/// two censuses are compared on the families a reader produced.
fn families_without_marker(
    document: &serde_json::Value,
) -> serde_json::Map<String, serde_json::Value> {
    let mut families = document["metaharness"]["families"]
        .as_object()
        .expect("a family census")
        .clone();
    families.remove("stream_closed");
    families
}

/// The acceptance line of `story:project-reads-vendor-transcripts`: each golden record, through
/// its own production reader, exits 0 and lands in the same families as the adapter's expected
/// stream for it — which is projected here through the same verb, without `--from`.
#[test]
fn a_vendor_record_projects_through_its_production_reader_to_the_expected_census() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for (from, crate_name, record, expected) in VENDOR_RECORDS {
        let out = scratch.path().join(format!("{from}.json"));
        assert_eq!(
            project_from(&out, from, &golden(crate_name, record)),
            0,
            "{from}"
        );
        let document = read_document(&out);
        assert_eq!(document["format"], "trace-ir/1", "{from}");

        let reference = scratch.path().join(format!("{from}.expected.json"));
        assert_eq!(
            project_to(&reference, &golden(crate_name, expected)),
            0,
            "{from}"
        );
        let reference = read_document(&reference);
        assert_eq!(
            families_without_marker(&document),
            families_without_marker(&reference),
            "{from}: the census differs from the adapter's own expected stream"
        );
        assert_eq!(
            document["metaharness"]["families"]["session_start"], 1,
            "{from}"
        );
    }
}

/// Done when 1 — byte-stable: no clock, no network, one order, whichever reader produced the lines.
#[test]
fn the_same_vendor_record_projects_to_the_same_bytes_twice() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for (from, crate_name, record, _) in VENDOR_RECORDS {
        let first = scratch.path().join(format!("{from}-1.json"));
        let second = scratch.path().join(format!("{from}-2.json"));
        assert_eq!(project_from(&first, from, &golden(crate_name, record)), 0);
        assert_eq!(project_from(&second, from, &golden(crate_name, record)), 0);
        assert_eq!(
            std::fs::read(&first).expect("first"),
            std::fs::read(&second).expect("second"),
            "{from}: a projection that moved between two runs of itself is not evidence"
        );
    }
}

/// Done when 3, and P3 as a28 settles it: the document names the vendor format as its source, its
/// digest is over the record's own bytes — the file it was made from — and the vendor reference
/// beside it names the same file; the attestation claims nothing; the run is named after the file;
/// and the stream was closed by nobody, because metaharness drove nothing.
#[test]
fn a_projected_vendor_record_names_its_format_claims_no_control_and_is_closed_by_nobody() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for (from, crate_name, record, _) in VENDOR_RECORDS {
        let out = scratch.path().join(format!("{from}.json"));
        let path = golden(crate_name, record);
        assert_eq!(project_from(&out, from, &path), 0, "{from}");
        let document = read_document(&out);

        assert_eq!(document["metaharness"]["source"], from, "{from}");
        assert_eq!(document["adapter"]["name"], "metaharness/project");
        let digest = metaharness::protocol::Digest::of(
            &std::fs::read(&path).expect("the record is committed"),
        );
        assert_eq!(
            document["transcript_digest"],
            digest.as_str(),
            "{from}: the digest names the record the document was made from"
        );
        assert_eq!(
            document["metaharness"]["vendor_transcript"]["digest"],
            digest.as_str(),
            "{from}: the vendor reference names the same file, and says so"
        );
        assert_eq!(
            document["metaharness"]["vendor_transcript"]["path"], record,
            "{from}: the reference carries the file name, not the argv spelling"
        );

        let hermetic = &document["metaharness"]["hermetic"];
        assert_eq!(hermetic["mode"], "off", "{from}");
        assert_eq!(hermetic["imposed"], serde_json::json!([]), "{from}");
        assert_eq!(hermetic["unavailable"], serde_json::json!([]), "{from}");
        assert!(
            hermetic["declared"].is_null(),
            "{from}: nothing was declared"
        );
        assert_eq!(
            hermetic["installed_plugins"],
            serde_json::json!([]),
            "{from}"
        );
        assert_eq!(document["events"][0]["kind"]["event"], "session_start");

        assert_eq!(document["metaharness"]["stream_complete"], false, "{from}");
        assert!(document["metaharness"]["closed"].is_null(), "{from}");
        assert!(
            document["metaharness"]["families"]["stream_closed"].is_null(),
            "{from}: no marker, because nobody here closed this stream"
        );
        assert_eq!(
            document["metaharness"]["run"],
            record.trim_end_matches(".jsonl"),
            "{from}: the run is named after the file"
        );
    }
}

/// a28, correction 1 — under `--from` a node's `source_line` is a line of the only file the
/// document names: the record line the reader read the event from, several events of one line
/// sharing it, never earlier than the node before it, and an event the reader owed at end of
/// record citing the last line. The opaque nodes carry the reader's own statement of that line
/// (`kind.source_line`), and the two agree.
#[test]
fn a_replayed_nodes_source_line_is_the_record_line_the_reader_read_it_from() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for (from, crate_name, record, _) in VENDOR_RECORDS {
        let path = golden(crate_name, record);
        let line_count = u64::try_from(
            std::fs::read_to_string(&path)
                .expect("the record is committed")
                .lines()
                .count(),
        )
        .expect("a line count fits");
        let out = scratch.path().join(format!("{from}.json"));
        assert_eq!(project_from(&out, from, &path), 0, "{from}");
        let document = read_document(&out);
        let nodes = document["events"].as_array().expect("nodes");

        let mut previous = 1;
        for node in nodes {
            let line = node["source_line"].as_u64().expect("a line");
            assert!(
                (1..=line_count).contains(&line),
                "{from}: node {} cites line {line} of a {line_count}-line record",
                node["index"]
            );
            assert!(
                line >= previous,
                "{from}: node {} cites line {line} after a node on line {previous}",
                node["index"]
            );
            previous = line;
            if node["kind"]["event"] == "opaque" {
                assert_eq!(
                    node["source_line"], node["kind"]["source_line"],
                    "{from}: the reader's own statement of the line agrees"
                );
            }
        }
        assert_eq!(
            nodes.last().expect("a last node")["source_line"],
            line_count,
            "{from}: the terminal node the reader owed at end of record cites the last line"
        );
    }
}

/// Done when 2 — a format this build has no reader for is refused by name, and so is the event
/// stream's own name: `project` reads that without `--from`, and a spelling that silently meant
/// *no reader* would make the flag optional in disguise. Nothing is written.
#[test]
fn an_unknown_from_is_refused_by_name_and_writes_nothing() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for from in ["some-other-format", "metaharness-event-stream"] {
        let out = scratch.path().join(format!("{from}.json"));
        assert_eq!(
            project_from(
                &out,
                from,
                &golden("metaharness-claude", "transcript.jsonl")
            ),
            2,
            "{from}"
        );
        assert!(
            !out.exists(),
            "{from}: a refused projection writes no document"
        );
    }
}

/// Done when 2 — a record that is not the declared format is refused by the rule a28 states: the
/// reader's first event is not `session.started`. Never by a vendor error, because the readers
/// raise none: a line they cannot read is `opaque`, and a document opening with an opaque node over
/// the wrong reader would read exactly like a run whose first record was unreadable. Every reader
/// over every record that is not its own, the recorded event stream included.
#[test]
fn a_mis_declared_from_is_refused_rather_than_projected_as_opaque_nodes() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let claude = golden("metaharness-claude", "transcript.jsonl");
    let codex = golden("metaharness-codex", "rollout.jsonl");
    let harness = golden("metaharness-b10x", "loop.jsonl");
    let stream = run_a();
    for (from, record) in [
        ("claude-code-stream-json", codex.as_str()),
        ("claude-code-stream-json", harness.as_str()),
        ("claude-code-stream-json", stream.as_str()),
        ("codex-rollout", claude.as_str()),
        ("codex-rollout", harness.as_str()),
        ("codex-rollout", stream.as_str()),
        ("harness-json", claude.as_str()),
        ("harness-json", codex.as_str()),
        ("harness-json", stream.as_str()),
    ] {
        let out = scratch.path().join("mis-declared.json");
        assert_eq!(project_from(&out, from, record), 2, "{from} over {record}");
        assert!(!out.exists(), "{from} over {record}: nothing is written");
    }
}

/// What `--from` does not change: a vendor record handed over as an event stream is still refused
/// at its first line, exit 2. The verb's default reads one wire, and a28 adds a flag, not a guess.
#[test]
fn a_vendor_record_without_from_is_still_refused_at_its_first_line() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    for (_, crate_name, record, _) in VENDOR_RECORDS {
        assert_eq!(
            project_to(&scratch.path().join("x.json"), &golden(crate_name, record)),
            2,
            "{record}"
        );
    }
}

/// a28 point 1, correction 2 — two records that share a file name would render two columns with
/// one header. When the names are equal each column carries the first eight hex digits of its own
/// document's digest after the name; when they differ the headers are today's, which is what
/// keeps the committed `evals/aep/runs/side-by-side.html` byte-identical
/// (`the_committed_fixtures_are_what_this_build_produces`). The two records are the two recorded
/// Claude Code transcripts under `evals/aep/checks/transcripts/` (hand-written, committed; reading
/// them is not a paid run), copied under one name.
#[test]
fn two_same_named_records_are_told_apart_by_their_digests_in_the_page_header() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let transcripts = evals_dir().join("checks/transcripts");
    let a_dir = scratch.path().join("a");
    let b_dir = scratch.path().join("b");
    std::fs::create_dir_all(&a_dir).expect("a");
    std::fs::create_dir_all(&b_dir).expect("b");
    let a = a_dir.join("run.jsonl");
    let b = b_dir.join("run.jsonl");
    std::fs::copy(transcripts.join("decomposer-clean.jsonl"), &a).expect("copied");
    std::fs::copy(transcripts.join("plan-reviewer-clean.jsonl"), &b).expect("copied");
    let prefix = |path: &std::path::Path| -> String {
        metaharness::protocol::Digest::of(&std::fs::read(path).expect("read"))
            .as_str()
            .chars()
            .take(8)
            .collect()
    };

    let page = scratch.path().join("same-name.html");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            "--from",
            "claude-code-stream-json",
            "--html",
            &page.display().to_string(),
            &a.display().to_string(),
            &b.display().to_string(),
        ]),
        0
    );
    let html = std::fs::read_to_string(&page).expect("the page is written");
    assert!(
        html.contains(&format!("run · {}", prefix(&a))),
        "the left column names its digest: {html}"
    );
    assert!(
        html.contains(&format!("run · {}", prefix(&b))),
        "the right column names its digest: {html}"
    );

    // The same two records under two names keep plain headers.
    let other = b_dir.join("other.jsonl");
    std::fs::copy(&b, &other).expect("copied");
    let page = scratch.path().join("two-names.html");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            "--from",
            "claude-code-stream-json",
            "--html",
            &page.display().to_string(),
            &a.display().to_string(),
            &other.display().to_string(),
        ]),
        0
    );
    let html = std::fs::read_to_string(&page).expect("the page is written");
    assert!(
        html.contains("<div class=\"run\">run</div>")
            && html.contains("<div class=\"run\">other</div>"),
        "columns whose names differ keep their headers: {html}"
    );
    assert!(
        !html.contains("<div class=\"run\">run · ")
            && !html.contains("<div class=\"run\">other · "),
        "no digest is appended to a name nothing shares: {html}"
    );
}

/// `--from` names the format of every record given, so two vendor records of one kind read side
/// by side without first being written out as event streams.
#[test]
fn the_viewer_renders_two_vendor_records_of_one_format() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("vendor.html");
    let claude = golden("metaharness-claude", "transcript.jsonl");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            "--from",
            "claude-code-stream-json",
            "--html",
            &page.display().to_string(),
            &claude,
            &claude,
        ]),
        0
    );
    let html = std::fs::read_to_string(&page).expect("the page is written");
    assert!(
        html.contains("transcript"),
        "the column is named after the file"
    );
    assert!(html.contains("0 divergence(s)"), "{html}");
}

// --- trace-ir-summary/1 (amendment a42) ----------------------------------------------------------

/// The summary's keys, in the order a42 point 2 renders them.
const SUMMARY_KEYS: [&str; 21] = [
    "format",
    "source",
    "transcript_digest",
    "adapter",
    "model",
    "models",
    "turns",
    "tokens_basis",
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
    "cost_usd",
    "duration_ms",
    "duration_api_ms",
    "first_timestamp_ms",
    "last_timestamp_ms",
    "is_error",
    "stop_reason",
    "tool_calls",
    "unk",
];

fn project_as(out: &std::path::Path, from: Option<&str>, record: &str, to: &str) -> i32 {
    let out = out.display().to_string();
    let mut argv = vec!["metaharness", "project"];
    if let Some(from) = from {
        argv.extend(["--from", from]);
    }
    argv.extend([record, "--to", to, "--out", &out]);
    code_of(&argv)
}

/// Projects `record` twice to `trace-ir-summary` and once to `trace-ir`, holds the two summaries
/// to the same bytes in a42's key order, and returns the summary and the document.
fn summary_and_document(
    label: &str,
    from: Option<&str>,
    record: &str,
) -> (serde_json::Value, serde_json::Value) {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let summary_path = scratch.path().join("summary.json");
    let again_path = scratch.path().join("summary-again.json");
    let document_path = scratch.path().join("document.json");
    assert_eq!(
        project_as(&summary_path, from, record, "trace-ir-summary"),
        0,
        "{label}"
    );
    assert_eq!(
        project_as(&again_path, from, record, "trace-ir-summary"),
        0,
        "{label}"
    );
    assert_eq!(
        project_as(&document_path, from, record, "trace-ir"),
        0,
        "{label}"
    );

    let text = std::fs::read_to_string(&summary_path).expect("the summary is written");
    assert_eq!(
        text,
        std::fs::read_to_string(&again_path).expect("the summary is written"),
        "{label}: the same record summarizes to the same bytes"
    );
    assert!(text.ends_with("}\n"), "{label}: one document, one newline");
    let positions: Vec<usize> = SUMMARY_KEYS
        .iter()
        .map(|key| {
            text.find(&format!("\n  \"{key}\":"))
                .unwrap_or_else(|| panic!("{label}: the summary carries {key}"))
        })
        .collect();
    assert!(
        positions.windows(2).all(|pair| pair[0] < pair[1]),
        "{label}: keys render in a42's order"
    );
    let summary = serde_json::from_str(&text).expect("the summary is JSON");
    (summary, read_document(&document_path))
}

/// a42 point 3: the four token counts are the terminal node's `usage` when it is an object
/// (`"session"`), else the request nodes' `usage` summed per key with an unstated key `null`
/// (`"sum-of-requests"`), else `null` with no basis.
fn assert_tokens_as_the_basis_says(
    label: &str,
    summary: &serde_json::Value,
    events: &[serde_json::Value],
    terminal: &serde_json::Value,
) {
    let requests: Vec<&serde_json::Value> = events
        .iter()
        .map(|node| &node["kind"])
        .filter(|kind| {
            kind["event"] == "run_outcome"
                && kind
                    .as_object()
                    .is_some_and(|kind| !kind.contains_key("num_turns"))
                && kind["usage"].is_object()
        })
        .map(|kind| &kind["usage"])
        .collect();
    let basis = if terminal["usage"].is_object() {
        "session"
    } else if requests.is_empty() {
        ""
    } else {
        "sum-of-requests"
    };
    for key in [
        "input_tokens",
        "output_tokens",
        "cache_read_input_tokens",
        "cache_creation_input_tokens",
    ] {
        let stated = match basis {
            "session" => terminal["usage"].get(key).cloned().unwrap_or_default(),
            "sum-of-requests" => {
                let figures: Vec<u64> = requests
                    .iter()
                    .filter_map(|usage| usage[key].as_u64())
                    .collect();
                if figures.is_empty() {
                    serde_json::Value::Null
                } else {
                    serde_json::Value::from(figures.iter().sum::<u64>())
                }
            }
            _ => serde_json::Value::Null,
        };
        assert_eq!(
            summary[key], stated,
            "{label}: {key} read as {basis:?} says"
        );
    }
    if basis.is_empty() {
        assert!(summary["tokens_basis"].is_null(), "{label}");
    } else {
        assert_eq!(summary["tokens_basis"], basis, "{label}");
    }
}

/// a42 point 2: `models` names every model the opening and `run_outcome` nodes name, first seen,
/// once, each with the terminal node's `model_usage` figures for it and `null` where none is
/// stated.
fn assert_models_as_named(
    label: &str,
    summary: &serde_json::Value,
    events: &[serde_json::Value],
    terminal: &serde_json::Value,
) {
    let mut named: Vec<&str> = Vec::new();
    for kind in events
        .iter()
        .map(|node| &node["kind"])
        .filter(|kind| kind["event"] == "session_start" || kind["event"] == "run_outcome")
    {
        let usage_keys = kind["model_usage"]
            .as_object()
            .into_iter()
            .flat_map(|by_model| by_model.keys().map(String::as_str));
        let observed = kind["observed_models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["model"].as_str());
        for name in kind["model"]
            .as_str()
            .into_iter()
            .chain(usage_keys)
            .chain(observed)
        {
            if !named.contains(&name) {
                named.push(name);
            }
        }
    }
    let models = summary["models"].as_array().expect("a list of models");
    let summarized: Vec<&str> = models
        .iter()
        .map(|model| model["name"].as_str().expect("a name"))
        .collect();
    assert_eq!(
        summarized, named,
        "{label}: every named model, first seen, once"
    );
    for model in models {
        let per_model = &terminal["model_usage"][model["name"].as_str().expect("a name")];
        for key in [
            "input_tokens",
            "output_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "cost_usd",
        ] {
            let stated = per_model.get(key).cloned().unwrap_or_default();
            assert_eq!(model[key], stated, "{label}: {} {key}", model["name"]);
        }
    }
}

/// Holds the summary of `record` to the document the same verb writes: it names that document,
/// reads its opening and terminal nodes, and leaves exactly `nulls` null.
fn assert_summary_of_document(from: Option<&str>, record: &str, nulls: &[&str]) {
    let label = from.unwrap_or("metaharness.event/1");
    let (summary, document) = summary_and_document(label, from, record);
    assert_eq!(
        summary.as_object().map(serde_json::Map::len),
        Some(SUMMARY_KEYS.len()),
        "{label}: no key beyond a42's"
    );
    assert_eq!(summary["format"], "trace-ir-summary/1", "{label}");
    assert_eq!(summary["source"], label, "{label}");
    assert_eq!(
        summary["source"], document["metaharness"]["source"],
        "{label}"
    );
    assert_eq!(
        summary["transcript_digest"], document["transcript_digest"],
        "{label}: the summary names the document it was made from"
    );
    let events = document["events"].as_array().expect("nodes");
    assert_eq!(
        summary["adapter"], events[0]["kind"]["adapter"],
        "{label}: the opening node's adapter"
    );
    assert_eq!(
        summary["model"], events[0]["kind"]["model"],
        "{label}: the opening node's model"
    );
    let terminal = &events
        .iter()
        .rev()
        .find(|node| {
            node["kind"]["event"] == "run_outcome"
                && node["kind"]
                    .as_object()
                    .is_some_and(|kind| kind.contains_key("num_turns"))
        })
        .expect("a terminal node")["kind"];
    assert_eq!(summary["turns"], terminal["num_turns"], "{label}");
    assert_tokens_as_the_basis_says(label, &summary, events, terminal);
    assert_models_as_named(label, &summary, events, terminal);
    assert_eq!(summary["cost_usd"], terminal["total_cost_usd"], "{label}");
    for key in ["duration_ms", "duration_api_ms", "is_error", "stop_reason"] {
        assert_eq!(summary[key], terminal[key], "{label}: {key}");
    }
    let calls: Vec<&serde_json::Value> = events
        .iter()
        .filter(|node| node["kind"]["event"] == "tool_call")
        .map(|node| &node["kind"]["name"])
        .collect();
    let summarized: Vec<&serde_json::Value> = summary["tool_calls"]
        .as_array()
        .expect("a list of calls")
        .iter()
        .map(|call| &call["name"])
        .collect();
    assert_eq!(summarized, calls, "{label}: one entry per call, in order");
    let unk: u64 = document["metaharness"]["unk_kinds"]
        .as_object()
        .expect("the unk kinds")
        .values()
        .filter_map(serde_json::Value::as_u64)
        .sum();
    assert_eq!(summary["unk"], unk, "{label}: the document's unk count");

    for key in SUMMARY_KEYS {
        assert_eq!(
            summary[key].is_null(),
            nulls.contains(&key),
            "{label}: {key} is null exactly when the record does not state it ({})",
            summary[key]
        );
    }
}

/// a42 point 5, with `--from`: each golden vendor record summarizes through its production reader,
/// and each leaves `null` exactly what its record does not state (a42, *What the goldens leave
/// null*): never `0`, never `""`.
#[test]
fn each_golden_vendor_record_summarizes_to_trace_ir_summary() {
    let nulls: [(&str, &[&str]); 3] = [
        ("claude-code-stream-json", &[]),
        (
            "codex-rollout",
            &[
                "model",
                "cache_creation_input_tokens",
                "cost_usd",
                "duration_api_ms",
                "stop_reason",
            ],
        ),
        (
            "harness-json",
            &[
                "cache_creation_input_tokens",
                "cost_usd",
                "duration_ms",
                "duration_api_ms",
                "first_timestamp_ms",
                "last_timestamp_ms",
                "stop_reason",
            ],
        ),
    ];
    for ((from, crate_name, record, _), (named, nulls)) in VENDOR_RECORDS.iter().zip(nulls) {
        assert_eq!(*from, named);
        assert_summary_of_document(Some(from), &golden(crate_name, record), nulls);
    }
}

/// a42 point 5, without `--from`: the recorded event stream summarizes as the verb reads it, named
/// by the stream's own digest; none of its results states `is_error`, so each call reads
/// `answered: true, error: null`.
#[test]
fn the_recorded_event_stream_summarizes_without_from() {
    assert_summary_of_document(None, &run_a(), &[]);
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("summary.json");
    assert_eq!(project_as(&out, None, &run_a(), "trace-ir-summary"), 0);
    let summary = read_document(&out);
    let calls = summary["tool_calls"].as_array().expect("a list of calls");
    assert_eq!(calls.len(), 6);
    for call in calls {
        assert_eq!(call["answered"], true, "{call}");
        assert!(call["error"].is_null(), "{call}");
    }
}

/// a42 point 5: the page renders documents, so a summary asked for beside `--html` is refused,
/// not rendered as a page or written as a document.
#[test]
fn a_summary_with_html_is_refused_and_writes_nothing() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("page.html");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            &run_a(),
            "--to",
            "trace-ir-summary",
            "--html",
            &page.display().to_string(),
        ]),
        2
    );
    assert!(!page.exists(), "a refused summary writes no page");
}
