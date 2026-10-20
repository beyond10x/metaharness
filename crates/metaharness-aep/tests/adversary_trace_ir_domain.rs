//! Adversary cases for `story:trace-ir-domain`, written 2026-10-20 against the working tree over
//! base `d7fad095ab119ecd95b33c2d98b6a39b276f9ac3`.
//!
//! Each case either drives the production projection with an input the generated suite never
//! constructs, or reads the committed suite and asserts what it observes against what
//! `spec/domains/trace.yaml` states. Nothing here edits the target, the specification, a reader or
//! a fixture. Expected values are literals taken from the domain file, never from the code.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CLOSED_FAMILY, CloseReason, CommandOutcome, DecidedBy, Decision, DecisionCensus, Digest,
    EVENT_NAMES, Emission, Event, EventLine, EventStream, HermeticAttestation, HermeticMode,
    ProcessTermination, RateLimitInfo, RunId, Seam, StepOutcome, StepRef, TranscriptRef, Usage,
    parse_event_line, project_document,
};
use serde_json::{Value, json};

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `spec/domains/trace.yaml:149-197`, as literals: the twenty wire kinds and the family the domain
/// says each lands in. `unk` is the eight kinds the `unk` outcome's summary lists at line 197.
const DOMAIN_TABLE: [(&str, &str); 20] = [
    ("session.started", "session_start"),
    ("session.ended", "run_outcome"),
    ("usage", "run_outcome"),
    ("text", "assistant_text"),
    ("thinking", "assistant_thinking"),
    ("thinking.estimate", "thinking_estimate"),
    ("injection", "synthetic_injection"),
    ("tool.requested", "tool_call"),
    ("tool.result", "tool_result"),
    ("rate_limit", "rate_limit"),
    ("opaque", "opaque"),
    ("stream.closed", "stream_closed"),
    ("step.entered", "unk"),
    ("step.left", "unk"),
    ("turn.started", "unk"),
    ("turn.ended", "unk"),
    ("tool.decided", "unk"),
    ("command.result", "unk"),
    ("warning", "unk"),
    ("auth.expired", "unk"),
];

/// `spec/domains/trace.yaml:196`.
const DOMAIN_UNK_REASON: &str = "no trace-ir/1 family";

/// The bytes a stream of framed lines is, one JSON object per line.
fn stream_bytes(lines: &[EventLine]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend(serde_json::to_vec(line).unwrap());
        bytes.push(b'\n');
    }
    bytes
}

/// Frame through the production `EventStream`, the one place the marker is written.
fn framed(emissions: Vec<Emission>, close: Option<CloseReason>) -> (Vec<EventLine>, Vec<u8>) {
    let mut stream = EventStream::new(RunId::new("adversary"));
    let mut lines: Vec<EventLine> = emissions
        .into_iter()
        .map(|emission| stream.stamp(emission))
        .collect();
    if let Some(reason) = close {
        lines.extend(stream.close(reason));
    }
    let bytes = stream_bytes(&lines);
    (lines, bytes)
}

fn text() -> Emission {
    Emission::untimed(Event::Text {
        text: "done".to_owned(),
        request_id: None,
    })
}

