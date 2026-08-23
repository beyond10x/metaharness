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
//! The C1 and C2 fixtures are **synthesised**. No transcript from a real account is reproduced
//! in them: a fixture that carried a server name, an address or a path from somebody's machine
//! would be a leak wearing a test's clothes.
//!
//! The **golden** fixtures under `fixtures/golden/` are the deliberate exception (adapter
//! contract CT-2): **recorded real wire**, captured from one controlled hermetic run of the
//! installed binary through `metaharness run claude --hermetic --retain-dir …` — a scratch
//! config home, a scratch cwd, one `ls`, and nothing of anybody's account in the bytes
//! (reviewed before commit; provenance in `fixtures/golden/README.md`). A synthesised fixture
//! tests the adapter against this crate's own assumptions; the golden one tests it against what
//! the vendor actually wrote, which is the difference between a green test of a stale
//! assumption and a red replay when the vendor moves.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use metaharness_protocol::{
    ConformanceTier, ContractObligations, CredentialSource, DecisionMode, Digest, EventStream,
    HermeticAttestation, HermeticMode, Kind, Obligation, PluginContent, PluginTree, RunId, RunSpec,
    Seam, ToolSurface, TranscriptRef, VectorOutcome, tree_digest,
};
use serde_json::{Value, json};

/// What this adapter's contract owes, in the one shape every adapter fills (CT-4).
///
/// The rows are not a summary of the vectors below — they are the declaration the vectors are
/// checked **against**, so this constant going stale is a red test rather than a stale comment
/// (`metaharness::contract_obligations`, and the per-adapter test that reads it).
pub const CONTRACT_OBLIGATIONS: ContractObligations = ContractObligations {
    adapter: crate::ADAPTER_ID,
    launch: Obligation::Filled(&[
        "c1-strict-hermetic",
        "c1-api-key",
        "c1-shadow-refusal",
        "c1-memory-ancestor-refusal",
        "c1-plugin-empty-refusal",
        "c1-observe-mode",
        "c1-plugin-injection",
    ]),
    recorded_wire: Obligation::Filled(&["golden-transcript"]),
    recorded_hook_input: Obligation::Filled(&["golden-hook-input"]),
    version_pair: Obligation::Filled(&["golden-version-pair"]),
};

use crate::launch::{LaunchContext, LaunchPlan, LaunchRefusal, plan_launch};
use crate::transcript::TranscriptReader;

/// Every free vector this adapter carries, executed, each outcome saying what differed.
///
/// A vector that only reported pass or fail would send a reader back to the code to find out
/// what happened, so every failure carries the two values side by side.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    let mut outcomes = launch_vectors();
    outcomes.push(observe_mode_vector());
    outcomes.push(plugin_injection_vector());
    outcomes.extend(replay_vectors());
    outcomes.push(golden_transcript_vector(GOLDEN_TRANSCRIPT));
    outcomes.push(golden_hook_vector(GOLDEN_HOOK_INPUT));
    outcomes.push(golden_version_pair_vector(GOLDEN_TRANSCRIPT));
    outcomes
}

/// C1 — observe mode is attested, and **a run that did not ask for it never gets it**.
///
/// Both halves in one vector, because the second is the one that matters: a mode that allows every
/// call must be reachable only by asking for it, so the polarity is asserted rather than assumed.
/// The vector plans three launches from the same synthetic world and reads the mode off each
/// attestation — the block that reaches `session.started`, which is where a reader of the record
/// finds out what decided the calls.
fn observe_mode_vector() -> VectorOutcome {
    let id = "c1-observe-mode";
    let mut differences = Vec::new();
    for mode in DecisionMode::ALL {
        let mut spec = base_spec();
        spec.decisions = mode;
        let plan = match plan_launch(&spec, &base_context()) {
            Ok(plan) => plan,
            Err(refusal) => {
                differences.push(format!(
                    "--decisions {} was refused: {refusal}",
                    mode.as_str()
                ));
                continue;
            }
        };
        if plan.attestation.decisions != mode {
            differences.push(format!(
                "--decisions {} is attested as {}",
                mode.as_str(),
                plan.attestation.decisions.as_str()
            ));
        }
        let observing = plan.attestation.is_observing();
        if observing != (mode == DecisionMode::Observe) {
            differences.push(format!(
                "--decisions {} reads back as observing={observing}",
                mode.as_str()
            ));
        }
        // The price of the mode is stated in the record, not only in a document: an allow on this
        // wire grants, so an observe run is not a run with the seam switched off.
        let says_grant = plan
            .attestation
            .ambient_inputs
            .iter()
            .any(|input| input.contains("observe mode") && input.contains("grants"));
        if says_grant != (mode == DecisionMode::Observe) {
            differences.push(format!(
                "--decisions {} reports the grant caveat={says_grant}",
                mode.as_str()
            ));
        }
    }
    if differences.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C1)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C1, differences.join("; "))
    }
}

