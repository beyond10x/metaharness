//! Conformance C1 and C2, both free of a model, a network and a credential.
//!
//! Three of design D13's four tiers cost nothing to run, and that is what makes this adapter's
//! promises a **tested claim** rather than a paragraph in a document (design § 8.5).
//!
//! * **C1 — launch vectors.** The argv and the child environment [`crate::plan_launch`] would
//!   construct for a given `RunSpec`, against a recorded expectation. It proves H3, H5, H8 and
//!   the whole launch half of § 8.1 — and, for the two refusal vectors, that a run which would
//!   be silently weakened is refused instead.
//! * **C2 — replay vectors.** A `stream-json` fixture in, the metaharness event stream out,
//!   **byte-exact JSONL**. It proves O2 and O3, including that a record type this adapter has
//!   never heard of becomes `opaque` and is not dropped.
//!
//! The fixtures are **synthesised**. No transcript from a real account is reproduced here: a
//! fixture that carried a server name, an address or a path from somebody's machine would be a
//! leak wearing a test's clothes.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::PathBuf;

use metaharness_protocol::{
    ConformanceTier, CredentialSource, DecisionMode, Digest, EventStream, HermeticAttestation,
    HermeticMode, Kind, RunId, RunSpec, Seam, ToolSurface, TranscriptRef, VectorOutcome,
};
use serde_json::{Value, json};

use crate::launch::{LaunchContext, LaunchPlan, LaunchRefusal, plan_launch};
use crate::transcript::TranscriptReader;

/// Every free vector this adapter carries, executed, each outcome saying what differed.
///
/// A vector that only reported pass or fail would send a reader back to the code to find out
/// what happened, so every failure carries the two values side by side.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    let mut outcomes = launch_vectors();
    outcomes.extend(replay_vectors());
    outcomes
}

/// The recorded expectations, paired with the case that produces them.
const LAUNCH_FIXTURES: [(&str, &str); 4] = [
    (
        "c1-strict-hermetic",
        include_str!("../fixtures/c1/strict-hermetic.json"),
    ),
    ("c1-api-key", include_str!("../fixtures/c1/api-key.json")),
    (
        "c1-shadow-refusal",
        include_str!("../fixtures/c1/shadow-refusal.json"),
    ),
    (
        "c1-memory-ancestor-refusal",
        include_str!("../fixtures/c1/memory-ancestor-refusal.json"),
    ),
];

const REPLAY_FIXTURES: [(&str, &str, &str); 3] = [
    (
        "c2-session",
        include_str!("../fixtures/c2/session.in.jsonl"),
        include_str!("../fixtures/c2/session.expected.jsonl"),
    ),
    (
        "c2-unknown-records",
        include_str!("../fixtures/c2/unknown-records.in.jsonl"),
        include_str!("../fixtures/c2/unknown-records.expected.jsonl"),
    ),
    (
        "c2-auth-expired",
        include_str!("../fixtures/c2/auth-expired.in.jsonl"),
        include_str!("../fixtures/c2/auth-expired.expected.jsonl"),
    ),
];

/// The run id every replay vector is framed under, so `seq` and `run` are reproducible.
const REPLAY_RUN: &str = "c2";

fn launch_vectors() -> Vec<VectorOutcome> {
    LAUNCH_FIXTURES
        .iter()
        .map(|(id, expectation)| launch_vector(id, expectation))
        .collect()
}

fn launch_vector(id: &str, expectation: &str) -> VectorOutcome {
    let Some((spec, context)) = launch_case(id) else {
        return VectorOutcome::failed(id, ConformanceTier::C1, "no case is registered for this id");
    };
    let observed = match plan_launch(&spec, &context) {
        Ok(plan) => observed_plan(&plan),
        Err(refusal) => observed_refusal(&refusal),
    };
    compare(id, ConformanceTier::C1, expectation, &observed)
}

/// What a launch vector records: the command line and the whole child environment, or the
/// refusal by code and by the sentence it prints.
fn observed_plan(plan: &LaunchPlan) -> Value {
    json!({
        "program": plan.program,
        "args": plan.args,
        "env": plan.env,
    })
}

fn observed_refusal(refusal: &LaunchRefusal) -> Value {
    json!({
        "refusal": {
            "code": refusal.code().map(|code| code.as_str().to_string()),
            "display": refusal.to_string(),
        }
    })
}

/// The four cases the launch vectors cover.
fn launch_case(id: &str) -> Option<(RunSpec, LaunchContext)> {
    let mut spec = base_spec();
    let mut context = base_context();
    match id {
        "c1-strict-hermetic" => {}
        "c1-api-key" => {
            spec.credentials = CredentialSource::ApiKey;
            context.inherited_env.insert(
                "ANTHROPIC_API_KEY".to_string(),
                "sk-not-a-real-key".to_string(),
            );
        }
        "c1-shadow-refusal" => {
            spec.decisions = DecisionMode::Ask;
            spec.tool_surface = ToolSurface::Owned;
        }
        "c1-memory-ancestor-refusal" => {
            context.memory_ancestors = vec![PathBuf::from("/scratch/CLAUDE.md")];
        }
        _ => return None,
    }
    Some((spec, context))
}