/// One event of each wire kind, carrying the fields its variant requires and nothing else. Same
/// shape as the target's own sample, rebuilt here so this file does not depend on the file under
/// attack.
#[allow(clippy::too_many_lines)]
fn sample_event(wire: &str) -> Event {
    let step = || StepRef {
        workflow: "adversary".to_owned(),
        state: "attack".to_owned(),
        index: 1,
        attempt: 1,
    };
    match wire {
        "session.started" => Event::SessionStarted {
            adapter: "adversary".to_owned(),
            adapter_class: "harness".to_owned(),
            harness_version: None,
            session_id: None,
            model: None,
            permission_mode: None,
            credential_source: None,
            output_style: None,
            cwd: None,
            offered_tools: None,
            available_operations: None,
            withheld: None,
            slash_commands: None,
            skills: None,
            agents: None,
            plugins: None,
            mcp_servers: None,
            inputs_digest: None,
            transcript: TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            hermetic: HermeticAttestation::none(HermeticMode::Off),
        },
        "session.ended" => Event::SessionEnded {
            final_answer: None,
            observed_models: None,
            is_error: None,
            subtype: None,
            stop_reason: None,
            terminal_reason: None,
            api_error_status: None,
            num_turns: None,
            duration_ms: None,
            duration_api_ms: None,
            ttft_ms: None,
            time_to_request_ms: None,
            total_cost_usd: None,
            permission_denials: None,
            subagents_spawned: None,
            usage: None,
            model_usage: None,
            census: DecisionCensus::default(),
        },
        "step.entered" => Event::StepEntered {
            step: step(),
            frame_digest: None,
        },
        "step.left" => Event::StepLeft {
            step: step(),
            outcome: StepOutcome::Completed,
        },
        "turn.started" => Event::TurnStarted {
            turn: 1,
            frame_digest: None,
        },
        "turn.ended" => Event::TurnEnded {
            turn: 1,
            stop_reason: None,
        },
        "text" => Event::Text {
            text: "hello".to_owned(),
            request_id: None,
        },
        "thinking" => Event::Thinking {
            text: "reasoning".to_owned(),
            request_id: None,
        },
        "thinking.estimate" => Event::ThinkingEstimate {
            estimate: 1,
            delta: None,
        },
        "injection" => Event::Injection {
            text: "frame".to_owned(),
            origin: None,
        },
        "tool.requested" => Event::ToolRequested {
            call_id: "t1".to_owned(),
            name: "Read".to_owned(),
            input: json!({}),
            operations: Vec::new(),
            subjects: Vec::new(),
            decision_required: false,
            deadline_ms: None,
            seam: Seam::None,
        },
        "tool.decided" => Event::ToolDecided {
            call_id: "t1".to_owned(),
            decision: Decision::Allow,
            decided_by: DecidedBy::Embedder,
            seam: Seam::None,
            latency_ms: None,
        },
        "tool.result" => Event::ToolResult {
            exit_code: None,
            outcome_source: None,
            call_id: "t1".to_owned(),
            is_error: None,
            content: None,
            bytes: None,
            tool_use_result: None,
        },
        "usage" => Event::Usage {
            request_id: None,
            model: None,
            usage: Usage::default(),
        },
        "rate_limit" => Event::RateLimit {
            info: RateLimitInfo {
                status: None,
                window: None,
                resets_at: None,
                utilization: None,
                using_overage: None,
            },
        },
        "command.result" => Event::CommandResult {
            id: "c1".to_owned(),
            outcome: CommandOutcome::Ok { applies_at: None },
        },
        "warning" => Event::Warning {
            code: "ADVERSARY".to_owned(),
            message: "fixture".to_owned(),
        },
        "opaque" => Event::Opaque {
            vendor_type: None,
            vendor_subtype: None,
            digest: Digest::of(b"opaque"),
            source_line: None,
        },
        "auth.expired" => Event::AuthExpired {
            credential_source: None,
            detail: None,
            source_line: None,
        },
        "stream.closed" => Event::StreamClosed {
            process: ProcessTermination::Unknown,
            events: 0,
            reason: CloseReason::Completed,
            run_id: "adversary".to_owned(),
        },
        other => panic!("{other} is not on the wire"),
    }
}

/// Every wire kind, not the twelve the suite samples. `spec/domains/trace.yaml` states twenty rows;
/// the generated `ClassifyEventKind` scenarios drive twelve inputs, one of them an `unk` kind.
/// This case is the observation the other seven `unk` rows never get from the suite.
#[test]
fn every_wire_kind_lands_in_the_family_the_domain_states() {
    let declared: BTreeSet<&str> = DOMAIN_TABLE.iter().map(|(kind, _)| *kind).collect();
    let wire: BTreeSet<&str> = EVENT_NAMES.iter().copied().collect();
    assert_eq!(
        declared, wire,
        "the domain's twenty rows are the wire's twenty names"
    );
    let mut wrong = Vec::new();
    for (kind, family) in DOMAIN_TABLE {
        let sample = sample_event(kind);
        assert_eq!(sample.name(), kind, "the sample is of the kind asked for");
        let (lines, bytes) = framed(vec![Emission::untimed(sample)], None);
        let document = project_document(&lines, &bytes);
        assert_eq!(document.events.len(), 1, "{kind}: every event is a node");
        let node = &document.events[0].kind;
        let observed = node["event"].as_str().unwrap_or("<no family tag>");
        if observed != family {
            wrong.push(format!(
                "{kind}: the domain says {family}, production wrote {observed}"
            ));
            continue;
        }
        if family == "unk" {
            if node["event_kind"] != kind {
                wrong.push(format!(
                    "{kind}: the unk node names {} instead of its kind",
                    node["event_kind"]
                ));
            }
            if node["reason"] != DOMAIN_UNK_REASON {
                wrong.push(format!(
                    "{kind}: the unk reason is {} and the domain says {DOMAIN_UNK_REASON:?}",
                    node["reason"]
                ));
            }
            if document.metaharness.unk_kinds.get(kind) != Some(&1) {
                wrong.push(format!("{kind}: not counted in unk_kinds"));
            }
        } else if !document.metaharness.unk_kinds.is_empty() {
            wrong.push(format!(
                "{kind}: a mapped kind was counted as a vocabulary gap"
            ));
        }
        if document.metaharness.families.get(family) != Some(&1) {
            wrong.push(format!("{kind}: families[{family}] is not 1"));
        }
    }
    assert!(
        wrong.is_empty(),
        "production disagrees with spec/domains/trace.yaml on:\n{}",
        wrong.join("\n")
    );
}

