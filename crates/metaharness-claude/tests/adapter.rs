//! The adapter as an embedder sees it: only the public surface, from outside the crate.
//!
//! These are the assertions that would catch a rename or a retype before the crate that depends
//! on this one does, which is the whole reason the surface is small and named in one place.

use std::collections::BTreeMap;
use std::path::PathBuf;

use metaharness_claude::{
    ADAPTER_ID, BUILTIN_PLUGINS, CONTEXT_CHANNEL, HOOK_TIMEOUT_SECONDS, LaunchContext,
    LaunchRefusal, MEMORY_CANARIES, MEMORY_SHAPES, MENTION_CANARIES, MemoryCanary, Mention,
    PINNED_VERSIONS, TranscriptReader, builtin_plugin_source, builtin_plugins, capabilities,
    conformance_vectors, memory_canary, mention_canary, parse_hook_input, plan_launch,
    render_hook_response, render_operation,
};
use metaharness_protocol::{
    ConformanceTier, CredentialSource, Decision, DecisionCensus, DecisionMode, Digest, Event,
    HermeticMode, Kind, Operation, RefusalCode, RunSpec, Seam, ToolSurface, TranscriptRef,
};

fn context() -> LaunchContext {
    LaunchContext {
        context_files: Vec::new(),
        catalogue: None,
        scratch_root: PathBuf::from("/scratch/run-9"),
        cwd: PathBuf::from("/scratch/run-9/work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([("HOME".to_string(), "/operator".to_string())]),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"inputs")),
        plugins: Vec::new(),
        marketplace_plugins: Vec::new(),
        loopback: None,
        tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
        managed_settings: Vec::new(),
    }
}

fn spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("go".to_string());
    spec
}

#[test]
fn the_adapter_names_itself_and_its_pin() {
    // **The one deliberate literal.** Every other assertion about the pin reads `PINNED_VERSIONS`,
    // so a move shows up here — in a diff a reviewer reads — and nowhere else. Moved
    // 2.1.240 → 2.1.241 on 2026-08-24, 2.1.241 → 2.1.259 on 2026-09-03 and 2.1.259 → 2.1.304
    // and 2.1.305 on 2026-10-21; see `PINNED_VERSIONS`' own note for what was re-read on each new
    // binary and what was carried over unverified.
    assert_eq!(ADAPTER_ID, "claude");
    assert_eq!(PINNED_VERSIONS, ["2.1.304", "2.1.305"]);
    assert_eq!(
        capabilities().versions_pinned,
        PINNED_VERSIONS.map(ToString::to_string).to_vec(),
        "what the adapter publishes and what it pins cannot disagree"
    );
}

/// Every pinned version declares the plugins its binary compiles in, and no list outlives its pin
/// (design amendment a33). A pin added without re-reading its binary fails the first assertion.
#[test]
fn every_pinned_version_declares_the_plugins_its_binary_compiles_in() {
    let listed: Vec<&str> = BUILTIN_PLUGINS
        .iter()
        .map(|(version, _)| *version)
        .collect();
    assert_eq!(
        listed, PINNED_VERSIONS,
        "one built-in list per pinned version, in the pin's order"
    );
    assert_eq!(
        builtin_plugins("2.1.303"),
        None,
        "an unpinned binary has no list"
    );
}

