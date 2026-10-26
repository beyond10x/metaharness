//! The b10x adapter: an agent loop we own, **observed** by default and decided only when asked.
//!
//! # Why observe is the default, and why that is the design
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
//! So [`DecisionMode::Observe`](metaharness_protocol::spec::DecisionMode) is the arm the
//! evaluation runs, and on it this adapter decides nothing: every `tool.requested` carries
//! `decision_required: false` and [`Seam::None`] — *nobody adjudicated this call*, which is the
//! fact rather than the omission — and [`B10xSeam::decision_line`] answers a line no observe run
//! will ever send. What it contributes there is the half that is **not** control:
//!
//! * **attestation** — what metaharness imposed on the launch, in metaharness's own words, beside
//!   the loop's record so a reader can notice when the two disagree;
//! * **one wire** — the same `metaharness.event/1` stream every other arm is judged from, so a
//!   matrix that compares four cells is comparing runs and not instruments.
//!
//! # The caller seam: `frame` and `ask` (atlas ADR 0104, design amendment a61)
//!
//! ADR 0104 makes metaharness the one decider of tool permission, and harness 0.20.0 gave the loop
//! the other half: `run --approve caller`, under which each call above the loop's ceiling is put to
//! the process that launched it as an `approval-required` record line, and answered by one JSON
//! line on the loop's stdin. A `frame` or `ask` run launches that way, at the lowest ceiling the
//! loop offers (`--approve-up-to low`), and each question reaches the run as a `tool.requested` on
//! [`Seam::ControlRequest`] that the frame, the embedder or a deadline answers like any other.
//!
//! **What the caller seam does not cover is stated rather than hidden.** `low` is the loop's floor,
//! and the reads — `file_read`, `dir_list`, `search`, `find` — are `low`, so they run unasked and
//! reach the record on [`Seam::None`] exactly as on an observe run. A frame that did not admit them
//! would be a frame the model is held to and the loop is not, so such a frame is refused at start
//! ([`caller_frame_problem`]). `Tier::Call` therefore stays unverified: metaharness decides every
//! call the loop asks about, and not every call.
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
//! `--mcp-profile` and `--mcp-registry` and writes an `mcp` field in its opening record; the 0.20.0
//! capture in `fixtures/golden/` carries it as `"mcp":[]`. `mcp_servers` is still the hardcoded
//! `[]` the old standing fact justified, because every observation so far — 2026-09-15 on 0.12.1,
//! 2026-10-20 on 0.15.0 and 0.15.1, 2026-10-22 on 0.18.0, 2026-10-26 on 0.19.0 and 0.20.0 — saw it
//! only as `[]` and the shape of an element is unestablished — see `B10xSeam`'s own note,
//! `docs/research/2026-10-20-b10x-harness-0.15.1-adapter-surface.md`,
//! `docs/research/2026-10-22-b10x-harness-0.18.0-adapter-surface.md`,
//! `docs/research/2026-10-26-b10x-harness-0.19.0-adapter-surface.md` and
//! `docs/research/2026-10-26-b10x-harness-0.20.0-adapter-surface.md`.
//!
//! **The working directory is the loop's own word since 0.15.1.** The opening record's `cwd` is
//! the `started` line's `cwd` — the canonical root the loop's file tools act in — and `null` when
//! the line names none, never the directory metaharness planned (design a30, a31).

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

/// The channel `--context` files reach the loop through: the system prompt (design amendment a35,
/// correction round 2).
///
/// The adapter hands each file to the loop's own `--context`, and the loop at [`HARNESS_REVISION`]
/// renders it into its standing instruction — the instruction its wire codecs send as the system
/// prompt — as a `<context kind="provided_context" trust="operator" source="…">` layer it wraps
/// itself, with none of metaharness's reference-data preamble. So the file gains system-prompt
/// authority, as on Claude Code, and the record says so.
pub const CONTEXT_CHANNEL: metaharness_protocol::ContextChannel =
    metaharness_protocol::ContextChannel::SystemPrompt;

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
pub const PINNED_VERSIONS: [&str; 1] = ["0.20.0"];

/// The immutable harness source revision this adapter is built against.
///
/// The version identifies the released CLI; the revision identifies the Rust crates Cargo
/// resolves. Both are checked by the AEP eval before it trusts an installed
/// binary, so a filesystem timestamp is never mistaken for provenance.
pub const HARNESS_REVISION: &str = "684c99f5e8c7e1d22d7b29f717b71856db81766a";

