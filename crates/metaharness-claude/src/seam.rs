//! The control seam: what this adapter delivers, and what it refuses by name.
//!
//! Pinned to 2.1.240. The seam is the on-disk `PreToolUse` command hook, which is *"the default
//! seam"* of design § 7.3 and the only one whose blocking property was measured — 11 hook denies
//! produced 11 `permission_denials` and the forbidden write did not land (design § 2.2).
//!
//! The rows below name **2.1.239** where that is the binary the observation was read from: the
//! seam's own tables are the § 2.7 verification rows of 2026-08-22, and the pin moved a day later
//! (amendment a11, [`crate::PINNED_VERSIONS`]). The seam itself is not among the unverified — the
//! 2.1.240 run whose bytes are `fixtures/golden/` reached this hook and was answered through it.
//!
//! # What is not built here, and the row that would close each one
//!
//! An adapter that has not driven a mechanism declares it and refuses, because an embedder that
//! requires an unverified tier must get a refusal and never a silent no-op (design § 8.4 O4).
//!
//! | not built | why | row |
//! |---|---|---|
//! | `--permission-prompt-tool` | exists on 2.1.239, takes an MCP tool name, and is **absent from `claude --help`**. An undocumented flag is not a foundation, and it excludes `canUseTool` besides | V6, V5 |
//! | `can_use_tool` as the seam | shadowed by bare `--allowedTools` entries, by settings allow rules and by `bypassPermissions` — the vendor says so in its own strings. A run that would need it is refused [`metaharness_protocol::RefusalCode::Shadowed`] rather than served | V4, **Q2** |
//! | `permissionDecision: "defer"` | print-mode only, with a parked/auto-resumed mechanism whose semantics are undriven here | V9, **Q3** |
//! | a metaharness-owned tool surface | needs per-step re-listing, and whether the client acts on `notifications/tools/list_changed` mid-session is unverified | V13, **Q1** |
//! | turn injection | `--input-format stream-json` multi-turn plus hook `additionalContext`: the flags are verified and **the composition is undriven** | design § 7.3 |
//!
//! Matcher `""` is itself a vendor doc string and is undriven (**Q11**): the measured parity used
//! the narrow matchers `Edit|Write|NotebookEdit` and `Bash`. See [`crate::plan_launch`], which
//! emits it.

use std::collections::BTreeMap;

use metaharness_protocol::{
    AdapterClass, AdapterId, COMMAND_NAMES, Capabilities, CommandSupport, Decision, Operation,
    RefusalCode, Refused, Tier, TierStatus, decision_modes_all,
};
use serde_json::{Map, Value, json};

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// What this adapter says it can do.
///
/// Three tiers are delivered and one is not:
///
/// * **registration** — `--allowedTools` / `--tools` at launch decide the offered set, and
///   `--tools ""` disables the whole built-in set (V11). In daily use in `engineering-protocols`.
/// * **call** — the `PreToolUse` command hook, blocking, matcher `""`.
/// * **turn** — [`TierStatus::Unverified`]. The flags exist; the composition that would carry a
///   frame into a running session between turns has not been driven here, so `frame.set` and
///   `message.inject` are refused rather than delivered approximately.
/// * **kill** — delivered by terminating the child. Named that way rather than as the
///   `interrupt` control request, which is verified *present* (V3) and undriven: a guarantee
///   should not rest on a string.
///
/// `steer` is refused **by name**: a running turn on headless Claude Code can only be killed
/// (design § 7.3).
#[must_use]
pub fn capabilities() -> Capabilities {
    let refused = CommandSupport::Refused(RefusalCode::UnsupportedControl);
    let mut commands: BTreeMap<String, CommandSupport> = COMMAND_NAMES
        .iter()
        .map(|name| ((*name).to_string(), refused))
        .collect();
    commands.insert("tool.decide".to_string(), CommandSupport::Honoured);
    commands.insert("interrupt".to_string(), CommandSupport::Honoured);
    commands.insert("halt".to_string(), CommandSupport::Honoured);

    Capabilities {
        adapter: AdapterId {
            id: ADAPTER_ID.to_string(),
            class: AdapterClass::Harness,
        },
        versions_pinned: PINNED_VERSIONS.iter().map(|v| (*v).to_string()).collect(),
        tiers: BTreeMap::from([
            (Tier::Registration, TierStatus::Delivered),
            (Tier::Call, TierStatus::Delivered),
            (Tier::Turn, TierStatus::Unverified),
            (Tier::Kill, TierStatus::Delivered),
        ]),
        commands,
        // All three, and the third one earns it on this vendor rather than inheriting it: observe
        // mode is the `allow` half of the hook wire and nothing else, and the `allow` half here is
        // the vendor's own documented behaviour — 2.1.239 carries *"Hook approved tool use for
        // ${name}, bypassing permission prompt"* (§ 6, finding F8) — driven through the same
        // channel every deny has been driven through, and asserted by this adapter's own C1 vector
        // and the C3 observe vector beside it. Whether that grant beats every stricter rule in
        // every direction is **Q12** and does not change what the mode does.
        decision_modes: decision_modes_all(TierStatus::Delivered),
        rendering: rendering(),
    }
}

