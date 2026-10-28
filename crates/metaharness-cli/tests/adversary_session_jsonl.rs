//! Adversary cases for `metaharness project --from claude-code-session`
//! (story:session-jsonl-adapter). Every subject is either the committed synthetic fixture under
//! `metaharness-claude/fixtures/golden/session/`, copied into a scratch directory, or a record
//! written here by hand. No recorded content.

use std::path::{Path, PathBuf};
use std::str::FromStr as _;

use clap::Parser as _;
use metaharness_cli::{Cli, execute};
use serde_json::Value;

const SESSION: &str = "5e551011-0000-4000-8000-000000000001";
const AGENT: &str = "a0f1c2d3e4b5a697";

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../metaharness-claude/fixtures/golden/session")
}

fn fixture_session() -> Vec<u8> {
    std::fs::read(fixture_dir().join(format!("{SESSION}.jsonl"))).expect("the fixture")
}

fn fixture_subagent() -> Vec<u8> {
    std::fs::read(
        fixture_dir()
            .join(SESSION)
            .join("subagents")
            .join(format!("agent-{AGENT}.jsonl")),
    )
    .expect("the sub-agent fixture")
}

/// Writes `<dir>/<SESSION>.jsonl` and `<dir>/<SESSION>/subagents/<name>` for each sub-agent, in
/// the order given, and returns the session path.
fn stage(dir: &Path, session: &[u8], subagents: &[(&str, &[u8])]) -> PathBuf {
    let path = dir.join(format!("{SESSION}.jsonl"));
    std::fs::write(&path, session).expect("write the session");
    if !subagents.is_empty() {
        let directory = dir.join(SESSION).join("subagents");
        std::fs::create_dir_all(&directory).expect("mkdir");
        for (name, bytes) in subagents {
            std::fs::write(directory.join(name), bytes).expect("write a sub-agent");
        }
    }
    path
}

fn project(out: &Path, to: &str, record: &Path) -> i32 {
    let out = out.display().to_string();
    let record = record.display().to_string();
    code_of(&[
        "metaharness",
        "project",
        "--from",
        "claude-code-session",
        "--to",
        to,
        record.as_str(),
        "--out",
        out.as_str(),
    ])
}

fn read_json(path: &Path) -> Value {
    serde_json::from_str(&std::fs::read_to_string(path).expect("written")).expect("JSON")
}

fn nodes(document: &Value) -> Vec<Value> {
    document["events"].as_array().expect("nodes").clone()
}

