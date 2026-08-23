//! The hermetic launch, constructed and never spawned.
//!
//! [`plan_launch`] is a pure function on the Claude adapter's rule and for the Claude adapter's
//! reason: it reads no file, no clock and no environment of its own, everything arrives in
//! [`LaunchContext`] and everything leaves as a value on [`LaunchPlan`], so the argv, the child
//! environment and the hook definition are values a test reads **before** anything is spawned
//! (design § 8.4 O7) — *"because every one of the failures would be silent"*.
//!
//! # What the caller still has to do
//!
//! | value | what the caller does with it |
//! |---|---|
//! | [`LaunchPlan::credential_copies`] | copies each `from` to each `to` **immediately before every spawn** |
//! | [`LaunchPlan::config`] | writes it to [`config_path`] — **the hook is in it** |
//! | [`LaunchPlan::hook`] | is already inside `config`; the executable it names is the caller's to place |
//! | [`LaunchPlan::config_home`], [`LaunchPlan::cwd`] | creates them, empty, before the spawn |
//!
//! # The three codex flags that carry a decision each
//!
//! | flag | why it is in the argv | evidence |
//! |---|---|---|
//! | `--dangerously-bypass-hook-trust` | **without it the seam does not exist.** A hook in a scratch `CODEX_HOME` is not a *managed* hook, and a non-managed hook needs persisted trust a fresh home cannot have — so the guard would silently never fire, which is the failure H8 is written against | `codex exec --help`, 0.145.0: *"Run enabled hooks without requiring persisted hook trust for this invocation. DANGEROUS. Intended only for automation that already vets hook sources"* |
//! | `--skip-git-repo-check` | the scratch working directory is not a git repository, and codex refuses to run outside one | `codex exec --help`, 0.145.0: *"Allow running Codex outside a Git repository"* |
//! | `--json` | the thin stream is not the record, but it is the child's own account of a run that died before it opened a session | `codex exec --help`, 0.145.0: *"Print events to stdout as JSONL"* |
//!
//! The first one deserves its full reason, because its name says the opposite of what it does
//! here. The danger it warns about is running **somebody else's** hook without vetting it. The only
//! hook in this scratch home is the one metaharness wrote into it microseconds earlier, from
//! [`crate::hook_program`], at a path this plan names — so the flag's precondition (*"automation
//! that already vets hook sources"*) is met by construction, and what it buys is the guard
//! existing at all. It is still recorded in the attestation rather than left in the argv for a
//! reader to find.

use std::collections::BTreeMap;
use std::fmt;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use metaharness_protocol::{
    CredentialSource, DecisionMode, Digest, HermeticAttestation, HermeticRow, ImposedControl,
    InstalledPlugin, Kind, PluginContent, PluginInstall, PluginTree, RefusalCode, RunSpec,
    TierStatus, ToolSurface, UnavailableControl, required_commands,
};
use serde_json::{Value, json};

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// The declared timeout of the emitted `PreToolUse` hook, in seconds.
///
/// Published because design § 7.7 rule 2 makes metaharness's own decision deadline **strictly
/// less** than the vendor's hook timeout, and a deadline set from a number nobody could read would
/// be a guarantee that depends on two files agreeing by memory.
///
/// What 0.145.0 does when a command hook exceeds this is **undriven here** and is the codex
/// counterpart of Q10; metaharness closes the window from its own side regardless.
pub const HOOK_TIMEOUT_SECONDS: u64 = 60;

/// The directory under the scratch root that becomes `CODEX_HOME`.
const CONFIG_HOME: &str = "codex-home";

/// The directory under the scratch root that becomes `TMPDIR`.
///
/// Never `/tmp`: the machine's tmpfs drops writes under pressure (design § 2.1).
const TMP_DIR: &str = "tmp";

/// The config document, inside the scratch `CODEX_HOME`. **This is where the hook is declared.**
///
/// Not `hooks.json`, and that correction is worth its lines because the obvious guess is wrong.
/// `$CODEX_HOME/hooks.json` **is not a source codex 0.145.0 reads**: a `hooks.json` is a *plugin
/// manifest's* file (`"hooks": "./hooks.json"`), and both of the ones on this machine sit inside
/// plugin directories. What this binary reads is `[hooks]` in `config.toml`, with `PascalCase`
/// event names — `PreToolUse`, `PermissionRequest`, `PostToolUse`, `SessionStart`, `Stop`, … —
/// while `pre_tool_use` and `preToolUse` are **silently ignored**.
///
/// Silently ignored is the whole danger. An unrecognised key under `[hooks]` does not fail the
/// config load; it produces a run with no seam, which looks exactly like a run in which nothing
/// was attempted. So § 7.8's rule applies here with no discount: **the seam is asserted from the
/// run's own record** — a hook request that actually arrived — and never from this file.
const CONFIG_FILE: &str = "config.toml";

/// Where the caller must place the executable the `PreToolUse` hook runs.
///
/// Under the scratch root and **not** under the config home: the home is the vendor's to read and
/// a program sitting in it would be one more file for the vendor to have an opinion about.
const HOOK_PROGRAM: &str = "hooks/pretooluse";

/// Where an injected plugin is copied to: `$CODEX_HOME/plugins/<name>`.
///
/// **Inside the config home, unlike the claude adapter's placement, and for the opposite reason.**
/// `codex exec` has no `--plugin-dir`: `codex plugin` installs from *marketplace snapshots*, so
/// there is no flag with which to name a directory and the only candidate location is one the
/// vendor itself looks at. This constant is the candidate.
///
/// **The placement is undriven, and that is a claim about this adapter, not about codex.** What is
/// read — from strings in the 0.145.0 binary, which is weaker than a driven call and is labelled
/// so at every other point in this crate:
///
/// * the binary resolves `plugins/cache` and `plugins/data` **under the Codex home**, so
///   `$CODEX_HOME/plugins` is the vendor's own neighbourhood rather than an invention here;
/// * a marketplace's plugin entries are `./plugins/<plugin-name>` relative to a marketplace root,
///   which is the same shape this constant produces with the scratch home as that root.
///
/// What is **not** known: whether 0.145.0 loads a plugin from this path with no marketplace
/// manifest and no `codex plugin add` behind it. This launch writes **no** `[marketplaces]` table
/// to go with the copy, deliberately: an unrecognised key under a table this binary reads is
/// dropped without failing the config load (see [`CONFIG_FILE`]), and a *malformed* one could fail
/// it outright — which on this vendor is a run with no seam. So the copy is made, the attestation
/// says in as many words that the load is unverified, and H1a's verdict is left to the run's own
/// record. See `docs/design/metaharness-protocol-v0.1.md` **Q19**.
const PLUGIN_HOME: &str = "plugins";

/// The emitted hook declares **no matcher at all**, which is every tool.
///
/// A matcher is an optional per-group string, and this build's own shipped plugins use both forms
/// — `"matcher": "Write|Edit"` in one, no matcher at all in another. What a matcher *means* on
/// 0.145.0 — exact, alternation, glob — is **unverified**, and there is no
/// `invalid hook matcher regex` string to lean on, so a guessed matcher risks one that quietly
/// matches nothing, which is a seam that has already stopped guarding. An absent matcher is the
/// one form whose meaning is not in doubt: the group is not narrowed. The regime that buys is
/// stated rather than discovered — a child process per shell call, per `apply_patch` and per MCP
/// call, with the latency that implies.
const MATCHER: Option<&str> = None;

/// Where the caller must write [`LaunchPlan::config`].
#[must_use]
pub fn config_path(scratch_root: &Path) -> PathBuf {
    scratch_root.join(CONFIG_HOME).join(CONFIG_FILE)
}

/// Where the caller must place the `PreToolUse` executable, which is the path the hook definition
/// already names.
#[must_use]
pub fn hook_program_path(scratch_root: &Path) -> PathBuf {
    scratch_root.join(HOOK_PROGRAM)
}

/// The environment variables copied from the caller's own, when present.
///
/// An allowlist and not a denylist, because a denylist is a list of the leaks somebody thought of
/// (design § 8.1 H3). `SHELL` is deliberately absent: the vendor's shell tool would inherit the
/// operator's login shell and its startup files with it.
const INHERITED_KEYS: [&str; 7] = ["HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TERM", "TZ"];

/// The stated `PATH` the child gets, before the operator's own `~/.local/bin` is appended.
const BASE_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Arguments this adapter must never construct.
///
/// A guard over the value, not a spelling check on the source. Each one deletes something this
/// design depends on, and every failure would be silent:
///
/// * `--ephemeral` — *"Run without persisting session files to disk"*. The session file **is** the
///   record this adapter reads, so this flag is codex's way of producing a run with no transcript,
///   no O8 bytes and nothing for the auditor.
/// * `--ignore-user-config` — *"Do not load `$CODEX_HOME/config.toml`"*. The scratch home is where
///   the seam is declared, so this is the local shape of `--bare`.
/// * `--dangerously-bypass-approvals-and-sandbox` — *"Skip all confirmation prompts and execute
///   commands without sandboxing"*. It removes the vendor's own floor beneath the seam.
/// * `--add-dir` — widens the working directory H7 says is ours.
///
/// All four strings are `codex exec --help` on 0.145.0.
const DENIED_ARGUMENTS: [&str; 4] = [
    "--ephemeral",
    "--ignore-user-config",
    "--dangerously-bypass-approvals-and-sandbox",
    "--add-dir",
];

