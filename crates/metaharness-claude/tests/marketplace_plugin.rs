//! `--plugin <marketplace-repo>@<name>@<pin>` — the launch shape, before anything is spawned.
//!
//! Decided by `docs/design/runs-side-by-side-v0.1.md` § 3 (G1–G4) and amendment **a16**;
//! established by `docs/research/2026-09-03-claude-plugin-headless-install.md`. Nothing here
//! spawns, fetches or spends: every assertion is over a value the plan carries.

use std::path::PathBuf;

use metaharness_claude::{
    KNOWN_MARKETPLACES, MarketplaceRefusal, PLUGIN_REGISTRY_HOME, plan_launch, resolve_marketplace,
    scratch_registry,
};
use metaharness_protocol::{Digest, HermeticMode, Kind, MarketplacePlugin, PluginContent, RunSpec};

mod support;
use support::{context, plugin_tree};

fn asked_for(spelling: &str) -> MarketplacePlugin {
    spelling.parse().expect("the spelling parses")
}

// --- G1: the spelling, and the refusal ---------------------------------------------------------

#[test]
fn a_three_segment_spelling_parses_into_repo_name_and_pin() {
    let plugin = asked_for("bdfinst/agentic-dev-team@dev-team@1.4.0");
    assert_eq!(plugin.repo, "bdfinst/agentic-dev-team");
    assert_eq!(plugin.name, "dev-team");
    assert_eq!(plugin.pin, "1.4.0");
    assert_eq!(
        plugin.to_string(),
        "bdfinst/agentic-dev-team@dev-team@1.4.0"
    );
}

/// A commit is a pin too, and the parser does not care which it got.
#[test]
fn a_commit_is_a_pin() {
    let plugin =
        asked_for("beyond10x/agentplugins@aep-planning@21147b7667dfaefcfa45a094e9542891b1783541");
    assert_eq!(plugin.pin, "21147b7667dfaefcfa45a094e9542891b1783541");
}

/// G1 — an unpinned plugin is refused **by name**, and the refusal says what a pin looks like.
#[test]
fn an_unpinned_plugin_is_refused_and_the_refusal_says_what_a_pin_is() {
    let error = "bdfinst/agentic-dev-team@dev-team"
        .parse::<MarketplacePlugin>()
        .expect_err("an unpinned spelling is refused");
    let said = error.to_string();
    assert!(said.contains("pin"), "{said}");
    assert!(
        said.contains("<repo>@<name>@<version-or-commit>"),
        "the refusal shows the shape: {said}"
    );
}

#[test]
fn an_empty_segment_is_refused_rather_than_read_as_a_pin() {
    for spelling in ["@dev-team@1.0.0", "repo@@1.0.0", "repo@dev-team@"] {
        assert!(
            spelling.parse::<MarketplacePlugin>().is_err(),
            "{spelling} names nothing"
        );
    }
}

// --- G2: resolution is local ---------------------------------------------------------------------

fn known() -> serde_json::Value {
    serde_json::json!({
        "beyond10x": {
            "source": {"source": "github", "repo": "beyond10x/agentplugins"},
            "installLocation": "/operator/.claude/plugins/marketplaces/beyond10x",
            "lastUpdated": "2026-09-02T21:13:36.335Z"
        }
    })
}

fn installed() -> serde_json::Value {
    serde_json::json!({
        "version": 2,
        "plugins": {
            "aep-planning@beyond10x": [{
                "scope": "user",
                "installPath": "/operator/.claude/plugins/cache/beyond10x/aep-planning/0.4.0",
                "version": "0.4.0",
                "gitCommitSha": "21147b7667dfaefcfa45a094e9542891b1783541"
            }]
        }
    })
}

#[test]
fn a_repo_resolves_through_the_operators_own_marketplace_registry() {
    let found = resolve_marketplace(
        &asked_for("beyond10x/agentplugins@aep-planning@0.4.0"),
        &known(),
        &installed(),
    )
    .expect("it is fetched and installed");
    assert_eq!(found.marketplace, "beyond10x");
    assert_eq!(
        found.install_path,
        "/operator/.claude/plugins/cache/beyond10x/aep-planning/0.4.0"
    );
    assert_eq!(found.version, "0.4.0");
}

/// The commit is the other spelling of the same pin, and both find the same entry.
#[test]
fn a_commit_pin_resolves_to_the_same_entry_as_its_version() {
    let by_commit = resolve_marketplace(
        &asked_for("beyond10x/agentplugins@aep-planning@21147b7667dfaefcfa45a094e9542891b1783541"),
        &known(),
        &installed(),
    )
    .expect("resolves");
    assert_eq!(by_commit.version, "0.4.0");
}

/// Every failure names the two commands the operator runs **once, outside a run** — which is
/// where the network reach belongs.
#[test]
fn an_unfetched_marketplace_is_refused_and_names_the_command_that_fetches_it() {
    let refusal = resolve_marketplace(
        &asked_for("bdfinst/agentic-dev-team@dev-team@1.4.0"),
        &known(),
        &installed(),
    )
    .expect_err("nothing fetched it");
    assert!(matches!(
        refusal,
        MarketplaceRefusal::MarketplaceNotFetched { .. }
    ));
    let said = refusal.to_string();
    assert!(said.contains("claude plugin marketplace add"), "{said}");
}

#[test]
fn a_pin_nobody_installed_is_refused_and_lists_what_is_there() {
    let refusal = resolve_marketplace(
        &asked_for("beyond10x/agentplugins@aep-planning@9.9.9"),
        &known(),
        &installed(),
    )
    .expect_err("that pin is not installed");
    let said = refusal.to_string();
    assert!(matches!(
        refusal,
        MarketplaceRefusal::PinNotInstalled { .. }
    ));
    assert!(said.contains("0.4.0"), "it says what is available: {said}");
}

