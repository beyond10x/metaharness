//! The b10x adapter: an agent loop we own, **observed** rather than driven.
//!
//! # Why this adapter decides nothing, and why that is the design
//!
//! Every other adapter in this workspace exists to put metaharness between a vendor's loop and its
//! tools: a hook that blocks a call, a control request that answers one, a registration that bounds
//! the set. That is what makes a *driven* run driven.
//!
//! `b10x-harness` is not a vendor's loop. It holds its own, and the toolset it publishes is
//! computed from what the machine can confine — a tool outside the surface does not exist rather
//! than being refused. An evaluation arm exists to measure whether *that* changes what a model
//! does, and a seam that adjudicated its calls would put the driven arm's treatment back on top and
//! measure that instead. The two arms would then differ in name only.
//!
//! So this adapter runs in [`DecisionMode::Observe`](metaharness_protocol::spec::DecisionMode) and
//! nothing else. What it contributes is the half that is **not** control:
//!
//! * **attestation** — what metaharness imposed on the launch, in metaharness's own words, beside
//!   the loop's record so a reader can notice when the two disagree;
//! * **one wire** — the same `metaharness.event/1` stream every other arm is judged from, so a
//!   matrix that compares four cells is comparing runs and not instruments.
//!
//! Refusing to decide is asserted rather than assumed: [`B10xSeam::decision_line`] answers a line
//! no run will ever send, and every `tool.requested` carries `decision_required: false` and
//! [`Seam::None`] — *nobody adjudicated this call*, which is the fact rather than the omission.
//!
//! # `adapter_class` is `direct_provider`, not `harness`
//!
//! Design § 8.4 O5 requires that a harness adapter never silently becomes a direct API call. This
//! is the other direction of the same rule, and the protocol already had the word for it:
//! [`AdapterClass::DirectProvider`](metaharness_protocol::AdapterClass) — *"the embedder holds the
//! conversation and calls a model API"* — which carried a *not in v0.1* note because nothing had
//! ever been one. This is the first.
//!
//! # What the loop reports, and what it does not have
//!
//! Skills and named agents are read from the opening record: the loop gained both after this
//! adapter was introduced, so hardcoding empty lists would now assert something about a run that
//! may have been offered several. It has no slash-command surface and no vendor permission mode,
//! and those absences are standing facts; fields this adapter has not established remain `null`
//! rather than being guessed empty.
//!
//! **MCP has stopped being one of those absences.** 0.12.1 publishes `--mcp-profile` and
//! `--mcp-registry` and writes an `mcp` field in its opening record, which the 0.9.1 capture in
//! `fixtures/golden/` does not carry. `mcp_servers` is still the hardcoded `[]` the old standing
//! fact justified, because the 2026-09-15 observation saw that field only as `[]` and the shape of
//! an element is unestablished — see `B10xSeam`'s own note and
//! `docs/research/2026-09-15-b10x-harness-0.12.1-adapter-surface.md`.

#![allow(missing_docs)]

mod launch;
mod seam;
mod vectors;

pub use launch::{
    B10xLaunch, Confinement, Credential, Wire, argv, base_environment, child_path, emitted_flags,
    resolve_program,
};
pub use seam::{B10xSeam, B10xSeams, capabilities};
pub use vectors::{CONTRACT_OBLIGATIONS, conformance_vectors};

/// What this adapter calls itself on the wire.
pub const ADAPTER_ID: &str = "b10x";

/// Not `harness`, and **the protocol already had the word**.
///
/// `AdapterClass::DirectProvider` is documented as *"the embedder holds the conversation and calls
/// a model API"* and carried a *"not in v0.1"* note because nothing had ever been one. This adapter
/// is exactly that, so it takes the existing class rather than coining a synonym — an adapter that
/// invented `loop` beside it would have given a reader two words for one thing and no way to tell
/// which documents applied.
pub const ADAPTER_CLASS: &str = "direct_provider";

/// The `b10x-harness` versions this adapter's claims were read from.
///
/// Pinned for the reason the other adapters pin: every version-specific claim in here — the field
/// names of the loop record, the shape of its terminal event — was observed against these, and a
/// run against another is unverified rather than wrong.
pub const PINNED_VERSIONS: [&str; 1] = ["0.13.3"];

/// The immutable harness source revision this adapter is built against.
///
/// The version identifies the released CLI; the revision identifies the Rust crates Cargo
/// resolves. Both are checked by the AEP eval before it trusts an installed
/// binary, so a filesystem timestamp is never mistaken for provenance.
pub const HARNESS_REVISION: &str = "798325f03cf5a18df8fadb346d31b314826136ec";

/// Whether a directory component satisfies the pinned substrate adoption syntax.
///
/// This proves no confinement: the driver still checks identity beneath its pinned
/// root descriptor. Callers must resolve the explicitly selected directory first.
#[must_use]
pub fn workspace_component_is_adoptable(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with('-')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

/// Validate exact workspace-relative process write directories before native launch.
///
/// Harness 0.13.3 (`798325f0`) canonicalizes the set before Substrate 0.7.8
/// (`05695970`) validates workspace access. This adapter mirrors those syntax bounds
/// without importing a different native dependency. Glob spelling is additionally
/// refused: this declaration never expands a file-tool pattern into a mount.
/// Existence, symlink safety and actual mount admission remain the host's job.
///
/// # Errors
///
/// Names an invalid, excessive or overlapping `--process-write-subtree` declaration.
pub fn validate_process_write_subtrees(paths: &[String]) -> Result<(), String> {
    let mut directories: Vec<&str> = paths.iter().map(String::as_str).collect();
    directories.sort_unstable();
    directories.dedup();
    if directories.len() > 64 {
        return Err("--process-write-subtree admits at most 64 distinct directories".to_owned());
    }
    for directory in &directories {
        if directory.is_empty()
            || directory.starts_with('/')
            || directory.contains(['\0', '\\', '*', '?', '[', ']', '{', '}'])
            || directory.split('/').count() > 64
            || directory
                .split('/')
                .any(|part| part.is_empty() || matches!(part, "." | ".."))
        {
            return Err(format!(
                "--process-write-subtree requires an exact workspace-relative directory without root, traversal or glob syntax: {directory:?}"
            ));
        }
    }
    for (index, directory) in directories.iter().enumerate() {
        if directories[index + 1..].iter().any(|other| {
            other
                .strip_prefix(directory)
                .is_some_and(|suffix| suffix.starts_with('/'))
        }) {
            return Err(format!(
                "--process-write-subtree directories must not overlap: {directory:?}"
            ));
        }
    }
    Ok(())
}