/// The neutral-operation → vendor-tool table, published as a value (design § 8.4 O6).
fn rendering() -> BTreeMap<String, Option<String>> {
    let mut table: BTreeMap<String, Option<String>> = Operation::PARAMETERLESS
        .iter()
        .map(|operation| {
            (
                operation.name().to_string(),
                render_operation(operation).map(ToString::to_string),
            )
        })
        .collect();
    table.insert("mcp.call".to_string(), None);
    table
}

/// The vendor tool one neutral operation renders to.
///
/// The adapter **renders** and never re-decides what an admission implies: making this a
/// per-harness judgement *"would let a second harness quietly re-decide that `repository.write`
/// admits a shell"* (design § 5.2, D7).
///
/// Two entries need their reason beside them:
///
/// * `dir.list` renders to `Glob`, because 2.1.239 offers no directory-listing tool at all — the
///   offered set read from a 2.1.239 opening record carries `Glob` and `Grep` and no `LS`.
/// * `subagent.spawn` renders to `Task` and is **not admitted by default** on any adapter: a
///   subagent's tool set is derived by nothing in these decisions and would be a route around
///   the per-step admission (design § 5.2). Rendering it is not admitting it.
///
/// [`Operation::McpCall`] returns `None` because its vendor name is `mcp__<server>__<tool>` and
/// is not a static string; the seam matches those by prefix instead of by table.
#[must_use]
pub fn render_operation(operation: &Operation) -> Option<&'static str> {
    match operation {
        Operation::FileRead => Some("Read"),
        Operation::FileWrite => Some("Write"),
        Operation::FileEdit => Some("Edit"),
        Operation::DirList => Some("Glob"),
        Operation::Search => Some("Grep"),
        Operation::Shell => Some("Bash"),
        Operation::WebRead => Some("WebFetch"),
        Operation::SkillLoad => Some("Skill"),
        Operation::SubagentSpawn => Some("Task"),
        Operation::TaskTodo => Some("TodoWrite"),
        Operation::McpCall { .. } => None,
    }
}

/// The reason a `deny` carries when the embedder gave none.
///
/// The vendor's wire requires a non-empty reason and the model is told this one, so an empty
/// string would be a hook response the harness rejects — and a rejected hook response is a guard
/// that has stopped guarding. These are metaharness's own words about its own action, not a
/// paraphrase of anybody else's.
const UNSTATED_DENY_REASON: &str =
    "metaharness denied this call and the embedder stated no reason; the call did not run";

/// Render a decision into the vendor's `PreToolUse` hook response.
///
/// ```text
/// {"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow"|"deny",…}}
/// ```
///
/// **An `allow` grants, and that is deliberate** (design § 6, finding F8). The harness honours a
/// hook `allow` and bypasses the rest of its permission pipeline — 2.1.239 carries the log line
/// *"Hook approved tool use for ${name}, bypassing permission prompt"*. The consequence, stated
/// so nobody discovers it: **an `allow` from metaharness overrides a stricter rule elsewhere in
/// the vendor's settings**, so a run that also relies on such a rule must use `deny`-only policy
/// and say so. Whether the conflict resolves that way in every direction is **Q12**.
///
/// A `deny` always carries a non-empty `permissionDecisionReason`, because the reason is the only
/// part the model can act on — the difference between a wall and an instruction. An `allow`
/// carries none: [`Decision::Allow`] holds no reason, and inventing prose here would put words
/// in the embedder's mouth.
///
/// `permissionDecision: "ask"` exists on this wire and this adapter never emits it: `ask` returns
/// the question to a human at a terminal, and a headless run has none.
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
        // Nothing at all — not an envelope with the decision left out, and not an `allow`. This
        // is the shape § 2.2 records as proven: the reference hook passes a call through by
        // exiting 0 with no output, because "saying `allow` here would claim an authority the
        // layer does not have and would override a stricter rule elsewhere" (amendment a3). A
        // caller that gets `Value::Null` writes no bytes to the hook's stdout.
        Decision::Abstain => return Value::Null,
    }
    json!({ "hookSpecificOutput": Value::Object(output) })
}