/// Environment variables that must never reach the child, named by H8 rather than by H3's prefix
/// scrub, so a failure here is attributed to the row that forbids it.
///
/// `CODEX_API_KEY` and `OPENAI_API_KEY` are H4's, not H8's, and are handled by the scrub.
const DENIED_ENVIRONMENT: [&str; 1] = ["CODEX_DISABLE_HOOKS"];

/// Everything the launch needs that this crate will not go and find for itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchContext {
    /// The run's scratch root. The config home, the temporary directory, the hook program and the
    /// decision channel all live under it.
    pub scratch_root: PathBuf,
    /// The working directory the child is spawned in — the evidence for H7, and the directory
    /// [`LaunchContext::memory_ancestors`] was walked upward from.
    pub cwd: PathBuf,
    /// The operator's credential file, when the run declared
    /// [`CredentialSource::OperatorLogin`]. `~/.codex/auth.json`, and nothing else in that home.
    pub credentials_file: Option<PathBuf>,
    /// The caller's own environment. Read here and **not** inherited (design § 8.1 H3).
    pub inherited_env: BTreeMap<String, String>,
    /// What an ancestor walk from [`LaunchContext::cwd`] found. Non-empty is a refusal before the
    /// run, because `AGENTS.md` discovery is native to codex — root-to-cwd walk, one file per
    /// directory, observed live in rollouts as `world_state.state.agents_md` — so a memory file in
    /// any ancestor enters the context of a run this design calls hermetic (H11).
    pub memory_ancestors: Vec<PathBuf>,
    /// The digest of the copied input tree, carried into `session.started.inputs_digest` (H10).
    pub inputs_digest: Option<Digest>,
    /// Every directory `spec.plugin_dir` named, **as the caller read it** (crossing #4). The
    /// caller does the walk and the per-file digests; this function only decides where the copy
    /// goes and whether the run may proceed.
    pub plugins: Vec<PluginTree>,
}

/// One file to copy into the scratch config home, and the only one.
///
/// **This copy is performed immediately before every spawn, never once per run** (design § 8.1 H6,
/// as amended). A copied token is a snapshot with a lifetime, and a snapshot has nothing to
/// refresh against.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialCopy {
    /// The operator's file.
    pub from: PathBuf,
    /// Where it goes in the scratch config home.
    pub to: PathBuf,
}

/// The launch, as a value.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchPlan {
    /// The program to run.
    pub program: String,
    /// Its arguments, in the order they are passed.
    pub args: Vec<String>,
    /// The child's whole environment. **Constructed, not inherited** (design § 8.1 H3).
    pub env: BTreeMap<String, String>,
    /// The directory the child is spawned in.
    pub cwd: PathBuf,
    /// The scratch `CODEX_HOME`. Fresh per run, which is what keeps the operator's own config,
    /// plugins, MCP servers and session history out of it.
    pub config_home: PathBuf,
    /// What to copy into that home before **every** spawn. Exactly one entry under an operator
    /// login, and none otherwise; nothing else is ever copied (H6).
    pub credential_copies: Vec<CredentialCopy>,
    /// What to copy into the scratch `CODEX_HOME` **once**, before the child starts: one entry per
    /// declared plugin directory, each carrying the digest of what was read (crossing #4). A value
    /// on the plan, so the copy list and the digest are readable before any process exists —
    /// whatever this vendor then does with the directory (see [`PLUGIN_HOME`]).
    pub plugin_installs: Vec<PluginInstall>,
    /// The whole `config.toml` the scratch home carries, as text — **the seam included**.
    ///
    /// One document rather than two, because 0.145.0 reads its hooks out of `[hooks]` in this
    /// file and reads no standalone `hooks.json` at all (see [`CONFIG_FILE`]).
    pub config: String,
    /// The `PreToolUse` hook definition, as a value, so its shape is testable before a spawn:
    /// `type = "command"`, a stated timeout, and **no `async` key**.
    ///
    /// JSON rather than TOML because it is a value a test reads and a number
    /// `metaharness::vendor_hook_timeout_ms` derives the decision deadline from;
    /// [`LaunchPlan::config`] is where it is rendered into the form the vendor parses.
    pub hook: Value,
    /// What metaharness imposed and what it could not.
    ///
    /// **Not evidence.** It is metaharness's own claim about its own actions; the independent
    /// evidence is the vendor's own record (design § 8.3).
    pub attestation: HermeticAttestation,
}

impl LaunchPlan {
    /// The credential copies to perform **before this spawn**, and before the next one too.
    #[must_use]
    pub fn credential_copies_for_next_spawn(&self) -> &[CredentialCopy] {
        &self.credential_copies
    }
}

/// Why a launch was refused before anything was spawned.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchRefusal {
    /// The spec asks for a harness this adapter is not.
    WrongKind {
        /// What it asked for.
        asked_for: Kind,
    },
    /// The run has no prompt.
    NoPrompt,
    /// The working directory is not under the scratch root, so H7's *"a directory metaharness
    /// created"* cannot be claimed for it.
    CwdOutsideScratch {
        /// The directory asked for.
        cwd: PathBuf,
        /// The root it must be under.
        scratch_root: PathBuf,
    },
    /// The ancestor walk found memory files above the scratch working directory (H11).
    MemoryAncestorsFound {
        /// What the walk found, in the order it found them.
        found: Vec<PathBuf>,
    },
    /// The run declared an operator login and named no credential file to copy.
    CredentialFileMissing,
    /// The run declared `credentials: api-key` and no key was in the caller's environment.
    ApiKeyMissing,
    /// The run declared a model endpoint together with a real credential source.
    ///
    /// Refused rather than composed: an operator credential in the scratch home beside a
    /// foreign provider is the operator's token sitting where a run that never needs it runs —
    /// the endpoint provider names no `env_key` and sends no auth header at all (MA-V2).
    EndpointWithCredential {
        /// The endpoint that was declared.
        endpoint: String,
    },
    /// The constructed argv carries an argument the denylist forbids.
    DeniedArgument {
        /// Which one.
        argument: String,
    },
    /// The constructed environment carries a variable the denylist forbids, or one H3's scrub
    /// should have dropped.
    DeniedEnvironment {
        /// Which key.
        key: String,
        /// Which row forbids it.
        row: HermeticRow,
    },
    /// The run declared `credentials: loopback`, which this adapter does not carry yet.
    ///
    /// **Refused by name, never degraded to the copy path.** The model-adapter rule is that there
    /// is no silent fallback between adapter classes, and the copy path is exactly the thing the
    /// loopback provider exists to replace — a run that asked for "no credential in the child"
    /// and got a credential copied into the scratch home would be wrong in the direction that
    /// matters, and nothing in its record would say so.
    LoopbackUnsupported,
    /// A declared plugin directory cannot be installed (crossing #4).
    ///
    /// The same two silent-nothing cases the claude adapter refuses, refused here for the same
    /// reason: a run that installed no plugin and reported one is the untreated run wearing the
    /// treated run's label.
    PluginDirUnusable {
        /// The directory the run named.
        directory: PathBuf,
        /// Which of the two, in the words that say what to do about it.
        why: String,
    },
    /// The run asked for a decision mode this adapter has not driven.
    ///
    /// Design § 8.4 O4 over the mode table: `observe` is the `allow` half of this vendor's
    /// decision wire and only the `deny` half has been driven, so it is refused by name rather
    /// than served with a grant the vendor may discard.
    DecisionModeUnverified {
        /// The mode that was asked for.
        mode: DecisionMode,
        /// What the adapter declares about it.
        status: TierStatus,
    },
    /// The run asked for something `codex exec` has no way to express.
    ///
    /// A refusal and never a silent drop: an option the caller set and the adapter ignored is a
    /// run that is not the run they asked for, and they would find out from the bill.
    UnsupportedOption {
        /// The spec field, by the name the CLI spells.
        option: &'static str,
        /// Why this vendor surface cannot carry it.
        why: &'static str,
    },
}

impl LaunchRefusal {
    /// The protocol refusal code this maps to, where one applies.
    #[must_use]
    pub fn code(&self) -> Option<RefusalCode> {
        match self {
            LaunchRefusal::UnsupportedOption { .. }
            | LaunchRefusal::LoopbackUnsupported
            | LaunchRefusal::DecisionModeUnverified { .. } => Some(RefusalCode::UnsupportedControl),
            _ => None,
        }
    }
}