/// C1 — **the plan is a value**: the copy list and the digest are readable before any process
/// exists, and the argv names the copy rather than the operator's own directory.
///
/// The mutation half lives in this crate's own tests (`a_mutated_plugin_file_changes_the_digest`);
/// what a *contract* consumer needs from this row is that the two facts are on the plan at all,
/// because an injection whose digest only appeared after the spawn could not be pinned by anybody.
fn plugin_injection_vector() -> VectorOutcome {
    let id = "c1-plugin-injection";
    let (source, tree) = synthetic_plugin();
    let PluginContent::Files { digest, .. } = tree.content.clone() else {
        return VectorOutcome::failed(id, ConformanceTier::C1, "the synthetic tree is not files");
    };

    let mut spec = base_spec();
    spec.plugin_dir.push(source.clone());
    let mut context = base_context();
    context.plugins.push(tree);
    let plan = match plan_launch(&spec, &context) {
        Ok(plan) => plan,
        Err(refusal) => {
            return VectorOutcome::failed(
                id,
                ConformanceTier::C1,
                format!("the injection was refused: {refusal}"),
            );
        }
    };

    let mut differences = Vec::new();
    let installed = PathBuf::from("/scratch/run-1/plugins/claude-code");
    match plan.plugin_installs.as_slice() {
        [install]
            if install.from == source && install.to == installed && install.digest == digest => {}
        other => differences.push(format!("the copy list is {other:?}")),
    }
    let named = plan
        .args
        .windows(2)
        .any(|pair| pair[0] == "--plugin-dir" && pair[1] == installed.display().to_string());
    if !named {
        differences.push(format!(
            "the argv does not name the copy at {}: {:?}",
            installed.display(),
            plan.args
        ));
    }
    if plan
        .args
        .iter()
        .any(|argument| *argument == source.display().to_string())
    {
        differences.push("the argv still names the operator's own directory".to_string());
    }
    match plan.attestation.installed_plugins.as_slice() {
        [attested]
            if attested.name == "claude-code"
                && attested.digest == digest
                && attested.source == source.display().to_string() => {}
        other => differences.push(format!("the attestation says {other:?}")),
    }

    // The explicit absence: a run with no plugin carries the key with an empty list, never no key.
    let uninjected = match plan_launch(&base_spec(), &base_context()) {
        Ok(plan) => plan,
        Err(refusal) => {
            return VectorOutcome::failed(
                id,
                ConformanceTier::C1,
                format!("the plugin-less launch was refused: {refusal}"),
            );
        }
    };
    if !uninjected.attestation.installed_plugins.is_empty()
        || !uninjected.plugin_installs.is_empty()
    {
        differences.push("a run that declared no plugin planned one".to_string());
    }
    if !serde_json::to_string(&uninjected.attestation)
        .unwrap_or_default()
        .contains("\"installed_plugins\":[]")
    {
        differences.push(
            "a plugin-less attestation drops the installed_plugins key instead of saying []"
                .to_string(),
        );
    }

    if differences.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C1)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C1, differences.join("; "))
    }
}

