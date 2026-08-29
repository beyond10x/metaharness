//! C1 launch vectors and C2 replay vectors for the rollout reader, all model-free.
//!
//! * **C1 — launch vectors.** The argv, the child environment **and the scratch `config.toml`**
//!   [`crate::plan_launch`] would construct for a given `RunSpec`, against a recorded expectation.
//!   The config document is in the observation and that is the difference from the Claude
//!   adapter's C1: on this vendor the seam, the model provider and the sandbox posture are keys in
//!   a file rather than flags on a command line, so a launch vector that recorded only the argv
//!   would pin **nothing** about the hook — and an unrecognised key under `[hooks]` is dropped
//!   without failing the config load, which is the silent failure the whole adapter is written
//!   against.
//! * **C2 — replay vectors.** Each fixture line below is **synthesized to the shapes read off a
//!   real codex-cli 0.145.0 install** (the research record's method: 2,437 local rollouts; field
//!   names verified structurally, content invented here so no session's words travel with the
//!   code). A vector is one recorded stimulus and its complete expectation over the emitted event
//!   names and the load-bearing fields — a reader that dropped a record or guessed a missing field
//!   fails here, with the difference in the detail.
//!
//! No path, name or value in the C1 fixtures comes from a real machine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use metaharness_protocol::{
    ConformanceTier, ContractObligations, CredentialSource, Digest, Emission, Event, EventStream,
    HermeticAttestation, HermeticMode, Kind, Obligation, RunId, RunSpec, TranscriptRef,
    VectorOutcome,
};
use serde_json::{Value, json};

use crate::launch::{
    CodexLogin, LaunchContext, LaunchPlan, LaunchRefusal, LoopbackParams, plan_launch,
};
use crate::rollout::RolloutReader;

/// What this adapter's contract owes, in the one shape every adapter fills (CT-4).
///
/// All four rows are filled. The launch row was a **named gap** until 2026-08-23 — this adapter's
/// argv and child environment were pinned by the unit tests in `src/launch.rs` and by nothing a
/// consumer could read — and closing it moved `checked` deliberately, which is the only way a
/// count is allowed to move.
pub const CONTRACT_OBLIGATIONS: ContractObligations = ContractObligations {
    adapter: crate::ADAPTER_ID,
    launch: Obligation::Filled(&[
        "c1-strict-hermetic",
        "c1-api-key",
        "c1-loopback",
        "c1-loopback-subscription-refusal",
        "c1-unsupported-option-refusal",
        "c1-memory-ancestor-refusal",
    ]),
    recorded_wire: Obligation::Filled(&["golden-rollout"]),
    recorded_hook_input: Obligation::Filled(&["golden-hook-input"]),
    version_pair: Obligation::Filled(&["golden-version-pair"]),
};

const META: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","session_id":"01a0-fixture","cli_version":"0.145.0","cwd":"/scratch/work","originator":"codex_exec","model_provider":"openai"}}"#;
const META_UNPINNED: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","cli_version":"0.999.0","cwd":"/scratch/work"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-08-22T10:00:01.000Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const CALL: &str = r#"{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{"type":"function_call","call_id":"call-1","name":"exec","arguments":"{\"command\":\"ls\"}"}}"#;
const OUTPUT: &str = r#"{"timestamp":"2026-08-22T10:00:03.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"src\ndocs\n"}}"#;
const TOKENS: &str = r#"{"timestamp":"2026-08-22T10:00:04.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":40,"cache_write_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":6,"total_tokens":120}},"rate_limits":{"limit_name":"weekly","plan_type":"pro","primary":{"used_percent":12.5}}}}"#;
const COMPLETE: &str = r#"{"timestamp":"2026-08-22T10:00:05.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","duration_ms":4200,"time_to_first_token_ms":800}}"#;
const APRIL_SHAPE: &str = r#"{"timestamp":"2026-04-01T10:00:02.000Z","type":"response_item","payload":{"type":"exec_command_begin","call_id":"call-9","command":["ls"]}}"#;

