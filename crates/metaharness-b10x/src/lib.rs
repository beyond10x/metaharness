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
//! **MCP has stopped being one of those absences.** Since 0.12.1 the loop publishes
//! `--mcp-profile` and `--mcp-registry` and writes an `mcp` field in its opening record; the 0.15.0
//! capture in `fixtures/golden/` carries it as `"mcp":[]`. `mcp_servers` is still the hardcoded
//! `[]` the old standing fact justified, because every observation so far — 2026-09-15 on 0.12.1,
//! 2026-10-20 on 0.15.0 — saw that field only as `[]` and the shape of an element is unestablished
//! — see `B10xSeam`'s own note and
//! `docs/research/2026-10-20-b10x-harness-0.15.0-adapter-surface.md`.

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
pub const PINNED_VERSIONS: [&str; 1] = ["0.15.0"];

/// The immutable harness source revision this adapter is built against.
///
/// The version identifies the released CLI; the revision identifies the Rust crates Cargo
/// resolves. Both are checked by the AEP eval before it trusts an installed
/// binary, so a filesystem timestamp is never mistaken for provenance.
pub const HARNESS_REVISION: &str = "dd5c2a66218a5f2555c4a9fdab6fdfac8515cc6d";

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
/// Harness 0.15.0 (`dd5c2a66`) canonicalizes the set before Substrate 0.7.17
/// (`00fa634c`) validates workspace access. This adapter mirrors those syntax bounds
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

#[cfg(test)]
mod tests {
    use super::{HARNESS_REVISION, PINNED_VERSIONS};

    const HARNESS: &str = "beyond10x/harness";
    const TOOLS: &str = "beyond10x/tools";

    fn workspace_file(relative: &str) -> String {
        let path = format!("{}/../../{relative}", env!("CARGO_MANIFEST_DIR"));
        std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// The `owner/name` a git URL names, lower-cased, whatever its spelling: `https://` or
    /// `ssh://git@`, the scp-like `git@host:owner/name`, with or without `.git`, a trailing `/`, a
    /// `?rev=`/`?tag=` query or a `#revision` fragment. A check keyed on one spelling would let a
    /// second spelling of the same repository through unexamined, at another revision.
    fn repository_of(url: &str) -> Option<String> {
        let url = url.split(['?', '#']).next()?;
        let path = url.split_once("://").map_or_else(
            || url.split_once(':').map(|(_, path)| path),
            |(_, rest)| rest.split_once('/').map(|(_, path)| path),
        )?;
        let path = path.trim_end_matches('/');
        let path = path.strip_suffix(".git").unwrap_or(path);
        let (owner, name) = path.rsplit_once('/')?;
        let owner = owner.rsplit('/').next()?;
        (!owner.is_empty() && !name.is_empty()).then(|| format!("{owner}/{name}").to_lowercase())
    }

    /// The repository and revision of one `Cargo.lock` `source = "git+…#revision"` value, or
    /// `None` for any source that is not git.
    fn git_source(source: &str) -> Option<(String, String)> {
        let url = source.strip_prefix("git+")?;
        let (_, revision) = url.rsplit_once('#')?;
        Some((repository_of(url)?, revision.to_owned()))
    }

    /// Every `[[package]]` in `Cargo.lock` whose git source is `repository` (`owner/name`), as
    /// `(name, version, revision)`.
    fn locked_from(lock: &str, repository: &str) -> Vec<(String, String, String)> {
        lock.split("[[package]]")
            .filter_map(|block| {
                let field = |key: &str| {
                    block.lines().find_map(|line| {
                        line.strip_prefix(&format!("{key} = \""))
                            .and_then(|rest| rest.strip_suffix('"'))
                    })
                };
                let (source, revision) = git_source(field("source")?)?;
                (source == repository).then(|| {
                    Some((
                        field("name")?.to_owned(),
                        field("version")?.to_owned(),
                        revision,
                    ))
                })?
            })
            .collect()
    }

    /// The value of a `key = "…"` entry inside one manifest dependency line.
    fn manifest_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
        let start = line.find(&format!("{key} = \""))? + key.len() + 4;
        line[start..].split('"').next()
    }