/// `spec/domains/trace.yaml:125-128` puts two shapes under `misplaced-or-miscounting`; the target
/// builds only the misplaced one (a line after the marker). This is the other shape: the marker is
/// last and its count is wrong. The marker is the production one, with its count overwritten.
#[test]
fn a_marker_that_is_last_but_miscounts_is_not_complete() {
    let mut stream = EventStream::new(RunId::new("adversary"));
    let mut lines = vec![stream.stamp(text())];
    let mut marker = stream
        .close(CloseReason::Completed)
        .expect("a fresh stream closes once");
    let Event::StreamClosed { events, .. } = &mut marker.event else {
        panic!("close writes the marker");
    };
    assert_eq!(
        *events, 1,
        "the production marker counts the one node before it"
    );
    *events = 7;
    lines.push(marker);
    let bytes = stream_bytes(&lines);
    let document = project_document(&lines, &bytes);
    assert_eq!(
        document.metaharness.families.get(CLOSED_FAMILY),
        Some(&1),
        "the marker is present"
    );
    assert!(
        !document.metaharness.stream_complete,
        "a marker counting 7 over 1 preceding node was read as complete"
    );
    assert_eq!(document.metaharness.closed, None);
}

/// A third inconsistent shape the domain does not name: two markers. Production writes one
/// (`EventStream::close` returns `None` the second time); a file appended to can carry two.
#[test]
fn a_stream_with_two_markers_is_not_complete() {
    let mut stream = EventStream::new(RunId::new("adversary"));
    let mut lines = vec![stream.stamp(text())];
    let first = stream
        .close(CloseReason::Completed)
        .expect("a fresh stream closes once");
    assert!(
        stream.close(CloseReason::Completed).is_none(),
        "production writes one marker"
    );
    let mut second = first.clone();
    second.seq = first.seq + 1;
    if let Event::StreamClosed { events, .. } = &mut second.event {
        *events = 2;
    }
    lines.push(first);
    lines.push(second);
    let bytes = stream_bytes(&lines);
    let document = project_document(&lines, &bytes);
    assert_eq!(document.metaharness.families.get(CLOSED_FAMILY), Some(&2));
    assert!(
        !document.metaharness.stream_complete,
        "a stream with two markers was read as complete"
    );
    assert_eq!(document.metaharness.closed, None);
}

/// `spec/domains/trace.yaml:68-69`: the digest is over the stream's own bytes. The suite's
/// `NameDigest` compares against bytes the target serialized itself from the lines, so it cannot
/// tell "the bytes as read" from "re-serialized lines"; the recorded runs are files, and this
/// case reads them as files.
#[test]
fn the_recorded_runs_digest_names_the_committed_file_bytes() {
    for run in ["decomposer-clean", "plan-reviewer-clean"] {
        let path = repository_root().join(format!("evals/aep/runs/{run}.events.jsonl"));
        let bytes = fs::read(&path).unwrap();
        let lines: Vec<EventLine> = std::str::from_utf8(&bytes)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_event_line(line).unwrap())
            .collect();
        let document = project_document(&lines, &bytes);
        assert_eq!(
            document.transcript_digest,
            Digest::of(&bytes).as_str(),
            "{run}: transcript_digest is not the SHA-256 of {}",
            path.display()
        );
        let reserialized = stream_bytes(&lines);
        println!(
            "{run}: re-serialized lines {} the file bytes",
            if reserialized == bytes {
                "equal"
            } else {
                "differ from"
            }
        );
        if reserialized != bytes {
            assert_ne!(
                document.transcript_digest,
                Digest::of(&reserialized).as_str(),
                "{run}: the digest names re-serialized lines, a file that does not exist"
            );
        }
    }
}