/// Correction round 1, condition 3: every pinned version records its `@`-mention canary, measured
/// by the native fixture's `native_at_mentions_expand_in_the_user_turn_and_not_in_the_system_prompt`
/// on that binary, and no row outlives its pin. A pin without a row fails here, the way a pin
/// without its built-in list does; a row copied from the previous pin does not, and only the native
/// canary (ignored in `task check`) measures the new binary. The launch's channel rests on
/// the row: the context block goes to `--append-system-prompt` because no pinned binary expands a
/// mention there, and every one expands it in every user-turn route measured.
#[test]
fn every_pinned_version_records_its_mention_canary_and_the_context_channel_rests_on_it() {
    let listed: Vec<&str> = MENTION_CANARIES.iter().map(|row| row.version).collect();
    assert_eq!(
        listed, PINNED_VERSIONS,
        "one canary row per pinned version, in the pin's order"
    );
    assert_eq!(
        mention_canary("2.1.303"),
        None,
        "an unpinned binary has no row"
    );
    for row in MENTION_CANARIES {
        assert_eq!(
            row.append_system_prompt,
            Mention::Text,
            "{}: the context block may only go where mentions stay text",
            row.version
        );
        assert_eq!(
            [row.prompt_argument, row.stdin_text, row.stdin_stream_json],
            [Mention::Expanded; 3],
            "{}: measured on 2026-10-21, every user-turn route expands; a row that says otherwise \
             is a new measurement and moves the task-text story",
            row.version
        );
    }
    assert_eq!(
        CONTEXT_CHANNEL,
        metaharness_protocol::ContextChannel::SystemPrompt
    );
}

/// The memory canary H11 rests on (design amendment a36), held to the pin the way the built-in
/// list and the mention canary are: a version cannot be pinned without its row, so a pin move that
/// did not re-run the canary probe fails here before it lands. A row records what the probe
/// measured, and a row in which a canary reached a request is recordable: decision C turns H11 to
/// `gap` for it, which is the floor's job and not this test's (coordinator decision D3). So this
/// asks for a row per pin and a probe that could see every shape, never for zero leaks. The
/// research page is where the numbers come from
/// (`docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md`, § *H11: memory files*).
/// Like the other two, this cannot tell a measured row from a copied one; the native fixture's
/// canary case is the measurement.
#[test]
fn every_pinned_version_records_its_memory_canary() {
    let listed: Vec<&str> = MEMORY_CANARIES.iter().map(|row| row.version).collect();
    assert_eq!(
        listed, PINNED_VERSIONS,
        "one memory canary row per pinned version, in the pin's order"
    );
    assert_eq!(
        memory_canary("2.1.303"),
        None,
        "an unpinned binary has no row"
    );
    for row in MEMORY_CANARIES {
        assert_eq!(
            row.planted,
            2 * MEMORY_SHAPES.len(),
            "{}: each shape in the cwd and in its parent",
            row.version
        );
        assert_eq!(
            row.control_sent, row.planted,
            "{}: the positive control saw every canary",
            row.version
        );
        assert!(
            row.launch_none <= row.planted && row.launch_loopback <= row.planted,
            "{}: no more canaries reached a request than were planted",
            row.version
        );
    }
    assert_eq!(
        MEMORY_SHAPES,
        [
            "CLAUDE.md",
            "CLAUDE.local.md",
            "AGENTS.md",
            ".claude/CLAUDE.md",
            ".claude/AGENTS.md",
            ".claude/rules",
        ],
        "the binary's own list, in its order"
    );
}

/// A row in which a canary reached the model, or in which the control missed one, keeps nothing
/// out: the first is the leak, the second a probe that could not have seen it.
#[test]
fn a_canary_that_reached_a_request_or_a_blind_control_is_not_kept_out() {
    let measured = MEMORY_CANARIES[0];
    let leaked = MemoryCanary {
        launch_loopback: 1,
        ..measured
    };
    assert!(!leaked.kept_out());
    let blind = MemoryCanary {
        control_sent: measured.planted - 1,
        ..measured
    };
    assert!(!blind.kept_out());
}

