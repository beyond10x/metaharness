//! C2 — replay vectors for the rollout reader, model-free.
//!
//! Each fixture line below is **synthesized to the shapes read off a real codex-cli 0.145.0
//! install** (the research record's method: 2,437 local rollouts; field names verified
//! structurally, content invented here so no session's words travel with the code). A vector is
//! one recorded stimulus and its complete expectation over the emitted event names and the
//! load-bearing fields — a reader that dropped a record or guessed a missing field fails here,
//! with the difference in the detail.

use metaharness_protocol::{
    ConformanceTier, Emission, Event, EventStream, HermeticAttestation, HermeticMode, RunId,
    TranscriptRef, VectorOutcome,
};

use crate::rollout::RolloutReader;

const META: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","session_id":"01a0-fixture","cli_version":"0.145.0","cwd":"/scratch/work","originator":"codex_exec","model_provider":"openai"}}"#;
const META_UNPINNED: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","cli_version":"0.999.0","cwd":"/scratch/work"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-08-22T10:00:01.000Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const CALL: &str = r#"{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{"type":"function_call","call_id":"call-1","name":"exec","arguments":"{\"command\":\"ls\"}"}}"#;
const OUTPUT: &str = r#"{"timestamp":"2026-08-22T10:00:03.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"src\ndocs\n"}}"#;
const TOKENS: &str = r#"{"timestamp":"2026-08-22T10:00:04.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":40,"cache_write_input_tokens":10,"output_tokens":20,"total_tokens":120}},"rate_limits":{"limit_name":"weekly","plan_type":"pro","primary":{"used_percent":12.5}}}}"#;
const COMPLETE: &str = r#"{"timestamp":"2026-08-22T10:00:05.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","duration_ms":4200,"time_to_first_token_ms":800}}"#;
const APRIL_SHAPE: &str = r#"{"timestamp":"2026-04-01T10:00:02.000Z","type":"response_item","payload":{"type":"exec_command_begin","call_id":"call-9","command":["ls"]}}"#;

/// The rollout reader's replay vectors.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    vec![
        vector_full_session(),
        vector_version_gate(),
        vector_drifted_shape_is_opaque_not_fatal(),
        vector_nothing_is_dropped(),
        golden_rollout_vector(GOLDEN_ROLLOUT),
        golden_hook_vector(GOLDEN_HOOK_INPUT),
    ]
}

/// Recorded real wire (adapter contract CT-2): one hermetic run's session rollout and the raw
/// `PreToolUse` stdin its one tool call produced, byte for byte as codex-cli 0.145.0 wrote them
/// — including its `session_meta` claiming `cli_version` **0.144.0**, which is Q18 on disk.
/// The synthesized fixtures above test the reader against this crate's own assumptions; these
/// test it against what the vendor actually wrote — the real call arrives as `custom_tool_call`,
/// a shape no synthesized vector had thought to use. Capture provenance is
/// `fixtures/golden/README.md`; re-capture per pin with
/// `metaharness run codex --hermetic --retain-dir …`.
const GOLDEN_HOOK_INPUT: &str = include_str!("../fixtures/golden/hook-input.json");
const GOLDEN_ROLLOUT: &str = include_str!("../fixtures/golden/rollout.jsonl");
const GOLDEN_ROLLOUT_EXPECTED: &str = include_str!("../fixtures/golden/rollout.expected.jsonl");

/// The run id the golden replay is framed under.
const GOLDEN_RUN: &str = "golden";

/// The recorded rollout in, the committed event stream out, byte-exact.
fn golden_rollout_vector(input: &str) -> VectorOutcome {
    let id = "golden-rollout";
    let observed = golden_replay(input);
    if observed == GOLDEN_ROLLOUT_EXPECTED {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            first_difference(GOLDEN_ROLLOUT_EXPECTED, &observed),
        )
    }
}

/// Frame every event the reader emits for this input, one JSON line each.
fn golden_replay(input: &str) -> String {
    let mut rollout = reader();
    let mut stream = EventStream::new(RunId::new(GOLDEN_RUN));
    let mut out = String::new();
    for line in input.lines() {
        for emission in rollout.push_line(line) {
            push_line(&mut out, &stream.stamp(emission));
        }
    }
    for emission in rollout.finish() {
        push_line(&mut out, &stream.stamp(emission));
    }
    out
}