/// The `credential_source` the pinned loop records when its key is named by `--api-key-secret`
/// (design amendment a50 § 7).
///
/// Read from the loop's own `credential_source`, which returns this string for that flag and no
/// other. The hermetic floor's H4 compares a secret-named run's opening record with exactly this,
/// so a key that reached the loop by any other road reads as a gap.
pub const SECRET_KEY_SOURCE: &str = "api-key:secrets";

/// The variables a secret-named run forwards to the loop, by name, when metaharness has them
/// (design amendment a50 § 6).
///
/// The loop opens the operator's secret store over the D-Bus session bus, and its client finds
/// that bus by these two and by nothing else: the address itself, or the runtime directory the
/// default socket sits in. Neither holds a credential. Every other variable stays outside the
/// constructed environment, as on every other route.
pub const SECRET_SERVICE_SESSION_BUS_VARIABLES: [&str; 2] =
    ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"];

/// The operations the pinned loop runs **without asking**, even at its lowest ceiling (design
/// amendment a61).
///
/// `--approve-up-to low` is the floor harness 0.20.0 offers, and its read entries are declared
/// `low` (`harness-catalogue`): `file_read`, `dir_list`, `search` and `find`. The first three render
/// to these operations; `find` has no word in the v0.1 vocabulary and reads as they do. A caller
/// seam therefore decides every write and every `run`, and none of these.
pub const UNASKED_OPERATIONS: [metaharness_protocol::Operation; 3] = [
    metaharness_protocol::Operation::FileRead,
    metaharness_protocol::Operation::DirList,
    metaharness_protocol::Operation::Search,
];

/// Why this frame cannot govern a caller-seam run, or `None` when it can (design amendment a61).
///
/// A frame says *strictly these operations*, and on this arm the reads in [`UNASKED_OPERATIONS`]
/// run whether the frame admits them or not: the loop never asks about a call below its ceiling.
/// A frame that refused them, in its operation set or in any subject rule, would therefore be text
/// the model is held to and the loop is not — the advisory frame finding F9 forbids. Refused at
/// start, by name, rather than recorded as enforced.
///
/// `None` for a frame-mode run is a refusal too: with no frame metaharness has nothing to decide
/// from and every write the loop asks about would be refused, which reads as a model that could
/// not do its work. Pass `--decisions observe` for the undecided arm, or name a frame.
#[must_use]
pub fn caller_frame_problem(
    decisions: metaharness_protocol::DecisionMode,
    frame: Option<&metaharness_protocol::Frame>,
) -> Option<String> {
    use metaharness_protocol::DecisionMode;
    let Some(frame) = frame else {
        return (decisions == DecisionMode::Frame).then(|| {
            "a frame-mode b10x run needs a frame: metaharness decides each call the loop asks \
             about from it, and with none every write would be refused. Name one with --frame, \
             or pass --decisions observe for the arm that decides nothing"
                .to_owned()
        });
    };
    let names = |operations: &[&metaharness_protocol::Operation]| {
        operations
            .iter()
            .map(|operation| operation.name())
            .collect::<Vec<_>>()
            .join(", ")
    };
    let missing: Vec<_> = UNASKED_OPERATIONS
        .iter()
        .filter(|operation| !frame.operations.admits(operation))
        .collect();
    if !missing.is_empty() {
        return Some(format!(
            "the frame does not admit {}, and the b10x loop runs every read without asking even at \
             its lowest ceiling (--approve-up-to low), so the frame would not hold for them; admit \
             {} in the frame, or run this step on another kind",
            names(&missing),
            names(&UNASKED_OPERATIONS.iter().collect::<Vec<_>>())
        ));
    }
    for rule in &frame.subjects.rules {
        let missing: Vec<_> = UNASKED_OPERATIONS
            .iter()
            .filter(|operation| !rule.operations.admits(operation))
            .collect();
        if !missing.is_empty() {
            return Some(format!(
                "the frame's subject rule for {:?} does not admit {}, and the b10x loop runs every \
                 read without asking, so the rule would not hold for them; admit the reads in every \
                 rule, or run this step on another kind",
                rule.subjects,
                names(&missing)
            ));
        }
    }
    None
}

