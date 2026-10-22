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
//!   fails here, with the difference in the detail. The `MEDIATED_*` lines (amendment a47) are
//!   written from the 0.153.4 source at the lines each one cites, with values invented here.
//!
//! No path, name or value in the C1 fixtures comes from a real machine.

use std::collections::BTreeSet;
use std::path::PathBuf;

use metaharness_protocol::{
    ConformanceTier, ContractObligations, CredentialSource, DeclaredContextFile, Digest, Emission,
    Event, EventStream, HarnessSeam, HermeticAttestation, HermeticMode, HermeticRow, Kind,
    Obligation, RunId, RunSpec, Seam, ToolSurface, TranscriptRef, VectorOutcome,
};
use serde_json::{Value, json};

use crate::bridge::{CodexSeam, hook_request_line};

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
        "c1-scratch-cwd-declaration",
        "c1-context-files",
        "c1-caller-catalogue-refusal",
    ]),
    recorded_wire: Obligation::Filled(&["golden-rollout"]),
    recorded_hook_input: Obligation::Filled(&["golden-hook-input"]),
    version_pair: Obligation::Filled(&["golden-version-pair"]),
};

const META: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","session_id":"01a0-fixture","cli_version":"0.153.4","cwd":"/scratch/work","originator":"codex_exec","model_provider":"openai"}}"#;
const META_UNPINNED: &str = r#"{"timestamp":"2026-08-22T10:00:00.000Z","type":"session_meta","payload":{"id":"01a0-fixture","cli_version":"0.999.0","cwd":"/scratch/work"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-08-22T10:00:01.000Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const CALL: &str = r#"{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{"type":"function_call","call_id":"call-1","name":"exec","arguments":"{\"command\":\"ls\"}"}}"#;
const OUTPUT: &str = r#"{"timestamp":"2026-08-22T10:00:03.000Z","type":"response_item","payload":{"type":"function_call_output","call_id":"call-1","output":"src\ndocs\n"}}"#;
const TOKENS: &str = r#"{"timestamp":"2026-08-22T10:00:04.000Z","type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":100,"cached_input_tokens":40,"cache_write_input_tokens":10,"output_tokens":20,"reasoning_output_tokens":6,"total_tokens":120}},"rate_limits":{"limit_name":"weekly","plan_type":"pro","primary":{"used_percent":12.5}}}}"#;
const COMPLETE: &str = r#"{"timestamp":"2026-08-22T10:00:05.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","duration_ms":4200,"time_to_first_token_ms":800}}"#;
const APRIL_SHAPE: &str = r#"{"timestamp":"2026-04-01T10:00:02.000Z","type":"response_item","payload":{"type":"exec_command_begin","call_id":"call-9","command":["ls"]}}"#;
/// The records 0.153.4 writes between `session_meta` and the first `turn_context`, in the order a
/// native run wrote them (`docs/research/2026-10-21-codex-0.153.4-record-surface.md`).
const PREAMBLE_MESSAGE: &str = r#"{"timestamp":"2026-08-22T10:00:01.100Z","type":"response_item","payload":{"type":"message","role":"developer","content":[{"type":"input_text","text":"<permissions instructions>"}]}}"#;
const WORLD_STATE: &str = r#"{"timestamp":"2026-08-22T10:00:01.200Z","type":"world_state","payload":{"full":true,"state":{"agents_md":{}}}}"#;
const TURN_CONTEXT: &str = r#"{"timestamp":"2026-08-22T10:00:01.300Z","type":"turn_context","payload":{"turn_id":"t1","model":"a-model","personality":"pragmatic"}}"#;
const TURN_CONTEXT_FOREIGN: &str = r#"{"timestamp":"2026-08-22T10:00:01.300Z","type":"turn_context","payload":{"turn_id":"t1","model":"a-model","personality":"friendly"}}"#;
const TURN_CONTEXT_SILENT: &str = r#"{"timestamp":"2026-08-22T10:00:01.300Z","type":"turn_context","payload":{"turn_id":"t1","model":"a-model"}}"#;
/// Amendment a47: one code-mode `exec` call that ran two commands, in the 0.153.4 shapes and with
/// values invented here. The rollout records the outer call under the model's `call_*` id. Each
/// command reaches the seam twice: its `PreToolUse` request, under the `exec-<uuid>` id code mode
/// minted (`codex-rs/core/src/tools/code_mode/mod.rs:374`, `core/src/hook_runtime.rs:184-203`,
/// `hooks/src/schema.rs:278-296`), and its `item_completed`, under the same id and turn id
/// (`core/src/tools/events.rs:563-586`, `core/src/session/mod.rs:2439-2464`).
const MEDIATED_CALL: &str = r#"{"timestamp":"2026-08-22T10:00:02.000Z","type":"response_item","payload":{"type":"custom_tool_call","status":"completed","call_id":"call_fixture","name":"exec","input":"await tools.exec_command({cmd: \"true\"}); await tools.exec_command({cmd: \"exit 7\"});"}}"#;
const MEDIATED_HOOK_ZERO: &str = r#"{"session_id":"01a0-fixture","turn_id":"t1","transcript_path":null,"cwd":"/scratch/work","hook_event_name":"PreToolUse","model":"a-model","permission_mode":"bypassPermissions","tool_name":"Bash","tool_input":{"command":"true"},"tool_use_id":"exec-00000000-0000-4000-8000-000000000000"}"#;
const MEDIATED_DONE_ZERO: &str = r#"{"timestamp":"2026-08-22T10:00:02.400Z","type":"event_msg","payload":{"type":"item_completed","thread_id":"01a0-fixture","turn_id":"t1","item":{"type":"CommandExecution","id":"exec-00000000-0000-4000-8000-000000000000","command":["/bin/bash","-lc","true"],"cwd":"file:///scratch/work","parsed_cmd":[],"source":"unified_exec_startup","status":"completed","aggregated_output":"","exit_code":0},"started_at_ms":1787392802100,"completed_at_ms":1787392802400}}"#;
const MEDIATED_HOOK_SEVEN: &str = r#"{"session_id":"01a0-fixture","turn_id":"t1","transcript_path":null,"cwd":"/scratch/work","hook_event_name":"PreToolUse","model":"a-model","permission_mode":"bypassPermissions","tool_name":"Bash","tool_input":{"command":"exit 7"},"tool_use_id":"exec-00000000-0000-4000-8000-000000000007"}"#;
const MEDIATED_DONE_SEVEN: &str = r#"{"timestamp":"2026-08-22T10:00:02.800Z","type":"event_msg","payload":{"type":"item_completed","thread_id":"01a0-fixture","turn_id":"t1","item":{"type":"CommandExecution","id":"exec-00000000-0000-4000-8000-000000000007","command":["/bin/bash","-lc","exit 7"],"cwd":"file:///scratch/work","parsed_cmd":[],"source":"unified_exec_startup","status":"failed","aggregated_output":"","exit_code":7},"started_at_ms":1787392802500,"completed_at_ms":1787392802800}}"#;
const MEDIATED_OUTPUT: &str = r#"{"timestamp":"2026-08-22T10:00:03.000Z","type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"call_fixture","output":"Script completed\n"}}"#;

