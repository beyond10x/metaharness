//! Adversary cases, wave 3 of 2026-10-20, unit `story:b10x-adapter-on-harness-over-llm`.
//!
//! Amendment a29 (`docs/design/metaharness-protocol-v0.1.md`) says of the move to harness 0.15.0:
//! "Two model-facing tool replies change (`dir_list` of a non-regular entry, a capped `find`)".
//! `CHANGELOG.md` `[Unreleased]` describes the capped `find` change as `depth_bound_reached: false`.
//!
//! These cases drive the served surface (`metaharness mcp-serve`'s `Server` over the catalogue)
//! and compare each reply with the one harness 0.13.3 (`798325f0`, the base pin) gave for the same
//! call. The 0.13.3 replies are written as literals; each was read from
//! `crates/harness-tools/src/local.rs` at `798325f0` and observed by running that crate on the
//! same inputs. A reply that moved must be named by the document that claims to list the moves.

use metaharness_tools::{Catalogue, INVOKE_VERB, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

fn writable(root: &std::path::Path) -> Server {
    Server::new(Verbs::new(Catalogue::of(
        LocalOperations::unconfined(root, Vec::new()).expect("opens"),
    )))
}

/// The reply as the model reads it: `{"isError", "output"}`, the output parsed back from the
/// pretty JSON a success is rendered as, and kept as text for a failure.
fn invoke(server: &mut Server, entry: &str, arguments: &Value) -> Value {
    let answer = server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": INVOKE_VERB, "arguments": {"name": entry, "arguments": arguments}
            }}),
        )
        .expect("answered");
    let failed = answer["result"]["isError"] == true;
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("one text block")
        .to_owned();
    let output = if failed {
        Value::String(text)
    } else {
        serde_json::from_str(&text).expect("a success is JSON")
    };
    json!({"isError": failed, "output": output})
}

fn workspace_file(relative: &str) -> String {
    let path = format!("{}/../../{relative}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The text of amendment a29, from its heading to the next heading or the end of the document.
fn amendment_a29() -> String {
    let design = workspace_file("docs/design/metaharness-protocol-v0.1.md");
    let start = design
        .find("### Amendment a29")
        .expect("the design document carries amendment a29");
    let rest = &design[start + 3..];
    let end = rest.find("\n### ").map_or(rest.len(), |at| at + 1);
    rest[..end].to_owned()
}

/// The `[Unreleased]` section of `CHANGELOG.md`.
fn unreleased_changelog() -> String {
    let changelog = workspace_file("CHANGELOG.md");
    let start = changelog
        .find("## [Unreleased]")
        .expect("the changelog has an [Unreleased] section");
    let rest = &changelog[start + 2..];
    let end = rest.find("\n## [").map_or(rest.len(), |at| at + 1);
    rest[..end].to_owned()
}

/// a29 names two served entries whose replies changed, `dir_list` and `find`. `file_edit` moved
/// too: harness 0.15.0 performs it with `b10x-tool-code-editor`, which counts `old` overlapping
/// (`aa` fits twice in `aaa`) and appends ` (edit 0)` to its refusals. An edit 0.13.3 applied is
/// now refused, and two refusals the model reads are worded differently. `file_read` moved as
/// well: `b10x-tool-fs` refuses `limit: 0`, which 0.13.3 answered as an empty window.
#[test]
fn a29_names_every_served_entry_whose_reply_moved_from_0_13_3() {
    let a29 = amendment_a29();

    let dir = tempfile::tempdir().expect("a workspace");
    std::fs::write(dir.path().join("overlap.txt"), "aaa").expect("seed");
    std::fs::write(dir.path().join("plain.txt"), "hello\n").expect("seed");
    let mut server = writable(dir.path());

    let cases = [
        (
            "file_edit, `old` that overlaps itself",
            "file_edit",
            json!({"path": "overlap.txt", "old": "aa", "new": "b"}),
            json!({"isError": false, "output": {"path": "overlap.txt", "bytes": 2}}),
        ),
        (
            "file_edit, `old` that matches nothing",
            "file_edit",
            json!({"path": "plain.txt", "old": "zzz", "new": "y"}),
            json!({"isError": true, "output": "that text appears nowhere in `plain.txt`"}),
        ),
        (
            "file_edit, empty `old`",
            "file_edit",
            json!({"path": "plain.txt", "old": "", "new": "y"}),
            json!({"isError": true, "output": "`old` must not be empty: an edit has to name where it lands"}),
        ),
        (
            "file_read, `limit: 0`",
            "file_read",
            json!({"path": "plain.txt", "limit": 0}),
            json!({"isError": false, "output": {
                "path": "plain.txt", "bytes": 6, "text": "",
                "lines": {"from": 1, "to": 0, "total": 1},
                "truncated": true, "truncated_lines": []
            }}),
        ),
    ];

    let mut unnamed = Vec::new();
    for (label, entry, arguments, at_0_13_3) in cases {
        let observed = invoke(&mut server, entry, &arguments);
        if observed != at_0_13_3 && !a29.contains(&format!("`{entry}`")) {
            unnamed.push(format!(
                "{label}: 0.13.3 answered {at_0_13_3} and 0.15.0 answers {observed}"
            ));
        }
    }
    assert!(
        unnamed.is_empty(),
        "amendment a29 says two model-facing replies change (`dir_list`, a capped `find`) and \
         does not name these:\n{}",
        unnamed.join("\n")
    );
}

/// The `[Unreleased]` entry says a capped `find` "answers `depth_bound_reached: false`". The same
/// change moved `truncated`: 0.13.3 set it on any file visited after the cap, 0.15.0 only on a
/// further match. Exactly `max_results` matches followed by a file the glob does not name now
/// answers `truncated: false` where it answered `true`.
#[test]
fn the_changelog_names_every_field_a_capped_find_changed() {
    let changelog = unreleased_changelog();

    let dir = tempfile::tempdir().expect("a workspace");
    std::fs::write(dir.path().join("a.txt"), "a").expect("seed");
    std::fs::write(dir.path().join("b.md"), "b").expect("seed");
    let mut server = writable(dir.path());

    let observed = invoke(
        &mut server,
        "find",
        &json!({"glob": "*.txt", "max_results": 1}),
    );
    let at_0_13_3 = json!({"isError": false, "output": {
        "paths": ["a.txt"], "truncated": true, "depth_bound_reached": false
    }});

    let mut unnamed = Vec::new();
    for field in ["paths", "truncated", "depth_bound_reached"] {
        let moved = observed["output"][field] != at_0_13_3["output"][field];
        if moved && !changelog.contains(&format!("`{field}")) {
            unnamed.push(format!(
                "`{field}`: 0.13.3 answered {} and 0.15.0 answers {}",
                at_0_13_3["output"][field], observed["output"][field]
            ));
        }
    }
    assert!(
        unnamed.is_empty(),
        "CHANGELOG [Unreleased] describes the capped `find` change and does not name:\n{}",
        unnamed.join("\n")
    );
}