impl fmt::Display for LaunchRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchRefusal::WrongKind { asked_for } => write!(
                f,
                "this is the {ADAPTER_ID} adapter and the run asked for {}",
                asked_for.as_str()
            ),
            LaunchRefusal::NoPrompt => f.write_str(
                "the run carries no prompt, and a headless session with nothing to do is a paid \
                 call for no observation",
            ),
            LaunchRefusal::CwdOutsideScratch { cwd, scratch_root } => write!(
                f,
                "the working directory {} is not under the scratch root {}, so H7 cannot claim it \
                 is a directory metaharness created",
                cwd.display(),
                scratch_root.display()
            ),
            LaunchRefusal::MemoryAncestorsFound { found } => write!(
                f,
                "H11: the ancestor walk found {}, and AGENTS.md discovery is native to codex — a \
                 root-to-cwd walk on every run",
                join_paths(found)
            ),
            LaunchRefusal::CredentialFileMissing => f.write_str(
                "the run declared an operator login and named no credential file, so the scratch \
                 home would hold no credential at all",
            ),
            LaunchRefusal::ApiKeyMissing => f.write_str(
                "the run declared credentials: api-key and neither OPENAI_API_KEY nor \
                 CODEX_API_KEY was in the caller's environment",
            ),
            LaunchRefusal::EndpointWithCredential { endpoint } => write!(
                f,
                "the run declared a model endpoint ({endpoint}) together with a real credential \
                 source; the endpoint provider authenticates with nothing, so declare \
                 credentials: none rather than parking an operator credential in a scratch home \
                 no request will read it from"
            ),
            LaunchRefusal::DeniedArgument { argument } => write!(
                f,
                "H8: the constructed argv carries {argument}, which deletes something this run \
                 depends on"
            ),
            LaunchRefusal::DeniedEnvironment { key, row } => write!(
                f,
                "{}: the constructed child environment carries {key}",
                row.id()
            ),
            LaunchRefusal::LoopbackUnsupported => f.write_str(
                "the run declared credentials: loopback and the codex adapter does not carry the \
                 loopback provider yet: that is milestone LP-4, shaped by V-LP6, which verified \
                 that routing subscription traffic through a metaharness-owned provider is \
                 possible and whose confirming paid run has not been done. It is refused by name \
                 rather than degraded to the credential-copy path, because a run that asked for \
                 no credential in the child and got one copied into its scratch home would be \
                 wrong in the direction that matters",
            ),
            LaunchRefusal::PluginDirUnusable { directory, why } => write!(
                f,
                "--plugin-dir {} cannot be installed: {why}. It is refused rather than skipped, \
                 because a run that installed no plugin and reported one would be the untreated \
                 run wearing the treated run's label",
                directory.display()
            ),
            LaunchRefusal::DecisionModeUnverified { mode, status } => write!(
                f,
                "the run asked for --decisions {} and the {ADAPTER_ID} adapter declares that mode \
                 {}. Observe mode is the allow half of this vendor's PreToolUse wire and nothing \
                 else, and only the deny half has been driven (CX-M2); the allow half is milestone \
                 R2.4. It is refused by name rather than served with a grant this binary may \
                 discard, because a discarded allow on a capture run looks exactly like a capture \
                 that worked",
                mode.as_str(),
                match status {
                    TierStatus::Unverified => "unverified",
                    TierStatus::Absent => "absent",
                    TierStatus::Delivered =>
                        "delivered, which is not a refusal and is a defect here",
                }
            ),
            LaunchRefusal::UnsupportedOption { option, why } => write!(
                f,
                "the run asked for {option} and codex exec 0.145.0 has no way to express it: \
                 {why}. It is refused rather than dropped, because an option that was set and \
                 ignored is a run that is not the one that was asked for"
            ),
        }
    }
}

impl std::error::Error for LaunchRefusal {}

fn join_paths(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

/// Construct the launch, or refuse it.
///
/// Nothing is spawned, nothing is written and nothing is read from the machine this runs on.
///
/// # Errors
///
/// Every variant of [`LaunchRefusal`]. The order is deliberate: the conditions that make the run
/// wrong are checked before the argv exists, and the guards over the constructed argv and
/// environment run last, because a guard can only read a value that has been built.
pub fn plan_launch(spec: &RunSpec, context: &LaunchContext) -> Result<LaunchPlan, LaunchRefusal> {
    if spec.kind != Kind::Codex {
        return Err(LaunchRefusal::WrongKind {
            asked_for: spec.kind,
        });
    }
    let Some(prompt) = &spec.prompt else {
        return Err(LaunchRefusal::NoPrompt);
    };
    guard_decision_mode(spec)?;
    unsupported_options(spec)?;
    // An operator-named cwd (amendment a6) is a declaration, not a defect: the two refusals below
    // keep a *scratch* cwd honest, and a run the operator pointed at a real tree loses rows H7 and
    // H11 in the attestation instead of being refused here.
    if spec.cwd.is_none() {
        if !context.cwd.starts_with(&context.scratch_root) {
            return Err(LaunchRefusal::CwdOutsideScratch {
                cwd: context.cwd.clone(),
                scratch_root: context.scratch_root.clone(),
            });
        }
        if !context.memory_ancestors.is_empty() {
            return Err(LaunchRefusal::MemoryAncestorsFound {
                found: context.memory_ancestors.clone(),
            });
        }
    }

    if let Some(endpoint) = &spec.model_endpoint
        && spec.credentials != CredentialSource::None
    {
        return Err(LaunchRefusal::EndpointWithCredential {
            endpoint: endpoint.clone(),
        });
    }

    let config_home = context.scratch_root.join(CONFIG_HOME);
    let credential_copies = credential_copies(spec, context, &config_home)?;
    let plugin_installs = plugin_installs(spec, context, &config_home)?;
    let args = build_args(spec, prompt);
    guard_arguments(&args)?;
    let env = build_env(spec, context, &config_home)?;
    let hook = build_hook(&hook_program_path(&context.scratch_root));

    Ok(LaunchPlan {
        program: ADAPTER_ID.to_string(),
        args,
        env,
        cwd: context.cwd.clone(),
        config_home: config_home.clone(),
        credential_copies,
        config: build_config(spec, &hook),
        hook,
        attestation: attest(spec, context, &config_home, &plugin_installs),
        plugin_installs,
    })
}

/// The plugin copy list, one entry per declared directory, or the refusal that says why not.
///
/// The digest is computed **before** the copy, over the operator's own directory: it is a claim
/// about the plugin as it stood when the run took its snapshot.
fn plugin_installs(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
) -> Result<Vec<PluginInstall>, LaunchRefusal> {
    let mut installs = Vec::new();
    for directory in &spec.plugin_dir {
        let tree = context
            .plugins
            .iter()
            .find(|tree| tree.source == *directory)
            .ok_or_else(|| LaunchRefusal::PluginDirUnusable {
                directory: directory.clone(),
                why: "the caller planned a launch without reading it, so nothing digested it and \
                      there is nothing to copy"
                    .to_string(),
            })?;
        let digest = match &tree.content {
            PluginContent::Files { digest, .. } => digest.clone(),
            PluginContent::Empty => {
                return Err(LaunchRefusal::PluginDirUnusable {
                    directory: directory.clone(),
                    why: "it is a directory and it holds no file at all".to_string(),
                });
            }
            PluginContent::Unreadable { detail } => {
                return Err(LaunchRefusal::PluginDirUnusable {
                    directory: directory.clone(),
                    why: format!("it could not be read: {detail}"),
                });
            }
        };
        installs.push(PluginInstall {
            from: directory.clone(),
            to: config_home.join(PLUGIN_HOME).join(tree.name()),
            digest,
        });
    }
    Ok(installs)
}

/// What the attestation says about one installed plugin — and, on this vendor, what it refuses to
/// say.
///
/// `loaded_by` is where the honesty lives. The claude adapter's names a flag the vendor documents;
/// this one names a path read out of a binary and driven by nobody, in as many words, so a reader
/// of the record can tell the two apart without leaving it (see [`PLUGIN_HOME`]).
fn installed_plugins(installs: &[PluginInstall]) -> Vec<InstalledPlugin> {
    installs
        .iter()
        .map(|install| InstalledPlugin {
            name: install
                .to
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned()),
            source: install.from.display().to_string(),
            installed_at: install.to.display().to_string(),
            digest: install.digest.clone(),
            loaded_by: format!(
                "nothing names it to the vendor: codex exec has no --plugin-dir, and this launch \
                 writes no marketplace entry. It is copied to {} because the 0.145.0 binary keeps \
                 its own plugins under CODEX_HOME/plugins — a string read from a binary, not a \
                 driven call. **Whether codex loads it from there is unverified (Q19).** The \
                 opening record's plugin list is the only thing that can say, and H1a reads it",
                install.to.display()
            ),
        })
        .collect()
}

/// The mode this run asked for against the mode table this adapter publishes.
fn guard_decision_mode(spec: &RunSpec) -> Result<(), LaunchRefusal> {
    let status = crate::capabilities().decision_mode(spec.decisions);
    if status == TierStatus::Delivered {
        return Ok(());
    }
    Err(LaunchRefusal::DecisionModeUnverified {
        mode: spec.decisions,
        status,
    })
}