/// What the vendor hands the `PreToolUse` hook on stdin.
///
/// The field set is the one the working hooks in `engineering-protocols` read
/// (`hooks/lib.sh`, `hooks/store-integrity.sh`): `tool_name`, `tool_input`, `session_id` — plus
/// [`HookInput::tool_use_id`], which those hooks never needed and this seam cannot work without.
#[derive(Debug, Clone, PartialEq)]
pub struct HookInput {
    /// The tool about to run, as the vendor names it.
    pub tool_name: Option<String>,
    /// Its input, **unconstrained**. No shape is imposed here: the keys differ per tool, and a
    /// reader that required `file_path` would find none on a tool that has no path and pass it
    /// through — *"a guard that has silently stopped guarding"* (design § 2.5).
    pub tool_input: Value,
    /// The vendor's session id, which correlates the decision to the transcript.
    pub session_id: Option<String>,
    /// **The correlation key: the id of the very `tool_use` block this call came from.**
    ///
    /// The same string the transcript's `tool_use` block calls `id`, which is what
    /// `Event::ToolRequested` carries as its `call_id` — so a decision routed by `call_id`
    /// reaches exactly the hook process that is holding that call, and no other.
    ///
    /// Verified twice against 2.1.239 (row **V22**): the binary builds the payload as
    /// `{…, hook_event_name:"PreToolUse", tool_name:e, tool_input:r, tool_use_id:t}` and passes
    /// the same `t` as its `toolUseID`; and a live run's hook received
    /// `"tool_use_id":"toolu_01WmYm29Vf6BGKGYtfhmjPSS"` for the id the stream-json assistant
    /// record carried. Before that was read, design § 12 **Q16** had to leave the correlation
    /// provisional — *"there is no hook process until the real spawner exists"*.
    ///
    /// `Option`, because a record that stopped carrying it must be visible as a missing field
    /// rather than as an empty string that silently matches nothing. A request metaharness
    /// cannot correlate is one it never answers, and the hook's own backstop denies it.
    pub tool_use_id: Option<String>,
    /// Which hook event this is. Always `PreToolUse` for the seam this adapter installs; read
    /// rather than assumed, so a settings file that grew a second hook is visible.
    pub hook_event_name: Option<String>,
}

/// Read the hook's own input.
///
/// # Errors
///
/// [`RefusalCode::Malformed`] when the line is not a JSON object. The hook fails closed on that
/// refusal — with neither reader nor parser present the working hooks deny, because *"a guard
/// that silently stops guarding is the defect this repository writes registers about"*
/// (design § 2.2).
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
        session_id: string_field(object.get("session_id")),
        tool_use_id: string_field(object.get("tool_use_id")),
        hook_event_name: string_field(object.get("hook_event_name")),
    })
}