/// Check that a secret name has the shape `<namespace>/<name>`: a first `/` with something on
/// each side of it (design amendment a50 § 5).
///
/// Only the shape. Each half's grammar is the loop's to check, before it opens the store; a copy
/// of that grammar here would be a second answer to the loop's question. So `llm/team/openai`
/// passes, as it does in the loop.
///
/// # Errors
///
/// Names the value as given, escaped as the loop escapes it, so a control character typed into
/// the name cannot rewrite the line that refuses it.
pub fn validate_api_key_secret(name: &str) -> Result<(), String> {
    match name.split_once('/') {
        Some((namespace, rest)) if !namespace.is_empty() && !rest.is_empty() => Ok(()),
        _ => Err(format!(
            "--api-key-secret `{}` is not <namespace>/<name>: the secret store names a key by a \
             namespace, a `/` and a name, each present",
            name.escape_debug()
        )),
    }
}

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
/// Harness 0.20.0 (`684c99f5`, `crates/harness-substrate/src/lib.rs` unchanged from 0.15.0)
/// canonicalizes the set before Substrate 0.7.17 (`00fa634c`) validates workspace
/// access. This adapter mirrors those syntax bounds
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
    use super::{
        HARNESS_REVISION, PINNED_VERSIONS, SECRET_KEY_SOURCE, SECRET_SERVICE_SESSION_BUS_VARIABLES,
        validate_api_key_secret,
    };

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
    /// 0.13.3 / `798325f0` → 0.15.0 / `dd5c2a66` → 0.15.1 / `f0870131` on 2026-10-20, and then →
    /// 0.18.0 / `330e2cde` on 2026-10-22 (amendment a50), → 0.19.0 / `13d72c06` on 2026-10-26
    /// (a59), and then → 0.20.0 / `684c99f5` on 2026-10-26 (a60): the commit the annotated tag
    /// `0.20.0` (tag object `ab2abfd9`) points at, never the tag object, because the eval compares
    /// this with `git rev-parse HEAD` in the harness checkout.
    #[test]
    fn the_pin_names_the_harness_0_20_0_release_commit() {
        assert_eq!(PINNED_VERSIONS, ["0.20.0"]);
        assert_eq!(HARNESS_REVISION, "684c99f5e8c7e1d22d7b29f717b71856db81766a");
    }

    /// D30: the string the pinned loop records for `--api-key-secret`, read from
    /// `crates/harness-cli/src/lib.rs:1226-1227` at `330e2cde` (`credential_source`). H4 compares
    /// the opening record with this and nothing looser.
    #[test]
    fn the_secret_route_records_the_loops_own_source_string() {
        assert_eq!(SECRET_KEY_SOURCE, "api-key:secrets");
    }

    /// D29: exactly the two variables zbus 5.19.0 finds the session bus by
    /// (`src/address/mod.rs:68`, `:76-78`), and nothing that could carry a credential.
    #[test]
    fn the_session_bus_variables_are_the_two_the_store_client_reads() {
        assert_eq!(
            SECRET_SERVICE_SESSION_BUS_VARIABLES,
            ["DBUS_SESSION_BUS_ADDRESS", "XDG_RUNTIME_DIR"]
        );
    }

    #[test]
    fn a_secret_name_is_namespace_slash_name_with_both_halves_present() {
        for good in [
            "llm/opencode-go-api-key",
            "llm/team/openai",
            "a/b",
            "-llm/opencode-go-api-key",
        ] {
            assert_eq!(validate_api_key_secret(good), Ok(()), "{good}");
        }
        for bad in [
            "",
            "opencode-go-api-key",
            "llm/",
            "/opencode-go-api-key",
            "/",
        ] {
            let refusal = validate_api_key_secret(bad).expect_err(bad);
            assert!(refusal.contains(&format!("`{bad}`")), "{refusal}");
            assert!(refusal.contains("<namespace>/<name>"), "{refusal}");
            assert!(refusal.contains("--api-key-secret"), "{refusal}");
        }
    }

    #[test]
    fn a_refused_secret_name_is_escaped_as_the_loop_escapes_it() {
        // `harness-llm/src/lib.rs:173-178` at `330e2cde` names the reference with `escape_debug`,
        // so a control character in a name reaches the operator's terminal as its escape, never
        // as itself.
        let refusal = validate_api_key_secret("bad\u{1b}[2Jname").expect_err("no slash");
        assert!(refusal.contains("`bad\\u{1b}[2Jname`"), "{refusal:?}");
        assert!(!refusal.contains('\u{1b}'), "{refusal:?}");
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