    #[test]
    fn every_spelling_of_a_git_source_names_its_repository() {
        for (source, repository, revision) in [
            (
                "git+https://github.com/beyond10x/harness?rev=abc#abc",
                HARNESS,
                "abc",
            ),
            (
                "git+https://github.com/beyond10x/harness#abc",
                HARNESS,
                "abc",
            ),
            (
                "git+https://github.com/beyond10x/harness.git?tag=0.15.0#abc",
                HARNESS,
                "abc",
            ),
            (
                "git+https://github.com/beyond10x/harness/#abc",
                HARNESS,
                "abc",
            ),
            (
                "git+ssh://git@github.com/beyond10x/harness.git?rev=abc#abc",
                HARNESS,
                "abc",
            ),
            ("git+git@github.com:beyond10x/tools.git#def", TOOLS, "def"),
            (
                "git+ssh://git@github.com/Beyond10x/Tools?tag=0.2.0#def",
                TOOLS,
                "def",
            ),
            (
                "git+https://github.com/beyond10x/tools?tag=0.2.0#def",
                TOOLS,
                "def",
            ),
        ] {
            assert_eq!(
                git_source(source),
                Some((repository.to_owned(), revision.to_owned())),
                "{source}"
            );
        }
        assert_eq!(
            git_source("git+https://github.com/beyond10x/harness-extras#abc"),
            Some(("beyond10x/harness-extras".to_owned(), "abc".to_owned()))
        );
        assert_eq!(
            git_source("registry+https://github.com/rust-lang/crates.io-index"),
            None
        );
        assert_eq!(
            repository_of("git@github.com:beyond10x/harness"),
            Some(HARNESS.to_owned())
        );
        assert_eq!(
            manifest_value(
                r#"x = { git = "ssh://git@github.com/beyond10x/harness.git", rev = "abc" }"#,
                "rev"
            ),
            Some("abc")
        );
    }

    /// **The one deliberate literal pair.** Every other reader of the pin goes through these two
    /// constants, so a move that forgot one of them would agree with itself everywhere else. Moved
    /// 0.13.3 / `798325f0` → 0.15.0 / `dd5c2a66` on 2026-10-20: the commit the annotated tag
    /// `0.15.0` points at, never the tag object, because the eval compares this with
    /// `git rev-parse HEAD` in the harness checkout.
    #[test]
    fn the_pin_names_the_harness_0_15_0_release_commit() {
        assert_eq!(PINNED_VERSIONS, ["0.15.0"]);
        assert_eq!(HARNESS_REVISION, "dd5c2a66218a5f2555c4a9fdab6fdfac8515cc6d");
    }

    /// The constants, both manifests and the lock name one harness: one revision, the released
    /// version at it, and no crate that release deleted; and one revision of the `tools` crates
    /// harness pins. A pin pair that moved without the Cargo pin — or the reverse, as on
    /// 2026-09-15 — is a claim about a binary this workspace does not compile against.
    #[test]
    fn cargo_resolves_exactly_the_pinned_harness() {
        let lock = workspace_file("Cargo.lock");
        let harness = locked_from(&lock, HARNESS);
        assert!(!harness.is_empty(), "the lock names no harness crate");
        for (name, version, revision) in &harness {
            assert_eq!(revision, HARNESS_REVISION, "{name} is locked off the pin");
            assert_eq!(
                version, PINNED_VERSIONS[0],
                "{name} is not the pinned release"
            );
        }
        // Exactly the three crates this workspace takes, and through them nothing else of harness's:
        // the catalogue layer, the record's crate and the wire values both of those speak.
        let mut names: Vec<&str> = harness.iter().map(|(name, ..)| name.as_str()).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "b10x-harness-catalogue",
                "b10x-harness-loop",
                "b10x-harness-wire"
            ]
        );

        let mut tools: Vec<String> = locked_from(&lock, TOOLS)
            .into_iter()
            .map(|(.., revision)| revision)
            .collect();
        tools.sort_unstable();
        tools.dedup();
        assert_eq!(
            tools.len(),
            1,
            "the lock holds one tools revision, the one harness pins: {tools:?}"
        );

        for manifest in [
            "crates/metaharness-b10x/Cargo.toml",
            "crates/metaharness-tools/Cargo.toml",
        ] {
            let text = workspace_file(manifest);
            let pins: Vec<&str> = text
                .lines()
                .filter(|line| {
                    manifest_value(line, "git")
                        .and_then(repository_of)
                        .is_some_and(|repository| repository == HARNESS)
                })
                .collect();
            assert!(!pins.is_empty(), "{manifest} takes no harness crate");
            for pin in pins {
                assert_eq!(
                    manifest_value(pin, "rev"),
                    Some(HARNESS_REVISION),
                    "{manifest}: {pin}"
                );
            }
        }
    }
}
