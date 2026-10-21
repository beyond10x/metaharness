//! Adversary cases for `trace-ir-summary/1` (design amendment a42), wave 7 unit 1.
//!
//! Every document here is written by the production writer, `project_document`, from framed
//! lines read by the production reader, `parse_event_line`, which is what
//! `metaharness project --to trace-ir-summary` does without `--from`. Where a case builds nodes by
//! hand it says so.

use metaharness_protocol::{
    EventLine, TraceIrDocument, TraceIrSummary, parse_event_line, project_document, summarize,
};
use serde_json::{Value, json};

/// A framed `session.ended` line with the given `total_cost_usd` text and `num_turns`, spelled as
/// `evals/aep/runs/decomposer-clean.events.jsonl` line 15 spells it.
fn session_ended_line(seq: u64, cost_text: &str, num_turns: u64, output_tokens: u64) -> String {
    format!(
        concat!(
            r#"{{"format":"metaharness.event/1","seq":{seq},"run":"adv","event":"session.ended","#,
            r#""final_answer":null,"observed_models":null,"is_error":false,"subtype":"success","#,
            r#""stop_reason":"end_turn","terminal_reason":"completed","api_error_status":null,"#,
            r#""num_turns":{num_turns},"duration_ms":1,"duration_api_ms":1,"ttft_ms":null,"#,
            r#""time_to_request_ms":null,"total_cost_usd":{cost},"permission_denials":[],"#,
            r#""subagents_spawned":null,"usage":{{"input_tokens":1,"output_tokens":{output},"#,
            r#""cache_read_input_tokens":null,"cache_creation_input_tokens":null,"#,
            r#""service_tier":null,"thinking_tokens":null,"iterations":null,"speed":null,"#,
            r#""cost_usd":null}},"model_usage":null,"census":{{"allowed":0,"denied":0,"#,
            r#""replaced":0,"abstained":0,"by_seam":{{}},"by_decider":{{}}}}}}"#
        ),
        seq = seq,
        cost = cost_text,
        num_turns = num_turns,
        output = output_tokens,
    )
}

fn session_started_line(seq: u64) -> String {
    format!(
        concat!(
            r#"{{"format":"metaharness.event/1","seq":{seq},"run":"adv","event":"session.started","#,
            r#""adapter":"claude","adapter_class":"harness","harness_version":null,"#,
            r#""session_id":null,"model":"m","permission_mode":null,"credential_source":null,"#,
            r#""output_style":null,"cwd":null,"offered_tools":[],"withheld":null,"#,
            r#""slash_commands":[],"skills":[],"agents":[],"plugins":[],"mcp_servers":null,"#,
            r#""inputs_digest":null,"transcript":{{"path":"t.jsonl","digest":"#,
            r#""6327b6fcc14fe1c3c5bdc1e1b54fa245c82654e4e0990eafbec174e6193c2aa1","bytes":1}},"#,
            r#""hermetic":{{"mode":"off","decisions":"frame","imposed":[],"unavailable":[],"#,
            r#""ambient_inputs":[],"installed_plugins":[]}}}}"#
        ),
        seq = seq,
    )
}

fn text_line(seq: u64) -> String {
    format!(
        r#"{{"format":"metaharness.event/1","seq":{seq},"run":"adv","event":"text","text":"x","request_id":null}}"#
    )
}

/// The verb's path without `--from`: parse each line, project the stream's bytes.
fn document_of_stream(lines: &[String]) -> TraceIrDocument {
    let text = lines.join("\n") + "\n";
    let framed: Vec<EventLine> = lines
        .iter()
        .map(|line| parse_event_line(line).expect("the line parses"))
        .collect();
    project_document(&framed, text.as_bytes())
}

/// The `serde_json` value a summary renders to.
fn rendered(summary: &TraceIrSummary) -> Value {
    serde_json::from_str(&summary.render().expect("the summary renders")).expect("JSON")
}

/// a42 point 3: "The cost is the vendor's float, passed through". The committed Claude golden
/// itself carries a cost of this shape: `modelUsage."claude-haiku-4-5-20251001".costUSD` is
/// `0.0010919999999999999` (`crates/metaharness-claude/fixtures/golden/transcript.jsonl`, last
/// line), a value Claude Code writes as JavaScript's shortest round-trip form. A
/// `total_cost_usd` written that way must come out of the summary as the same number and the same
/// text, or the summary carries a figure the vendor did not write.
#[test]
fn a_seventeen_digit_vendor_cost_is_passed_through_as_written() {
    let written = "0.0010919999999999999";
    let lines = [
        session_started_line(1),
        session_ended_line(2, written, 1, 5),
    ];
    let summary = summarize(&document_of_stream(&lines));
    let exact: f64 = written.parse().expect("std parses it correctly rounded");
    assert_eq!(
        summary.cost_usd.map(f64::to_bits),
        Some(exact.to_bits()),
        "summary cost {:?} is not the vendor's {written} ({exact:?})",
        summary.cost_usd
    );
    let text = summary.render().expect("renders");
    assert!(
        text.contains(&format!("\"cost_usd\": {written},")),
        "the rendered cost is not the text the vendor wrote: {}",
        text.lines()
            .find(|line| line.contains("cost_usd"))
            .unwrap_or_default()
    );
}

/// `TraceIrSummary::render`'s documentation: "# Errors — Whatever `serde_json` says, which for
/// this type is a non-finite cost". `TraceIrSummary` is public with public fields, so an embedder
/// can hold one with a non-finite cost; the render must then refuse, not write a value that reads
/// as "the document did not state a cost" (invariant 3).
#[test]
fn render_refuses_a_non_finite_cost_as_its_documentation_says() {
    let lines = [session_started_line(1), session_ended_line(2, "0.5", 1, 5)];
    let mut summary = summarize(&document_of_stream(&lines));
    summary.cost_usd = Some(f64::NAN);
    let render = summary.render();
    assert!(
        render.is_err(),
        "a NaN cost rendered as {:?}",
        render.as_deref().map(|text| text
            .lines()
            .find(|line| line.contains("cost_usd"))
            .unwrap_or_default()
            .to_owned())
    );
}

/// a42 point 2: the terminal node is the **last** `run_outcome` carrying `num_turns`. No case in
/// the suite holds two session closes, so taking the first one passes every existing test.
#[test]
fn the_last_session_close_is_the_terminal_node_not_the_first() {
    let lines = [
        session_started_line(1),
        session_ended_line(2, "0.5", 1, 10),
        text_line(3),
        session_ended_line(4, "0.75", 3, 30),
    ];
    let summary = rendered(&summarize(&document_of_stream(&lines)));
    assert_eq!(summary["turns"], 3);
    assert_eq!(summary["output_tokens"], 30);
    assert_eq!(summary["cost_usd"], json!(0.75));
}

/// a42 point 2: the opening node is node 0 **when** its family is `session_start`; a
/// `session.started` later in the stream opens nothing. No case in the suite holds a later one,
/// so searching the document for the first `session_start` passes every existing test.
#[test]
fn a_session_start_after_node_zero_opens_nothing() {
    let lines = [
        text_line(1),
        session_started_line(2),
        session_ended_line(3, "0.5", 1, 5),
    ];
    let summary = rendered(&summarize(&document_of_stream(&lines)));
    assert!(summary["adapter"].is_null(), "{}", summary["adapter"]);
    assert!(summary["model"].is_null(), "{}", summary["model"]);
}
