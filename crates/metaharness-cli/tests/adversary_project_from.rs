//! Adversary cases over `metaharness project --from` (amendment a28), written against the
//! working tree of `story:project-reads-vendor-transcripts`. Not the unit's own suite: these are
//! the statements its cases do not make. Every case drives the built binary, so a refusal's words
//! are read off stderr rather than assumed, and nothing here reaches a model or a credential.

use std::path::{Path, PathBuf};
use std::process::Command;

fn golden(crate_name: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_name)
        .join("fixtures/golden")
        .join(file)
}

struct Outcome {
    code: i32,
    stderr: String,
}

fn project_from(from: &str, record: &Path, out: &Path) -> Outcome {
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["project", "--from", from])
        .arg(record)
        .arg("--out")
        .arg(out)
        .output()
        .expect("the binary runs");
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn read_document(path: &Path) -> serde_json::Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("the document is written"))
        .expect("the document is JSON")
}

/// a28 point 4: the rule is about the reader's **first** event. Every cross pair the unit pins
/// hands a reader a record in which no line is its own, so a rule asking whether *any* event is
/// `session.started` would refuse each of them just the same. This record is one foreign line and
/// then a real Claude Code transcript: it must be refused at line 1, naming what that line was,
/// and never projected behind an opaque opening node.
#[test]
fn a_record_with_a_foreign_first_line_is_refused_at_line_one_not_projected_behind_it() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let claude = std::fs::read_to_string(golden("metaharness-claude", "transcript.jsonl"))
        .expect("the golden transcript is committed");
    let codex = std::fs::read_to_string(golden("metaharness-codex", "rollout.jsonl"))
        .expect("the golden rollout is committed");
    let foreign_first = codex.lines().next().expect("the rollout has a first line");
    let record = scratch.path().join("prefixed.jsonl");
    std::fs::write(&record, format!("{foreign_first}\n{claude}")).expect("the record is written");
    let out = scratch.path().join("prefixed.json");

    let outcome = project_from("claude-code-stream-json", &record, &out);
    assert_eq!(outcome.code, 2, "stderr: {}", outcome.stderr);
    assert!(!out.exists(), "a refused projection writes no document");
    assert!(
        outcome.stderr.contains("claude-code-stream-json"),
        "{}",
        outcome.stderr
    );
    assert!(outcome.stderr.contains("line 1"), "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("session_meta"),
        "{}",
        outcome.stderr
    );
}

/// a28 point 2: the b10x factory is given no version, model or cwd, "because the verb knows none",
/// so the opening record reads `harness_version: null` and the model comes from the record.
/// Invariant 4 in one node: a replay that named a version nobody measured would be an adapter
/// claim about a vendor binary that was never pinned or verified for this file. Since a31 the
/// `cwd` comes from the record too, like the model: the golden's `started` line names
/// `/scratch/run/work` (harness 0.15.1 writes it), and the verb, told none, adds nothing.
#[test]
fn a_harness_replay_claims_no_version_the_verb_was_not_told_and_the_records_own_cwd() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("loop.json");
    let outcome = project_from(
        "harness-json",
        &golden("metaharness-b10x", "loop.jsonl"),
        &out,
    );
    assert_eq!(outcome.code, 0, "stderr: {}", outcome.stderr);
    let document = read_document(&out);
    let opening = &document["events"][0]["kind"];
    assert_eq!(opening["event"], "session_start", "{opening}");
    assert_eq!(opening["adapter"], "b10x", "{opening}");
    assert!(
        opening["harness_version"].is_null(),
        "the verb was told no version: {opening}"
    );
    assert_eq!(
        opening["cwd"], "/scratch/run/work",
        "the verb was told no cwd, so the node states the record's own: {opening}"
    );
    assert_eq!(
        opening["model"], "b10x-emulated",
        "the model is the record's own: {opening}"
    );
}

/// Pass 2, on the F5 correction. a28 point 4 promises the refusal names "what the first event was
/// (an opaque node's vendor type and source line, where that is what it was)". The loop's reader
/// writes its adapter id as `vendor_type` and the line's own `kind` as `vendor_subtype`
/// (`metaharness-b10x/src/seam.rs:113-120`), so for a harness record opening with a kind this
/// adapter does not know, the line's own word is the subtype — and the corrected wording drops it.
#[test]
fn a_harness_record_opening_with_an_unknown_kind_is_refused_naming_that_kind() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let loop_record = std::fs::read_to_string(golden("metaharness-b10x", "loop.jsonl"))
        .expect("the golden loop record is committed");
    let record = scratch.path().join("newer-loop.jsonl");
    std::fs::write(
        &record,
        format!("{{\"kind\":\"something-new\",\"x\":1}}\n{loop_record}"),
    )
    .expect("the record is written");
    let out = scratch.path().join("newer-loop.json");

    let outcome = project_from("harness-json", &record, &out);
    assert_eq!(outcome.code, 2, "stderr: {}", outcome.stderr);
    assert!(!out.exists(), "a refused projection writes no document");
    assert!(outcome.stderr.contains("line 1"), "{}", outcome.stderr);
    assert!(
        outcome.stderr.contains("something-new"),
        "the refusal names the line's own kind, which the reader put in vendor_subtype: {}",
        outcome.stderr
    );
}