/// Every free vector this adapter carries: the launch face, then the replay face.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    let mut outcomes = launch_vectors();
    outcomes.push(scratch_cwd_vector());
    outcomes.extend([
        vector_full_session(),
        vector_version_gate(),
        vector_drifted_shape_is_opaque_not_fatal(),
        vector_nothing_is_dropped(),
        vector_mediated_exec_exit(),
        vector_output_style_recorded(),
        vector_output_style_foreign(),
        vector_output_style_absent(),
        golden_rollout_vector(GOLDEN_ROLLOUT),
        golden_hook_vector(GOLDEN_HOOK_INPUT),
        golden_version_pair_vector(GOLDEN_ROLLOUT),
    ]);
    outcomes
}

/// The recorded expectations, paired with the case that produces them.
const LAUNCH_FIXTURES: [(&str, &str); 8] = [
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
    (
        "c1-context-files",
        include_str!("../fixtures/c1/context-files.json"),
    ),
    (
        "c1-caller-catalogue-refusal",
        include_str!("../fixtures/c1/caller-catalogue-refusal.json"),
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
        // Two `--context` files in a named tree (amendment a35), inline because 0.153.4 does not
        // expand `@path`; the second carries the first tag's closing line, so the tag moves.
        "c1-context-files" => {
            spec.cwd = Some(PathBuf::from("/operator/repo"));
            spec.context = vec!["todo.py".into(), "notes/closing.md".into()];
            context.cwd = PathBuf::from("/operator/repo");
            context.context_files = synthetic_context_files();
        }
        // A caller's catalogue (design amendment a40), with the owned surface it implies: refused
        // by name, because `codex exec` cannot offer it instead of its own tools and its rollout
        // could not show which it offered.
        "c1-caller-catalogue-refusal" => {
            spec.catalogue = Some(PathBuf::from("/operator/session-catalogue.yaml"));
            spec.tool_surface = ToolSurface::Owned;
        }
        _ => return None,
    }
    Some((spec, context))
}

