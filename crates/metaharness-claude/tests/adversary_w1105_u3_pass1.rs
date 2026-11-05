//! Adversary pass 1 for `story:claude-attestation-names-actor` (design amendment a74 point 4).
//!
//! The acceptance: a Claude plan with `actor: agent:x` carries an H3 imposed control that names
//! `AEP_ACTOR`, and a plan without an actor does not. The unit derives H3 from the plan's own
//! environment, by name and never by value, and keeps the old sentence when nothing is added.
//! These cases plan launches as pure values: no vendor process, no credential, no network.

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use metaharness_claude::{LaunchContext, LaunchPlan, LoopbackParams, plan_launch};
use metaharness_protocol::{
    CredentialSource, Digest, HermeticMode, HermeticRow, Kind, PromptCacheTtl, RunSpec,
};

/// The keys the child copies from the caller, as the launch's allowlist names them.
const INHERITED: [&str; 7] = ["HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TERM", "TZ"];

/// The four H3 has always named.
const ALWAYS: [&str; 4] = [
    "PATH",
    "TMPDIR",
    "CLAUDE_CONFIG_DIR",
    "CLAUDE_CODE_DISABLE_AUTO_MEMORY",
];

/// The H3 sentence before the unit, written out as a literal rather than read from the code.
const OLD_H3: &str = "the child environment is constructed from an allowlist of 7 keys plus PATH, \
                      TMPDIR, CLAUDE_CONFIG_DIR and CLAUDE_CODE_DISABLE_AUTO_MEMORY=1, which turns \
                      auto-memory off so no per-run memory path enters the system prompt (a70)";

const ADDED: &str = "; the run's declarations add ";

/// Values that must never reach the attestation's text, each distinct enough that a substring
/// hit can only be the value itself.
const OPERATOR_KEY: &str = "sk-ant-operator-secret-5d1c";
const OPERATOR_ACTOR: &str = "human:exported-by-the-shell";
const DECLARED_ACTOR: &str = "agent:x-declared-value";
const ENDPOINT: &str = "https://endpoint-value.example/v1";
const LOOPBACK_URL: &str = "http://127.0.0.1:47613";
const LOOPBACK_TOKEN: &str = "mh-run-claude-9-placeholder-value";