/// Pass 2, on the F6 correction: the `--html` digest label over a vendor record is the document's
/// `metaharness.source`, and the wire path keeps `stream` (pinned by the committed
/// `evals/aep/runs/side-by-side.html`). Nothing else pins the vendor half; this does.
#[test]
fn the_viewer_labels_a_vendor_records_digest_by_its_source() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let page = scratch.path().join("codex.html");
    let record = golden("metaharness-codex", "rollout.jsonl");
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["project", "--from", "codex-rollout", "--html"])
        .arg(&page)
        .arg(&record)
        .output()
        .expect("the binary runs");
    assert_eq!(output.status.code(), Some(0));
    let html = std::fs::read_to_string(&page).expect("the page is written");
    let digest = metaharness::protocol::Digest::of(&std::fs::read(&record).expect("committed"));
    assert!(
        html.contains(&format!("codex-rollout sha256:{}", digest.as_str())),
        "{html}"
    );
    assert!(
        !html.contains("stream sha256:"),
        "a page over a record never calls the record's digest a stream's: {html}"
    );
}

/// `TraceIrEvent::source_line` is documented as "the 1-based line of the event stream it came
/// from", total because one stream line is one event. Under `--from` there is no stream file: the
/// only file the document names (`transcript_digest`, `vendor_transcript`) is the record, and one
/// record line can carry several events. The readers say which record line an opaque node came
/// from (`kind.source_line`); the node's own `source_line` should agree with it, or a verdict that
/// cites it points a reader at the wrong line of the only file there is.
#[test]
fn a_node_of_a_replayed_record_cites_the_record_line_it_came_from() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("rollout.json");
    let outcome = project_from(
        "codex-rollout",
        &golden("metaharness-codex", "rollout.jsonl"),
        &out,
    );
    assert_eq!(outcome.code, 0, "stderr: {}", outcome.stderr);
    let document = read_document(&out);
    let opaque: Vec<&serde_json::Value> = document["events"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["kind"]["event"] == "opaque")
        .collect();
    assert!(!opaque.is_empty(), "the golden rollout has opaque nodes");
    for node in opaque {
        assert_eq!(
            node["source_line"], node["kind"]["source_line"],
            "node {} cites stream line {} but the reader read it off record line {}",
            node["index"], node["source_line"], node["kind"]["source_line"]
        );
    }
}

/// Done when 1 reads "byte-stable across two runs", and P1 says the document is a function of the
/// record: no clock, no network, one order. A record named by an absolute path and the same record
/// named by a relative one are one record. If the two documents differ, the document carries the
/// spelling of its own argv, and a projection committed from one working directory moves when it
/// is regenerated from another.
#[test]
fn the_same_record_projects_to_the_same_bytes_however_its_path_is_spelled() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let absolute = golden("metaharness-claude", "transcript.jsonl")
        .canonicalize()
        .expect("the golden transcript is committed");
    // `cargo test` runs an integration test with the package directory as its working directory.
    let relative = Path::new("../metaharness-claude/fixtures/golden/transcript.jsonl");
    assert!(
        relative.exists(),
        "the relative spelling names the same file from {}",
        std::env::current_dir().expect("a cwd").display()
    );

    let from_absolute = scratch.path().join("absolute.json");
    let from_relative = scratch.path().join("relative.json");
    let outcome = project_from("claude-code-stream-json", &absolute, &from_absolute);
    assert_eq!(outcome.code, 0, "stderr: {}", outcome.stderr);
    let outcome = project_from("claude-code-stream-json", relative, &from_relative);
    assert_eq!(outcome.code, 0, "stderr: {}", outcome.stderr);

    let a = read_document(&from_absolute);
    let r = read_document(&from_relative);
    assert_eq!(
        a["transcript_digest"], r["transcript_digest"],
        "one file, one digest"
    );
    assert_eq!(
        std::fs::read(&from_absolute).expect("absolute"),
        std::fs::read(&from_relative).expect("relative"),
        "the document is a function of the record, not of how its path was typed; the fields \
         that moved: vendor_transcript {} vs {}",
        a["metaharness"]["vendor_transcript"],
        r["metaharness"]["vendor_transcript"]
    );
}