fn push_line(out: &mut String, line: &metaharness_protocol::EventLine) {
    match serde_json::to_string(line) {
        Ok(json) => {
            out.push_str(&json);
            out.push('\n');
        }
        Err(error) => {
            use std::fmt::Write as _;
            let _ = writeln!(out, "UNSERIALIZABLE: {error}");
        }
    }
}

/// The first line that differs, so a byte-exact failure names a line rather than a file.
fn first_difference(expected: &str, observed: &str) -> String {
    let mut expected_lines = expected.lines();
    let mut observed_lines = observed.lines();
    let mut number = 0;
    loop {
        number += 1;
        match (expected_lines.next(), observed_lines.next()) {
            (None, None) => return "the streams differ only in trailing bytes".to_string(),
            (left, right) if left == right => {}
            (left, right) => {
                return format!(
                    "line {number}: expected {}, observed {}",
                    left.unwrap_or("<end of stream>"),
                    right.unwrap_or("<end of stream>")
                );
            }
        }
    }
}

/// The recorded hook stdin parses to exactly the recorded values, and the rendering table
/// agrees with the wire.
///
/// The second half is what CX-M2 paid to learn: the hook speaks Claude Code's tool vocabulary
/// (`tool_name` **`Bash`**) on a vendor whose rollout calls the same call `exec` and whose
/// model-facing list says `shell`. The recorded input must name the same tool the capability
/// descriptor renders `operation.shell` to, because that table is what the seam matches a live
/// call against — and it is a fact about the vendor this adapter asserts and could be wrong
/// about.
fn golden_hook_vector(input: &str) -> VectorOutcome {
    let id = "golden-hook-input";
    let parsed = match crate::parse_hook_input(input) {
        Ok(parsed) => parsed,
        Err(refused) => {
            return VectorOutcome::failed(
                id,
                ConformanceTier::C2,
                format!("the recorded hook input was refused: {}", refused.reason),
            );
        }
    };
    let expected = crate::HookInput {
        tool_name: Some("Bash".to_string()),
        tool_input: serde_json::json!({"command": "ls"}),
        tool_use_id: Some("exec-9d0c7ef7-57b5-4ddd-b0ce-b64736b8ee9d".to_string()),
        session_id: Some("01a02c8e-5288-70e2-8458-487f54cbfd7a".to_string()),
        turn_id: Some("01a02c8e-5293-7d63-ae7d-fb2dbf46c68b".to_string()),
        cwd: Some("/home/timo/.cache/claude-tmp/.tmpXbOIdF/work".to_string()),
        permission_mode: Some("bypassPermissions".to_string()),
        hook_event_name: Some("PreToolUse".to_string()),
    };
    if parsed != expected {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("parsed {parsed:?}, expected {expected:?}"),
        );
    }
    let capabilities = crate::capabilities();
    let rendered = capabilities.renders(&metaharness_protocol::Operation::Shell);
    if rendered != parsed.tool_name.as_deref() {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!(
                "the rendering table says operation.shell is {rendered:?}, the recorded wire \
                 says {:?}",
                parsed.tool_name
            ),
        );
    }
    VectorOutcome::passed(id, ConformanceTier::C2)
}

fn reader() -> RolloutReader {
    RolloutReader::new(
        TranscriptRef {
            path: Some("/scratch/rollout.jsonl".to_string()),
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
    )
}

fn names(emissions: &[Emission]) -> Vec<&'static str> {
    emissions.iter().map(|line| line.event.name()).collect()
}

fn vector_full_session() -> VectorOutcome {
    let id = "CX2-session";
    let mut reader = reader();
    let mut emitted = Vec::new();
    for line in [META, TASK_STARTED, CALL, OUTPUT, TOKENS, COMPLETE] {
        emitted.extend(reader.push_line(line));
    }
    emitted.extend(reader.finish());
    let seen = names(&emitted);
    let expected = [
        "session.started",
        "turn.started",
        "tool.requested",
        "tool.result",
        "usage",
        "rate_limit",
        "turn.ended",
        "session.ended",
    ];
    if seen != expected {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("expected {expected:?}, saw {seen:?}"),
        );
    }
    // The load-bearing fields: the call correlates, the arguments decode, the terminal record
    // carries the vendor's duration and usage and never invents a cost.
    let call_ok = emitted.iter().any(|line| {
        matches!(
            &line.event,
            Event::ToolRequested { call_id, name, input, decision_required: false, .. }
                if call_id == "call-1" && name == "exec" && input["command"] == "ls"
        )
    });
    let end_ok = emitted.iter().any(|line| {
        matches!(
            &line.event,
            Event::SessionEnded { duration_ms: Some(4200), ttft_ms: Some(800), total_cost_usd: None, usage: Some(usage), .. }
                if usage.input_tokens == Some(100) && usage.cache_read_input_tokens == Some(40)
        )
    });
    if !call_ok || !end_ok {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("call_ok={call_ok} end_ok={end_ok}"),
        );
    }
    VectorOutcome::passed(id, ConformanceTier::C2)
}

