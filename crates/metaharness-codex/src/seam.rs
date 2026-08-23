//! What this adapter can honour, declared rather than discovered by refusal.
//!
//! Every status below is labelled the way the research record labels its facts. The **call** tier
//! is now `Delivered` and the rest are not, and the difference is one driven run (CX-M2): a real
//! `codex exec` asked to run `echo <marker>`, the installed `PreToolUse` program received the call
//! and printed metaharness's `deny`, and the vendor's own session record shows
//! *"Command blocked by `PreToolUse` hook: this step admits no shell…"* with an **empty**
//! output —
//! the deny reached the child before the effect. Design amendment a7.
//!
//! Everything else stays where it was. A tier upgrades when a driven run proves it, never because
//! the mechanism beside it worked.

use std::collections::BTreeMap;

use metaharness_protocol::{
    AdapterClass, AdapterId, COMMAND_NAMES, Capabilities, CommandSupport, Decision, DecisionMode,
    Operation, RefusalCode, Refused, Tier, TierStatus,
};
use serde_json::{Map, Value, json};

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// The codex adapter's declared capabilities.
#[must_use]
pub fn capabilities() -> Capabilities {
    let refused = CommandSupport::Refused(RefusalCode::UnsupportedControl);
    let mut commands: BTreeMap<String, CommandSupport> = COMMAND_NAMES
        .iter()
        .map(|name| ((*name).to_string(), refused))
        .collect();
    // Process-level controls the run loop delivers itself, adapter-independent.
    commands.insert("interrupt".to_string(), CommandSupport::Honoured);
    commands.insert("halt".to_string(), CommandSupport::Honoured);
    // Honoured as of CX-M2's live proof (design amendment a7). The three things that had to be
    // true, each read from the run's own record and not from the config that was written:
    //   1. the hook fired — a PreToolUse process published `{"tool_name":"Bash","tool_use_id":
    //      "exec-96257928-…", …}` into this run's own channel;
    //   2. the deny reached the child before the effect — the rollout's own output record for that
    //      call reads `Output:\n` (empty) and `Command blocked by PreToolUse hook: this step
    //      admits no shell, so the command did not run`;
    //   3. the model was told, not walled — its last message was "The command was blocked and did
    //      not run."
    // What is **not** claimed by this row: `allow`. Only the deny path has been driven, and the
    // 0.145.0 binary carries a string suggesting some `permissionDecision` values are refused at
    // `PreToolUse`. An `allow` that the vendor rejects would be a hook response it discards, so the
    // grant half of this wire stays unverified until a run drives it.
    commands.insert("tool.decide".to_string(), CommandSupport::Honoured);

    Capabilities {
        adapter: AdapterId {
            id: ADAPTER_ID.to_string(),
            class: AdapterClass::Harness,
        },
        versions_pinned: PINNED_VERSIONS.iter().map(|v| (*v).to_string()).collect(),
        tiers: BTreeMap::from([
            // Not driven. `codex exec` takes no tool allowlist, and the launch-level narrowing
            // this vendor does have — `sandbox_mode`, `approval_policy` — constrains the
            // *process*, not the offered tool set. Declared rather than claimed.
            (Tier::Registration, TierStatus::Unverified),
            // **Driven** (CX-M2). See the capability note above.
            (Tier::Call, TierStatus::Delivered),
            // Not driven. `thread/inject` is an app-server method (V14) and this adapter drives
            // `codex exec`, which has no channel for text between turns.
            (Tier::Turn, TierStatus::Unverified),
            // Delivered by terminating the child — metaharness's own act on a process it started,
            // not a claim about a vendor surface. Named that way rather than as `turn/interrupt`,
            // which is verified *present* on the app-server surface and undriven here.
            (Tier::Kill, TierStatus::Delivered),
        ]),
        commands,
        // Two delivered and one not, and the split is the same driven/undriven line the tier table
        // draws. `frame` and `ask` reach this wire through a **deny**, which CX-M2 drove and the
        // vendor's own record confirmed. `observe` is the **allow** half and nothing else — the
        // half no run here has driven, and the 0.145.0 binary carries a string suggesting some
        // `permissionDecision` values are refused at `PreToolUse`. An allow the vendor discards is
        // a hook response that decided nothing, which on a capture run would be indistinguishable
        // from a capture that worked. Refused by name at plan time until a run drives it (R2.4).
        decision_modes: BTreeMap::from([
            (
                DecisionMode::Frame.as_str().to_string(),
                TierStatus::Delivered,
            ),
            (
                DecisionMode::Ask.as_str().to_string(),
                TierStatus::Delivered,
            ),
            (
                DecisionMode::Observe.as_str().to_string(),
                TierStatus::Unverified,
            ),
        ]),
        rendering: rendering(),
    }
}