// --- G3: placement, in the layout read from a real config home ------------------------------------

#[test]
fn a_declared_marketplace_plugin_is_placed_inside_the_scratch_config_home() {
    let (spec, context, digest) = marketplace_world();
    let plan = plan_launch(&spec, &context).expect("the run plans");

    assert_eq!(plan.marketplace_installs.len(), 1);
    let install = &plan.marketplace_installs[0];
    assert_eq!(
        install.to,
        PathBuf::from("/scratch/run-1/claude-home/plugins/cache/beyond10x/aep-planning/0.4.0")
    );
    assert!(
        install.to.starts_with(&plan.config_home),
        "the marketplace layout lives in the config home, unlike --plugin-dir's copy"
    );
    assert_eq!(install.digest, digest);
}

/// G3 — the two registry documents are on the plan as values, so a test reads them before any
/// process exists, and nothing in them names a path outside the scratch home.
#[test]
fn the_scratch_home_carries_the_two_registry_documents_and_nothing_points_outside_it() {
    let (spec, context, _) = marketplace_world();
    let plan = plan_launch(&spec, &context).expect("plans");

    let paths: Vec<String> = plan
        .scratch_files
        .iter()
        .map(|file| file.path.display().to_string())
        .collect();
    assert!(
        paths.iter().any(|path| path.ends_with(KNOWN_MARKETPLACES)),
        "{paths:?}"
    );
    assert!(
        paths
            .iter()
            .any(|path| path.ends_with("plugins/installed_plugins.json")),
        "{paths:?}"
    );

    let rendered = serde_json::to_string(
        &plan
            .scratch_files
            .iter()
            .map(|file| file.document.clone())
            .collect::<Vec<_>>(),
    )
    .expect("renders");
    assert!(
        !rendered.contains("/operator/"),
        "no assembled document may name the operator's own tree: {rendered}"
    );
    assert!(rendered.contains("/scratch/run-1/claude-home/plugins/cache"));
}

/// **`--plugin-dir` is not also passed.** Two mechanisms loading one plugin would report it twice.
#[test]
fn a_marketplace_plugin_is_not_also_named_by_plugin_dir_in_the_argv() {
    let (spec, context, _) = marketplace_world();
    let plan = plan_launch(&spec, &context).expect("plans");
    assert!(!plan.args.iter().any(|argument| argument == "--plugin-dir"));
}

// --- G4: what the attestation says ----------------------------------------------------------------

#[test]
fn the_attestation_lists_the_plugin_with_its_pin_and_says_the_claim_is_not_driven() {
    let (spec, context, digest) = marketplace_world();
    let plan = plan_launch(&spec, &context).expect("plans");

    let attested = &plan.attestation.installed_plugins;
    assert_eq!(attested.len(), 1);
    assert_eq!(attested[0].name, "aep-planning");
    assert_eq!(attested[0].digest, digest);
    assert!(attested[0].source.contains("beyond10x/agentplugins"));
    assert!(
        attested[0].source.contains("0.4.0"),
        "the pin is in the record"
    );
    assert!(
        attested[0].loaded_by.contains("not driven"),
        "a vendor surface nobody has driven is documented as undriven: {}",
        attested[0].loaded_by
    );
    assert!(attested[0].loaded_by.contains(PLUGIN_REGISTRY_HOME));
}

/// A run with no plugin at all still states an empty list rather than dropping the key.
#[test]
fn a_run_with_no_plugin_attests_an_empty_list() {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.prompt = Some("work".to_string());
    let plan = plan_launch(&spec, &context()).expect("plans");
    assert!(plan.attestation.installed_plugins.is_empty());
    assert!(plan.marketplace_installs.is_empty());
    let json = serde_json::to_string(&plan.attestation).expect("renders");
    assert!(json.contains(r#""installed_plugins":[]"#), "{json}");
}

/// The scratch registry is a value, and it is the same value twice.
#[test]
fn the_scratch_registry_is_deterministic() {
    let (spec, context, _) = marketplace_world();
    let first = plan_launch(&spec, &context).expect("plans");
    let second = plan_launch(&spec, &context).expect("plans");
    assert_eq!(first.scratch_files, second.scratch_files);
    let _ = scratch_registry(&[]);
}

/// A spec and a context that agree on one resolved marketplace plugin, as the builder produces
/// them.
fn marketplace_world() -> (RunSpec, metaharness_claude::LaunchContext, Digest) {
    let (_source, tree, digest) = plugin_tree("aep-planning");
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::On;
    spec.prompt = Some("do the work".to_string());
    spec.plugin
        .push(asked_for("beyond10x/agentplugins@aep-planning@0.4.0"));

    let mut context = context();
    context
        .marketplace_plugins
        .push(metaharness_protocol::ResolvedMarketplacePlugin {
            requested: asked_for("beyond10x/agentplugins@aep-planning@0.4.0"),
            marketplace: "beyond10x".to_string(),
            version: "0.4.0".to_string(),
            commit: Some("21147b7667dfaefcfa45a094e9542891b1783541".to_string()),
            tree: metaharness_protocol::PluginTree {
                source: PathBuf::from(
                    "/operator/.claude/plugins/cache/beyond10x/aep-planning/0.4.0",
                ),
                content: match &tree.content {
                    PluginContent::Files { count, digest } => PluginContent::Files {
                        count: *count,
                        digest: digest.clone(),
                    },
                    other => other.clone(),
                },
            },
        });
    (spec, context, digest)
}
