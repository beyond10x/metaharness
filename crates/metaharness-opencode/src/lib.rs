//! Contract-first evidence for a future `OpenCode` adapter.
//!
//! `OpenCode` is not a [`metaharness_protocol::Kind`] yet. This crate pins the 1.4.7 command surface
//! and its pure, scratch-home JSON launch shape while naming the two facts still missing: a
//! recorded event stream and a verified per-call blocking seam. It therefore cannot be mistaken
//! for a runnable adapter.

use std::collections::BTreeMap;
use std::path::Path;

use metaharness_protocol::{ConformanceTier, ContractObligations, Obligation, VectorOutcome};
use serde_json::{Value, json};

/// Adapter id reserved by this evidence pack.
pub const ADAPTER_ID: &str = "opencode";

/// Version whose own banner and help were read on 2026-08-31.
pub const PINNED_VERSIONS: [&str; 1] = ["1.4.7"];

/// The contract rows answered before this adapter is admitted to runtime dispatch.
pub const CONTRACT_OBLIGATIONS: ContractObligations = ContractObligations {
    adapter: ADAPTER_ID,
    launch: Obligation::Filled(&["c1-pure-json-launch"]),
    recorded_wire: Obligation::Gap(
        "no model-backed `run --format json` capture is committed yet, so no OpenCode event is mapped or claimed",
    ),
    recorded_hook_input: Obligation::Gap(
        "OpenCode 1.4.7 exposes permissions and plugins, but no blocking per-call wire has been recorded and driven",
    ),
    version_pair: Obligation::Filled(&["observed-version-pair"]),
};

const LAUNCH_EXPECTED: &str = include_str!("../fixtures/c1/pure-json-launch.json");
const VERSION: &str = include_str!("../fixtures/version.txt");

/// A deterministic launch baseline with external plugins disabled and JSON events requested.
#[must_use]
pub fn argv(program: &str, model: &str, cwd: &Path, prompt: &str) -> Vec<String> {
    [
        program.to_owned(),
        "run".to_owned(),
        "--format".to_owned(),
        "json".to_owned(),
        "--pure".to_owned(),
        "--model".to_owned(),
        model.to_owned(),
        "--dir".to_owned(),
        cwd.display().to_string(),
        prompt.to_owned(),
    ]
    .into_iter()
    .collect()
}

/// Scratch XDG roots and no credential or home directory.
#[must_use]
pub fn base_environment(scratch: &Path) -> BTreeMap<String, String> {
    BTreeMap::from([
        ("PATH".to_owned(), "/usr/local/bin:/usr/bin:/bin".to_owned()),
        (
            "XDG_CACHE_HOME".to_owned(),
            scratch.join("cache").display().to_string(),
        ),
        (
            "XDG_CONFIG_HOME".to_owned(),
            scratch.join("config").display().to_string(),
        ),
        (
            "XDG_DATA_HOME".to_owned(),
            scratch.join("data").display().to_string(),
        ),
    ])
}

/// The free evidence available before runtime admission.
#[must_use]
pub fn conformance_vectors() -> Vec<VectorOutcome> {
    vec![launch_vector(), version_pair_vector()]
}

fn launch_vector() -> VectorOutcome {
    let observed = json!({
        "program": "/opt/opencode/bin/opencode",
        "args": argv(
            "/opt/opencode/bin/opencode",
            "anthropic/claude-opus-5",
            Path::new("/scratch/ws_run"),
            "inspect the tree",
        ),
        "env": base_environment(Path::new("/scratch/run")),
    });
    compare(
        "c1-pure-json-launch",
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
        assert!(
            CONTRACT_OBLIGATIONS
                .unmet(&vectors, "opencode 1.4.7")
                .is_empty()
        );
    }

    #[test]
    fn the_baseline_is_pure_json_and_has_no_ambient_home() {
        let argv = argv(
            "opencode",
            "anthropic/claude-opus-5",
            Path::new("/work"),
            "inspect",
        );
        assert!(argv.windows(2).any(|pair| pair == ["--format", "json"]));
        assert!(argv.iter().any(|word| word == "--pure"));
        assert!(argv.windows(2).any(|pair| pair == ["--dir", "/work"]));
        assert!(!base_environment(Path::new("/scratch")).contains_key("HOME"));
    }
}
