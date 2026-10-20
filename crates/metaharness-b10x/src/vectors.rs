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
    recorded_wire: Obligation::Filled(&[
        "golden-loop-record",
        "provider-emulated-unpublished-tool",
        "provider-emulated-approval-denied",
        "provider-emulated-budget-stop",
        "provider-emulated-cancellation",
    ]),
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
const UNPUBLISHED: &str = include_str!("../fixtures/enforcement/unpublished.jsonl");
const UNPUBLISHED_EXPECTED: &str =
    include_str!("../fixtures/enforcement/unpublished.expected.jsonl");
const APPROVAL_DENIED: &str = include_str!("../fixtures/enforcement/approval-denied.jsonl");
const APPROVAL_DENIED_EXPECTED: &str =
    include_str!("../fixtures/enforcement/approval-denied.expected.jsonl");
const BUDGET_STOP: &str = include_str!("../fixtures/enforcement/budget-stop.jsonl");
const BUDGET_STOP_EXPECTED: &str =
    include_str!("../fixtures/enforcement/budget-stop.expected.jsonl");
const CANCELLED: &str = include_str!("../fixtures/enforcement/cancelled.jsonl");
const CANCELLED_EXPECTED: &str = include_str!("../fixtures/enforcement/cancelled.expected.jsonl");

/// Every free vector this adapter carries.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    vec![
        launch_vector(),
        golden_loop_vector(GOLDEN_LOOP),
        enforcement_vector(
            "provider-emulated-unpublished-tool",
            UNPUBLISHED,
            UNPUBLISHED_EXPECTED,
        ),
        enforcement_vector(
            "provider-emulated-approval-denied",
            APPROVAL_DENIED,
            APPROVAL_DENIED_EXPECTED,
        ),
        enforcement_vector(
            "provider-emulated-budget-stop",
            BUDGET_STOP,
            BUDGET_STOP_EXPECTED,
        ),
        enforcement_vector(
            "provider-emulated-cancellation",
            CANCELLED,
            CANCELLED_EXPECTED,
        ),
        golden_version_pair_vector(GOLDEN_VERSION),
    ]
}

fn enforcement_vector(id: &str, input: &str, expected: &str) -> VectorOutcome {
    let observed = golden_replay(input);
    if observed == expected {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            first_difference(expected, &observed),
        )
    }
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
    // First, what the expected stream's `"cwd":null` rests on: a recorded `started` line that
    // names no working directory. This reader ignores the key, so a recording from a release that
    // writes one (0.15.1 does) would replay as `null` and pass the byte comparison below while
    // contradicting the record it came from (design a30).
    if let Some(named) = recorded_working_directory(input) {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!(
                "the recorded `started` line names a working directory ({named}), and this reader \
                 writes `cwd: null`: read it (story:b10x-h7-from-the-loops-cwd) before \
                 re-recording"
            ),
        );
    }
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

/// The `cwd` the recorded loop's `started` line states, if it states one. The `started` line and
/// not the first one: a `credential-renewed` line comes before it when a token was renewed.
fn recorded_working_directory(input: &str) -> Option<String> {
    input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("kind").and_then(Value::as_str) == Some("started"))?
        .get("cwd")
        .map(Value::to_string)
}