/// The spec fields `codex exec` cannot carry, refused by name.
fn unsupported_options(spec: &RunSpec) -> Result<(), LaunchRefusal> {
    // First, and before any argv or environment exists: a loopback run must not get as far as a
    // plan on this vendor, because the library reads the plan to decide what to start.
    if spec.credentials == CredentialSource::Loopback {
        return Err(LaunchRefusal::LoopbackUnsupported);
    }
    if spec.max_turns.is_some() {
        return Err(LaunchRefusal::UnsupportedOption {
            option: "--max-turns",
            why: "codex exec has no turn ceiling; the run ends when the task does, and a ceiling \
                  metaharness enforced by killing the child would be a different thing wearing \
                  the same name",
        });
    }
    // `--plugin-dir` **was** refused here, on the grounds that codex loads plugins from its own
    // config and marketplace snapshots rather than from a directory named on the command line.
    // Both halves of that sentence are still true; what changed is that the refusal was hiding a
    // mechanism this repository needs (crossing #4) behind a fact about a *flag*. The copy is now
    // planned, its placement is a named constant, and what is not known is labelled where a reader
    // meets it — `PLUGIN_HOME`, the attestation's `loaded_by`, and Q19 — instead of being refused
    // as if it were impossible.
    if spec.tool_surface == ToolSurface::Owned {
        return Err(LaunchRefusal::UnsupportedOption {
            option: "--tool-surface owned",
            why: "strategy C needs `dynamicTools` at thread/start, which is an app-server surface \
                  and not `codex exec`",
        });
    }
    Ok(())
}

/// The copy list, which is one file or none.
fn credential_copies(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
) -> Result<Vec<CredentialCopy>, LaunchRefusal> {
    match spec.credentials {
        CredentialSource::OperatorLogin => {
            let Some(from) = &context.credentials_file else {
                return Err(LaunchRefusal::CredentialFileMissing);
            };
            Ok(vec![CredentialCopy {
                from: from.clone(),
                to: config_home.join("auth.json"),
            }])
        }
        CredentialSource::ApiKey => {
            if api_key(&context.inherited_env).is_some() {
                Ok(Vec::new())
            } else {
                Err(LaunchRefusal::ApiKeyMissing)
            }
        }
        // Unreachable through `plan_launch`, which refuses loopback before it gets here. Repeated
        // rather than left to an `unreachable!`, so a future caller that reaches this function by
        // another road gets the refusal instead of a panic or an empty copy list that would read
        // as "loopback is supported and copies nothing".
        CredentialSource::Loopback => Err(LaunchRefusal::LoopbackUnsupported),
        CredentialSource::None => Ok(Vec::new()),
    }
}

/// The key an api-key run would use, and which variable it came from.
fn api_key(env: &BTreeMap<String, String>) -> Option<(&'static str, String)> {
    for key in ["OPENAI_API_KEY", "CODEX_API_KEY"] {
        if let Some(value) = env.get(key) {
            return Some((
                if key == "OPENAI_API_KEY" {
                    "OPENAI_API_KEY"
                } else {
                    "CODEX_API_KEY"
                },
                value.clone(),
            ));
        }
    }
    None
}

/// The command line.
///
/// The prompt is **last**, because it is positional and a flag added after it would be read as
/// part of it.
fn build_args(spec: &RunSpec, prompt: &str) -> Vec<String> {
    let mut args = vec![
        "exec".to_string(),
        // Not the record — the record is the rollout — but the only account of a run that died
        // before it opened a session.
        "--json".to_string(),
        // Without it a hook in a fresh CODEX_HOME is not trusted and never fires, and a seam that
        // never fires is the silent failure H8 exists for. See this module's header.
        "--dangerously-bypass-hook-trust".to_string(),
        // The scratch working directory is not a git repository.
        "--skip-git-repo-check".to_string(),
        // stdout is a JSONL stream and stderr is for people; colour codes in either are noise in
        // a retained record.
        "--color".to_string(),
        "never".to_string(),
    ];
    if let Some(model) = &spec.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }
    args.push("--".to_string());
    args.push(prompt.to_string());
    args
}

/// The child environment, constructed from [`INHERITED_KEYS`] and then checked.
fn build_env(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
) -> Result<BTreeMap<String, String>, LaunchRefusal> {
    let mut env = BTreeMap::new();
    for key in INHERITED_KEYS {
        if let Some(value) = context.inherited_env.get(key) {
            env.insert(key.to_string(), value.clone());
        }
    }
    env.insert("PATH".to_string(), reduced_path(&env));
    env.insert(
        "TMPDIR".to_string(),
        context.scratch_root.join(TMP_DIR).display().to_string(),
    );
    env.insert("CODEX_HOME".to_string(), config_home.display().to_string());
    if spec.credentials == CredentialSource::ApiKey
        && let Some((key, value)) = api_key(&context.inherited_env)
    {
        env.insert(key.to_string(), value);
    }
    guard_environment(&env, spec.credentials)?;
    Ok(env)
}

/// The stated `PATH`, and why it is not shorter.
///
/// The operator's `~/.local/bin` is on it because that is where `codex` is installed, and a child
/// that cannot find its own program is not a hermetic run, it is no run.
fn reduced_path(env: &BTreeMap<String, String>) -> String {
    child_path(env.get("HOME").map(String::as_str))
}

/// The `PATH` a spawned child gets, given the inherited `HOME`.
///
/// Public because `doctor` must resolve the vendor binary **the way the spawn will** (CT-3).
/// Q18's cause was exactly the two resolutions disagreeing: this machine holds a pacman codex
/// 0.145.0 at `/usr/bin` and an npm codex 0.144.0 at `~/.local/bin`, the operator's shell puts
/// `/usr/bin` first, and this constructed `PATH` puts `~/.local/bin` first — so the pre-flight
/// blessed a binary the run never executed, and the run's own record told the truth about it.
#[must_use]
pub fn child_path(home: Option<&str>) -> String {
    match home {
        Some(home) => format!("{home}/.local/bin:{BASE_PATH}"),
        None => BASE_PATH.to_string(),
    }
}

/// H8 over the argv, as a guard rather than as a spelling check.
fn guard_arguments(args: &[String]) -> Result<(), LaunchRefusal> {
    for argument in args {
        let head = argument.split('=').next().unwrap_or(argument);
        if DENIED_ARGUMENTS.contains(&head) {
            return Err(LaunchRefusal::DeniedArgument {
                argument: argument.clone(),
            });
        }
    }
    Ok(())
}

/// H8's environment half, then H3's scrub, over the constructed map.
fn guard_environment(
    env: &BTreeMap<String, String>,
    credentials: CredentialSource,
) -> Result<(), LaunchRefusal> {
    for key in env.keys() {
        if DENIED_ENVIRONMENT.contains(&key.as_str()) {
            return Err(LaunchRefusal::DeniedEnvironment {
                key: key.clone(),
                row: HermeticRow::H8,
            });
        }
        if is_scrubbed(key, credentials) {
            return Err(LaunchRefusal::DeniedEnvironment {
                key: key.clone(),
                row: HermeticRow::H3,
            });
        }
    }
    Ok(())
}

/// Whether this key must be absent from the child, given what the run declared.
///
/// `CODEX_HOME` is the one variable this adapter sets on purpose and is therefore exempt from the
/// `CODEX_` sweep — the sweep exists to catch the operator's own, not metaharness's.
fn is_scrubbed(key: &str, credentials: CredentialSource) -> bool {
    if key == "CODEX_HOME" {
        return false;
    }
    if key == "OPENAI_API_KEY" || key == "CODEX_API_KEY" {
        return credentials != CredentialSource::ApiKey;
    }
    key.starts_with("OPENAI_")
        || key.starts_with("CODEX_")
        || key.starts_with("DISABLE_")
        || key.starts_with("GIT_")
        || key == "SSH_AUTH_SOCK"
        || key.to_ascii_uppercase().ends_with("_PROXY")
}

/// Whether this run's configuration will need a decision at call time.
fn needs_call_seam(spec: &RunSpec) -> bool {
    required_commands(spec)
        .iter()
        .any(|name| matches!(*name, "tool.decide" | "frame.set"))
}

/// The `PreToolUse` hook definition.
///
/// `type: command`, which is the **only handler type that works** on 0.145.0 — the binary carries
/// `: prompt hooks are not supported yet` and `: agent hooks are not supported yet` beside it —
/// and no matcher, which is every tool (see [`MATCHER`]).
///
/// There is deliberately **no `async` key**, and its absence is asserted rather than assumed. The
/// field parses on this vendor and is rejected at runtime (`: async hooks are not supported yet`),
/// which is a *stronger* reason to keep it out than the Claude adapter has: there, an `async` hook
/// *"runs in background without blocking"* and is a guard that has stopped guarding (V7b, finding
/// F6); here it would be a run that fails on its own configuration. Either way the assertion is
/// the same and it is a value a test reads (§ 8.4 O7).
fn build_hook(hook_path: &Path) -> Value {
    let mut group = serde_json::Map::new();
    if let Some(matcher) = MATCHER {
        group.insert("matcher".to_string(), json!(matcher));
    }
    group.insert(
        "hooks".to_string(),
        json!([{
            "type": "command",
            "command": hook_path.display().to_string(),
            "timeout": HOOK_TIMEOUT_SECONDS,
        }]),
    );
    Value::Object(group)
}