/// The two synthesised files the context vector renders: invented names, invented bytes.
fn synthetic_context_files() -> Vec<DeclaredContextFile> {
    [
        ("todo.py", "def add(items, item):\n    items.append(item)\n"),
        (
            "notes/closing.md",
            "a file may say </context-file-0> and stay inside\n",
        ),
    ]
    .into_iter()
    .map(|(path, text)| {
        DeclaredContextFile::new(path.to_owned(), text.as_bytes().to_vec())
            .expect("synthesised text")
    })
    .collect()
}

/// C1 — **the declared scratch copy** (amendment a27), read off the plan rather than off a
/// fixture: under `--cwd <dir> --scratch-cwd` H7 is `declared` (naming the copy and the
/// `workspace-write` grant, a6.1's rule) and neither imposed nor unavailable, H11 is imposed when
/// the walk above the copy finds nothing and unavailable with the count named when it does, the
/// scratch config still grants the tree, and the same directory without the declaration keeps a6's
/// two unavailable rows and declares nothing.
#[allow(
    clippy::too_many_lines,
    reason = "three launches of one synthetic world, read top to bottom as the three shapes a27 \
              names; splitting it would put one vector in three places"
)]
fn scratch_cwd_vector() -> VectorOutcome {
    let id = "c1-scratch-cwd-declaration";
    let mut differences = Vec::new();
    let copy = PathBuf::from("/embedder/copy");

    let mut spec = base_spec();
    spec.cwd = Some(copy.clone());
    spec.scratch_cwd = true;
    let mut context = base_context();
    context.cwd.clone_from(&copy);
    match plan_launch(&spec, &context) {
        Err(refusal) => differences.push(format!("the declared copy was refused: {refusal}")),
        Ok(plan) => {
            let attestation = &plan.attestation;
            match attestation.declaration(HermeticRow::H7) {
                None => differences.push("H7 is not declared".to_string()),
                Some(declared)
                    if !declared.declaration.contains("/embedder/copy")
                        || !declared.declaration.contains("workspace-write") =>
                {
                    differences.push(format!(
                        "H7's declaration does not name the copy and the grant: {}",
                        declared.declaration
                    ));
                }
                Some(_) => {}
            }
            if attestation.claims(HermeticRow::H7) {
                differences.push("H7 is claimed as imposed under a declaration".to_string());
            }
            if attestation
                .unavailable
                .iter()
                .any(|control| control.row == HermeticRow::H7)
            {
                differences.push("H7 is attested unavailable under a declaration".to_string());
            }
            if !attestation.claims(HermeticRow::H11) {
                differences.push(
                    "H11 is not imposed although the walk above the copy found nothing".to_string(),
                );
            }
            if !plan.config.contains(r#"sandbox_mode = "workspace-write""#) {
                differences.push("the declared copy is not writable by the child".to_string());
            }
            if plan.cwd != copy {
                differences.push(format!("the plan's cwd is {}", plan.cwd.display()));
            }
        }
    }

    // A memory file above the copy is measured, not declared away.
    let mut polluted = base_context();
    polluted.cwd.clone_from(&copy);
    polluted.memory_ancestors = vec![PathBuf::from("/embedder/AGENTS.md")];
    match plan_launch(&spec, &polluted) {
        Err(refusal) => differences.push(format!(
            "a memory file above the declared copy refused the launch instead of being \
             attested: {refusal}"
        )),
        Ok(plan) => {
            if !plan.attestation.declares(HermeticRow::H7) {
                differences.push("H7 is not declared when a memory file is above".to_string());
            }
            if plan.attestation.claims(HermeticRow::H11) {
                differences.push("H11 is imposed over a memory file above the copy".to_string());
            }
            let measured = plan.attestation.unavailable.iter().any(|control| {
                control.row == HermeticRow::H11 && control.why.contains("1 memory file")
            });
            if !measured {
                differences
                    .push("H11 is not attested unavailable with the count named".to_string());
            }
        }
    }

    // Without the declaration, a6 as written.
    let mut named = base_spec();
    named.cwd = Some(copy.clone());
    let mut context = base_context();
    context.cwd = copy;
    match plan_launch(&named, &context) {
        Err(refusal) => differences.push(format!("the a6 directory was refused: {refusal}")),
        Ok(plan) => {
            if !plan.attestation.declared.is_empty() {
                differences.push(format!(
                    "a directory nobody declared carries a declaration: {:?}",
                    plan.attestation.declared
                ));
            }
            for row in [HermeticRow::H7, HermeticRow::H11] {
                if !plan
                    .attestation
                    .unavailable
                    .iter()
                    .any(|control| control.row == row)
                {
                    differences.push(format!("{} is not unavailable under a6", row.id()));
                }
            }
        }
    }

    if differences.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C1)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C1, differences.join("; "))
    }
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
        context_files: Vec::new(),
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
        config_layers: Vec::new(),
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
        cwd: Some("/fixture/operator/work".to_string()),
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
    // The opening record and its warning are held until the record that completes it, and
    // released together (amendment a34).
    let mut emitted = reader.push_line(META_UNPINNED);
    emitted.extend(reader.push_line(TURN_CONTEXT));
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