fn reader() -> Box<dyn HarnessSeam> {
    let mut attestation = HermeticAttestation::none(HermeticMode::Strict);
    attestation.decisions = DecisionMode::Observe;
    B10xSeams::new(Some("0.15.0".to_owned()), Some("b10x-emulated".to_owned())).build(
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

    /// The golden loop is the pinned binary's own record, and the replay says so. A pin that moved
    /// without a re-recording leaves `golden-version-pair` passing with a warning — which it did
    /// from 0.10.2 to 0.13.3 over a 0.9.1 capture — and a reader that stamps another version than
    /// the banner beside the record attributes those bytes to a binary that did not write them.
    #[test]
    fn the_golden_loop_was_recorded_by_the_pinned_binary() {
        let pair = golden_version_pair_vector(GOLDEN_VERSION);
        assert!(pair.passed && pair.detail.is_empty(), "{pair:#?}");
        let opening: Value = serde_json::from_str(
            GOLDEN_LOOP_EXPECTED
                .lines()
                .next()
                .expect("the expected stream opens"),
        )
        .expect("the opening line is JSON");
        assert_eq!(
            Some(opening["harness_version"].as_str().unwrap_or_default()),
            GOLDEN_VERSION.trim().strip_prefix("b10x-harness ")
        );
    }

    /// The pinned loop names no working directory, so the replay states none (design a30).
    ///
    /// Line 1 is checked first, because `"cwd":null` in the expected stream is true only of a
    /// record that names no directory: the harness's own `started` record has no such field at
    /// 0.15.0 (`LoopEvent::Started`, harness `crates/harness-loop/src/event.rs`). The whole key
    /// set is pinned beside it, because the reason H7 prints on this kind — *the loop did not
    /// report its working directory* — is a claim about every key of that line, not only about one
    /// spelled `cwd`; a re-recording that adds a key fails here and a reader decides whether it
    /// names the directory before anything else moves.
    #[test]
    fn the_pinned_loop_names_no_working_directory_and_the_replay_records_null() {
        let started: Value =
            serde_json::from_str(GOLDEN_LOOP.lines().next().expect("the recorded loop opens"))
                .expect("the opening line is JSON");
        assert_eq!(started["kind"], "started");
        assert!(
            started.get("cwd").is_none(),
            "the recorded opening line names a working directory: {started}"
        );
        let mut keys: Vec<&str> = started
            .as_object()
            .expect("the opening line is an object")
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "agents",
                "context",
                "credential_source",
                "kind",
                "mcp",
                "memories",
                "model",
                "operations",
                "profiles",
                "published_tools",
                "skills",
                "toolchains",
                "withheld",
            ],
            "the recorded `started` line gained or lost a key: read whether it names the working \
             directory before re-pinning this list"
        );

        let opening: Value = serde_json::from_str(
            GOLDEN_LOOP_EXPECTED
                .lines()
                .next()
                .expect("the expected stream opens"),
        )
        .expect("the expected opening line is JSON");
        assert!(
            opening.get("cwd").is_some_and(Value::is_null),
            "the committed expectation states no working directory: {opening}"
        );
        let replayed: Value = serde_json::from_str(
            golden_replay(GOLDEN_LOOP)
                .lines()
                .next()
                .expect("the replay opens"),
        )
        .expect("the replayed opening line is JSON");
        assert!(
            replayed.get("cwd").is_some_and(Value::is_null),
            "a loop that names no directory is replayed as `cwd: null`, never as the plan \
             `reader()` was handed: {replayed}"
        );
    }

    /// The vector refuses a recording whose opening line names a working directory, instead of
    /// replaying it as `null`: this reader ignores the key, so a recording from 0.15.1, which
    /// writes one, would otherwise leave a passing vector over a record that contradicts it.
    #[test]
    fn a_recorded_opening_line_that_names_a_working_directory_fails_the_golden_vector() {
        let named = GOLDEN_LOOP.replacen(
            "{\"kind\":\"started\",",
            "{\"kind\":\"started\",\"cwd\":\"/somewhere\",",
            1,
        );
        assert_ne!(
            named, GOLDEN_LOOP,
            "the substitution found the opening line"
        );
        let outcome = golden_loop_vector(&named);
        assert!(!outcome.passed, "{outcome:#?}");
        assert!(
            outcome.detail.contains("working directory"),
            "the failure names what the record said: {}",
            outcome.detail
        );
    }

    /// The check reads the `started` line, wherever it falls: the loop writes `credential-renewed`
    /// before it when a stale token was renewed (harness `LoopEvent::CredentialRenewed`), and a
    /// check that read the first line would pass that recording on the renewal and fail it later
    /// for a reason that does not name the directory.
    #[test]
    fn a_started_line_after_a_credential_renewal_that_names_a_working_directory_fails_the_vector() {
        let renewed = r#"{"kind":"credential-renewed","provider":"codex","expires_unix":1,"refresh_token_rotated":false,"byte_preserving":true}"#;
        let named = format!(
            "{renewed}\n{}",
            GOLDEN_LOOP.replacen(
                "{\"kind\":\"started\",",
                "{\"kind\":\"started\",\"cwd\":\"/somewhere\",",
                1,
            )
        );
        let outcome = golden_loop_vector(&named);
        assert!(!outcome.passed, "{outcome:#?}");
        assert!(
            outcome.detail.contains("working directory"),
            "the failure names what the `started` line said: {}",
            outcome.detail
        );
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

    #[test]
    #[ignore = "writes the normalized enforcement expectations from captured loop records"]
    fn regenerate_the_enforcement_event_streams() {
        for (name, input) in [
            ("unpublished.expected.jsonl", UNPUBLISHED),
            ("approval-denied.expected.jsonl", APPROVAL_DENIED),
            ("budget-stop.expected.jsonl", BUDGET_STOP),
            ("cancelled.expected.jsonl", CANCELLED),
        ] {
            let path = format!("{}/fixtures/enforcement/{name}", env!("CARGO_MANIFEST_DIR"));
            std::fs::write(path, golden_replay(input)).expect("the expectation is written");
        }
    }
}