/// The `config.toml` the scratch home carries, seam and all.
///
/// Rendered by hand rather than through a TOML serializer, because the whole document is four
/// keys metaharness chose and a dependency for it would be a dependency on the shape of a file
/// this function already states in full. Every key here is one the 0.145.0 deserializer
/// recognises — `[[hooks.PreToolUse]]`, `[[hooks.PreToolUse.hooks]]`, `type`, `command`,
/// `timeout` — and an unrecognised one would be **ignored in silence**, which is why the seam is
/// asserted from a hook request that arrived and never from this text.
fn build_config(spec: &RunSpec, hook: &Value) -> String {
    let mut document = String::from(
        "# metaharness scratch CODEX_HOME. Generated per run; edited by nobody.\n\n\
         # The seam is the only thing that may refuse a call. `codex exec` on 0.145.0 has no\n\
         # --ask-for-approval flag, so the posture is a config key or nothing, and the operator\'s\n\
         # own default is `on-request` — which in a headless run means a call can be turned away\n\
         # by a prompt nobody is there to answer, before the hook ever sees it. `never` makes the\n\
         # hook the one thing that decides, so a denial is attributable to metaharness.\n\
         approval_policy = \"never\"\n\n\
         # The vendor\'s own floor beneath the seam, and Claude Code has no counterpart for it\n\
         # (design § 7.4, V17): the child cannot write outside its workspace or reach the network,\n\
         # whatever the seam allows. Named here so the attestation can claim it.\n\
         sandbox_mode = \"read-only\"\n",
    );
    // Top-level keys must precede the first table header, or TOML files them under it.
    if let Some(effort) = &spec.effort {
        let _ = writeln!(document, "\nmodel_reasoning_effort = {}", quote(effort));
    }
    if let Some(endpoint) = &spec.model_endpoint {
        // The generic model adapter (MA-1): the run's brain is this gateway's, reached on the
        // Responses wire at {root}/v1/responses. No `env_key`, and therefore no auth header at
        // all — verified against the pin (MA-V2): a provider that names no env_key spawns and
        // runs with no auth.json in the scratch home and no Authorization on the wire.
        let base = format!("{}/v1", endpoint.trim_end_matches('/'));
        let _ = writeln!(
            document,
            "\n# The declared model endpoint. Everything about it is this launch's choice;\n\
             # the operator's own provider list never enters a scratch home.\n\
             model_provider = \"metaharness_endpoint\"\n\n\
             [model_providers.metaharness_endpoint]\n\
             name = \"metaharness_endpoint\"\n\
             base_url = {}\n\
             wire_api = \"responses\"",
            quote(&base)
        );
    }
    document.push_str(
        "\n# H5: the MCP surface is exactly what this launch gave, which is nothing.\n\
         [mcp_servers]\n\n\
         # The control seam (design § 7.1, call tier). 0.145.0 reads its hooks from this table\n\
         # and not from a hooks.json, which is a plugin manifest's file.\n",
    );
    if let Some(matcher) = hook.get("matcher").and_then(Value::as_str) {
        let _ = writeln!(
            document,
            "[[hooks.PreToolUse]]\nmatcher = {}",
            quote(matcher)
        );
    } else {
        document.push_str("[[hooks.PreToolUse]]\n");
    }
    for handler in hook["hooks"].as_array().into_iter().flatten() {
        document.push_str("\n[[hooks.PreToolUse.hooks]]\n");
        for key in ["type", "command"] {
            if let Some(value) = handler.get(key).and_then(Value::as_str) {
                let _ = writeln!(document, "{key} = {}", quote(value));
            }
        }
        if let Some(timeout) = handler.get("timeout").and_then(Value::as_u64) {
            let _ = writeln!(document, "timeout = {timeout}");
        }
    }
    document
}

/// One TOML basic string.
///
/// The values here are paths and words metaharness chose, and escaping them is still not optional:
/// a scratch root with a quote or a backslash in it would otherwise produce a config file that
/// parses as something else — or does not parse, which on this vendor is a run with no seam.
fn quote(value: &str) -> String {
    let escaped = value.replace('\\', "\\\\").replace('"', "\\\"");
    format!("\"{escaped}\"")
}

/// What metaharness claims it imposed, and what it says it could not.
fn attest(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
    plugin_installs: &[PluginInstall],
) -> HermeticAttestation {
    let home = config_home.display();
    let mut imposed = vec![
        control_imposed(
            HermeticRow::H1a,
            plugin_posture(&home.to_string(), plugin_installs),
        ),
        control_imposed(HermeticRow::H1b, format!("CODEX_HOME={home}")),
        control_imposed(
            HermeticRow::H2,
            format!(
                "the only config sources are {home}/config.toml and {home}/hooks.json, both \
                 written by this launch; --ignore-user-config is never passed, because it would \
                 switch off the file the seam is declared in"
            ),
        ),
        control_imposed(
            HermeticRow::H3,
            format!(
                "the child environment is constructed from an allowlist of {} keys plus PATH, \
                 TMPDIR and CODEX_HOME",
                INHERITED_KEYS.len()
            ),
        ),
        control_imposed(HermeticRow::H4, api_key_posture(spec.credentials)),
        control_imposed(
            HermeticRow::H5,
            "the scratch config.toml declares an empty [mcp_servers] and the scratch home has no \
             other source; codex has no --strict-mcp-config, so this is the whole of it",
        ),
        control_imposed(
            HermeticRow::H8,
            "neither --ephemeral, --ignore-user-config nor \
             --dangerously-bypass-approvals-and-sandbox is in the argv, and \
             --dangerously-bypass-hook-trust is, which is what makes a hook in a fresh CODEX_HOME \
             fire at all; approval_policy = \"never\" in the scratch config, so no vendor prompt \
             can turn a call away before the seam sees it",
        ),
    ];
    let mut unavailable = vec![control_unavailable(
        HermeticRow::H9,
        format!(
            "this plan spawns nothing and runs no doctor; the pin is {} and the version is \
             asserted from the rollout's session_meta.cli_version",
            PINNED_VERSIONS.join(", ")
        ),
    )];

    attest_cwd(spec, context, &mut imposed, &mut unavailable);

    if spec.credentials == CredentialSource::OperatorLogin {
        imposed.push(control_imposed(
            HermeticRow::H6,
            "one auth.json copied into the scratch home immediately before every spawn, and \
             nothing else (Q13)",
        ));
    } else {
        unavailable.push(control_unavailable(
            HermeticRow::H6,
            "the run declared no operator login, so there is no credential file to copy",
        ));
    }
    match &context.inputs_digest {
        Some(digest) => imposed.push(control_imposed(
            HermeticRow::H10,
            format!("the copied input tree digests to {digest}"),
        )),
        None => unavailable.push(control_unavailable(
            HermeticRow::H10,
            "the caller declared no inputs digest, so nothing pins the copied tree",
        )),
    }
    imposed.sort_by_key(|control| control.row);
    unavailable.sort_by_key(|control| control.row);

    HermeticAttestation {
        mode: spec.hermetic,
        decisions: spec.decisions,
        imposed,
        unavailable,
        ambient_inputs: ambient_inputs(spec),
        installed_plugins: installed_plugins(plugin_installs),
    }
}

/// H1a's `how`: the home is scratch, and what was put in it — with this vendor's caveat attached.
fn plugin_posture(config_home: &str, installs: &[PluginInstall]) -> String {
    if installs.is_empty() {
        return format!(
            "CODEX_HOME={config_home}, and no plugin was injected: the declared set is empty"
        );
    }
    let names: Vec<String> = installs
        .iter()
        .map(|install| {
            install
                .to
                .file_name()
                .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
        })
        .collect();
    format!(
        "CODEX_HOME={config_home}, and the declared set is {names:?} — each copied into the scratch \
         home at launch and digested before the copy. **Nothing names them to the vendor**: this \
         binary has no --plugin-dir and this launch writes no marketplace entry, so whether they \
         are loaded is unverified (Q19) and the record is what answers it"
    )
}

/// H7 and H11, which are impositions only over a scratch working directory.
///
/// An operator-named directory (amendment a6) is real work in a real tree: the rows are attested
/// unavailable with the declaration named, which is what makes `--hermetic strict` refuse such a
/// run instead of this attestation quietly claiming a directory metaharness never made.
fn attest_cwd(
    spec: &RunSpec,
    context: &LaunchContext,
    imposed: &mut Vec<ImposedControl>,
    unavailable: &mut Vec<UnavailableControl>,
) {
    if spec.cwd.is_some() {
        unavailable.push(control_unavailable(
            HermeticRow::H7,
            format!(
                "the run was pointed at the operator's directory {} (--cwd), which metaharness did \
                 not create; --add-dir is still never passed",
                context.cwd.display()
            ),
        ));
        unavailable.push(control_unavailable(
            HermeticRow::H11,
            format!(
                "the ancestor walk from the operator's directory found {} memory file(s), and the \
                 operator declared the tree as the run's context by naming it",
                context.memory_ancestors.len()
            ),
        ));
    } else {
        imposed.push(control_imposed(
            HermeticRow::H7,
            format!(
                "cwd {} is under the scratch root, and --add-dir is never passed",
                context.cwd.display()
            ),
        ));
        imposed.push(control_imposed(
            HermeticRow::H11,
            "the ancestor walk from the scratch cwd found no AGENTS.md and no CLAUDE.md",
        ));
    }
}