fn header_messages(nodes: &[Value]) -> Vec<(usize, String)> {
    nodes
        .iter()
        .enumerate()
        .filter(|(_, node)| {
            node["kind"]["event"] == "unk"
                && node["kind"]["payload"]["code"]
                    == metaharness::protocol::warning_code::VENDOR_SUBAGENT
        })
        .map(|(index, node)| {
            (
                index,
                node["kind"]["payload"]["message"]
                    .as_str()
                    .unwrap_or_default()
                    .to_owned(),
            )
        })
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Totality
// ---------------------------------------------------------------------------------------------

/// A session file cut mid-write ends in a partial line. When the cut falls inside a multi-byte
/// character the file is not UTF-8, and a reader that is total over records still owes the whole
/// session up to the cut, with the partial line as `opaque`.
#[test]
fn adv_a_session_cut_inside_a_multibyte_character_still_projects() {
    let scratch = tempfile::tempdir().expect("scratch");
    let mut session = fixture_session();
    session.extend_from_slice(
        b"{\"type\":\"user\",\"sessionId\":\"5e551011-0000-4000-8000-000000000001\",\"message\":{\"role\":\"user\",\"content\":\"caf\xC3",
    );
    let path = stage(
        scratch.path(),
        &session,
        &[(&format!("agent-{AGENT}.jsonl"), &fixture_subagent())],
    );
    let out = scratch.path().join("doc.json");
    assert_eq!(
        project(&out, "trace-ir", &path),
        0,
        "a session cut inside a multi-byte character is refused whole instead of read up to the cut"
    );
}

/// The same cut on an ASCII byte: the partial last line is the last `opaque` of the session's
/// own events, cited at its line.
#[test]
fn adv_a_truncated_last_record_is_opaque_at_its_line() {
    let scratch = tempfile::tempdir().expect("scratch");
    let mut session = fixture_session();
    session.extend_from_slice(b"{\"type\":\"user\",\"sessionId\":\"5e5");
    let path = stage(scratch.path(), &session, &[]);
    let out = scratch.path().join("doc.json");
    assert_eq!(project(&out, "trace-ir", &path), 0);
    let nodes = nodes(&read_json(&out));
    let last = nodes.last().expect("a node");
    assert_eq!(last["kind"]["event"], "opaque", "{last}");
    assert_eq!(last["source_line"], 14);
}

/// An empty file is no session: refused, exit 2, nothing written.
#[test]
fn adv_an_empty_session_file_is_refused_and_writes_nothing() {
    let scratch = tempfile::tempdir().expect("scratch");
    let path = stage(scratch.path(), b"", &[]);
    let out = scratch.path().join("doc.json");
    assert_eq!(project(&out, "trace-ir", &path), 2);
    assert!(!out.exists());
}

/// JSON that is not an object, and a record with a message but no `type`, are each one `opaque`
/// node at their own line.
#[test]
fn adv_json_that_is_not_an_object_and_a_typeless_record_are_opaque() {
    let scratch = tempfile::tempdir().expect("scratch");
    let mut session = fixture_session();
    session.extend_from_slice(
        b"[1,2]\n\"text\"\nnull\n{\"sessionId\":\"x\",\"message\":{\"role\":\"user\",\"content\":\"hi\"}}\n",
    );
    let path = stage(scratch.path(), &session, &[]);
    let out = scratch.path().join("doc.json");
    assert_eq!(project(&out, "trace-ir", &path), 0);
    let nodes = nodes(&read_json(&out));
    for line in 14..=17 {
        let at: Vec<&Value> = nodes
            .iter()
            .filter(|node| node["source_line"] == line)
            .collect();
        assert_eq!(at.len(), 1, "line {line}: {at:?}");
        assert_eq!(at[0]["kind"]["event"], "opaque", "line {line}");
    }
}

// ---------------------------------------------------------------------------------------------
// The acceptance census, derived by hand from the fixture, not from the reader's own output
// ---------------------------------------------------------------------------------------------

/// The committed expected stream is written by the reader under test (`regenerate_…`), so the
/// acceptance's census comparison cannot fail for a mapping defect. This is the census read off
/// the fixture by hand: 13 session lines and 4 sub-agent lines.
#[test]
fn adv_the_census_equals_the_one_read_off_the_fixture_by_hand() {
    let scratch = tempfile::tempdir().expect("scratch");
    let out = scratch.path().join("doc.json");
    let path = fixture_dir().join(format!("{SESSION}.jsonl"));
    assert_eq!(project(&out, "trace-ir", &path), 0);
    let document = read_json(&out);
    let mut families = document["metaharness"]["families"]
        .as_object()
        .expect("families")
        .clone();
    families.remove("stream_closed");
    let expected = serde_json::json!({
        "session_start": 1,
        "opaque": 5,
        "synthetic_injection": 3,
        "assistant_thinking": 1,
        "run_outcome": 5,
        "tool_call": 3,
        "tool_result": 3,
        "assistant_text": 2,
        "unk": 1
    });
    let families: serde_json::Map<String, Value> = families
        .into_iter()
        .filter(|(_, count)| count != &Value::from(0))
        .collect();
    assert_eq!(Value::Object(families), expected);
    assert_eq!(document["metaharness"]["events_total"], 24);
}

/// `SourceKind` in `spec/domains/trace.yaml` and the names `--from` admits are one list, less
/// the event stream, which is not a `--from`.
#[test]
fn adv_the_declared_source_kinds_are_the_ones_the_verb_admits() {
    let spec = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spec/domains/trace.yaml"),
    )
    .expect("the domain");
    let start = spec
        .find("name: metaharness.trace.SourceKind")
        .expect("SourceKind");
    let variants_line = spec[start..]
        .lines()
        .find(|line| line.trim_start().starts_with("variants:"))
        .expect("variants");
    let inner = variants_line
        .split_once('[')
        .and_then(|(_, rest)| rest.split_once(']'))
        .expect("a list")
        .0;
    let mut declared: Vec<&str> = inner
        .split(',')
        .map(str::trim)
        .filter(|name| *name != "metaharness-event-stream")
        .collect();
    declared.sort_unstable();
    let mut admitted: Vec<&str> = metaharness::VendorSource::names().to_vec();
    admitted.sort_unstable();
    assert_eq!(declared, admitted);
    for name in admitted {
        assert!(metaharness::VendorSource::from_str(name).is_ok(), "{name}");
    }
}

