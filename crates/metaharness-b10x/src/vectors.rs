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
    let observed = golden_replay(input);
    // First, what the expected stream's `cwd` rests on: the replayed opening record states the
    // directory the recorded `started` line names, and `null` only when it names none (design
    // a31). The expectation is regenerated from this reader, so a reader that dropped or replaced
    // the key would pass the byte comparison below over a regenerated file while contradicting
    // the record it came from; this compares the replay with the record instead.
    if let Err(detail) = working_directory_carried(input, &observed) {
        return VectorOutcome::failed(id, ConformanceTier::C2, detail);
    }
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

/// Whether the replay's opening record states the working directory the recorded `started` line
/// names — that string, or `null` when the line names none — and in which words it does not.
///
/// The `started` line and not the first one: a `credential-renewed` line comes before it when a
/// token was renewed. A `cwd` that is not a string names no directory, on both sides.
fn working_directory_carried(input: &str, replay: &str) -> Result<(), String> {
    let recorded = input
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|value| value.get("kind").and_then(Value::as_str) == Some("started"))
        .and_then(|started| {
            started
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    let replayed = replay
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .find(|event| event.get("event").and_then(Value::as_str) == Some("session.started"))
        .and_then(|opening| {
            opening
                .get("cwd")
                .and_then(Value::as_str)
                .map(str::to_owned)
        });
    if recorded == replayed {
        Ok(())
    } else {
        Err(format!(
            "the recorded `started` line names the working directory {recorded:?} and the replayed \
             `session.started` states {replayed:?}: the record's `cwd` is the loop's own word \
             (design a31)"
        ))
    }
}

fn reader() -> Box<dyn HarnessSeam> {
    let mut attestation = HermeticAttestation::none(HermeticMode::Strict);
    attestation.decisions = DecisionMode::Observe;
    B10xSeams::new(Some("0.15.1".to_owned()), Some("b10x-emulated".to_owned())).build(
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

    /// The opening record with its `cwd` key removed: the absent shape, which the pinned loop
    /// writes under `--substrate <socket>` and for a root whose name is not UTF-8 (harness
    /// `LoopEvent::Started`), and which no committed recording carries.
    fn without_a_working_directory(record: &str) -> String {
        record
            .lines()
            .map(|line| match serde_json::from_str::<Value>(line) {
                Ok(mut value) if value["kind"] == "started" => {
                    value
                        .as_object_mut()
                        .expect("the started line is an object")
                        .remove("cwd")
                        .expect("the recorded started line names a working directory");
                    format!("{value}\n")
                }
                _ => format!("{line}\n"),
            })
            .collect()
    }

    fn opening_cwd(stream: &str) -> Value {
        stream
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .find(|event| event["event"] == "session.started")
            .expect("the stream opens")
            .get("cwd")
            .cloned()
            .expect("`session.started` always carries the `cwd` key")
    }

    /// The pinned loop names its working directory, and the replay states exactly that one
    /// (design a31): present and carried, never `reader()`'s plan and never `null`.
    ///
    /// Line 1 is read first, because the expected stream's `cwd` is true only of a record that
    /// names it: the harness's own `started` record carries `cwd` from 0.15.1
    /// (`LoopEvent::Started`, harness `crates/harness-loop/src/event.rs` at `f0870131`), the
    /// canonical root its file tools act in. The whole key set is pinned beside it, so a
    /// re-recording that adds or drops a key fails here and a reader decides what it says before
    /// anything else moves.
    #[test]
    fn the_pinned_loop_names_its_working_directory_and_the_replay_carries_it() {
        let started: Value =
            serde_json::from_str(GOLDEN_LOOP.lines().next().expect("the recorded loop opens"))
                .expect("the opening line is JSON");
        assert_eq!(started["kind"], "started");
        assert_eq!(
            started["cwd"], "/scratch/run/work",
            "the recording's workspace, at the neutral path the fixture README names: {started}"
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
                "cwd",
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
            "the recorded `started` line gained or lost a key: read what it says before \
             re-pinning this list"
        );

        assert_eq!(
            opening_cwd(GOLDEN_LOOP_EXPECTED),
            started["cwd"],
            "the committed expectation states the directory the loop reported"
        );
        assert_eq!(
            opening_cwd(&golden_replay(GOLDEN_LOOP)),
            started["cwd"],
            "the replay states the directory the loop reported, never the plan `reader()` was \
             handed and never `null`"
        );
    }

    /// The absent shape, over a record derived from the golden by removing its `cwd` key (the
    /// golden carries one, so this shape cannot be recorded from the pinned emulator run): the
    /// replay states `null`, never `reader()`'s plan (design a30, kept by a31), and the stream
    /// differs from the expectation in that one field of line 1.
    #[test]
    fn a_recorded_opening_line_without_a_working_directory_replays_as_null() {
        let absent = without_a_working_directory(GOLDEN_LOOP);
        assert_ne!(absent, GOLDEN_LOOP, "the derivation removed the key");
        let replay = golden_replay(&absent);
        assert_eq!(opening_cwd(&replay), Value::Null, "{replay}");
        let expected_lines: Vec<&str> = GOLDEN_LOOP_EXPECTED.lines().collect();
        let replayed_lines: Vec<&str> = replay.lines().collect();
        assert_eq!(expected_lines.len(), replayed_lines.len());
        assert_eq!(
            &expected_lines[1..],
            &replayed_lines[1..],
            "only line 1 moves"
        );
        assert_eq!(
            replayed_lines[0],
            expected_lines[0].replacen("\"cwd\":\"/scratch/run/work\"", "\"cwd\":null", 1),
            "and in line 1 only the `cwd` value"
        );
    }

    /// The golden vector holds the replay to the record, not only to a file regenerated from the
    /// same reader: a replay that drops the recorded directory, or states another one (the plan
    /// `reader()` might be handed), fails it in words that name both; a replay of a record that
    /// names none must state none.
    #[test]
    fn the_golden_vector_fails_a_replay_that_does_not_state_the_recorded_directory() {
        let replay = golden_replay(GOLDEN_LOOP);
        assert_eq!(working_directory_carried(GOLDEN_LOOP, &replay), Ok(()));
        for wrong in ["\"cwd\":null", "\"cwd\":\"/scratch/run/plan\""] {
            let replayed = replay.replacen("\"cwd\":\"/scratch/run/work\"", wrong, 1);
            assert_ne!(
                replayed, replay,
                "the substitution found the opening record"
            );
            let detail = working_directory_carried(GOLDEN_LOOP, &replayed)
                .expect_err("a replay that does not state the recorded directory fails");
            assert!(
                detail.contains("/scratch/run/work") && detail.contains("working directory"),
                "{detail}"
            );
        }
        let absent = without_a_working_directory(GOLDEN_LOOP);
        assert_eq!(
            working_directory_carried(&absent, &golden_replay(&absent)),
            Ok(())
        );
        assert!(working_directory_carried(&absent, &replay).is_err());
    }

    /// The seam reads the `started` line wherever it falls: the loop writes `credential-renewed`
    /// before it when a stale token was renewed (harness `LoopEvent::CredentialRenewed`), and the
    /// opening record still states the directory that line names.
    #[test]
    fn a_started_line_after_a_credential_renewal_carries_its_working_directory() {
        let renewed = r#"{"kind":"credential-renewed","provider":"codex","expires_unix":1,"refresh_token_rotated":false,"byte_preserving":true}"#;
        let named = format!(
            "{renewed}\n{}",
            GOLDEN_LOOP.replacen("\"cwd\":\"/scratch/run/work\"", "\"cwd\":\"/somewhere\"", 1)
        );
        assert_eq!(opening_cwd(&golden_replay(&named)), "/somewhere");
        let absent = format!("{renewed}\n{}", without_a_working_directory(GOLDEN_LOOP));
        assert_eq!(opening_cwd(&golden_replay(&absent)), Value::Null);
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