/// C2 — **a command run through the code-mode `exec` tool reaches `tool.result` with its exit**
/// (amendment a47). The seam reads the rollout and the hook requests merged, as on a live run. Each
/// completion is joined to the hook request that carried its id, in the same turn; exit 0 is a
/// success and exit 7 an error, both from the retained completion, and nothing is left unverified.
fn vector_mediated_exec_exit() -> VectorOutcome {
    let id = "CX2-mediated-exec-exit";
    let mut seam = CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::Hook,
    );
    let mut emitted = Vec::new();
    for line in [
        META.to_owned(),
        TASK_STARTED.to_owned(),
        TURN_CONTEXT.to_owned(),
        MEDIATED_CALL.to_owned(),
        hook_request_line("hook-zero", MEDIATED_HOOK_ZERO),
        MEDIATED_DONE_ZERO.to_owned(),
        hook_request_line("hook-seven", MEDIATED_HOOK_SEVEN),
        MEDIATED_DONE_SEVEN.to_owned(),
        MEDIATED_OUTPUT.to_owned(),
        COMPLETE.to_owned(),
    ] {
        emitted.extend(seam.push_line(&line));
    }
    emitted.extend(seam.finish());
    let outcome = |call: &str| {
        emitted.iter().rev().find_map(|line| match &line.event {
            Event::ToolResult {
                call_id,
                is_error,
                exit_code,
                outcome_source,
                ..
            } if call_id == call => Some((*is_error, *exit_code, outcome_source.clone())),
            _ => None,
        })
    };
    let retained = Some("retained_tool_completion".to_owned());
    let zero = outcome("exec-00000000-0000-4000-8000-000000000000");
    let seven = outcome("exec-00000000-0000-4000-8000-000000000007");
    let unverified = emitted
        .iter()
        .filter(|line| {
            matches!(&line.event, Event::Warning { code, .. } if code == "TOOL_OUTCOME_UNVERIFIED")
        })
        .count();
    if zero == Some((Some(false), Some(0), retained.clone()))
        && seven == Some((Some(true), Some(7), retained))
        && unverified == 0
    {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            id,
            ConformanceTier::C2,
            format!("exit 0 read {zero:?}, exit 7 read {seven:?}, {unverified} unverified"),
        )
    }
}

/// C2 — **the opening record carries the personality the first `turn_context` recorded**, which is
/// what the launch wrote (amendment a34). The record arrives after `session_meta`, so the reader
/// holds the opening record, and everything after it, until that record: nothing is emitted before
/// it, the opening record keeps `session_meta`'s timestamp, and every held record keeps its order
/// and its own line.
fn vector_output_style_recorded() -> VectorOutcome {
    output_style_vector(
        "CX2-output-style-recorded",
        TURN_CONTEXT,
        Some(crate::PERSONALITY),
    )
}

/// C2 — a personality other than the launch's is carried **verbatim**. The reader states what the
/// record says and decides nothing; the floor compares it with what the launch wrote and reads
/// `gap` (amendment a34).
fn vector_output_style_foreign() -> VectorOutcome {
    output_style_vector(
        "CX2-output-style-foreign",
        TURN_CONTEXT_FOREIGN,
        Some("friendly"),
    )
}