/// The neutral-operation → vendor-tool table, published as a value (design § 8.4 O6).
///
/// **These are the names the `PreToolUse` wire uses, not the names the rollout uses**, and the
/// difference is not cosmetic — it is the table the seam matches a live call against. One driven
/// run settled it: the hook received `"tool_name":"Bash"` for `echo …`, while the same call's
/// record in the rollout is a `custom_tool_call` whose output line carries
/// `call_id: "call_YeJ…"`. The vendor speaks Claude Code's tool vocabulary at the hook and its own
/// at the record, so a table built from the record would have denied every shell call in frame
/// mode and called it a frame decision.
///
/// Codex's surface is narrower than the vocabulary: most operations have no dedicated vendor tool
/// because the model reaches files through the shell. `None` means exactly that, and the adapter
/// never re-decides what an admission implies.
fn rendering() -> BTreeMap<String, Option<String>> {
    let mut table: BTreeMap<String, Option<String>> = Operation::PARAMETERLESS
        .iter()
        .map(|operation| ((operation.name().to_string()), render(operation)))
        .collect();
    table.insert("mcp.call".to_string(), None);
    table
}

fn render(operation: &Operation) -> Option<String> {
    render_operation(operation).map(ToString::to_string)
}

/// The vendor tool one neutral operation renders to, as a borrowed name.
///
/// Two rows and two different strengths of evidence, said out loud because the design forbids
/// levelling them (design § 8.4 O1, O4):
///
/// | operation | vendor tool | evidence |
/// |---|---|---|
/// | `shell` | `Bash` | **driven.** A live 0.145.0 run's hook received `"tool_name":"Bash"` for a shell call — not `exec`, which is what the *rollout* calls the same call, and not `shell`, which is what the binary's model-facing tool list calls it |
/// | `file.write`, `file.edit` | `apply_patch` | **unverified.** The vendor's own hook documentation lists `apply_patch` beside `Bash` as a matcher, and no run here has driven a patch call, so the string is the vendor's and the confirmation is not |
///
/// A patch call whose hook name turns out not to be `apply_patch` would be admitted by no frame
/// and denied by name, which is the fail-closed direction — but it would be denied for the wrong
/// reason, so it is labelled rather than assumed.
#[must_use]
pub fn render_operation(operation: &Operation) -> Option<&'static str> {
    match operation {
        Operation::Shell => Some("Bash"),
        Operation::FileWrite | Operation::FileEdit => Some("apply_patch"),
        _ => None,
    }
}

/// The reason a `deny` carries when the embedder gave none.
///
/// This wire requires a non-empty reason on a deny (§ 2.5), and the model is told this one, so an
/// empty string would be a hook response the harness rejects — and a rejected hook response is a
/// guard that has stopped guarding.
const UNSTATED_DENY_REASON: &str =
    "metaharness denied this call and the embedder stated no reason; the call did not run";

/// Render a decision into the vendor's `PreToolUse` hook response.
///
/// ```text
/// {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"|"deny",…}}
/// ```
///
/// **The same shape as Claude Code's, and that is the finding rather than an assumption**: the
/// research record reads `PreToolUseHookSpecificOutputWire` in the 0.145.0 binary and the vendor's
/// own documentation states `permissionDecision`, `permissionDecisionReason` and `updatedInput`
/// (§ 2.5). What is *not* shared is the code: the renderer lives in this crate because a single
/// function serving two vendors would be one file that must stay true of two binaries nobody
/// synchronises.
///
/// `permissionDecision: "ask"` is **refused by this wire for `PreToolUse`** (§ 2.5) and this
/// adapter never emits it: a headless run has no human at a terminal to return the question to.
#[must_use]
pub fn render_hook_response(decision: &Decision) -> Value {
    let mut output = Map::new();
    output.insert("hookEventName".to_string(), json!("PreToolUse"));
    match decision {
        Decision::Allow => {
            output.insert("permissionDecision".to_string(), json!("allow"));
        }
        Decision::Deny { reason } => {
            let reason = if reason.trim().is_empty() {
                UNSTATED_DENY_REASON
            } else {
                reason.as_str()
            };
            output.insert("permissionDecision".to_string(), json!("deny"));
            output.insert("permissionDecisionReason".to_string(), json!(reason));
        }
        Decision::Replace { input } => {
            output.insert("permissionDecision".to_string(), json!("allow"));
            output.insert("updatedInput".to_string(), input.clone());
        }
        // Nothing at all — not an envelope with the decision left out, and not an `allow`. A
        // caller that gets `Value::Null` writes no bytes to the hook's stdout, so the vendor's own
        // approval policy decides and metaharness has claimed nothing (amendment a3).
        Decision::Abstain => return Value::Null,
    }
    json!({ "hookSpecificOutput": Value::Object(output) })
}