/// The version pair (CT-3, Q18): the recorded sample's own version claim against the pin.
///
/// The recorded wire and the pin are two sources that can disagree — the codex adapter's did,
/// caused by two installs resolved differently by two `PATH`s — so the contract asserts they
/// agree **or names the gap**: a disagreement is a warning the reader must see, never a silent
/// pass, and never a failure either, because the recorded fact is known and reddening the
/// contract over it teaches operators to ignore red.
///
/// **A golden carries its own capture version, and the pin is free to move without it.** The
/// bytes are a fact about the binary that wrote them and are never edited to match a pin — this
/// vector is the one place the two are related, so moving the pin either reconciles the pair or
/// leaves a named warning standing until a real re-capture. Claude's pair reconciled that way on
/// 2026-08-23 (amendment a10): the capture was already 2.1.240 and the pin came to it.
fn golden_version_pair_vector(transcript: &str) -> VectorOutcome {
    let recorded = transcript
        .lines()
        .next()
        .and_then(|line| serde_json::from_str::<Value>(line).ok())
        .and_then(|init| {
            init.get("claude_code_version")?
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
            "the golden sample's init record carries no claude_code_version, so the pair cannot \
             be reconciled",
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

/// Recorded real wire (adapter contract CT-2): one hermetic run's `stream-json` transcript and
/// the raw `PreToolUse` stdin its one tool call produced, byte for byte as the vendor wrote
/// them — **2.1.240's own bytes, captured 2026-08-23**, which is the version the adapter pins.
/// Capture provenance is `fixtures/golden/README.md`; re-capture per pin with
/// `metaharness run claude --hermetic --retain-dir …`.
const GOLDEN_HOOK_INPUT: &str = include_str!("../fixtures/golden/hook-input.json");
const GOLDEN_TRANSCRIPT: &str = include_str!("../fixtures/golden/transcript.jsonl");
const GOLDEN_TRANSCRIPT_EXPECTED: &str =
    include_str!("../fixtures/golden/transcript.expected.jsonl");

/// The run id the golden replay is framed under.
const GOLDEN_RUN: &str = "golden";

/// The recorded transcript in, the committed event stream out, byte-exact.
///
/// The input is what the pinned binary actually wrote, so a pass means the mapping holds
/// against the vendor's real bytes — including record types the synthesised fixtures never
/// thought to invent. On a re-capture at a new pin, a red run of this vector is the vendor
/// having moved, which is exactly the news it exists to carry.
fn golden_transcript_vector(input: &str) -> VectorOutcome {
    let id = "golden-transcript";
    let observed = replay_as(GOLDEN_RUN, input);
    if observed == GOLDEN_TRANSCRIPT_EXPECTED {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            first_difference(GOLDEN_TRANSCRIPT_EXPECTED, &observed),
        )
    }
}

/// The recorded hook stdin parses to exactly the recorded values, and the rendering table
/// agrees with the wire.
///
/// The second half is the CX-M2 lesson: what a hook receives in `tool_name` is a fact about the
/// vendor the adapter asserts and could be wrong about, so the recorded input must name the
/// same tool [`crate::render_operation`] renders `operation.shell` to — read off the published
/// capability descriptor, the same table the run loop admits calls by.
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
        tool_input: json!({"command": "ls", "description": "List files in current directory"}),
        session_id: Some("2c430a9a-faa4-4305-a3f4-e012153e600a".to_string()),
        tool_use_id: Some("toolu_0126wZX9VgSsd4Hw7AbgMkc3".to_string()),
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

/// The recorded expectations, paired with the case that produces them.
const LAUNCH_FIXTURES: [(&str, &str); 5] = [
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
    (
        "c1-plugin-empty-refusal",
        include_str!("../fixtures/c1/plugin-empty-refusal.json"),
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
        // A directory that exists and holds nothing: what a mistyped `--plugin-dir` looks like
        // after somebody "fixed" the error by creating the directory. The run would spawn, cost
        // money, install nothing, and report an injected plugin.
        "c1-plugin-empty-refusal" => {
            let (source, _) = synthetic_plugin();
            spec.plugin_dir.push(source.clone());
            context.plugins.push(PluginTree {
                source,
                content: PluginContent::Empty,
            });
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
        // No plugin unless a case says so. A synthetic tree, like everything else here: the
        // digest below is over invented file names and invented bytes, and no directory on any
        // machine was read to produce it.
        plugins: Vec::new(),
        // No proxy: every launch vector here is a pure plan against a synthetic world, and a
        // loopback endpoint is by construction a port something really bound.
        loopback: None,
    }
}

/// The plugin directory the injection vectors declare, and the tree a caller "read" in it.
///
/// Two files, so the digest is over more than one entry and the ordering rule is exercised.
fn synthetic_plugin() -> (PathBuf, PluginTree) {
    let source = PathBuf::from("/operator/integrations/claude-code");
    let files: BTreeMap<String, Digest> = [
        (
            ".claude-plugin/plugin.json".to_string(),
            Digest::of(b"{\"name\":\"claude-code\"}"),
        ),
        (
            "skills/planning/SKILL.md".to_string(),
            Digest::of(b"classify the request, then route it"),
        ),
    ]
    .into_iter()
    .collect();
    let tree = PluginTree {
        source: source.clone(),
        content: PluginContent::Files {
            count: files.len(),
            digest: tree_digest(&files),
        },
    };
    (source, tree)
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
    replay_as(REPLAY_RUN, input)
}

/// [`replay`] framed under a caller-named run id, so the golden stream says what it is.
fn replay_as(run: &str, input: &str) -> String {
    let mut reader = TranscriptReader::new(replay_transcript(), replay_attestation())
        .with_seam(Seam::Hook)
        .with_inputs_digest(Digest::of(b"the copied input tree"));
    let mut stream = EventStream::new(RunId::new(run));
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
            "golden-transcript",
            "golden-hook-input",
            "golden-version-pair",
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

    /// CT-2's acceptance clause, exercised: a mutated byte in the recorded wire fails its
    /// vector. A golden sample a mutation cannot redden is decoration, not a contract.
    #[test]
    fn a_mutated_byte_in_the_golden_transcript_fails_its_vector() {
        let mutated = GOLDEN_TRANSCRIPT.replacen("\"command\":\"ls\"", "\"command\":\"lz\"", 1);
        assert_ne!(mutated, GOLDEN_TRANSCRIPT, "the mutation found its byte");
        let outcome = golden_transcript_vector(&mutated);
        assert!(!outcome.passed, "the mutated transcript still passed");
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

    /// CT-3's acceptance clause: a recorded sample whose version differs from the doctor pin is
    /// a **named** contract warning — passed, non-empty detail, both versions in it.
    #[test]
    fn a_recorded_version_off_the_pin_is_a_named_warning_not_a_silent_pass() {
        let outcome = version_pair_outcome(Some("9.9.9"));
        assert!(outcome.is_warning(), "{outcome:?}");
        assert!(outcome.detail.contains("9.9.9"), "{}", outcome.detail);
        assert!(outcome.detail.contains("2.1.240"), "{}", outcome.detail);
    }

    #[test]
    fn a_recorded_version_on_the_pin_passes_with_nothing_to_say() {
        let outcome = version_pair_outcome(Some("2.1.240"));
        assert!(outcome.passed && outcome.detail.is_empty(), "{outcome:?}");
    }

    /// The pair is reconciled: the committed golden sample was captured from 2.1.240, the pin
    /// moved to 2.1.240 on 2026-08-23 (amendment a10), and the two now agree.
    ///
    /// The sample's bytes did not move to make this true — the pin did. This test reads the
    /// **committed capture**, never the machine's installed binary, so it says the same thing on
    /// a machine with no `claude` on it at all; that is why the whole C2 tier is free.
    #[test]
    fn the_committed_golden_sample_now_agrees_with_the_pin_and_has_nothing_to_warn_about() {
        let outcome = golden_version_pair_vector(GOLDEN_TRANSCRIPT);
        assert!(
            outcome.passed && !outcome.is_warning(),
            "the recorded capture and the pin disagree again: {outcome:?}"
        );
        assert!(outcome.detail.is_empty(), "{}", outcome.detail);
    }

    /// Regenerate the golden expectation from the committed recorded wire. `#[ignore]`d because
    /// it writes into the source tree; it is the second half of a re-capture, not of the gate:
    ///
    /// ```console
    /// metaharness run claude --hermetic --retain-dir <dir> -p "…"   # new pin, new wire
    /// cp <dir>/transcript.jsonl fixtures/golden/transcript.jsonl   # review it first
    /// cargo test -p metaharness-claude --lib regenerate -- --ignored
    /// ```
    #[test]
    #[ignore = "writes fixtures/golden/transcript.expected.jsonl from the committed input; run after a re-capture"]
    fn regenerate_the_golden_expectation() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/fixtures/golden/transcript.expected.jsonl"
        );
        std::fs::write(path, replay_as(GOLDEN_RUN, GOLDEN_TRANSCRIPT))
            .expect("the expectation is written");
    }

    /// The same operation for the three synthesised C2 expectations, whose inputs are committed
    /// beside them. `#[ignore]`d for the same reason: it writes into the source tree.
    ///
    /// It exists because a protocol amendment moves every expectation at once — a field added to
    /// an event changes seven fixture files, and hand-editing JSONL to match a serde field order
    /// is how a fixture stops describing what the reader does. Regenerating and **reading the
    /// diff** is the mapping's changelog; the diff is the review, not the write.
    #[test]
    #[ignore = "writes fixtures/c2/*.expected.jsonl from the committed inputs; run after a protocol change, then read the diff"]
    fn regenerate_the_replay_expectations() {
        for (id, input, _) in REPLAY_FIXTURES {
            let name = id.strip_prefix("c2-").unwrap_or(id);
            let path = format!(
                "{}/fixtures/c2/{name}.expected.jsonl",
                env!("CARGO_MANIFEST_DIR")
            );
            std::fs::write(&path, replay(input)).expect("the expectation is written");
        }
    }
}
