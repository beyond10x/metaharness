//! Contract-first evidence for a future Pi adapter.
//!
//! This crate deliberately does not make `pi` a [`metaharness_protocol::Kind`]. It pins the
//! installed 0.80.3 command surface and a deterministic, read-only launch face first, while its
//! vendor event reader and control seam remain named contract gaps. Advertising a runnable kind
//! before those faces exist would turn absence of evidence into an adapter claim.

use std::collections::BTreeMap;
use std::path::Path;

use metaharness_protocol::{ConformanceTier, ContractObligations, Obligation, VectorOutcome};
use serde_json::{Value, json};

/// Adapter id reserved by this evidence pack.
pub const ADAPTER_ID: &str = "pi";

/// Version whose own banner and help were read on 2026-08-31.
pub const PINNED_VERSIONS: [&str; 1] = ["0.80.3"];

/// The contract rows answered before this adapter is admitted to runtime dispatch.
pub const CONTRACT_OBLIGATIONS: ContractObligations = ContractObligations {
    adapter: ADAPTER_ID,
    launch: Obligation::Filled(&["c1-read-only-launch"]),
    recorded_wire: Obligation::Gap(
        "no model-backed `--mode json` capture is committed yet, so no Pi event is mapped or claimed",
    ),
    recorded_hook_input: Obligation::Gap(
        "Pi 0.80.3 exposes extensions but no verified blocking per-call hook contract has been driven",
    ),
    version_pair: Obligation::Filled(&["observed-version-pair"]),
};

const LAUNCH_EXPECTED: &str = include_str!("../fixtures/c1/read-only-launch.json");
const VERSION: &str = include_str!("../fixtures/version.txt");

/// A deterministic baseline which cannot write or execute and discovers no ambient resource.
#[must_use]
pub fn argv(program: &str, model: &str, prompt: &str) -> Vec<String> {
    [
        program,
        "--mode",
        "json",
        "--print",
        "--no-session",
        "--no-extensions",
        "--no-skills",
        "--no-prompt-templates",
        "--no-themes",
        "--no-context-files",
        "--no-approve",
        "--offline",
        "--tools",
        "read,grep,find,ls",
        "--provider",
        "anthropic",
        "--model",
        model,
        prompt,
    ]
    .into_iter()
    .map(ToOwned::to_owned)
    .collect()
}

/// The closed environment paired with [`argv`]. It contains no credential and no home directory.
#[must_use]
pub fn base_environment(scratch: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("PATH".to_owned(), "/usr/local/bin:/usr/bin:/bin".to_owned()),
        (
            "PI_CODING_AGENT_DIR".to_owned(),
            scratch.join("config").display().to_string(),
        ),
        (
            "PI_CODING_AGENT_SESSION_DIR".to_owned(),
            scratch.join("sessions").display().to_string(),
        ),
        ("PI_OFFLINE".to_owned(), "1".to_owned()),
    ])
}

/// The free evidence available before runtime admission.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    vec![launch_vector(), version_pair_vector()]
}

fn launch_vector() -> VectorOutcome {
    let observed = json!({
        "program": "/opt/pi/bin/pi",
        "args": argv("/opt/pi/bin/pi", "claude-opus-5", "inspect the tree"),
        "env": base_environment(Path::new("/scratch/run")),
    });
    compare(
        "c1-read-only-launch",
        ConformanceTier::C1,
        LAUNCH_EXPECTED,
        &observed,
    )
}

fn version_pair_vector() -> VectorOutcome {
    let observed = VERSION.trim();
    if PINNED_VERSIONS.contains(&observed) {
        VectorOutcome::passed("observed-version-pair", ConformanceTier::C2)
    } else {
        VectorOutcome::failed(
            "observed-version-pair",
            ConformanceTier::C2,
            format!("the recorded banner is {observed:?}; the pin is {PINNED_VERSIONS:?}"),
        )
    }
}

fn compare(id: &str, tier: ConformanceTier, expectation: &str, observed: &Value) -> VectorOutcome {
    match serde_json::from_str::<Value>(expectation) {
        Ok(expected) if expected == *observed => VectorOutcome::passed(id, tier),
        Ok(expected) => VectorOutcome::failed(
            id,
            tier,
            format!("expected {expected}, observed {observed}"),
        ),
        Err(error) => VectorOutcome::failed(id, tier, format!("fixture does not parse: {error}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_filled_contract_row_has_a_green_vector() {
        let vectors = conformance_vectors();
        assert!(vectors.iter().all(|vector| vector.passed), "{vectors:#?}");
        assert!(CONTRACT_OBLIGATIONS.unmet(&vectors, "pi 0.80.3").is_empty());
    }

    #[test]
    fn the_baseline_cannot_write_execute_or_discover_operator_resources() {
        let argv = argv("pi", "claude-opus-5", "inspect");
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["--tools", "read,grep,find,ls"])
        );
        for flag in [
            "--no-extensions",
            "--no-skills",
            "--no-prompt-templates",
            "--no-themes",
            "--no-context-files",
            "--no-approve",
            "--no-session",
        ] {
            assert!(argv.iter().any(|word| word == flag), "missing {flag}");
        }
        assert!(!base_environment(Path::new("/scratch")).contains_key("HOME"));
    }
}
