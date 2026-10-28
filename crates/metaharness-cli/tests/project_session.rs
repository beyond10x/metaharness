//! `metaharness project --from claude-code-session` — story:session-jsonl-adapter.
//!
//! A Claude Code session as Claude Code stores it on disk, with its sub-agent transcripts under
//! `<session>/subagents/`, read through `metaharness_claude::SessionReader`. Every subject is the
//! committed synthetic fixture under `metaharness-claude/fixtures/golden/session/`: hand-written,
//! no recorded content (`fixtures/golden/session/README.md`).

use std::path::PathBuf;

use clap::Parser as _;
use metaharness_cli::{Cli, execute};
use serde_json::Value;

const SESSION: &str = "5e551011-0000-4000-8000-000000000001";
const SPAWNING_CALL: &str = "toolu_fixture_02";
const AGENT: &str = "a0f1c2d3e4b5a697";
const SUBAGENT_CALL: &str = "toolu_fixture_11";

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

fn session_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../metaharness-claude/fixtures/golden/session")
}

fn session_record() -> String {
    session_dir()
        .join(format!("{SESSION}.jsonl"))
        .display()
        .to_string()
}

fn expected_stream() -> String {
    session_dir()
        .join(format!("{SESSION}.expected.jsonl"))
        .display()
        .to_string()
}

fn golden(crate_name: &str, file: &str) -> String {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_name)
        .join("fixtures/golden")
        .join(file)
        .display()
        .to_string()
}

fn project(out: &std::path::Path, from: Option<&str>, record: &str) -> i32 {
    let out = out.display().to_string();
    let mut argv = vec!["metaharness", "project"];
    if let Some(from) = from {
        argv.extend(["--from", from]);
    }
    argv.extend([record, "--out", out.as_str()]);
    code_of(&argv)
}

fn read_document(path: &std::path::Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("the document is written"))
        .expect("the document is JSON")
}

fn families_without_marker(document: &Value) -> serde_json::Map<String, Value> {
    let mut families = document["metaharness"]["families"]
        .as_object()
        .expect("a family census")
        .clone();
    families.remove("stream_closed");
    families
}

/// The index of the first node that satisfies `matches`.
fn position(nodes: &[Value], matches: impl Fn(&Value) -> bool) -> Option<usize> {
    nodes.iter().position(|node| matches(&node["kind"]))
}

/// The story's acceptance line: the session exits 0 and writes a `trace-ir/1` document whose node
/// census equals the census of the committed expected stream, with the sub-agent's events linked
/// to the parent's spawning tool call.
#[test]
fn a_session_on_disk_projects_with_its_subagent_linked_to_the_spawning_call() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("session.json");
    assert_eq!(
        project(&out, Some("claude-code-session"), &session_record()),
        0
    );
    let document = read_document(&out);
    assert_eq!(document["format"], "trace-ir/1");
    assert_eq!(document["metaharness"]["source"], "claude-code-session");

    let reference = scratch.path().join("expected.json");
    assert_eq!(project(&reference, None, &expected_stream()), 0);
    let reference = read_document(&reference);
    assert_eq!(
        families_without_marker(&document),
        families_without_marker(&reference),
        "the census differs from the committed expected stream"
    );
    assert_eq!(document["metaharness"]["families"]["session_start"], 1);
    assert_eq!(document["events"][0]["kind"]["event"], "session_start");
    assert_eq!(document["events"][0]["kind"]["adapter"], "claude");

    let nodes = document["events"].as_array().expect("nodes");
    let call = position(nodes, |kind| {
        kind["event"] == "tool_call" && kind["call_id"] == SPAWNING_CALL
    })
    .expect("the parent's spawning call is a node");
    assert_eq!(nodes[call]["kind"]["name"], "Agent");
    let result = position(nodes, |kind| {
        kind["event"] == "tool_result" && kind["call_id"] == SPAWNING_CALL
    })
    .expect("the spawning call is answered");
    assert_eq!(
        nodes[result]["kind"]["tool_use_result"]["agentId"], AGENT,
        "the parent's result names the sub-agent it spawned"
    );

    let header = position(nodes, |kind| {
        kind["event"] == "unk"
            && kind["payload"]["code"] == metaharness::protocol::warning_code::VENDOR_SUBAGENT
    })
    .expect("the sub-agent block opens with its header");
    let message = nodes[header]["kind"]["payload"]["message"]
        .as_str()
        .expect("a message");
    assert!(
        message.contains(&format!(
            "sub-agent `{AGENT}` spawned by call `{SPAWNING_CALL}`"
        )),
        "{message}"
    );
    assert!(
        message.contains(&format!("`{SESSION}/subagents/agent-{AGENT}.jsonl`")),
        "{message}"
    );
    assert!(
        call < header && result < header,
        "the block follows the session's own events"
    );

    let subagent_call = position(nodes, |kind| {
        kind["event"] == "tool_call" && kind["call_id"] == SUBAGENT_CALL
    })
    .expect("the sub-agent's own call is a node");
    assert!(
        subagent_call > header,
        "the sub-agent's call is inside its block"
    );
    assert_eq!(
        nodes
            .iter()
            .filter(|node| node["kind"]["event"] == "unk"
                && node["kind"]["payload"]["code"]
                    == metaharness::protocol::warning_code::VENDOR_SUBAGENT)
            .count(),
        1,
        "one block per sub-agent file"
    );
}

