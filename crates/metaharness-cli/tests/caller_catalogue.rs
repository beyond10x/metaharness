//! `--catalogue` through the built binary (design amendment a40).
//!
//! The exit code and the refusal's first words are read off the process. No vendor binary can
//! run: `HOME` is a scratch directory and `PATH` is an empty scratch directory, so a refusal that
//! failed to fire would end at "not found", never at a spawn.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use clap::Parser as _;
use metaharness::protocol::{Kind, RunSpec, ToolSurface};
use metaharness_cli::{Cli, Verb};
use serde_json::{Value, json};

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../metaharness-tools/tests/fixtures/caller-catalogue.yaml")
}

fn metaharness(base: &Path, args: &[&str], stdin: &str) -> Outcome {
    let home = base.join("home");
    let path = base.join("path");
    let tmp = base.join("tmp");
    for dir in [&home, &path, &tmp] {
        std::fs::create_dir_all(dir).expect("a scratch directory");
    }
    let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(args)
        .env_clear()
        .env("HOME", &home)
        .env("PATH", &path)
        .env("TMPDIR", &tmp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin.as_bytes())
        .expect("written");
    let output = child.wait_with_output().expect("it ends");
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn run_with(base: &Path, kind: &str, catalogue: &Path, extra: &[&str]) -> Outcome {
    let catalogue = catalogue.to_str().expect("UTF-8");
    let mut args = vec![
        "run",
        kind,
        "--prompt",
        "go",
        "--credentials",
        "none",
        "--catalogue",
        catalogue,
    ];
    args.extend_from_slice(extra);
    metaharness(base, &args, "")
}

fn refused_with(outcome: &Outcome, code: &str) {
    assert_eq!(outcome.code, 2, "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .starts_with(&format!("metaharness: {code}: ")),
        "stderr: {}",
        outcome.stderr
    );
    assert!(
        !outcome.stderr.contains("not found"),
        "refused before looking for a vendor binary: {}",
        outcome.stderr
    );
}

fn parsed(args: &[&str]) -> RunSpec {
    let cli = Cli::try_parse_from(args).expect("parses");
    let Verb::Run(run) = cli.command else {
        panic!("expected the run verb");
    };
    run.spec
}

#[test]
fn catalogue_makes_the_owned_surface_the_default_and_an_explicit_surface_still_wins() {
    let spec = parsed(&[
        "metaharness",
        "run",
        "claude",
        "--catalogue",
        "session.yaml",
    ]);
    assert_eq!(spec.catalogue, Some(PathBuf::from("session.yaml")));
    assert_eq!(spec.tool_surface, ToolSurface::Owned);

    let spec = parsed(&[
        "metaharness",
        "run",
        "claude",
        "--catalogue",
        "session.yaml",
        "--tool-surface",
        "native",
    ]);
    assert_eq!(
        spec.tool_surface,
        ToolSurface::Native,
        "refused later, by name"
    );

    let spec = parsed(&["metaharness", "run", "claude"]);
    assert_eq!(spec.catalogue, None);
    assert_eq!(spec.tool_surface, ToolSurface::Native);
}

#[test]
fn a_serialized_run_spec_without_a_catalogue_has_no_catalogue_key() {
    let spec = RunSpec::new(Kind::Claude);
    let document = serde_json::to_value(&spec).expect("serialises");
    assert!(document.get("catalogue").is_none(), "{document}");
    let back: RunSpec = serde_json::from_value(document).expect("reads back");
    assert_eq!(back, spec);

    let mut declared = RunSpec::new(Kind::Claude);
    declared.catalogue = Some(PathBuf::from("session.yaml"));
    let document = serde_json::to_value(&declared).expect("serialises");
    assert_eq!(document["catalogue"], "session.yaml");
}

#[test]
fn run_b10x_with_a_catalogue_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let outcome = run_with(
        base.path(),
        "b10x",
        &fixture(),
        &[
            "--decisions",
            "observe",
            "--model",
            "synthetic-never-called",
            "--model-endpoint",
            "http://127.0.0.1:1",
        ],
    );
    refused_with(&outcome, "catalogue-not-carried");
}

#[test]
fn run_codex_with_a_catalogue_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    refused_with(
        &run_with(base.path(), "codex", &fixture(), &[]),
        "catalogue-not-carried",
    );
}

#[test]
fn run_claude_with_an_unreadable_catalogue_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    refused_with(
        &run_with(base.path(), "claude", &base.path().join("absent.yaml"), &[]),
        "catalogue-unreadable",
    );
}