/// What the vendor hands the `PreToolUse` hook on stdin.
///
/// The field set is **the vendor's own schema**, not a guess: 0.145.0 embeds a draft-07 document
/// titled `pre-tool-use.command.input` and it declares ten required properties —
/// `cwd`, `hook_event_name` (const `"PreToolUse"`), `model`, `permission_mode`, `session_id`,
/// `tool_input`, `tool_name`, `tool_use_id`, `transcript_path` (nullable) and `turn_id` — plus
/// optional `agent_id` and `agent_type`, and `additionalProperties: false`.
///
/// Every field is read as an `Option` all the same. A record that stopped carrying one must be
/// visible as a **missing field** rather than as an empty string that silently matches nothing,
/// and "the schema says required" is a claim about the schema, not about the bytes that arrived.
///
/// `tool_input` is **unconstrained** — the schema itself says `"tool_input": true` — because the
/// keys differ per tool and a reader that required `file_path` would find none on `apply_patch`
/// and pass it through: *"a guard that has silently stopped guarding"* (design § 2.5).
#[derive(Debug, Clone, PartialEq)]
pub struct HookInput {
    /// The tool about to run, as the vendor names it.
    ///
    /// **Three vocabularies, and this is the third.** The binary's model-facing tool list says
    /// `shell`; the rollout records the same call as a `custom_tool_call` named `exec`; and the
    /// hook — driven, on 0.145.0 — receives `"tool_name":"Bash"`. Claude Code's word, on a
    /// different vendor's wire. [`crate::render_operation`] renders to *this* one, because this is
    /// the one the seam matches a live call against.
    pub tool_name: Option<String>,
    /// Its input, unconstrained.
    pub tool_input: Value,
    /// **The correlation key: the vendor's own per-call id.**
    ///
    /// `tool_use_id`, and the schema marks it **required** — the same spelling Claude Code 2.1.239
    /// uses (row V22). It is what makes a codex `tool.requested` carry the vendor's id rather than
    /// a name metaharness invented, so the live call and the rollout's own record of it can be
    /// read as one call by anybody downstream.
    ///
    /// It is **not** what a decision is routed by: that is the hook process's own rendezvous name,
    /// which is correct even for a payload that arrives without this field (see
    /// [`crate::hook_program`]).
    pub tool_use_id: Option<String>,
    /// The vendor's session id, which correlates the call to the rollout.
    pub session_id: Option<String>,
    /// The turn this call belongs to. `turn_id` is a Codex extension over the Claude Code payload
    /// and appears on nearly every rollout record, so it is the join between a hook request and
    /// the `turn_context` that says which approval policy was in force. It is **not** a per-call
    /// id: one turn carries many calls.
    pub turn_id: Option<String>,
    /// The working directory the call would run in.
    pub cwd: Option<String>,
    /// The posture the vendor says is in force for this call, in its own vocabulary.
    pub permission_mode: Option<String>,
    /// Which hook event this is. Always `PreToolUse` for the seam this adapter installs; read
    /// rather than assumed, so a config that grew a second hook is visible.
    pub hook_event_name: Option<String>,
}

/// Read the hook's own input.
///
/// # Errors
///
/// [`RefusalCode::Malformed`] when the line is not a JSON object. The installed program fails
/// closed on that refusal: a request metaharness cannot read is one it never answers, and the
/// program's own backstop denies it.
pub fn parse_hook_input(input: &str) -> Result<HookInput, Refused> {
    let value: Value = serde_json::from_str(input).map_err(|error| {
        Refused::new(
            RefusalCode::Malformed,
            format!("the PreToolUse hook input did not parse: {error}"),
        )
    })?;
    let Some(object) = value.as_object() else {
        return Err(Refused::new(
            RefusalCode::Malformed,
            "the PreToolUse hook input was not a JSON object",
        ));
    };
    Ok(HookInput {
        tool_name: string_field(object.get("tool_name")),
        tool_input: object.get("tool_input").cloned().unwrap_or(Value::Null),
        tool_use_id: string_field(object.get("tool_use_id")),
        session_id: string_field(object.get("session_id")),
        turn_id: string_field(object.get("turn_id")),
        cwd: string_field(object.get("cwd")),
        permission_mode: string_field(object.get("permission_mode")),
        hook_event_name: string_field(object.get("hook_event_name")),
    })
}

fn string_field(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToString::to_string)
}