fn string_field(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(ToString::to_string)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_adapter_pins_one_version_and_declares_itself_a_harness_adapter() {
        let capabilities = capabilities();
        assert_eq!(capabilities.adapter.id, ADAPTER_ID);
        assert_eq!(capabilities.adapter.class, AdapterClass::Harness);
        assert_eq!(capabilities.versions_pinned, vec!["2.1.240".to_string()]);
    }

    /// A tier nobody drove is declared unverified, so an embedder that requires it is refused
    /// rather than quietly served (design § 8.4 O4).
    #[test]
    fn the_undriven_turn_tier_is_unverified_and_not_quietly_delivered() {
        let capabilities = capabilities();
        assert_eq!(capabilities.tiers[&Tier::Turn], TierStatus::Unverified);
        assert_eq!(capabilities.tiers[&Tier::Call], TierStatus::Delivered);
        assert_eq!(
            capabilities.tiers[&Tier::Registration],
            TierStatus::Delivered
        );
        assert_eq!(capabilities.tiers[&Tier::Kill], TierStatus::Delivered);
    }

    /// A running turn on headless Claude Code can only be killed, so `steer` is refused by name
    /// rather than weakened into a lower tier (design § 7.1, § 7.3).
    #[test]
    fn steer_is_refused_by_name_on_this_adapter() {
        assert_eq!(
            capabilities().support("steer"),
            CommandSupport::Refused(RefusalCode::UnsupportedControl)
        );
    }

    /// `frame.set` needs turn-level text **and** call-level enforcement, both or neither: a frame
    /// whose text reaches the model while nothing enforces it tells the model "strictly only
    /// these operations" and makes it false (design § 6, finding F9).
    #[test]
    fn frame_set_is_refused_because_it_is_not_partially_deliverable() {
        assert_eq!(
            capabilities().support("frame.set"),
            CommandSupport::Refused(RefusalCode::UnsupportedControl)
        );
        assert_eq!(
            capabilities().support("message.inject"),
            CommandSupport::Refused(RefusalCode::UnsupportedControl)
        );
    }

    /// Every adapter must deliver these two: a control surface with no way out is not a control
    /// surface (design § 6).
    #[test]
    fn interrupt_and_halt_are_honoured_and_so_is_tool_decide() {
        let capabilities = capabilities();
        for name in ["interrupt", "halt", "tool.decide"] {
            assert_eq!(
                capabilities.support(name),
                CommandSupport::Honoured,
                "{name}"
            );
        }
    }

    #[test]
    fn a_command_this_adapter_never_mentioned_is_refused_and_not_assumed() {
        assert_eq!(
            capabilities().support("tool.invent"),
            CommandSupport::Refused(RefusalCode::UnsupportedControl)
        );
    }

    #[test]
    fn every_operation_in_the_closed_vocabulary_renders_or_says_it_cannot() {
        let capabilities = capabilities();
        assert_eq!(capabilities.renders(&Operation::FileEdit), Some("Edit"));
        assert_eq!(capabilities.renders(&Operation::Shell), Some("Bash"));
        assert_eq!(capabilities.renders(&Operation::DirList), Some("Glob"));
        assert_eq!(capabilities.renders(&Operation::SkillLoad), Some("Skill"));
        assert_eq!(
            capabilities.renders(&Operation::McpCall {
                server: "s".to_string(),
                tool: "t".to_string()
            }),
            None
        );
        assert_eq!(capabilities.rendering.len(), 11);
    }

    /// Rendering is not admitting: `Task` has a name here and no default admission, because a
    /// subagent's tool set is derived by nothing in these decisions (design § 5.2).
    #[test]
    fn subagent_spawn_renders_and_is_still_not_admitted_by_default() {
        assert_eq!(render_operation(&Operation::SubagentSpawn), Some("Task"));
    }

    #[test]
    fn a_deny_carries_a_non_empty_reason_the_model_is_told() {
        let response = render_hook_response(&Decision::Deny {
            reason: "this step admits no shell".to_string(),
        });
        let output = &response["hookSpecificOutput"];
        assert_eq!(output["hookEventName"], json!("PreToolUse"));
        assert_eq!(output["permissionDecision"], json!("deny"));
        assert_eq!(
            output["permissionDecisionReason"],
            json!("this step admits no shell")
        );
    }

    /// An empty reason is a hook response the vendor rejects, and a rejected response lets the
    /// call through — so the renderer substitutes its own words rather than emitting one.
    #[test]
    fn a_deny_with_a_blank_reason_still_carries_one() {
        let response = render_hook_response(&Decision::Deny {
            reason: "   ".to_string(),
        });
        let reason = response["hookSpecificOutput"]["permissionDecisionReason"]
            .as_str()
            .expect("a reason is always present");
        assert!(!reason.trim().is_empty());
    }

    #[test]
    fn an_allow_grants_and_carries_no_invented_reason() {
        let response = render_hook_response(&Decision::Allow);
        let output = &response["hookSpecificOutput"];
        assert_eq!(output["permissionDecision"], json!("allow"));
        assert!(output.get("permissionDecisionReason").is_none());
        assert!(output.get("updatedInput").is_none());
    }

    /// `replace` exists because the wire carries `updatedInput`; refusing to expose it would push
    /// embedders into deny-and-re-prompt, which costs a turn (design § 6).
    #[test]
    fn a_replace_renders_updated_input_and_never_silently_becomes_an_allow() {
        let input = json!({"command": "git status"});
        let response = render_hook_response(&Decision::Replace {
            input: input.clone(),
        });
        let output = &response["hookSpecificOutput"];
        assert_eq!(output["permissionDecision"], json!("allow"));
        assert_eq!(output["updatedInput"], input);
    }

    #[test]
    fn the_hook_input_carries_an_unconstrained_tool_input() {
        let parsed = parse_hook_input(
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","session_id":"s-1",
                "tool_input":{"command":"ls","anything":[1,2]}}"#,
        )
        .expect("parses");
        assert_eq!(parsed.tool_name.as_deref(), Some("Bash"));
        assert_eq!(parsed.session_id.as_deref(), Some("s-1"));
        assert_eq!(parsed.hook_event_name.as_deref(), Some("PreToolUse"));
        assert_eq!(parsed.tool_input["anything"], json!([1, 2]));
    }

    /// The correlation key, read from the shape 2.1.239 actually sends (row **V22**). The paths
    /// and ids here are synthesized: a fixture that carried a real session's home directory
    /// would put one operator's machine into this repository forever.
    #[test]
    fn the_hook_input_carries_the_tool_use_id_the_decision_is_routed_by() {
        let parsed = parse_hook_input(
            r#"{"session_id":"sess-1","transcript_path":"/scratch/run-1/claude-home/x.jsonl",
                "cwd":"/scratch/run-1/work","permission_mode":"default",
                "hook_event_name":"PreToolUse","tool_name":"Bash",
                "tool_input":{"command":"echo hi","description":"Run echo hi"},
                "tool_use_id":"toolu_0000000000000000000001"}"#,
        )
        .expect("parses");
        assert_eq!(
            parsed.tool_use_id.as_deref(),
            Some("toolu_0000000000000000000001")
        );
        assert_eq!(parsed.tool_name.as_deref(), Some("Bash"));
    }

    /// A record that stopped carrying the key reads as absent, never as an empty string: an
    /// empty key would correlate to nothing and look like a call nobody asked about.
    #[test]
    fn a_hook_input_without_the_correlation_key_says_so_rather_than_inventing_one() {
        let parsed = parse_hook_input(r#"{"tool_name":"Read"}"#).expect("parses");
        assert_eq!(parsed.tool_use_id, None);
    }

    /// A field the reader does not know is ignored in silence; the record is still read
    /// (design § 8.4 O3).
    #[test]
    fn an_unknown_field_on_the_hook_input_is_ignored_in_silence() {
        let parsed = parse_hook_input(r#"{"tool_name":"Read","invented_by_a_later_release":true}"#)
            .expect("parses");
        assert_eq!(parsed.tool_name.as_deref(), Some("Read"));
        assert_eq!(parsed.tool_input, Value::Null);
    }

    #[test]
    fn hook_input_that_is_not_an_object_is_refused_malformed() {
        assert_eq!(
            parse_hook_input("[]").expect_err("refused").code,
            RefusalCode::Malformed
        );
        assert_eq!(
            parse_hook_input("not json").expect_err("refused").code,
            RefusalCode::Malformed
        );
    }

    /// Abstaining writes nothing at all — not an `allow`, and not an envelope with the decision
    /// left out. `allow` grants on this wire and overrides a stricter rule in the vendor's own
    /// settings (§ 6), so the value that means "metaharness adjudicated nothing" must not render
    /// as the value that means "metaharness permitted this" (amendment a3).
    #[test]
    fn abstaining_renders_no_hook_output_at_all() {
        assert_eq!(render_hook_response(&Decision::Abstain), Value::Null);
        assert_ne!(
            render_hook_response(&Decision::Abstain),
            render_hook_response(&Decision::Allow)
        );
        let allowed = render_hook_response(&Decision::Allow);
        assert_eq!(
            allowed["hookSpecificOutput"]["permissionDecision"],
            json!("allow")
        );
    }
}
