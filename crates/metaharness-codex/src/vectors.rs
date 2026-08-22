//! C2 — replay vectors for the rollout reader, model-free.
//!
//! Each fixture line below is **synthesized to the shapes read off a real codex-cli 0.145.0
//! install** (the research record's method: 2,437 local rollouts; field names verified
//! structurally, content invented here so no session's words travel with the code). A vector is
//! one recorded stimulus and its complete expectation over the emitted event names and the
//! load-bearing fields — a reader that dropped a record or guessed a missing field fails here,
//! with the difference in the detail.

use metaharness_protocol::{
    ConformanceTier, Emission, Event, HermeticAttestation, HermeticMode, TranscriptRef,
    VectorOutcome,
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
    ]
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
