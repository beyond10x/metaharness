//! Shared fixtures for the launch-plan tests: a context and a plugin tree, both pure values.
//!
//! Nothing here touches a filesystem. `plan_launch` reads nothing by design — the caller does the
//! walk and the digests — so a launch-plan test is a test over values and never over a directory.

use std::collections::BTreeMap;
use std::path::PathBuf;

use metaharness_claude::LaunchContext;
use metaharness_protocol::{Digest, PluginContent, PluginTree, tree_digest};

/// The scratch root every expectation in these tests names.
pub const SCRATCH_ROOT: &str = "/scratch/run-1";

/// A context with no plugin and nothing exotic.
#[must_use]
pub fn context() -> LaunchContext {
    LaunchContext {
        scratch_root: PathBuf::from(SCRATCH_ROOT),
        cwd: PathBuf::from("/scratch/run-1/work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([("HOME".to_string(), "/operator".to_string())]),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"inputs")),
        plugins: Vec::new(),
        marketplace_plugins: Vec::new(),
        loopback: None,
        tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
    }
}

/// A plugin directory the caller has "read": a manifest and one skill, digested over both.
#[must_use]
pub fn plugin_tree(name: &str) -> (PathBuf, PluginTree, Digest) {
    let source = PathBuf::from(format!("/operator/plugins/{name}"));
    let files = BTreeMap::from([
        (
            ".claude-plugin/plugin.json".to_string(),
            Digest::of(b"{\"name\":\"x\"}"),
        ),
        (
            "skills/one/SKILL.md".to_string(),
            Digest::of(b"do the thing"),
        ),
    ]);
    let digest = tree_digest(&files);
    let tree = PluginTree {
        source: source.clone(),
        content: PluginContent::Files {
            count: files.len(),
            digest: digest.clone(),
        },
    };
    (source, tree, digest)
}