/// Every free vector this adapter carries: the launch face, then the replay face.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    let mut outcomes = launch_vectors();
    outcomes.extend([
        vector_full_session(),
        vector_version_gate(),
        vector_drifted_shape_is_opaque_not_fatal(),
        vector_nothing_is_dropped(),
        golden_rollout_vector(GOLDEN_ROLLOUT),
        golden_hook_vector(GOLDEN_HOOK_INPUT),
        golden_version_pair_vector(GOLDEN_ROLLOUT),
    ]);
    outcomes
}

/// The recorded expectations, paired with the case that produces them.
const LAUNCH_FIXTURES: [(&str, &str); 6] = [
    (
        "c1-strict-hermetic",
        include_str!("../fixtures/c1/strict-hermetic.json"),
    ),
    ("c1-api-key", include_str!("../fixtures/c1/api-key.json")),
    ("c1-loopback", include_str!("../fixtures/c1/loopback.json")),
    (
        "c1-loopback-subscription-refusal",
        include_str!("../fixtures/c1/loopback-subscription-refusal.json"),
    ),
    (
        "c1-unsupported-option-refusal",
        include_str!("../fixtures/c1/unsupported-option-refusal.json"),
    ),
    (
        "c1-memory-ancestor-refusal",
        include_str!("../fixtures/c1/memory-ancestor-refusal.json"),
    ),
];

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

/// What a launch vector records: the command line, the whole child environment, **the whole
/// scratch `config.toml`** and every credential copy — or the refusal by code and by the sentence
/// it prints.
///
/// The config document is here because on this vendor it carries the seam, and the copy list is
/// here because "how many credentials travel" is the H6 claim and the LP-4 upgrade both: a
/// loopback run's empty list is the evidence, not a comment about it.
fn observed_plan(plan: &LaunchPlan) -> Value {
    json!({
        "program": plan.program,
        "args": plan.args,
        "env": plan.env,
        "config": plan.config,
        "credential_copies": plan
            .credential_copies
            .iter()
            .map(|copy| json!({
                "from": copy.from.display().to_string(),
                "to": copy.to.display().to_string(),
            }))
            .collect::<Vec<Value>>(),
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

/// The six cases the launch vectors cover.
fn launch_case(id: &str) -> Option<(RunSpec, LaunchContext)> {
    let mut spec = base_spec();
    let mut context = base_context();
    match id {
        "c1-strict-hermetic" => {}
        "c1-api-key" => {
            spec.credentials = CredentialSource::ApiKey;
            context.inherited_env.insert(
                "OPENAI_API_KEY".to_string(),
                "sk-not-a-real-key".to_string(),
            );
        }
        "c1-loopback" => {
            spec.credentials = CredentialSource::Loopback;
            context.loopback = Some(loopback_params(CodexLogin::ApiKey));
        }
        "c1-loopback-subscription-refusal" => {
            spec.credentials = CredentialSource::Loopback;
            context.loopback = Some(loopback_params(CodexLogin::Subscription));
        }
        "c1-unsupported-option-refusal" => {
            // `codex exec` has no turn ceiling, and an option that was set and ignored is a run
            // that is not the one that was asked for — the caller would find out from the bill.
            spec.max_turns = Some(8);
        }
        "c1-memory-ancestor-refusal" => {
            context.memory_ancestors = vec![PathBuf::from("/scratch/AGENTS.md")];
        }
        _ => return None,
    }
    Some((spec, context))
}

/// A proxy that never bound anything: the port is fixed here so the fixture is stable, which is
/// the one liberty a launch vector may take with a value the real run discovers.
fn loopback_params(login: CodexLogin) -> LoopbackParams {
    LoopbackParams {
        base_url: "http://127.0.0.1:45999".to_string(),
        placeholder: "mh-run-codex-1-not-a-real-nonce".to_string(),
        login,
    }
}

fn base_spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Codex);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("state the workflow you are in".to_string());
    spec.model = Some("a-model".to_string());
    spec
}

/// A synthetic world: no path, name or value here comes from a real machine.
fn base_context() -> LaunchContext {
    LaunchContext {
        scratch_root: PathBuf::from("/scratch/run-1"),
        cwd: PathBuf::from("/scratch/run-1/work"),
        credentials_file: Some(PathBuf::from("/operator/.codex/auth.json")),
        inherited_env: [
            ("HOME", "/operator"),
            ("USER", "operator"),
            ("LANG", "C.UTF-8"),
            ("TERM", "dumb"),
            ("OPENAI_BASE_URL", "https://example.invalid"),
            ("CODEX_DISABLE_HOOKS", "1"),
            ("HTTPS_PROXY", "http://proxy.invalid:8080"),
            ("SSH_AUTH_SOCK", "/run/agent.sock"),
            ("GIT_DIR", "/operator/repo/.git"),
            ("DISABLE_TELEMETRY", "1"),
        ]
        .into_iter()
        .map(|(key, value)| (key.to_string(), value.to_string()))
        .collect(),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"the copied input tree")),
        plugins: Vec::new(),
        loopback: None,
    }
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