fn base_spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("state the workflow you are in".to_string());
    spec.model = Some("a-model".to_string());
    spec.max_turns = Some(8);
    spec
}

/// A synthetic world: no path, name or value here comes from a real machine.
fn base_context() -> LaunchContext {
    LaunchContext {
        scratch_root: PathBuf::from("/scratch/run-1"),
        cwd: PathBuf::from("/scratch/run-1/work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: [
            ("HOME", "/operator"),
            ("USER", "operator"),
            ("LANG", "C.UTF-8"),
            ("TERM", "dumb"),
            ("ANTHROPIC_BASE_URL", "https://example.invalid"),
            ("ANTHROPIC_MODEL", "another-model"),
            ("HTTPS_PROXY", "http://proxy.invalid:8080"),
            ("http_proxy", "http://proxy.invalid:8080"),
            ("CLAUDE_CODE_SAFE_MODE", "1"),
            ("CLAUDE_CODE_SIMPLE", "1"),
            ("DISABLE_TELEMETRY", "1"),
            ("SSH_AUTH_SOCK", "/run/agent.sock"),
            ("GIT_DIR", "/operator/repo/.git"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect(),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"the copied input tree")),
    }
}

fn replay_vectors() -> Vec<VectorOutcome> {
    REPLAY_FIXTURES
        .iter()
        .map(|(id, input, expected)| replay_vector(id, input, expected))
        .collect()
}

fn replay_vector(id: &str, input: &str, expected: &str) -> VectorOutcome {
    let observed = replay(input);
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

/// Read a fixture transcript and frame every event, byte for byte.
fn replay(input: &str) -> String {
    let mut reader = TranscriptReader::new(replay_transcript(), replay_attestation())
        .with_seam(Seam::Hook)
        .with_inputs_digest(Digest::of(b"the copied input tree"));
    let mut stream = EventStream::new(RunId::new(REPLAY_RUN));
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

fn push_line<T: serde::Serialize>(out: &mut String, line: &T) {
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

/// The retained-transcript reference the replay vectors carry.
///
/// Fixed rather than measured, because a digest that changed with the file would make the
/// expected stream unstable for a reason that has nothing to do with the mapping.
fn replay_transcript() -> TranscriptRef {
    TranscriptRef {
        path: Some("/scratch/run-1/transcript.jsonl".to_string()),
        digest: Some(Digest::of(b"the replay fixture")),
        bytes: Some(0),
    }
}

fn replay_attestation() -> HermeticAttestation {
    HermeticAttestation::none(HermeticMode::Strict)
}

/// Compare two JSON documents and say what differed, key by key.
fn compare(id: &str, tier: ConformanceTier, expectation: &str, observed: &Value) -> VectorOutcome {
    let expected: Value = match serde_json::from_str(expectation) {
        Ok(value) => value,
        Err(error) => {
            return VectorOutcome::failed(id, tier, format!("the fixture did not parse: {error}"));
        }
    };
    if expected == *observed {
        return VectorOutcome::passed(id, tier);
    }
    VectorOutcome::failed(id, tier, describe_difference(&expected, observed))
}

fn describe_difference(expected: &Value, observed: &Value) -> String {
    let (Some(expected), Some(observed)) = (expected.as_object(), observed.as_object()) else {
        return format!("expected {expected}, observed {observed}");
    };
    let mut differences = Vec::new();
    let keys: BTreeSet<&String> = expected.keys().chain(observed.keys()).collect();
    for key in keys {
        let left = expected.get(key);
        let right = observed.get(key);
        if left != right {
            differences.push(format!(
                "{key}: expected {}, observed {}",
                left.map_or_else(|| "absent".to_string(), ToString::to_string),
                right.map_or_else(|| "absent".to_string(), ToString::to_string)
            ));
        }
    }
    differences.join("; ")
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

#[cfg(test)]
mod tests {
    use super::*;

    /// C1, C2 and nothing that needs a model: the default gate must never reach for a credential
    /// (design D13).
    #[test]
    fn every_vector_passes_and_none_of_them_needs_a_model() {
        for outcome in conformance_vectors() {
            assert!(outcome.passed, "{}: {}", outcome.id, outcome.detail);
            assert!(!outcome.tier.needs_a_model(), "{}", outcome.id);
        }
    }

    #[test]
    fn the_vectors_cover_the_launch_cases_the_design_names() {
        let ids: Vec<String> = conformance_vectors()
            .into_iter()
            .map(|outcome| outcome.id)
            .collect();
        for id in [
            "c1-strict-hermetic",
            "c1-api-key",
            "c1-shadow-refusal",
            "c1-memory-ancestor-refusal",
            "c2-session",
            "c2-unknown-records",
            "c2-auth-expired",
        ] {
            assert!(ids.iter().any(|seen| seen == id), "{id} is not covered");
        }
    }

    /// A failure that did not say what differed would send the reader back to the code.
    #[test]
    fn a_failing_vector_reports_the_line_that_differed() {
        let detail = first_difference("a\nb\n", "a\nc\n");
        assert!(detail.contains("line 2"), "{detail}");
    }
}