// ---------------------------------------------------------------------------------------------
// Determinism and privacy
// ---------------------------------------------------------------------------------------------

/// The order sub-agent files were created on disk does not move a byte of the document.
#[test]
fn adv_subagent_creation_order_does_not_change_the_document() {
    let other: &[u8] = b"{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":\"go\"}}\n";
    let name = format!("agent-{AGENT}.jsonl");
    let fixture = fixture_subagent();
    let one = tempfile::tempdir().expect("scratch");
    let two = tempfile::tempdir().expect("scratch");
    let first = stage(
        one.path(),
        &fixture_session(),
        &[
            (name.as_str(), &fixture),
            ("agent-0000.jsonl", other),
            ("agent-ffff.jsonl", other),
        ],
    );
    let second = stage(
        two.path(),
        &fixture_session(),
        &[
            ("agent-ffff.jsonl", other),
            ("agent-0000.jsonl", other),
            (name.as_str(), &fixture),
        ],
    );
    let out_one = one.path().join("doc.json");
    let out_two = two.path().join("doc.json");
    assert_eq!(project(&out_one, "trace-ir", &first), 0);
    assert_eq!(project(&out_two, "trace-ir", &second), 0);
    assert_eq!(
        std::fs::read(&out_one).expect("one"),
        std::fs::read(&out_two).expect("two")
    );
}