/// Each list, written out: a fact about one binary, read from its registration function
/// (`docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md`), in the order the binary
/// registers them, with the one source its opening record gives a built-in.
#[test]
fn each_pinned_binarys_compiled_in_plugins_are_declared_by_name_with_their_builtin_source() {
    let read_from_2_1_304 = [
        "cc-plugin-sec-default",
        "cc-plugin-agents-md",
        "cc-plugin-telemetry",
        "cc-plugin-plugin-authoring",
        "cc-plugin-mods-guide",
        "cc-plugin-tips",
        "cc-plugin-mermaid",
        "cc-plugin-responsive-mode",
        "cc-plugin-diff",
        "cc-plugin-you-should-know",
        "cc-plugin-easel",
        "cc-plugin-claude-test",
    ];
    let read_from_2_1_305 = read_from_2_1_304;
    assert_eq!(builtin_plugins("2.1.304"), Some(&read_from_2_1_304[..]));
    assert_eq!(builtin_plugins("2.1.305"), Some(&read_from_2_1_305[..]));
    assert_eq!(
        builtin_plugin_source("cc-plugin-telemetry"),
        "cc-plugin-telemetry@builtin"
    );
}

/// The launch disables the one built-in it can, and only where nothing of the operator's travels.
///
/// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` keeps `cc-plugin-telemetry` out of a 2.1.304 session
/// (read with `claude plugin details`, and in the native fixture's opening record). Under
/// `credentials: none` the child talks to a declared endpoint or to nobody, so first-party
/// analytics would be the one request that bypasses it; under an operator login or an API key the
/// switch also turns off server-side behaviour no credential-free probe can observe, so it is not
/// set there and the built-in is admitted by name instead.
#[test]
fn the_traffic_switch_is_set_in_every_credential_free_launch_and_in_no_credentialed_one() {
    let switch = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";
    let mut free = spec();
    free.credentials = CredentialSource::None;
    free.model_endpoint = Some("http://127.0.0.1:9".to_string());
    let plan = plan_launch(&free, &context()).expect("an endpoint run plans");
    assert_eq!(plan.env.get(switch).map(String::as_str), Some("1"));

    let mut nowhere = spec();
    nowhere.credentials = CredentialSource::None;
    let plan = plan_launch(&nowhere, &context()).expect("a credential-free run plans");
    assert_eq!(plan.env.get(switch).map(String::as_str), Some("1"));

    let plan = plan_launch(&spec(), &context()).expect("an operator-login run plans");
    assert_eq!(plan.env.get(switch), None);

    let mut keyed = spec();
    keyed.credentials = CredentialSource::ApiKey;
    let mut keyed_context = context();
    keyed_context.inherited_env.insert(
        "ANTHROPIC_API_KEY".to_string(),
        "sk-not-a-real-key".to_string(),
    );
    let plan = plan_launch(&keyed, &keyed_context).expect("an api-key run plans");
    assert_eq!(plan.env.get(switch), None);
}

#[test]
fn the_operation_rendering_is_a_value_an_embedder_can_assert_on_without_a_run() {
    assert_eq!(render_operation(&Operation::FileWrite), Some("Write"));
    assert_eq!(
        render_operation(&Operation::McpCall {
            server: "s".to_string(),
            tool: "t".to_string()
        }),
        None
    );
    assert_eq!(capabilities().rendering["shell"], Some("Bash".to_string()));
}

#[test]
fn a_hook_response_is_the_vendors_own_shape() {
    let response = render_hook_response(&Decision::Deny {
        reason: "this step admits no shell".to_string(),
    });
    assert_eq!(
        response["hookSpecificOutput"]["hookEventName"],
        serde_json::json!("PreToolUse")
    );
    assert_eq!(
        response["hookSpecificOutput"]["permissionDecision"],
        serde_json::json!("deny")
    );
}