fn api_key_posture(credentials: CredentialSource) -> &'static str {
    if credentials == CredentialSource::ApiKey {
        "an API key is in the child environment because the run declared credentials: api-key"
    } else {
        "neither OPENAI_API_KEY nor CODEX_API_KEY is in the child environment"
    }
}

/// Inputs metaharness reports and does **not** claim to have removed.
fn ambient_inputs(spec: &RunSpec) -> Vec<String> {
    let mut inputs = vec![
        "the vendor's own .system skills: a scratch CODEX_HOME isolates the operator's config, \
         and codex's built-in skills still appear (design § 2.5)"
            .to_string(),
        "git state: codex records the working tree's commit and branch in session_meta, so the \
         repository the run stands in is an input to it"
            .to_string(),
        "the rollout format carries no documented stability guarantee, so what this record does \
         not say is not evidence that it did not happen (design § 2.5)"
            .to_string(),
    ];
    if !needs_call_seam(spec) {
        inputs.push(
            "this run needs no call-level decision, so the installed PreToolUse hook adjudicates \
             from the frame alone and an embedder is never asked"
                .to_string(),
        );
    }
    inputs
}

fn control_imposed(row: HermeticRow, how: impl Into<String>) -> ImposedControl {
    ImposedControl {
        row,
        how: how.into(),
    }
}