fn context() -> LaunchContext {
    LaunchContext {
        context_files: Vec::new(),
        catalogue: None,
        scratch_root: PathBuf::from("/scratch/run-1"),
        cwd: PathBuf::from("/scratch/run-1/work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([
            ("HOME".to_string(), "/operator".to_string()),
            ("USER".to_string(), "operator".to_string()),
            ("LANG".to_string(), "C.UTF-8".to_string()),
            ("AEP_ACTOR".to_string(), OPERATOR_ACTOR.to_string()),
            ("ANTHROPIC_API_KEY".to_string(), OPERATOR_KEY.to_string()),
        ]),
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

fn h3(plan: &LaunchPlan) -> String {
    let rows: Vec<&str> = plan
        .attestation
        .imposed
        .iter()
        .filter(|control| control.row == HermeticRow::H3)
        .map(|control| control.how.as_str())
        .collect();
    assert_eq!(rows.len(), 1, "exactly one imposed H3 row: {rows:?}");
    rows[0].to_owned()
}

/// The names H3 lists after the old sentence, with the actor's phrase read back to its key.
fn listed(how: &str) -> Vec<String> {
    let Some(rest) = how.strip_prefix(OLD_H3) else {
        panic!("H3 no longer opens with the old sentence: {how}");
    };
    if rest.is_empty() {
        return Vec::new();
    }
    let Some(names) = rest.strip_prefix(ADDED) else {
        panic!("H3 adds something that is not the declarations list: {rest:?}");
    };
    names
        .split(", ")
        .map(|name| {
            if name == "the declared AEP_ACTOR" {
                "AEP_ACTOR".to_owned()
            } else {
                name.to_owned()
            }
        })
        .collect()
}

/// Every way a world can differ in what `build_env` adds, combined, and nothing else varied.
fn worlds() -> Vec<(String, RunSpec, LaunchContext)> {
    let credentials = [
        ("operator-login", CredentialSource::OperatorLogin),
        ("api-key", CredentialSource::ApiKey),
        ("none", CredentialSource::None),
        ("loopback", CredentialSource::Loopback),
    ];
    let mut worlds = Vec::new();
    for (credential_name, credential) in credentials {
        for endpoint in [false, true] {
            for actor in [false, true] {
                for ttl in [
                    None,
                    Some(PromptCacheTtl::FiveMinutes),
                    Some(PromptCacheTtl::OneHour),
                ] {
                    for managed in [false, true] {
                        let mut spec = spec();
                        spec.credentials = credential;
                        if endpoint {
                            spec.model_endpoint = Some(ENDPOINT.to_string());
                        }
                        if actor {
                            spec.actor = Some(DECLARED_ACTOR.to_string());
                        }
                        spec.prompt_cache_ttl = ttl;
                        let mut world = context();
                        if credential == CredentialSource::Loopback {
                            world.loopback = Some(LoopbackParams {
                                base_url: LOOPBACK_URL.to_string(),
                                placeholder: LOOPBACK_TOKEN.to_string(),
                            });
                        }
                        if managed {
                            world.managed_settings =
                                vec![PathBuf::from("/etc/claude-code/managed-settings.json")];
                        }
                        let name = format!(
                            "{credential_name} endpoint={endpoint} actor={actor} ttl={ttl:?} \
                             managed={managed}"
                        );
                        worlds.push((name, spec, world));
                    }
                }
            }
        }
    }
    worlds
}

/// A74 point 4 and the CHANGELOG: "a run that sets none of them keeps the old text". The unit's
/// tests read H3 only through `contains`, so a separator printed with an empty list, or any
/// rewording of the fixed sentence, passes them. This holds the plain world to the old literal.
#[test]
fn a_plan_that_adds_nothing_keeps_the_old_h3_sentence_word_for_word() {
    let plan = plan_launch(&spec(), &context()).expect("the plain run plans");
    assert_eq!(h3(&plan), OLD_H3);
}

/// The attestation is a value a reader diffs between runs, so the order of the names is part of
/// it. Every variable `build_env` can add, at once, in the byte order of their keys.
#[test]
fn every_addition_at_once_is_listed_in_key_order() {
    let mut spec = spec();
    spec.credentials = CredentialSource::Loopback;
    spec.actor = Some(DECLARED_ACTOR.to_string());
    spec.prompt_cache_ttl = Some(PromptCacheTtl::OneHour);
    let mut world = context();
    world.loopback = Some(LoopbackParams {
        base_url: LOOPBACK_URL.to_string(),
        placeholder: LOOPBACK_TOKEN.to_string(),
    });
    let plan = plan_launch(&spec, &world).expect("the loopback run plans");
    assert_eq!(
        h3(&plan),
        format!(
            "{OLD_H3}{ADDED}the declared AEP_ACTOR, ANTHROPIC_AUTH_TOKEN, ANTHROPIC_BASE_URL, \
             CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC, CLAUDE_CODE_PROMPT_CACHE_TTL"
        )
    );
}

/// The acceptance's second half against a world where something *could* put the name there: the
/// operator exported `AEP_ACTOR`. Undeclared, the row names no actor; declared, it names the
/// actor and carries neither the operator's value nor the declared one.
#[test]
fn an_exported_actor_is_never_named_and_a_declared_one_is_named_without_its_value() {
    let world = context();
    let undeclared = h3(&plan_launch(&spec(), &world).expect("plans"));
    assert!(!undeclared.contains("AEP_ACTOR"), "{undeclared}");
    assert!(!undeclared.contains(OPERATOR_ACTOR), "{undeclared}");

    let mut declared_spec = spec();
    declared_spec.actor = Some(DECLARED_ACTOR.to_string());
    let plan = plan_launch(&declared_spec, &world).expect("plans");
    assert_eq!(
        plan.env.get("AEP_ACTOR").map(String::as_str),
        Some(DECLARED_ACTOR)
    );
    let declared = h3(&plan);
    assert_eq!(declared, format!("{OLD_H3}{ADDED}the declared AEP_ACTOR"));
    assert!(!declared.contains(OPERATOR_ACTOR), "{declared}");
    assert!(!declared.contains(DECLARED_ACTOR), "{declared}");
}

/// The class, held to H3 itself and in both directions, over every combination of what the run
/// declares. The unit's class case accepts a name found in *any* imposed row, so H4's
/// "`ANTHROPIC_API_KEY` is absent from the child environment" would count as naming a key that is
/// present; and it never checks the other direction, a name for a variable the child lacks.
///
/// For every world that plans: the names H3 lists are exactly the child's keys beyond the
/// allowlist and the four, in key order; no value of any child variable outside the allowlist
/// appears in H3; no credential value appears anywhere in the attestation; and planning twice
/// gives the same attestation.
#[test]
fn h3_lists_exactly_the_added_keys_by_name_in_every_world() {
    let mut planned = 0_usize;
    let mut wrong = Vec::new();
    for (name, spec, world) in worlds() {
        let Ok(plan) = plan_launch(&spec, &world) else {
            continue;
        };
        planned += 1;
        let how = h3(&plan);
        let expected: Vec<String> = plan
            .env
            .keys()
            .filter(|key| !INHERITED.contains(&key.as_str()) && !ALWAYS.contains(&key.as_str()))
            .cloned()
            .collect();
        let names = listed(&how);
        if names != expected {
            wrong.push(format!("{name}: lists {names:?}, child has {expected:?}"));
        }
        let unique: BTreeSet<&String> = names.iter().collect();
        if unique.len() != names.len() {
            wrong.push(format!("{name}: a name is listed twice: {names:?}"));
        }
        for (key, value) in &plan.env {
            if INHERITED.contains(&key.as_str()) || value.len() < 4 {
                continue;
            }
            if how.contains(value.as_str()) {
                wrong.push(format!("{name}: H3 carries the value of {key}"));
            }
        }
        let whole = serde_json::to_string(&plan.attestation).expect("the attestation serialises");
        for secret in [OPERATOR_KEY, LOOPBACK_TOKEN, OPERATOR_ACTOR, DECLARED_ACTOR] {
            if whole.contains(secret) {
                wrong.push(format!("{name}: the attestation carries {secret:?}"));
            }
        }
        let again = plan_launch(&spec, &world).expect("plans twice");
        if again.attestation != plan.attestation {
            wrong.push(format!("{name}: two plans of one world attest differently"));
        }
    }
    assert!(
        planned >= 48,
        "too few worlds planned to say anything: {planned}"
    );
    assert_eq!(wrong, Vec::<String>::new());
}