#[test]
fn run_claude_with_a_document_in_another_format_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let path = base.path().join("ess.yaml");
    std::fs::write(&path, "format: ess/1\nentities: []\n").expect("written");
    refused_with(
        &run_with(base.path(), "claude", &path, &[]),
        "catalogue-not-tool-contract",
    );
}

#[test]
fn run_claude_with_a_malformed_catalogue_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let path = base.path().join("broken.yaml");
    std::fs::write(&path, "format: tool-contract/1\noperations: [unclosed\n").expect("written");
    refused_with(
        &run_with(base.path(), "claude", &path, &[]),
        "catalogue-malformed",
    );
}

#[test]
fn run_claude_with_a_catalogue_and_the_native_surface_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    refused_with(
        &run_with(
            base.path(),
            "claude",
            &fixture(),
            &["--tool-surface", "native"],
        ),
        "catalogue-needs-owned-surface",
    );
}

#[test]
fn run_claude_with_a_catalogue_and_allow_program_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let outcome = run_with(
        base.path(),
        "claude",
        &fixture(),
        &["--allow-program", "/usr/bin/cargo"],
    );
    refused_with(&outcome, "catalogue-with-allow-program");
    assert!(
        outcome.stderr.contains("--allow-program /usr/bin/cargo"),
        "{}",
        outcome.stderr
    );
}

/// The server refuses the pair itself, with or without `--writable`, before the catalogue file is
/// read: it would publish the catalogue's entries and no `run`, so the programs would start
/// nothing.
#[test]
fn mcp_serve_with_a_catalogue_and_allow_program_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let workspace = base.path().to_str().expect("UTF-8");
    let catalogue = fixture();
    let absent = base.path().join("absent.yaml");
    for (file, writable) in [(&catalogue, true), (&catalogue, false), (&absent, true)] {
        let mut args = vec![
            "mcp-serve",
            "--workspace",
            workspace,
            "--catalogue",
            file.to_str().expect("UTF-8"),
            "--allow-program",
            "/usr/bin/cargo",
        ];
        if writable {
            args.push("--writable");
        }
        let outcome = metaharness(base.path(), &args, "");
        refused_with(&outcome, "catalogue-with-allow-program");
        assert!(outcome.stdout.is_empty(), "{}", outcome.stdout);
    }
}

#[test]
fn mcp_serve_with_a_catalogue_lists_its_entries_and_nothing_else() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let workspace = base.path().join("workspace");
    std::fs::create_dir(&workspace).expect("a workspace");
    std::fs::write(workspace.join("README.md"), "hello\n").expect("a file");
    let mut requests = String::new();
    for request in [
        json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"}),
        json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call",
            "params": {"name": "fs_read", "arguments": {"path": "README.md"}}}),
        json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call",
            "params": {"name": "tool_invoke", "arguments": {"name": "file_read", "arguments": {"path": "README.md"}}}}),
    ] {
        requests.push_str(&request.to_string());
        requests.push('\n');
    }
    let outcome = metaharness(
        base.path(),
        &[
            "mcp-serve",
            "--workspace",
            workspace.to_str().expect("UTF-8"),
            "--catalogue",
            fixture().to_str().expect("UTF-8"),
        ],
        &requests,
    );
    assert_eq!(outcome.code, 0, "stderr: {}", outcome.stderr);
    let answers: Vec<Value> = outcome
        .stdout
        .lines()
        .map(|line| serde_json::from_str(line).expect("JSON"))
        .collect();
    let names: Vec<&str> = answers[0]["result"]["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names, ["fs_read", "fs_stat"]);
    assert_eq!(answers[1]["result"]["isError"], false, "{}", answers[1]);
    assert!(
        answers[1]["result"]["content"][0]["text"]
            .as_str()
            .is_some_and(|text| text.contains("hello")),
        "{}",
        answers[1]
    );
    assert_eq!(answers[2]["result"]["isError"], true, "{}", answers[2]);
}

#[test]
fn mcp_serve_with_a_malformed_catalogue_exits_2_naming_the_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let path = base.path().join("broken.yaml");
    std::fs::write(&path, "format: tool-contract/1\noperations: [unclosed\n").expect("written");
    let outcome = metaharness(
        base.path(),
        &[
            "mcp-serve",
            "--workspace",
            base.path().to_str().expect("UTF-8"),
            "--catalogue",
            path.to_str().expect("UTF-8"),
        ],
        "",
    );
    refused_with(&outcome, "catalogue-malformed");
}