fn output_style_vector(id: &str, turn_context: &str, expected: Option<&str>) -> VectorOutcome {
    let mut reader = reader();
    let mut faults = Vec::new();
    for line in [META, TASK_STARTED, PREAMBLE_MESSAGE, WORLD_STATE] {
        let early = reader.push_line(line);
        if !early.is_empty() {
            faults.push(format!(
                "{:?} was emitted before the record that completes the opening record",
                names(&early)
            ));
        }
    }
    let released = reader.push_line(turn_context);
    let seen = names(&released);
    let order = [
        "session.started",
        "turn.started",
        "opaque",
        "opaque",
        "opaque",
    ];
    if seen != order {
        faults.push(format!("released {seen:?}, expected {order:?}"));
    }
    match released.first() {
        Some(Emission {
            at,
            event: Event::SessionStarted { output_style, .. },
        }) => {
            if output_style.as_deref() != expected {
                faults.push(format!(
                    "the opening record's output style is {output_style:?}, expected {expected:?}"
                ));
            }
            if at.as_deref() != Some("2026-08-22T10:00:00.000Z") {
                faults.push(format!(
                    "the opening record is stamped {at:?}, not session_meta's"
                ));
            }
        }
        other => faults.push(format!("the first release is {other:?}")),
    }
    let lines: Vec<Option<u64>> = released
        .iter()
        .filter_map(|emission| match &emission.event {
            Event::Opaque { source_line, .. } => Some(*source_line),
            _ => None,
        })
        .collect();
    if lines != [Some(3), Some(4), Some(5)] {
        faults.push(format!("the held records cite lines {lines:?}"));
    }
    let after = reader.push_line(CALL);
    if names(&after) != ["tool.requested"] {
        faults.push(format!(
            "after the release a call emitted {:?}",
            names(&after)
        ));
    }
    if faults.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C2, faults.join("; "))
    }
}

/// C2 — **a record that names no personality leaves the opening record without one**, which is
/// `unk`, never the launch's value (invariant 3). Three shapes: a `turn_context` with no
/// `personality`; no `turn_context` before the first record that is not one of the preamble's
/// (the hold is bounded by that record, so a drifted record cannot hold the stream open); and a
/// stream that ends while the record is held, which `finish` releases.
fn vector_output_style_absent() -> VectorOutcome {
    let id = "CX2-output-style-absent";
    let mut faults = Vec::new();

    let mut silent = reader();
    for line in [META, TASK_STARTED, WORLD_STATE] {
        silent.push_line(line);
    }
    let released = silent.push_line(TURN_CONTEXT_SILENT);
    expect_unstyled_opening("a turn_context with no personality", &released, &mut faults);

    let mut drifted = reader();
    drifted.push_line(META);
    drifted.push_line(TASK_STARTED);
    let released = drifted.push_line(CALL);
    expect_unstyled_opening("a call before any turn_context", &released, &mut faults);
    if names(&released) != ["session.started", "turn.started", "tool.requested"] {
        faults.push(format!(
            "a call before any turn_context released {:?}",
            names(&released)
        ));
    }

    let mut ended = reader();
    ended.push_line(META);
    ended.push_line(TASK_STARTED);
    let released = ended.finish();
    expect_unstyled_opening("a stream that ended while held", &released, &mut faults);
    if names(&released) != ["session.started", "turn.started"] {
        faults.push(format!(
            "a stream that ended while held released {:?}",
            names(&released)
        ));
    }

    if faults.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C2)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C2, faults.join("; "))
    }
}

fn expect_unstyled_opening(shape: &str, released: &[Emission], faults: &mut Vec<String>) {
    match released.first().map(|emission| &emission.event) {
        Some(Event::SessionStarted {
            output_style: None,
            ..
        }) => {}
        other => faults.push(format!(
            "{shape}: the first release is {other:?}, expected an opening record with no output style"
        )),
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
            "c1-caller-catalogue-refusal",
            "CX2-session",
            "CX2-version-gate",
            "CX2-drift-opaque",
            "CX2-nothing-dropped",
            "CX2-mediated-exec-exit",
            "CX2-output-style-recorded",
            "CX2-output-style-foreign",
            "CX2-output-style-absent",
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
        // The seven recorded expectations, and the one computed vector (a27's declared scratch
        // copy) that reads the plan's attestation instead of comparing against a fixture.
        assert_eq!(ids.len(), LAUNCH_FIXTURES.len() + 1);
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
        assert!(
            outcome.detail.contains(crate::PINNED_VERSIONS[0]),
            "{}",
            outcome.detail
        );
    }

    #[test]
    fn a_recorded_version_on_the_pin_passes_with_nothing_to_say() {
        let outcome = version_pair_outcome(Some(crate::PINNED_VERSIONS[0]));
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