fn vector_version_gate() -> VectorOutcome {
    let id = "CX2-version-gate";
    let mut reader = reader();
    let emitted = reader.push_line(META_UNPINNED);
    let warned = emitted.iter().any(
        |line| matches!(&line.event, Event::Warning { code, .. } if code == "version_outside_pin"),
    );
    let started = emitted
        .iter()
        .any(|line| matches!(&line.event, Event::SessionStarted { .. }));
    if warned && started {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("warned={warned} started={started}"),
        )
    }
}

fn vector_drifted_shape_is_opaque_not_fatal() -> VectorOutcome {
    let id = "CX2-drift-opaque";
    let mut reader = reader();
    reader.push_line(META);
    let emitted = reader.push_line(APRIL_SHAPE);
    let opaque = emitted.iter().any(|line| {
        matches!(
            &line.event,
            Event::Opaque { vendor_type: Some(t), vendor_subtype: Some(s), source_line: Some(2), .. }
                if t == "response_item" && s == "exec_command_begin"
        )
    });
    if opaque {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("saw {:?}", names(&emitted)),
        )
    }
}

fn vector_nothing_is_dropped() -> VectorOutcome {
    let id = "CX2-nothing-dropped";
    let mut reader = reader();
    let lines = [
        META,
        "not json at all",
        r#"{"type":"world_state","payload":{}}"#,
    ];
    let mut emitted = Vec::new();
    for line in lines {
        emitted.extend(reader.push_line(line));
    }
    // Every input line produced at least one event, and the unmappable two are opaque.
    let opaque_count = emitted
        .iter()
        .filter(|line| matches!(&line.event, Event::Opaque { .. }))
        .count();
    if emitted.len() >= 3 && opaque_count == 2 {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("{} events, {opaque_count} opaque", emitted.len()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CT-2's acceptance clause, exercised: a mutated byte in the recorded wire fails its
    /// vector. A golden sample a mutation cannot redden is decoration, not a contract.
    #[test]
    fn a_mutated_byte_in_the_golden_rollout_fails_its_vector() {
        let mutated = GOLDEN_ROLLOUT.replacen("custom_tool_call", "custom_tool_cull", 1);
        assert_ne!(mutated, GOLDEN_ROLLOUT, "the mutation found its byte");
        let outcome = golden_rollout_vector(&mutated);
        assert!(!outcome.passed, "the mutated rollout still passed");
        assert!(outcome.detail.contains("line"), "{}", outcome.detail);
    }

    #[test]
    fn a_mutated_byte_in_the_golden_hook_input_fails_its_vector() {
        let mutated =
            GOLDEN_HOOK_INPUT.replacen("\"tool_name\":\"Bash\"", "\"tool_name\":\"Wash\"", 1);
        assert_ne!(mutated, GOLDEN_HOOK_INPUT, "the mutation found its byte");
        let outcome = golden_hook_vector(&mutated);
        assert!(!outcome.passed, "the mutated hook input still passed");
    }

    /// Regenerate the golden expectation from the committed recorded wire. `#[ignore]`d because
    /// it writes into the source tree; it is the second half of a re-capture, not of the gate:
    ///
    /// ```console
    /// metaharness run codex --hermetic --retain-dir <dir> -p "…"    # new pin, new wire
    /// cp <dir>/rollout.jsonl fixtures/golden/rollout.jsonl          # review it first
    /// cargo test -p metaharness-codex --lib regenerate -- --ignored
    /// ```
    #[test]
    #[ignore = "writes fixtures/golden/rollout.expected.jsonl from the committed input; run after a re-capture"]
    fn regenerate_the_golden_expectation() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/golden/rollout.expected.jsonl"
        );
        std::fs::write(path, golden_replay(GOLDEN_ROLLOUT)).expect("the expectation is written");
    }
}
