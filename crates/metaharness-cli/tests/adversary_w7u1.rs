//! Adversary cases for `metaharness project --to trace-ir-summary` (design amendment a42), wave 7
//! unit 1. Every subject is a committed golden record or a copy of one in a temporary directory;
//! nothing reaches a model, a network or a credential.

use std::path::PathBuf;

use clap::Parser as _;
use metaharness_cli::{Cli, execute};
use serde_json::Value;

fn code_of(argv: &[&str]) -> i32 {
    execute(Cli::try_parse_from(argv).expect("the command line parses"))
}

fn golden(crate_name: &str, file: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join(crate_name)
        .join("fixtures/golden")
        .join(file)
}

/// `metaharness project --from <from> <record> --to <to> --out <tmp>`, read back as JSON.
fn project(from: &str, record: &std::path::Path, to: &str) -> Value {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("out.json");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            "--from",
            from,
            &record.display().to_string(),
            "--to",
            to,
            "--out",
            &out.display().to_string(),
        ]),
        0
    );
    serde_json::from_str(&std::fs::read_to_string(&out).expect("written")).expect("JSON")
}

/// Story `retire-trace-formats-beside-trace-ir`, Decisions: takt's harness-step rows (model,
/// tokens, ...) "become a projection of the step's `trace-ir/1` document through
/// `trace-ir-summary/1`". The b10x loop record (`crates/metaharness-b10x/fixtures/golden/loop.jsonl`
/// lines 4 and 10) states its tokens: `usage` 42 in / 8 out / 7 cached, and 42 / 4 / 7. The
/// document carries them as two `run_outcome` usage nodes. The summary reads `null` for every
/// token count, which under invariant 3 says *the record did not state it*. On the native arm
/// (`--from harness-json`), every token figure a consumer reads from the summary is therefore
/// absent although the record billed 84 input and 12 output tokens.
#[test]
fn a_harness_json_summary_states_the_tokens_its_record_states() {
    let record = golden("metaharness-b10x", "loop.jsonl");
    let document = project("harness-json", &record, "trace-ir");
    let stated: Vec<u64> = document["events"]
        .as_array()
        .expect("nodes")
        .iter()
        .filter(|node| node["kind"]["event"] == "run_outcome")
        .filter_map(|node| node["kind"]["usage"]["input_tokens"].as_u64())
        .collect();
    assert_eq!(stated, [42, 42], "the document states the record's tokens");

    let summary = project("harness-json", &record, "trace-ir-summary");
    for key in ["input_tokens", "output_tokens", "cache_read_input_tokens"] {
        assert!(
            !summary[key].is_null(),
            "{key} is null although the document states it per request: {summary}"
        );
    }
}

/// Same story clause, the `model` column. takt fills it today from the Claude result's
/// `modelUsage` keys joined with `+` (takt `crates/takt/src/trace.rs`, the `modelUsage` arm,
/// takt 79da7e4). The committed Claude golden's terminal record bills two models,
/// `claude-haiku-4-5-20251001` and `claude-opus-5[1m]`, and its `total_cost_usd` 0.238876 is
/// their sum (0.001092 + 0.237784). The Codex golden states `gpt-5.6-sol` on two lines and the
/// document carries it in `observed_models`. The summary names only the opening record's model:
/// Claude `claude-opus-5[1m]`, Codex `null`. A takt row moved onto the summary loses the model it
/// records today.
#[test]
fn the_summary_names_every_model_the_record_states() {
    let mut missing = Vec::new();
    let claude = project(
        "claude-code-stream-json",
        &golden("metaharness-claude", "transcript.jsonl"),
        "trace-ir-summary",
    );
    if !claude.to_string().contains("claude-haiku-4-5-20251001") {
        missing.push(format!(
            "claude: haiku billed 0.001092 of the 0.238876 cost, summary model {}",
            claude["model"]
        ));
    }
    let codex = project(
        "codex-rollout",
        &golden("metaharness-codex", "rollout.jsonl"),
        "trace-ir-summary",
    );
    if !codex.to_string().contains("gpt-5.6-sol") {
        missing.push(format!(
            "codex: the rollout states gpt-5.6-sol, summary model {}",
            codex["model"]
        ));
    }
    assert!(missing.is_empty(), "{missing:#?}");
}

/// a42 point 3, "the vendor's float, passed through", on the `--from claude-code-stream-json`
/// path. The golden's own `modelUsage` carries `costUSD` `0.0010919999999999999`, Claude Code's
/// shortest round-trip spelling of a double. A copy of the golden whose `total_cost_usd` is
/// spelled that way must summarize to that number.
#[test]
fn a_claude_cost_spelled_with_seventeen_digits_survives_the_summary() {
    let written = "0.0010919999999999999";
    let original =
        std::fs::read_to_string(golden("metaharness-claude", "transcript.jsonl")).expect("golden");
    assert!(original.contains("\"total_cost_usd\":0.238876"));
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let record = scratch.path().join("transcript.jsonl");
    std::fs::write(
        &record,
        original.replace(
            "\"total_cost_usd\":0.238876",
            &format!("\"total_cost_usd\":{written}"),
        ),
    )
    .expect("copy");
    let summary = project("claude-code-stream-json", &record, "trace-ir-summary");
    let exact: f64 = written.parse().expect("std parses it correctly rounded");
    assert_eq!(
        summary["cost_usd"].as_f64().map(f64::to_bits),
        Some(exact.to_bits()),
        "summary cost {} is not the vendor's {written}",
        summary["cost_usd"]
    );
}

/// Two positionals under `--to trace-ir-summary` without `--html` are refused (exit 2), with
/// `Refusal::ViewerColumnCount`, whose words tell a caller who gave two streams to "Give one or
/// two event streams" and talk about a viewer the caller did not ask for.
#[test]
fn two_streams_for_one_summary_are_refused_in_words_that_fit() {
    let record = golden("metaharness-claude", "transcript.jsonl")
        .display()
        .to_string();
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let out = scratch.path().join("out.json");
    assert_eq!(
        code_of(&[
            "metaharness",
            "project",
            "--from",
            "claude-code-stream-json",
            &record,
            &record,
            "--to",
            "trace-ir-summary",
            "--out",
            &out.display().to_string(),
        ]),
        2
    );
    assert!(!out.exists(), "a refused summary writes nothing");
    // Revision 1 (coordinator's decision on F7): the summary has its own refusal,
    // `Refusal::SummaryStreamCount`; the viewer's words stay what they are for the viewer, so the
    // expectation reads the refusal the summary now raises.
    let words = metaharness::Refusal::SummaryStreamCount { given: 2 }.to_string();
    assert!(
        !words.contains("Give one or two"),
        "given two, told to give one or two: {words}"
    );
}