fn control_unavailable(row: HermeticRow, why: impl Into<String>) -> UnavailableControl {
    UnavailableControl {
        row,
        why: why.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::HermeticMode;

    fn context() -> LaunchContext {
        LaunchContext {
            scratch_root: PathBuf::from("/scratch/run-1"),
            cwd: PathBuf::from("/scratch/run-1/work"),
            credentials_file: Some(PathBuf::from("/operator/.codex/auth.json")),
            inherited_env: [
                ("HOME", "/operator"),
                ("USER", "operator"),
                ("LANG", "C.UTF-8"),
                ("OPENAI_BASE_URL", "https://example.invalid"),
                ("HTTPS_PROXY", "http://proxy.invalid:8080"),
                ("CODEX_DISABLE_HOOKS", "1"),
                ("SSH_AUTH_SOCK", "/run/agent.sock"),
                ("GIT_DIR", "/operator/repo/.git"),
                ("DISABLE_TELEMETRY", "1"),
            ]
            .into_iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
            memory_ancestors: Vec::new(),
            inputs_digest: Some(Digest::of(b"inputs")),
            plugins: Vec::new(),
        }
    }

    /// A spec and a context that agree on one declared plugin, as the builder produces them.
    fn plugin_world() -> (RunSpec, LaunchContext, Digest) {
        let source = PathBuf::from("/operator/integrations/codex");
        let files: BTreeMap<String, Digest> = [
            (".codex-plugin/plugin.json".to_string(), Digest::of(b"{}")),
            ("hooks/hooks.json".to_string(), Digest::of(b"[]")),
        ]
        .into_iter()
        .collect();
        let digest = metaharness_protocol::tree_digest(&files);
        let mut spec = spec();
        spec.plugin_dir.push(source.clone());
        let mut context = context();
        context.plugins.push(PluginTree {
            source,
            content: PluginContent::Files {
                count: files.len(),
                digest: digest.clone(),
            },
        });
        (spec, context, digest)
    }

    fn spec() -> RunSpec {
        let mut spec = RunSpec::new(Kind::Codex);
        spec.hermetic = HermeticMode::On;
        spec.prompt = Some("do the thing".to_string());
        spec
    }

    fn plan() -> LaunchPlan {
        plan_launch(&spec(), &context()).expect("the run plans")
    }

    /// The model-adapter door (MA-1): a declared endpoint becomes a provider entry in the
    /// scratch config — `{root}/v1` on the Responses wire, no `env_key` — and the top-level
    /// keys land **above** the first table header, because a TOML key written below one is
    /// silently a different key.
    #[test]
    fn a_declared_model_endpoint_becomes_a_no_credential_provider_entry() {
        let mut spec = spec();
        spec.credentials = CredentialSource::None;
        spec.model_endpoint = Some("https://llmgw.example/".to_string());
        spec.effort = Some("medium".to_string());
        let plan = plan_launch(&spec, &context()).expect("the endpoint launch plans");
        assert!(plan.credential_copies.is_empty(), "no auth.json travels");
        let config = &plan.config;
        for line in [
            "model_provider = \"metaharness_endpoint\"",
            "base_url = \"https://llmgw.example/v1\"",
            "wire_api = \"responses\"",
            "model_reasoning_effort = \"medium\"",
        ] {
            assert!(config.contains(line), "missing {line} in:\n{config}");
        }
        let first_table = config.find('[').expect("a table header exists");
        for key in ["model_provider = ", "model_reasoning_effort = "] {
            let at = config.find(key).expect("the key exists");
            assert!(
                at < first_table,
                "{key} sits below a table header:\n{config}"
            );
        }
        assert!(!config.contains("env_key"), "no auth variable is named");
    }

    /// An endpoint beside an operator credential is refused by name: the provider never reads
    /// it, so a copied auth.json would sit in the scratch home for nothing.
    #[test]
    fn a_model_endpoint_with_an_operator_credential_is_refused_by_name() {
        let mut spec = spec();
        spec.model_endpoint = Some("https://llmgw.example".to_string());
        assert_eq!(
            plan_launch(&spec, &context()).expect_err("the composition is refused"),
            LaunchRefusal::EndpointWithCredential {
                endpoint: "https://llmgw.example".to_string()
            }
        );
    }

    #[test]
    fn the_config_home_is_scratch_and_the_child_is_told_where_it_is() {
        let plan = plan();
        assert_eq!(plan.config_home, PathBuf::from("/scratch/run-1/codex-home"));
        assert_eq!(
            plan.env.get("CODEX_HOME").map(String::as_str),
            Some("/scratch/run-1/codex-home")
        );
    }

    #[test]
    fn tmpdir_is_in_the_scratch_tree_and_is_never_slash_tmp() {
        let plan = plan();
        let tmpdir = plan.env.get("TMPDIR").expect("TMPDIR is set");
        assert_eq!(tmpdir, "/scratch/run-1/tmp");
        assert!(!tmpdir.starts_with("/tmp"));
    }

    #[test]
    fn the_child_environment_is_constructed_and_not_inherited() {
        let plan = plan();
        for key in [
            "OPENAI_API_KEY",
            "CODEX_API_KEY",
            "OPENAI_BASE_URL",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "CODEX_DISABLE_HOOKS",
            "DISABLE_TELEMETRY",
            "SSH_AUTH_SOCK",
            "GIT_DIR",
        ] {
            assert!(!plan.env.contains_key(key), "{key} reached the child");
        }
        assert_eq!(plan.env.get("HOME").map(String::as_str), Some("/operator"));
    }

    #[test]
    fn the_path_is_a_stated_set_and_not_the_operators_own() {
        assert_eq!(
            plan().env.get("PATH").map(String::as_str),
            Some("/operator/.local/bin:/usr/local/bin:/usr/bin:/bin")
        );
    }

    #[test]
    fn the_api_key_reaches_the_child_only_when_the_run_declared_it() {
        let mut spec = spec();
        spec.credentials = CredentialSource::ApiKey;
        let mut context = context();
        context
            .inherited_env
            .insert("OPENAI_API_KEY".to_string(), "sk-test".to_string());
        let plan = plan_launch(&spec, &context).expect("the api-key run plans");
        assert_eq!(
            plan.env.get("OPENAI_API_KEY").map(String::as_str),
            Some("sk-test")
        );
        assert!(plan.credential_copies.is_empty());
    }

    #[test]
    fn an_api_key_run_with_no_key_is_refused_rather_than_started_uncredentialed() {
        let mut spec = spec();
        spec.credentials = CredentialSource::ApiKey;
        assert_eq!(
            plan_launch(&spec, &context()),
            Err(LaunchRefusal::ApiKeyMissing)
        );
    }

    /// Exactly one file is copied, and it is the credential. Anything else in that home would be
    /// an operator artefact in a directory H1a says is scratch.
    #[test]
    fn the_credential_copy_is_one_file_and_nothing_else() {
        let plan = plan();
        assert_eq!(
            plan.credential_copies,
            vec![CredentialCopy {
                from: PathBuf::from("/operator/.codex/auth.json"),
                to: PathBuf::from("/scratch/run-1/codex-home/auth.json"),
            }]
        );
        assert_eq!(plan.credential_copies_for_next_spawn().len(), 1);
    }

    /// The three flags that carry a decision each, asserted as values rather than left in a
    /// comment — and the prompt last, after `--`, so it is never read as one of them.
    #[test]
    fn the_argv_carries_the_three_load_bearing_flags_and_the_prompt_last() {
        let plan = plan();
        assert_eq!(plan.args[0], "exec");
        for flag in [
            "--json",
            "--dangerously-bypass-hook-trust",
            "--skip-git-repo-check",
        ] {
            assert!(plan.args.contains(&flag.to_string()), "{flag}");
        }
        assert_eq!(plan.args[plan.args.len() - 2], "--");
        assert_eq!(plan.args[plan.args.len() - 1], "do the thing");
    }

    /// `--ephemeral` would leave the run with no session file, which is the record this adapter
    /// reads; `--ignore-user-config` would switch off the file the seam is declared in.
    #[test]
    fn the_argv_denylist_is_a_guard_over_the_value_and_not_a_spelling_check() {
        for argument in DENIED_ARGUMENTS {
            let args = vec!["exec".to_string(), argument.to_string()];
            assert_eq!(
                guard_arguments(&args),
                Err(LaunchRefusal::DeniedArgument {
                    argument: argument.to_string()
                })
            );
        }
        assert_eq!(
            guard_arguments(&["--add-dir=/elsewhere".to_string()]),
            Err(LaunchRefusal::DeniedArgument {
                argument: "--add-dir=/elsewhere".to_string()
            })
        );
    }

    #[test]
    fn a_widened_allowlist_is_caught_by_the_scrub_rather_than_reaching_the_child() {
        let env = BTreeMap::from([("OPENAI_BASE_URL".to_string(), "x".to_string())]);
        assert_eq!(
            guard_environment(&env, CredentialSource::OperatorLogin),
            Err(LaunchRefusal::DeniedEnvironment {
                key: "OPENAI_BASE_URL".to_string(),
                row: HermeticRow::H3,
            })
        );
    }

    /// `CODEX_HOME` is the one `CODEX_`-prefixed variable this adapter sets on purpose: the sweep
    /// exists to catch the operator's own, and a sweep that caught this one would refuse every run.
    #[test]
    fn the_codex_home_this_adapter_sets_is_not_caught_by_its_own_sweep() {
        assert!(!is_scrubbed("CODEX_HOME", CredentialSource::OperatorLogin));
        assert!(is_scrubbed(
            "CODEX_UNSAFE_ALLOW_NO_SANDBOX",
            CredentialSource::OperatorLogin
        ));
    }

    /// A memory file above the scratch cwd enters the context of a run this design calls
    /// hermetic — and on codex `AGENTS.md` discovery is native, not optional.
    #[test]
    fn a_non_empty_ancestor_walk_is_a_refusal_before_the_run() {
        let mut context = context();
        context.memory_ancestors = vec![PathBuf::from("/scratch/AGENTS.md")];
        let refusal = plan_launch(&spec(), &context).expect_err("H11 refuses");
        assert_eq!(
            refusal,
            LaunchRefusal::MemoryAncestorsFound {
                found: vec![PathBuf::from("/scratch/AGENTS.md")]
            }
        );
    }

    /// A hook that does not block is a guard that has already stopped guarding, so the blocking
    /// form is asserted and the unsupported forms' absence with it.
    #[test]
    fn the_hook_is_a_blocking_command_hook_with_no_async_key() {
        let plan = plan();
        // No matcher: the one form whose meaning on this vendor is not in doubt.
        assert!(plan.hook.get("matcher").is_none());
        let entry = &plan.hook["hooks"][0];
        assert_eq!(entry["type"], json!("command"));
        assert_eq!(entry["timeout"], json!(HOOK_TIMEOUT_SECONDS));
        assert!(entry.get("async").is_none());
        assert_eq!(
            entry["command"],
            json!("/scratch/run-1/hooks/pretooluse".to_string())
        );
    }

    /// The seam is declared in `config.toml`, in the table 0.145.0 actually reads, spelled the way
    /// it actually spells it. A `hooks.json` beside it would be a plugin manifest's file and this
    /// vendor would never look at it — which is the failure mode § 7.8 is written against, because
    /// an ignored hook and an unattempted call look identical.
    #[test]
    fn the_seam_is_declared_in_the_config_table_this_vendor_reads() {
        let config = plan().config;
        assert!(config.contains("[[hooks.PreToolUse]]"), "{config}");
        assert!(config.contains("[[hooks.PreToolUse.hooks]]"), "{config}");
        assert!(config.contains(r#"type = "command""#), "{config}");
        assert!(
            config.contains(r#"command = "/scratch/run-1/hooks/pretooluse""#),
            "{config}"
        );
        assert!(config.contains("timeout = 60"), "{config}");
        // `pre_tool_use` and `preToolUse` are silently ignored by this deserializer, so a seam
        // spelled either way would be no seam at all.
        assert!(!config.contains("pre_tool_use"), "{config}");
        assert!(!config.contains("preToolUse"), "{config}");
        // The one handler type that works on 0.145.0; `prompt` and `agent` parse and are then
        // refused at runtime.
        assert!(!config.contains("async"), "{config}");
    }

    /// A path with a quote in it must not be able to close the string it is in: a config that
    /// parses as something else is, on this vendor, a run with no seam.
    #[test]
    fn a_path_that_could_close_its_own_toml_string_is_escaped() {
        let mut context = context();
        context.scratch_root = PathBuf::from(r#"/scratch/od"d\one"#);
        context.cwd = context.scratch_root.join("work");
        let plan = plan_launch(&spec(), &context).expect("plans");
        assert!(plan.config.contains(r#"\"d\\one"#), "{}", plan.config);
    }

    /// The config document goes **inside** the scratch `CODEX_HOME`, which is the opposite of
    /// where the Claude adapter puts its settings — because this is the file codex reads and
    /// there is no `--setting-sources` to switch that source off.
    #[test]
    fn the_config_document_is_inside_the_scratch_config_home() {
        let plan = plan();
        assert_eq!(
            config_path(Path::new("/scratch/run-1")),
            PathBuf::from("/scratch/run-1/codex-home/config.toml")
        );
        assert!(config_path(Path::new("/scratch/run-1")).starts_with(&plan.config_home));
        assert!(!hook_program_path(Path::new("/scratch/run-1")).starts_with(&plan.config_home));
    }

    #[test]
    fn the_scratch_config_declares_an_empty_mcp_surface() {
        assert!(plan().config.contains("[mcp_servers]"));
    }

    #[test]
    fn the_attestation_says_what_was_imposed_and_names_what_could_not_be() {
        let plan = plan();
        for row in [
            HermeticRow::H1a,
            HermeticRow::H1b,
            HermeticRow::H2,
            HermeticRow::H3,
            HermeticRow::H4,
            HermeticRow::H5,
            HermeticRow::H6,
            HermeticRow::H7,
            HermeticRow::H8,
            HermeticRow::H10,
            HermeticRow::H11,
        ] {
            assert!(plan.attestation.claims(row), "{} not claimed", row.id());
        }
        assert!(!plan.attestation.claims(HermeticRow::H9));
    }

    /// The trust flag is named in the attestation rather than left in the argv for a reader to
    /// find: it is the reason the guard exists at all, and a control nobody can see is one nobody
    /// can check.
    #[test]
    fn the_hook_trust_flag_is_named_in_the_attestation() {
        let plan = plan();
        let h8 = plan
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H8)
            .expect("H8 is imposed");
        assert!(
            h8.how.contains("--dangerously-bypass-hook-trust"),
            "{}",
            h8.how
        );
    }

    #[test]
    fn codexs_own_system_skills_are_reported_as_ambient_and_not_claimed_removed() {
        assert!(
            plan()
                .attestation
                .ambient_inputs
                .iter()
                .any(|input| input.contains(".system skills"))
        );
    }

    /// The declaration that trades two rows for real work (amendment a6).
    #[test]
    fn an_operator_named_cwd_plans_and_gives_up_h7_and_h11_by_name() {
        let mut spec = spec();
        spec.cwd = Some(PathBuf::from("/operator/repo"));
        let mut context = context();
        context.cwd = PathBuf::from("/operator/repo");
        context.memory_ancestors = vec![PathBuf::from("/operator/repo/AGENTS.md")];
        let plan = plan_launch(&spec, &context).expect("an operator cwd plans");
        for row in [HermeticRow::H7, HermeticRow::H11] {
            assert!(!plan.attestation.claims(row), "{} claimed", row.id());
            let unavailable = plan
                .attestation
                .unavailable
                .iter()
                .find(|control| control.row == row)
                .unwrap_or_else(|| panic!("{} must be attested unavailable", row.id()));
            assert!(unavailable.why.contains("operator"), "{}", unavailable.why);
        }
        assert_eq!(plan.cwd, PathBuf::from("/operator/repo"));
    }

    #[test]
    fn a_working_directory_outside_the_scratch_root_is_refused() {
        let mut context = context();
        context.cwd = PathBuf::from("/operator/repo");
        assert!(matches!(
            plan_launch(&spec(), &context),
            Err(LaunchRefusal::CwdOutsideScratch { .. })
        ));
    }

    /// An option this vendor surface cannot carry is refused by name, never dropped: the caller
    /// would otherwise find out from the bill.
    #[test]
    fn an_option_codex_exec_cannot_express_is_refused_and_never_ignored() {
        let mut capped = spec();
        capped.max_turns = Some(3);
        assert!(matches!(
            plan_launch(&capped, &context()),
            Err(LaunchRefusal::UnsupportedOption {
                option: "--max-turns",
                ..
            })
        ));

        let mut owned = spec();
        owned.tool_surface = ToolSurface::Owned;
        let refusal = plan_launch(&owned, &context()).expect_err("refused");
        assert_eq!(refusal.code(), Some(RefusalCode::UnsupportedControl));
    }

    // ------------------------------------------------------------ observe mode (a10) and #4

    /// Observe mode is the **allow** half of this vendor's decision wire, and only the deny half
    /// has been driven. So it is refused by name — naming the milestone that would close it —
    /// rather than served with a grant this binary may discard, which on a capture run would be
    /// indistinguishable from a capture that worked.
    #[test]
    fn a_codex_observe_run_is_refused_by_name_because_the_allow_half_is_undriven() {
        let mut spec = spec();
        spec.decisions = DecisionMode::Observe;
        let refusal = plan_launch(&spec, &context()).expect_err("observe is refused here");
        assert_eq!(
            refusal,
            LaunchRefusal::DecisionModeUnverified {
                mode: DecisionMode::Observe,
                status: TierStatus::Unverified,
            }
        );
        assert_eq!(refusal.code(), Some(RefusalCode::UnsupportedControl));
        let sentence = refusal.to_string();
        for named in ["allow half", "R2.4"] {
            assert!(sentence.contains(named), "{sentence}");
        }
    }

    /// The refusal and the published descriptor are one decision, not two: a mode the table calls
    /// unverified is refused, and a mode it calls delivered plans. A drift between them would be a
    /// capability an embedder queried and could not use, or a mode it was refused without warning.
    #[test]
    fn the_mode_table_and_the_plan_time_refusal_cannot_drift_apart() {
        for mode in DecisionMode::ALL {
            let mut spec = spec();
            spec.decisions = mode;
            let declared = crate::capabilities().decision_mode(mode);
            let planned = plan_launch(&spec, &context()).is_ok();
            assert_eq!(
                planned,
                declared == TierStatus::Delivered,
                "--decisions {} plans={planned} and the descriptor says {declared:?}",
                mode.as_str()
            );
        }
    }

    /// Crossing #4 on this vendor: the copy list and the digest are values on the plan, the
    /// placement is the named constant, and the attestation says **in as many words** that
    /// nothing names the plugin to the vendor.
    #[test]
    fn a_declared_plugin_is_copied_into_the_scratch_home_with_its_load_labelled_unverified() {
        let (spec, context, digest) = plugin_world();
        let plan = plan_launch(&spec, &context).expect("the injection plans");

        assert_eq!(plan.plugin_installs.len(), 1);
        let install = &plan.plugin_installs[0];
        assert_eq!(install.from, PathBuf::from("/operator/integrations/codex"));
        assert_eq!(
            install.to,
            PathBuf::from("/scratch/run-1/codex-home/plugins/codex"),
            "the placement is inside the scratch CODEX_HOME"
        );
        assert!(install.to.starts_with(&plan.config_home));
        assert_eq!(install.digest, digest, "the digest is over what was read");

        let attested = &plan.attestation.installed_plugins;
        assert_eq!(attested.len(), 1);
        assert_eq!(attested[0].name, "codex");
        assert_eq!(attested[0].digest, digest);
        assert_eq!(attested[0].source, "/operator/integrations/codex");
        assert!(
            attested[0].loaded_by.contains("unverified") && attested[0].loaded_by.contains("Q19"),
            "the record must say the load is undriven, not imply it: {}",
            attested[0].loaded_by
        );
        // No argv and no config key were invented to go with the copy: an unrecognised key under
        // a table this binary reads is dropped in silence, and a malformed one fails the config
        // load — which on this vendor is a run with no seam.
        assert!(!plan.args.iter().any(|argument| argument == "--plugin-dir"));
        assert!(!plan.config.contains("marketplace"), "{}", plan.config);
    }

    /// A directory that is not there is refused before the spawn, by name, with the path in it.
    #[test]
    fn a_plugin_directory_that_cannot_be_read_is_refused_by_name() {
        let source = PathBuf::from("/operator/integrations/codex");
        for (content, expected) in [
            (PluginContent::Empty, "holds no file"),
            (
                PluginContent::Unreadable {
                    detail: "No such file or directory (os error 2)".to_string(),
                },
                "could not be read",
            ),
        ] {
            let mut spec = spec();
            spec.plugin_dir.push(source.clone());
            let mut context = context();
            context.plugins.push(PluginTree {
                source: source.clone(),
                content,
            });
            let refusal = plan_launch(&spec, &context).expect_err("refused");
            let LaunchRefusal::PluginDirUnusable { directory, why } = &refusal else {
                panic!("expected a plugin refusal, got {refusal:?}");
            };
            assert_eq!(*directory, source);
            assert!(why.contains(expected), "{why}");
        }
    }

    /// A run that declared no plugin says so with an empty list rather than by dropping the key:
    /// "this run installed nothing" and "this build does not report installations" must not be
    /// the same bytes.
    #[test]
    fn a_run_with_no_plugin_attests_an_empty_list_and_never_an_absent_key() {
        let plan = plan();
        assert!(plan.plugin_installs.is_empty());
        assert!(plan.attestation.installed_plugins.is_empty());
        let json = serde_json::to_string(&plan.attestation).expect("the attestation serializes");
        assert!(json.contains(r#""installed_plugins":[]"#), "{json}");
        assert!(json.contains(r#""decisions":"frame""#), "{json}");
    }

    /// LP-3 vector 3. The loopback provider is Claude-Code-only this milestone, and this vendor
    /// says so **by name**: V-LP6 verified the route is feasible and its confirming paid run has
    /// not been done, so the door is stated as unbuilt rather than degraded in silence to the
    /// credential-copy path — which is precisely what the loopback design exists to replace.
    #[test]
    fn a_codex_loopback_run_is_refused_by_name_as_a_later_milestone() {
        let mut spec = spec();
        spec.credentials = CredentialSource::Loopback;
        let refusal = plan_launch(&spec, &context()).expect_err("codex has no loopback provider");
        assert_eq!(refusal, LaunchRefusal::LoopbackUnsupported);
        assert_eq!(
            refusal.code(),
            Some(RefusalCode::UnsupportedControl),
            "the refusal carries a code, so an embedder matches on it rather than on prose"
        );
        let sentence = refusal.to_string();
        for named in ["LP-4", "V-LP6"] {
            assert!(
                sentence.contains(named),
                "the refusal must name the milestone and the verification that shaped it, or the \
                 reader cannot tell 'not built' from 'impossible': {sentence}"
            );
        }
        assert!(
            sentence.contains("refused by name rather than degraded"),
            "the refusal must say it is not a silent fallback to the copy path: {sentence}"
        );
    }

    /// Defence in depth: the copy list itself refuses loopback, so a caller that reached this
    /// function by another road gets the refusal rather than an empty list that would read as
    /// "loopback is supported and copies nothing".
    #[test]
    fn the_copy_list_refuses_loopback_too_rather_than_returning_nothing() {
        let mut spec = spec();
        spec.credentials = CredentialSource::Loopback;
        assert_eq!(
            credential_copies(&spec, &context(), Path::new("/scratch/run-1/codex-home")),
            Err(LaunchRefusal::LoopbackUnsupported)
        );
    }

    #[test]
    fn a_run_for_another_harness_is_refused_by_name() {
        let mut spec = spec();
        spec.kind = Kind::Claude;
        assert_eq!(
            plan_launch(&spec, &context()),
            Err(LaunchRefusal::WrongKind {
                asked_for: Kind::Claude
            })
        );
    }

    #[test]
    fn every_refusal_says_what_it_refused_and_why() {
        let refusals = [
            LaunchRefusal::WrongKind {
                asked_for: Kind::Claude,
            },
            LaunchRefusal::NoPrompt,
            LaunchRefusal::CwdOutsideScratch {
                cwd: PathBuf::from("/a"),
                scratch_root: PathBuf::from("/b"),
            },
            LaunchRefusal::MemoryAncestorsFound {
                found: vec![PathBuf::from("/a/AGENTS.md")],
            },
            LaunchRefusal::CredentialFileMissing,
            LaunchRefusal::ApiKeyMissing,
            LaunchRefusal::DeniedArgument {
                argument: "--ephemeral".to_string(),
            },
            LaunchRefusal::DeniedEnvironment {
                key: "GIT_DIR".to_string(),
                row: HermeticRow::H3,
            },
            LaunchRefusal::UnsupportedOption {
                option: "--max-turns",
                why: "codex exec has no turn ceiling",
            },
            LaunchRefusal::LoopbackUnsupported,
        ];
        for refusal in refusals {
            let sentence = refusal.to_string();
            assert!(sentence.len() > 20, "{sentence}");
        }
    }
}
