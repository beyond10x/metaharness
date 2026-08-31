//! The b10x adapter contract: one launch plan and one recorded loop, both model-free.
//!
//! The recorded sample is `provider_emulated` evidence. It was produced by the released
//! `b10x-harness` binary against harness's deterministic local Responses endpoint; it says what
//! the real binary wrote and makes no claim about a live provider.

use std::fmt::Write as _;

use metaharness_protocol::{
    ConformanceTier, ContractObligations, DecisionMode, EventStream, HarnessSeam,
    HermeticAttestation, HermeticMode, Obligation, RunId, Seam, SeamFactory, TranscriptRef,
    VectorOutcome,
};
use serde_json::{Value, json};

use crate::{B10xLaunch, B10xSeams, PINNED_VERSIONS, argv, base_environment};

/// What this adapter owes in the same shape as every other adapter.
pub const CONTRACT_OBLIGATIONS: ContractObligations = ContractObligations {
    adapter: crate::ADAPTER_ID,
    launch: Obligation::Filled(&["c1-observe-launch"]),
    recorded_wire: Obligation::Filled(&["golden-loop-record"]),
    recorded_hook_input: Obligation::Gap(
        "not applicable: b10x is an observe-only direct-provider adapter and has no metaharness \
         hook or decision seam; inventing hook input would contradict the adapter's boundary",
    ),
    version_pair: Obligation::Filled(&["golden-version-pair"]),
};

const LAUNCH_EXPECTED: &str = include_str!("../fixtures/c1/observe-launch.json");
const GOLDEN_LOOP: &str = include_str!("../fixtures/golden/loop.jsonl");
const GOLDEN_LOOP_EXPECTED: &str = include_str!("../fixtures/golden/loop.expected.jsonl");
const GOLDEN_VERSION: &str = include_str!("../fixtures/golden/version.txt");

/// Every free vector this adapter carries.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    vec![
        launch_vector(),
        golden_loop_vector(GOLDEN_LOOP),
        golden_version_pair_vector(GOLDEN_VERSION),
    ]
}

fn launch_vector() -> VectorOutcome {
    let id = "c1-observe-launch";
    let mut launch = B10xLaunch::new(
        "https://example.invalid/v1",
        "b10x-emulated",
        "/scratch/run/work",
        "observe this run",
    );
    "/opt/b10x/bin/b10x-harness".clone_into(&mut launch.program);
    let mut args = argv(&launch);
    let program = args.remove(0);
    let observed = json!({
        "program": program,
        "args": args,
        "env": base_environment(
            Some("/operator"),
            std::path::Path::new("/scratch/run/config"),
        ),
    });
    compare(id, ConformanceTier::C1, LAUNCH_EXPECTED, &observed)
}

fn compare(id: &str, tier: ConformanceTier, expectation: &str, observed: &Value) -> VectorOutcome {
    let expected: Value = match serde_json::from_str(expectation) {
        Ok(value) => value,
        Err(error) => {
            return VectorOutcome::failed(id, tier, format!("the fixture did not parse: {error}"));
        }
    };
    if expected == *observed {
        VectorOutcome::passed(id, tier)
    } else {
        VectorOutcome::failed(
            id,
            tier,
            format!("expected {expected}, observed {observed}"),
        )
    }
}

fn golden_loop_vector(input: &str) -> VectorOutcome {
    let id = "golden-loop-record";
    let observed = golden_replay(input);
    if observed == GOLDEN_LOOP_EXPECTED {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            first_difference(GOLDEN_LOOP_EXPECTED, &observed),
        )
    }
}

fn reader() -> Box<dyn HarnessSeam> {
    let mut attestation = HermeticAttestation::none(HermeticMode::Strict);
    attestation.decisions = DecisionMode::Observe;
    B10xSeams::new(
        Some("0.8.0".to_owned()),
        Some("b10x-emulated".to_owned()),
        Some("/scratch/run/work".to_owned()),
    )
    .build(
        TranscriptRef {
            path: Some("/scratch/run/loop.jsonl".to_owned()),
            digest: None,
            bytes: None,
        },
        attestation,
        Seam::None,
    )
}

fn golden_replay(input: &str) -> String {
    let mut reader = reader();
    let mut stream = EventStream::new(RunId::new("golden"));
    let mut out = String::new();
    for line in input.lines() {
        for emission in reader.push_line(line) {
            push_line(&mut out, &stream.stamp(emission));
        }
    }
    for emission in reader.finish() {
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
            let _ = writeln!(out, "UNSERIALIZABLE: {error}");
        }
    }
}

fn first_difference(expected: &str, observed: &str) -> String {
    let mut expected_lines = expected.lines();
    let mut observed_lines = observed.lines();
    let mut number = 0;
    loop {
        number += 1;
        match (expected_lines.next(), observed_lines.next()) {
            (None, None) => return "the streams differ only in trailing bytes".to_owned(),
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

fn golden_version_pair_vector(banner: &str) -> VectorOutcome {
    let id = "golden-version-pair";
    let recorded = banner.trim().strip_prefix("b10x-harness ");
    let Some(recorded) = recorded else {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("the captured version banner has an unknown shape: {banner:?}"),
        );
    };
    if PINNED_VERSIONS.contains(&recorded) {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::passed_with_warning(
            id,
            ConformanceTier::C2,
            format!(
                "the binary that wrote the golden loop reported {recorded} and the adapter pins {}",
                PINNED_VERSIONS.join(", ")
            ),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_vector_passes() {
        let failed: Vec<_> = conformance_vectors()
            .into_iter()
            .filter(|outcome| !outcome.passed)
            .collect();
        assert!(failed.is_empty(), "{failed:#?}");
    }

    #[test]
    fn one_changed_record_byte_breaks_the_golden_replay() {
        let changed = GOLDEN_LOOP.replacen("\"name\":\"file_read\"", "\"name\":\"search\"", 1);
        assert_ne!(golden_replay(&changed), GOLDEN_LOOP_EXPECTED);
    }

    #[test]
    #[ignore = "writes fixtures/golden/loop.expected.jsonl from the committed input"]
    fn regenerate_the_golden_event_stream() {
        let path = format!(
            "{}/fixtures/golden/loop.expected.jsonl",
            env!("CARGO_MANIFEST_DIR")
        );
        std::fs::write(path, golden_replay(GOLDEN_LOOP)).expect("the expectation is written");
    }
}