/// Nothing of the reading machine's file system enters the document or its summary: the
/// fixture copied under a directory with a marker in its name projects with no trace of it.
#[test]
fn adv_the_document_carries_no_path_of_the_reading_machine() {
    let scratch = tempfile::tempdir().expect("scratch");
    let marked = scratch.path().join("adv-reading-machine-marker");
    std::fs::create_dir_all(&marked).expect("mkdir");
    let path = stage(
        &marked,
        &fixture_session(),
        &[(&format!("agent-{AGENT}.jsonl"), &fixture_subagent())],
    );
    for to in ["trace-ir", "trace-ir-summary"] {
        let out = scratch.path().join(format!("{to}.json"));
        assert_eq!(project(&out, to, &path), 0, "{to}");
        let text = std::fs::read_to_string(&out).expect("written");
        assert!(!text.contains("adv-reading-machine-marker"), "{to}");
        assert!(
            !text.contains(&scratch.path().display().to_string()),
            "{to}"
        );
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            assert!(!text.contains(&home), "{to} carries $HOME");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Sub-agent ordering and the summary
// ---------------------------------------------------------------------------------------------

/// A nested sub-agent's block is linked to the call in its parent sub-agent that spawned it; that
/// call precedes the block in the document, as the top-level block follows the session's own
/// events. Real sub-agent ids are random hex, so file-name order puts a nested block before its
/// spawning call whenever the child's id sorts first.
#[test]
fn adv_a_nested_subagent_block_follows_the_call_that_spawned_it() {
    let envelope = r#""sessionId":"s-1","version":"2.1.309","cwd":"/fixture/w""#;
    let session = format!(
        "{{\"type\":\"user\",{envelope},\"message\":{{\"role\":\"user\",\"content\":\"go\"}}}}\n\
         {{\"type\":\"assistant\",{envelope},\"requestId\":\"r-1\",\"message\":{{\"model\":\"m\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"call-a\",\"name\":\"Agent\",\"input\":{{}}}}]}}}}\n\
         {{\"type\":\"user\",{envelope},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"call-a\",\"content\":\"done\"}}]}},\"toolUseResult\":{{\"agentId\":\"1a\"}}}}\n"
    );
    let parent_agent = format!(
        "{{\"type\":\"assistant\",{envelope},\"requestId\":\"r-2\",\"message\":{{\"model\":\"m\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"call-b\",\"name\":\"Agent\",\"input\":{{}}}}]}}}}\n\
         {{\"type\":\"user\",{envelope},\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"tool_result\",\"tool_use_id\":\"call-b\",\"content\":\"done\"}}]}},\"toolUseResult\":{{\"agentId\":\"0b\"}}}}\n"
    );
    let child = format!(
        "{{\"type\":\"user\",{envelope},\"message\":{{\"role\":\"user\",\"content\":\"nested\"}}}}\n"
    );
    let scratch = tempfile::tempdir().expect("scratch");
    let path = stage(
        scratch.path(),
        session.as_bytes(),
        &[
            ("agent-1a.jsonl", parent_agent.as_bytes()),
            ("agent-0b.jsonl", child.as_bytes()),
        ],
    );
    let out = scratch.path().join("doc.json");
    assert_eq!(project(&out, "trace-ir", &path), 0);
    let nodes = nodes(&read_json(&out));
    let headers = header_messages(&nodes);
    let nested = headers
        .iter()
        .find(|(_, message)| message.starts_with("sub-agent `0b` spawned by call `call-b`"))
        .unwrap_or_else(|| panic!("the nested block is linked: {headers:?}"))
        .0;
    let call = nodes
        .iter()
        .position(|node| {
            node["kind"]["event"] == "tool_call" && node["kind"]["call_id"] == "call-b"
        })
        .expect("the nested spawning call is a node");
    assert!(
        call < nested,
        "the nested block (node {nested}) precedes the call that spawned it (node {call})"
    );
}

/// The fixture's session runs from 09:00:00.000 to its last record at 09:00:08.000; the
/// sub-agent ran inside it and finished at 09:00:05.900. A summary's last timestamp is the last
/// one in document order, and the reader appends the sub-agent after the session's own events, so
/// the summary says the session ended at the sub-agent's last record: 2.1 s before it did.
#[test]
fn adv_the_summary_ends_at_the_sessions_last_record() {
    let scratch = tempfile::tempdir().expect("scratch");
    let out = scratch.path().join("summary.json");
    let path = fixture_dir().join(format!("{SESSION}.jsonl"));
    assert_eq!(project(&out, "trace-ir-summary", &path), 0);
    let summary = read_json(&out);
    assert_eq!(summary["first_timestamp_ms"], 1_793_178_000_000_i64);
    assert_eq!(
        summary["last_timestamp_ms"], 1_793_178_008_000_i64,
        "the summary's session ends before the session's own last record"
    );
}

/// A sub-agent file that repeats records of another (on local sessions read 2026-10-28: 426
/// tool-call ids in 9 of 528 sessions sat in two files, 413 of them in two sub-agent files, every
/// copy with its `requestId`) is read twice for its content and once for its usage: the reader
/// bills the request once but lists the call twice, and the summary counts it twice.
#[test]
fn adv_a_record_copied_into_a_second_subagent_file_is_not_counted_twice() {
    let envelope = r#""sessionId":"s-1","version":"2.1.309","cwd":"/fixture/w""#;
    let session = format!(
        "{{\"type\":\"user\",{envelope},\"message\":{{\"role\":\"user\",\"content\":\"go\"}}}}\n"
    );
    let call = format!(
        "{{\"type\":\"assistant\",{envelope},\"uuid\":\"u-1\",\"requestId\":\"r-1\",\"message\":{{\"model\":\"m\",\"content\":[{{\"type\":\"tool_use\",\"id\":\"call-x\",\"name\":\"Read\",\"input\":{{}}}}],\"usage\":{{\"input_tokens\":1,\"output_tokens\":1}}}}}}\n"
    );
    let scratch = tempfile::tempdir().expect("scratch");
    let path = stage(
        scratch.path(),
        session.as_bytes(),
        &[
            ("agent-a.jsonl", call.as_bytes()),
            ("agent-b.jsonl", call.as_bytes()),
        ],
    );
    let out = scratch.path().join("doc.json");
    assert_eq!(project(&out, "trace-ir", &path), 0);
    let nodes = nodes(&read_json(&out));
    let usages = nodes
        .iter()
        .filter(|node| node["kind"]["event"] == "run_outcome")
        .count();
    let calls = nodes
        .iter()
        .filter(|node| node["kind"]["event"] == "tool_call" && node["kind"]["call_id"] == "call-x")
        .count();
    assert_eq!(usages, 1, "the request is billed once");
    assert_eq!(
        calls, usages,
        "one request, billed once, lists its one call {calls} times"
    );
}

// ---------------------------------------------------------------------------------------------
// Real sessions, counts only. Never run by the suite.
// ---------------------------------------------------------------------------------------------

/// `ADV_SESSIONS_ROOT=<projects dir> ADV_SESSIONS_PREFIX=<dir name prefix>` and `--ignored`.
/// Prints counts and nothing read from a session.
#[test]
#[ignore = "reads local sessions; counts only"]
#[allow(clippy::too_many_lines)]
fn adv_real_sessions_counts_only() {
    let root = PathBuf::from(std::env::var("ADV_SESSIONS_ROOT").expect("ADV_SESSIONS_ROOT"));
    let prefix = std::env::var("ADV_SESSIONS_PREFIX").unwrap_or_default();
    let mut sessions = Vec::new();
    for dir in std::fs::read_dir(&root).expect("root") {
        let dir = dir.expect("entry").path();
        let name = dir
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        if !dir.is_dir() || !name.starts_with(&prefix) {
            continue;
        }
        for file in std::fs::read_dir(&dir).expect("dir") {
            let file = file.expect("entry").path();
            if file.extension().is_some_and(|e| e == "jsonl") && file.is_file() {
                sessions.push(file);
            }
        }
    }
    sessions.sort();
    let mut counts: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
    let mut bump = |key: &str, by: u64| *counts.entry(key.to_owned()).or_default() += by;
    // `ADV_SESSIONS_MIN_AGE_S`: skip a session whose file or sub-agent directory was written
    // more recently than this, so a live session being appended to is not read twice.
    let min_age = std::env::var("ADV_SESSIONS_MIN_AGE_S")
        .ok()
        .and_then(|age| age.parse::<u64>().ok());
    let young = |path: &Path| {
        min_age.is_some_and(|age| {
            std::fs::metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .and_then(|modified| modified.elapsed().ok())
                .is_some_and(|elapsed| elapsed.as_secs() < age)
        })
    };
    for session in &sessions {
        let subagents = session.with_extension("").join("subagents");
        if young(session) || young(&subagents) {
            bump("skipped_recently_written", 1);
            continue;
        }
        bump("sessions", 1);
        let projected = std::panic::catch_unwind(|| {
            metaharness::project_vendor_record(
                session,
                metaharness::VendorSource::ClaudeCodeSession,
            )
        });
        let document = match projected {
            Err(_) => {
                bump("panicked", 1);
                continue;
            }
            Ok(Err(refusal)) => {
                let debug = format!("{refusal:?}");
                let variant: String = debug.chars().take_while(|c| c.is_alphanumeric()).collect();
                let kind = if debug.contains("not UTF-8") {
                    "refused_not_utf8".to_owned()
                } else if debug.contains("produced no event") {
                    "refused_no_event".to_owned()
                } else if debug.contains("first event") {
                    "refused_no_opening".to_owned()
                } else {
                    format!("refused_{variant}")
                };
                bump(&kind, 1);
                continue;
            }
            Ok(Ok(document)) => document,
        };
        bump("projected", 1);
        let again = metaharness::project_vendor_record(
            session,
            metaharness::VendorSource::ClaudeCodeSession,
        )
        .expect("projects twice");
        if document.render().expect("renders") != again.render().expect("renders") {
            bump("render_differs_between_two_projections", 1);
        }
        let kinds: Vec<&Value> = document.events.iter().map(|event| &event.kind).collect();
        bump("nodes", kinds.len() as u64);
        bump(
            "opaque_nodes",
            kinds.iter().filter(|k| k["event"] == "opaque").count() as u64,
        );
        // Duplicate tool calls: one call id requested more than once in one document.
        let mut calls: std::collections::BTreeMap<String, u64> = std::collections::BTreeMap::new();
        for kind in &kinds {
            if kind["event"] == "tool_call" {
                *calls
                    .entry(kind["call_id"].as_str().unwrap_or_default().to_owned())
                    .or_default() += 1;
            }
        }
        let duplicated = calls.values().filter(|n| **n > 1).count() as u64;
        if duplicated > 0 {
            bump("documents_with_a_tool_call_id_twice", 1);
            bump("tool_call_ids_twice", duplicated);
        }
        // Summary end versus the latest stamped node.
        let stamps: Vec<i64> = document
            .events
            .iter()
            .filter_map(|e| e.timestamp_ms)
            .collect();
        if let (Some(last), Some(max)) = (stamps.last(), stamps.iter().max())
            && last < max
        {
            bump("documents_whose_last_stamp_is_not_the_latest", 1);
            bump(
                "ms_understated_total",
                u64::try_from(max - last).unwrap_or(0),
            );
        }
        // ADV-SJ-1 as the summary states it: its end against the latest stamped node.
        let summary = metaharness::protocol::summarize(&document);
        if let (Some(end), Some(max)) = (summary.last_timestamp_ms, stamps.iter().max())
            && end < *max
        {
            bump("summaries_ending_before_the_latest_stamp", 1);
        }
        // Sub-agent blocks.
        let headers: Vec<(usize, String)> = kinds
            .iter()
            .enumerate()
            .filter(|(_, k)| {
                k["event"] == "unk"
                    && k["payload"]["code"] == metaharness::protocol::warning_code::VENDOR_SUBAGENT
            })
            .map(|(i, k)| {
                (
                    i,
                    k["payload"]["message"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                )
            })
            .collect();
        bump("subagent_blocks", headers.len() as u64);
        for (index, message) in &headers {
            if message.contains("spawned by no recorded call") {
                bump("subagent_blocks_unlinked", 1);
                continue;
            }
            let call = message
                .split("spawned by call `")
                .nth(1)
                .and_then(|rest| rest.split('`').next())
                .unwrap_or_default();
            match kinds
                .iter()
                .position(|k| k["event"] == "tool_call" && k["call_id"] == call)
            {
                Some(at) if at > *index => bump("subagent_blocks_before_their_spawning_call", 1),
                Some(_) => {}
                None => bump("subagent_blocks_linked_to_a_call_not_in_the_document", 1),
            }
        }
        // Spawns whose sub-agent has no block, and of those the ones no missing-file warning names
        // (ADV-SJ-5).
        let missing: Vec<&str> = kinds
            .iter()
            .filter(|k| {
                k["event"] == "unk"
                    && k["payload"]["code"]
                        == metaharness::protocol::warning_code::VENDOR_SUBAGENT_MISSING
            })
            .map(|k| k["payload"]["message"].as_str().unwrap_or_default())
            .collect();
        bump("subagent_missing_warnings", missing.len() as u64);
        for kind in &kinds {
            if kind["event"] == "tool_result"
                && let Some(agent) = kind["tool_use_result"]["agentId"].as_str()
                && !headers
                    .iter()
                    .any(|(_, message)| message.starts_with(&format!("sub-agent `{agent}` ")))
            {
                bump("spawn_results_without_a_block", 1);
                if !missing
                    .iter()
                    .any(|message| message.starts_with(&format!("sub-agent `{agent}` ")))
                {
                    bump("spawn_results_without_a_block_or_a_warning", 1);
                }
            }
        }
        // ADV-SJ-2: records dropped as copies, as the block warnings state them.
        for (_, message) in &headers {
            if let Some(count) = message
                .split("; ")
                .nth(1)
                .and_then(|tail| tail.split(' ').next())
                .and_then(|count| count.parse::<u64>().ok())
            {
                bump("copied_records_dropped", count);
            }
        }
    }
    for (key, value) in &counts {
        println!("{key}={value}");
    }
}