#[test]
fn the_hook_input_parses_and_a_malformed_one_is_refused_by_name() {
    let parsed = parse_hook_input(r#"{"tool_name":"Bash","tool_input":{"command":"ls"}}"#)
        .expect("the hook input parses");
    assert_eq!(parsed.tool_name.as_deref(), Some("Bash"));
    assert_eq!(
        parse_hook_input("{").expect_err("refused").code,
        RefusalCode::Malformed
    );
}

/// The plan is a value: the argv, the environment, the copy list, the settings and the hook are
/// all readable before any process exists (design § 8.4 O7).
#[test]
fn a_plan_is_readable_in_full_before_anything_is_spawned() {
    let plan = plan_launch(&spec(), &context()).expect("the run plans");
    assert_eq!(plan.program, ADAPTER_ID);
    assert!(plan.args.contains(&"--strict-mcp-config".to_string()));
    assert_eq!(
        plan.config_home,
        PathBuf::from("/scratch/run-9/claude-home")
    );
    assert_eq!(plan.credential_copies.len(), 1);
    assert_eq!(
        plan.hook["hooks"][0]["timeout"],
        serde_json::json!(HOOK_TIMEOUT_SECONDS)
    );
    assert!(plan.hook["hooks"][0].get("async").is_none());
    assert_eq!(plan.attestation.mode, HermeticMode::Strict);
}

#[test]
fn a_shadowed_run_is_refused_with_the_protocols_own_code() {
    let mut spec = spec();
    spec.decisions = DecisionMode::Ask;
    spec.tool_surface = ToolSurface::Owned;
    let refusal = plan_launch(&spec, &context()).expect_err("refused");
    assert_eq!(refusal.code(), Some(RefusalCode::Shadowed));
    assert!(matches!(refusal, LaunchRefusal::Shadowed { .. }));
    // It is an error, so an embedder can put it anywhere an error goes.
    let _: &dyn std::error::Error = &refusal;
}

/// The attestation metaharness built at launch is the one the opening event carries, so a reader
/// sees the intent beside the vendor's outcome (design § 8.3).
///
/// Driven with the two fields amendment a10 added set to **non-default values**, because a
/// whole-struct comparison over a default-valued attestation would pass even if the mode and the
/// installed plugins were dropped on the way to the record — and those two are exactly what an
/// eval's arm column is read from.
#[test]
fn the_plans_attestation_reaches_the_opening_event() {
    let mut spec = spec();
    spec.decisions = DecisionMode::Observe;
    spec.plugin_dir
        .push(PathBuf::from("/operator/integrations/claude-code"));
    let mut context = context();
    context.plugins.push(metaharness_protocol::PluginTree {
        source: PathBuf::from("/operator/integrations/claude-code"),
        content: metaharness_protocol::PluginContent::Files {
            count: 1,
            digest: Digest::of(b"one file"),
        },
    });
    let plan = plan_launch(&spec, &context).expect("the run plans");
    assert_eq!(plan.attestation.decisions, DecisionMode::Observe);
    assert_eq!(plan.attestation.installed_plugins.len(), 1);
    let mut reader = TranscriptReader::new(
        TranscriptRef {
            path: Some("/scratch/run-9/transcript.jsonl".to_string()),
            digest: Some(Digest::of(b"bytes")),
            bytes: Some(5),
        },
        plan.attestation.clone(),
    )
    .with_seam(Seam::Hook);
    reader.set_census(DecisionCensus::default());
    let events = reader.push_line(
        r#"{"type":"system","subtype":"init","claude_code_version":"2.1.240","tools":[],
            "mcp_servers":[]}"#,
    );
    let Event::SessionStarted { hermetic, .. } = &events[0].event else {
        panic!("expected session.started");
    };
    assert_eq!(*hermetic, plan.attestation);
    assert_eq!(reader.finish(), [] as [metaharness_protocol::Emission; 0]);
}

#[test]
fn every_conformance_vector_passes_without_a_model_a_network_or_a_credential() {
    let outcomes = conformance_vectors();
    // Six recorded launch expectations (the sixth is a35's `--context` block, in
    // `--append-system-prompt`), the four computed launch vectors that carry observe mode, the
    // declared scratch copy (a27), plugin injection (a10, crossing #4) and the pinned marketplace
    // placement (a16), three synthesised replays, the two golden recorded-wire vectors (CT-2), the
    // version pair (CT-3) and the caller's catalogue (a40).
    assert_eq!(outcomes.len(), 17);
    for outcome in &outcomes {
        assert!(outcome.passed, "{}: {}", outcome.id, outcome.detail);
        assert!(matches!(
            outcome.tier,
            ConformanceTier::C1 | ConformanceTier::C2
        ));
    }
}