/// `spec/domains/trace.yaml:24` declares twenty wire kinds and `:197` lists eight `unk` kinds.
/// This reads which kinds the committed suite actually drives through `ClassifyEventKind`. Red
/// while kinds the domain states are never observed by the suite that claims to execute it.
#[test]
fn the_suite_classifies_every_wire_kind_the_domain_declares() {
    let suite: Value =
        serde_json::from_slice(&fs::read(repository_root().join("spec/conformance.json")).unwrap())
            .unwrap();
    let mut driven = BTreeSet::new();
    for scenario in suite["scenarios"].as_object().unwrap().values() {
        for step in scenario["steps"].as_array().unwrap() {
            if step["step"] == "execute_command"
                && step["command"] == "metaharness.trace.ClassifyEventKind"
            {
                driven.insert(
                    step["input"]["kind"]["value"]
                        .as_str()
                        .unwrap()
                        .replace('-', "."),
                );
            }
        }
    }
    let missing: Vec<&str> = EVENT_NAMES
        .iter()
        .copied()
        .filter(|kind| !driven.contains(*kind))
        .collect();
    assert!(
        missing.is_empty(),
        "spec/conformance.json classifies {} of 20 wire kinds; stated in spec/domains/trace.yaml \
         and never driven: {}",
        driven.len(),
        missing.join(", ")
    );
}

// --- pass 2 (2026-10-20), against the corrected domain: 79 scenarios, 34 of them trace.

/// `spec/domains/trace.yaml:51-54`: `timestamp` is the vendor's, verbatim, and `timestamp_ms` is
/// derived from it and from nothing else. The retained view exposes both on `opening_node` and no
/// predicate reads either, so this pins them to literals: the first line of each recorded run
/// carries `at` as written in the file, and the millisecond value is the one
/// `trace_ir.rs` tests give for exactly that string (`2026-08-21T09:00:00.000Z` is
/// 1 787 302 800 000; `09:20` is 1 200 000 later).
#[test]
fn the_opening_node_carries_the_vendors_timestamp_and_its_millisecond_derivation() {
    for (run, at, at_ms) in [
        (
            "decomposer-clean",
            "2026-08-21T09:00:00.000Z",
            1_787_302_800_000_i64,
        ),
        (
            "plan-reviewer-clean",
            "2026-08-21T09:20:00.000Z",
            1_787_304_000_000_i64,
        ),
    ] {
        let path = repository_root().join(format!("evals/aep/runs/{run}.events.jsonl"));
        let bytes = fs::read(&path).unwrap();
        let lines: Vec<EventLine> = std::str::from_utf8(&bytes)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_event_line(line).unwrap())
            .collect();
        assert_eq!(
            lines[0].at.as_deref(),
            Some(at),
            "{run}: the fixture's first `at`"
        );
        let document = project_document(&lines, &bytes);
        let opening = &document.events[0];
        assert_eq!(
            opening.timestamp.as_deref(),
            Some(at),
            "{run}: the opening node's timestamp is not the vendor's verbatim"
        );
        assert_eq!(
            opening.timestamp_ms,
            Some(at_ms),
            "{run}: timestamp_ms is not derived from {at}"
        );
    }
}

/// P1 across builds, not across two renders by one build: a committed rendered document exists
/// for each recorded run (`evals/aep/runs/<run>.trace-ir.json`), and a fresh projection of the
/// committed stream must render to exactly those bytes. The suite's `byte_stable` compares two
/// renders of the same build in the same process (`trace.yaml:121,129-130`) and never reads
/// these files.
#[test]
fn a_fresh_projection_renders_to_the_committed_golden_document() {
    for run in ["decomposer-clean", "plan-reviewer-clean"] {
        let stream = repository_root().join(format!("evals/aep/runs/{run}.events.jsonl"));
        let golden = repository_root().join(format!("evals/aep/runs/{run}.trace-ir.json"));
        let bytes = fs::read(&stream).unwrap();
        let lines: Vec<EventLine> = std::str::from_utf8(&bytes)
            .unwrap()
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| parse_event_line(line).unwrap())
            .collect();
        let rendered = project_document(&lines, &bytes).render().unwrap();
        let committed = fs::read_to_string(&golden).unwrap();
        if rendered != committed {
            let first_difference = rendered
                .lines()
                .zip(committed.lines())
                .position(|(fresh, kept)| fresh != kept)
                .map_or_else(
                    || "line counts differ".to_owned(),
                    |index| {
                        format!(
                            "line {}: fresh {:?} vs committed {:?}",
                            index + 1,
                            rendered.lines().nth(index).unwrap_or(""),
                            committed.lines().nth(index).unwrap_or("")
                        )
                    },
                );
            panic!(
                "{run}: a fresh projection does not render to {} ({} vs {} bytes); first \
                 difference at {first_difference}",
                golden.display(),
                rendered.len(),
                committed.len()
            );
        }
    }
}