/// The version pair (CT-3, Q18): the recorded sample's own version claim against the pin.
///
/// The recorded wire and the pin are two sources that can disagree — this adapter's did, and
/// the cause was two installs resolved differently by two `PATH`s — so the contract asserts
/// they agree **or names the gap**: a disagreement is a warning the reader must see, never a
/// silent pass, and never a failure either, because the recorded fact is known and reddening
/// the contract over it teaches operators to ignore red.
fn golden_version_pair_vector(rollout: &str) -> VectorOutcome {
    let recorded = rollout
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .and_then(|meta| {
            meta.get("payload")?
                .get("cli_version")?
                .as_str()
                .map(ToString::to_string)
        });
    version_pair_outcome(recorded.as_deref())
}

fn version_pair_outcome(recorded: Option<&str>) -> VectorOutcome {
    let id = "golden-version-pair";
    let Some(recorded) = recorded else {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            "the golden sample's session_meta carries no cli_version, so the pair cannot be \
             reconciled",
        );
    };
    if crate::PINNED_VERSIONS.contains(&recorded) {
        return VectorOutcome::passed(id, ConformanceTier::C2);
    }
    VectorOutcome::passed_with_warning(
        id,
        ConformanceTier::C2,
        format!(
            "the recorded sample was written by {recorded} and the adapter pins {}; every claim \
             the golden vectors hold is the recorded binary's, not the pin's (Q18)",
            crate::PINNED_VERSIONS.join(", ")
        ),
    )
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
        cwd: Some("/home/operator/work".to_string()),
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
    // Amendment a9, from this vendor's side: the one figure the rollout really carries is
    // carried, under the vendor's own name for it, and the three it does not are absent rather
    // than filled from a neighbour. A record that reported `iterations: 0` or a cost of zero for
    // a vendor that reports neither would read as a quiet run instead of an unanswered question.
    let a9_ok = emitted.iter().any(|line| {
        matches!(
            &line.event,
            Event::Usage { usage, .. }
                if usage.thinking_tokens == Some(6)
                    && usage.iterations.is_none()
                    && usage.speed.is_none()
                    && usage.cost_usd.is_none()
        )
    });
    // …and no `tool.result` this vendor writes carries a per-tool result record, because no
    // `*_call_output` payload has one.
    let sibling_absent = emitted.iter().all(|line| {
        !matches!(
            &line.event,
            Event::ToolResult {
                tool_use_result: Some(_),
                ..
            }
        )
    });
    if !call_ok || !end_ok || !a9_ok || !sibling_absent {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!(
                "call_ok={call_ok} end_ok={end_ok} a9_ok={a9_ok} sibling_absent={sibling_absent}"
            ),
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
            "c1-loopback",
            "c1-loopback-subscription-refusal",
            "c1-unsupported-option-refusal",
            "c1-memory-ancestor-refusal",
            "CX2-session",
            "CX2-version-gate",
            "CX2-drift-opaque",
            "CX2-nothing-dropped",
            "golden-rollout",
            "golden-hook-input",
            "golden-version-pair",
        ] {
            assert!(ids.iter().any(|seen| seen == id), "{id} is not covered");
        }
    }

    /// Every id the contract declaration names is a vector this run really produces, in C1.
    ///
    /// The workspace-level `contract_symmetry` test asserts the same thing through the library;
    /// this one keeps the crate honest on its own, so a rename here reddens the crate that made it.
    #[test]
    fn the_declared_launch_obligation_names_only_vectors_this_crate_produces() {
        let Obligation::Filled(ids) = CONTRACT_OBLIGATIONS.launch else {
            panic!("the launch row is filled since 2026-08-23; a gap here needs its own reason");
        };
        let outcomes = conformance_vectors();
        for id in ids {
            let outcome = outcomes
                .iter()
                .find(|outcome| outcome.id == *id)
                .unwrap_or_else(|| panic!("{id} is declared and not produced"));
            assert_eq!(outcome.tier, ConformanceTier::C1, "{id}");
            assert!(outcome.passed, "{id}: {}", outcome.detail);
        }
        assert_eq!(ids.len(), LAUNCH_FIXTURES.len());
    }

    /// A launch fixture must be able to go red. The mutation is in the **config document**,
    /// because that is the half of this adapter's launch face nothing else pins — and a fixture
    /// that could not notice a missing seam would be decoration.
    #[test]
    fn a_mutated_launch_expectation_fails_its_vector() {
        let (_, expectation) = LAUNCH_FIXTURES[0];
        let mutated = expectation.replacen("[[hooks.PreToolUse]]", "[[hooks.PreToolUze]]", 1);
        assert_ne!(mutated, expectation, "the mutation found its bytes");
        let outcome = launch_vector("c1-strict-hermetic", &mutated);
        assert!(!outcome.passed, "a seam spelled wrong still passed");
        assert!(outcome.detail.contains("config"), "{}", outcome.detail);
    }

    /// Regenerate the C1 expectations from the cases beside them. `#[ignore]`d because it writes
    /// into the source tree; it is the second half of a deliberate change, not of the gate:
    ///
    /// ```console
    /// cargo test -p metaharness-codex --lib regenerate_the_launch -- --ignored
    /// ```
    ///
    /// Read the diff afterwards: every changed line is a claim about what a `codex exec` child is
    /// given, and moving a count here moves the `contract_result` a consumer reads.
    #[test]
    #[ignore = "writes fixtures/c1/*.json from the cases in this file; run after a deliberate launch change, then read the diff"]
    fn regenerate_the_launch_expectations() {
        for (id, _) in LAUNCH_FIXTURES {
            let (spec, context) = launch_case(id).expect("a case is registered");
            let observed = match plan_launch(&spec, &context) {
                Ok(plan) => observed_plan(&plan),
                Err(refusal) => observed_refusal(&refusal),
            };
            let name = id.strip_prefix("c1-").unwrap_or(id);
            let path = format!("{}/fixtures/c1/{name}.json", env!("CARGO_MANIFEST_DIR"));
            let mut body = serde_json::to_string_pretty(&observed).expect("the expectation");
            body.push('\n');
            std::fs::write(&path, body).expect("the expectation is written");
        }
    }

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
    /// CT-3's acceptance clause: a recorded sample whose version differs from the doctor pin is
    /// a **named** contract warning — passed, non-empty detail, both versions in it.
    #[test]
    fn a_recorded_version_off_the_pin_is_a_named_warning_not_a_silent_pass() {
        let outcome = version_pair_outcome(Some("9.9.9"));
        assert!(outcome.is_warning(), "{outcome:?}");
        assert!(outcome.detail.contains("9.9.9"), "{}", outcome.detail);
        assert!(outcome.detail.contains("0.145.0"), "{}", outcome.detail);
    }

    #[test]
    fn a_recorded_version_on_the_pin_passes_with_nothing_to_say() {
        let outcome = version_pair_outcome(Some("0.145.0"));
        assert!(outcome.passed && outcome.detail.is_empty(), "{outcome:?}");
    }

    /// The committed golden sample really was written by 0.144.0 out of a 0.145.0-pinned
    /// adapter, so the shipped contract carries this warning today. A re-capture from an on-pin
    /// binary flips this expectation deliberately — that is the pair being reconciled.
    #[test]
    fn the_committed_golden_sample_carries_the_q18_warning() {
        let outcome = golden_version_pair_vector(GOLDEN_ROLLOUT);
        assert!(outcome.is_warning(), "{outcome:?}");
        assert!(outcome.detail.contains("0.144.0"), "{}", outcome.detail);
    }

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