/// No clock and one order: the same session projects to the same bytes twice.
#[test]
fn the_same_session_projects_to_the_same_bytes_twice() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let first = scratch.path().join("1.json");
    let second = scratch.path().join("2.json");
    assert_eq!(
        project(&first, Some("claude-code-session"), &session_record()),
        0
    );
    assert_eq!(
        project(&second, Some("claude-code-session"), &session_record()),
        0
    );
    assert_eq!(
        std::fs::read(&first).expect("first"),
        std::fs::read(&second).expect("second")
    );
}

/// a28 point 4 for the new source: a record that is not a session file is refused, exit 2, and a
/// session file declared as any other format is refused too. Nothing is written.
#[test]
fn a_mis_declared_session_is_refused_both_ways() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let session = session_record();
    let stream = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../evals/aep/runs/decomposer-clean.events.jsonl")
        .display()
        .to_string();
    let claude = golden("metaharness-claude", "transcript.jsonl");
    let codex = golden("metaharness-codex", "rollout.jsonl");
    let harness = golden("metaharness-b10x", "loop.jsonl");
    for (from, record) in [
        ("claude-code-session", claude.as_str()),
        ("claude-code-session", codex.as_str()),
        ("claude-code-session", harness.as_str()),
        ("claude-code-session", stream.as_str()),
        ("claude-code-stream-json", session.as_str()),
        ("codex-rollout", session.as_str()),
        ("harness-json", session.as_str()),
    ] {
        let out = scratch.path().join("mis-declared.json");
        assert_eq!(project(&out, Some(from), record), 2, "{from} over {record}");
        assert!(!out.exists(), "{from} over {record}: nothing is written");
    }
}

/// The document names the session file and claims nothing: its digest is over the session file's
/// own bytes, the attestation imposes nothing, nobody closes the stream, and every node cites a
/// line of the session file; a sub-agent's nodes cite the last one, because they are read after
/// the whole session file, as an event a reader owes at end of record does (a28).
#[test]
fn a_projected_session_names_its_file_and_cites_its_lines() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("session.json");
    let path = session_record();
    assert_eq!(project(&out, Some("claude-code-session"), &path), 0);
    let document = read_document(&out);
    let bytes = std::fs::read(&path).expect("the record is committed");
    let digest = metaharness::protocol::Digest::of(&bytes);
    assert_eq!(document["transcript_digest"], digest.as_str());
    assert_eq!(
        document["metaharness"]["vendor_transcript"]["path"],
        format!("{SESSION}.jsonl")
    );
    assert_eq!(document["metaharness"]["hermetic"]["mode"], "off");
    assert_eq!(
        document["metaharness"]["hermetic"]["imposed"],
        serde_json::json!([])
    );
    assert_eq!(document["metaharness"]["stream_complete"], false);
    assert_eq!(document["metaharness"]["run"], SESSION);

    let lines = u64::try_from(String::from_utf8_lossy(&bytes).lines().count()).expect("fits");
    let nodes = document["events"].as_array().expect("nodes");
    let mut previous = 1;
    for node in nodes {
        let line = node["source_line"].as_u64().expect("a line");
        assert!((1..=lines).contains(&line), "line {line} of {lines}");
        assert!(line >= previous, "line {line} after {previous}");
        previous = line;
    }
    assert_eq!(
        nodes.last().expect("a node")["source_line"],
        lines,
        "the sub-agent's nodes cite the session file's last line"
    );
}
