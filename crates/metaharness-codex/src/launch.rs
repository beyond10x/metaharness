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
    CredentialSource, Digest, HermeticAttestation, HermeticRow, ImposedControl, Kind, RefusalCode,
    RunSpec, ToolSurface, UnavailableControl, required_commands,
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
            LaunchRefusal::UnsupportedOption { .. } => Some(RefusalCode::UnsupportedControl),
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

    let config_home = context.scratch_root.join(CONFIG_HOME);
    let credential_copies = credential_copies(spec, context, &config_home)?;
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
        config: build_config(&hook),
        hook,
        attestation: attest(spec, context, &config_home),
    })
}

/// The spec fields `codex exec` cannot carry, refused by name.
fn unsupported_options(spec: &RunSpec) -> Result<(), LaunchRefusal> {
    if spec.max_turns.is_some() {
        return Err(LaunchRefusal::UnsupportedOption {
            option: "--max-turns",
            why: "codex exec has no turn ceiling; the run ends when the task does, and a ceiling \
                  metaharness enforced by killing the child would be a different thing wearing \
                  the same name",
        });
    }
    if !spec.plugin_dir.is_empty() {
        return Err(LaunchRefusal::UnsupportedOption {
            option: "--plugin-dir",
            why: "codex loads plugins from its own config and marketplace snapshots, not from a \
                  directory named on the command line",
        });
    }
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
fn build_config(hook: &Value) -> String {
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
         sandbox_mode = \"read-only\"\n\n\
         # H5: the MCP surface is exactly what this launch gave, which is nothing.\n\
         [mcp_servers]\n\n\
         # The control seam (design § 7.1, call tier). 0.145.0 reads its hooks from this table\n\
         # and not from a hooks.json, which is a plugin manifest\'s file.\n",
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
fn attest(spec: &RunSpec, context: &LaunchContext, config_home: &Path) -> HermeticAttestation {
    let home = config_home.display();
    let mut imposed = vec![
        control_imposed(HermeticRow::H1a, format!("CODEX_HOME={home}")),
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
        imposed,
        unavailable,
        ambient_inputs: ambient_inputs(spec),
    }
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
        }
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

        let mut plugins = spec();
        plugins.plugin_dir.push(PathBuf::from("/plugins/x"));
        assert!(matches!(
            plan_launch(&plugins, &context()),
            Err(LaunchRefusal::UnsupportedOption {
                option: "--plugin-dir",
                ..
            })
        ));

        let mut owned = spec();
        owned.tool_surface = ToolSurface::Owned;
        let refusal = plan_launch(&owned, &context()).expect_err("refused");
        assert_eq!(refusal.code(), Some(RefusalCode::UnsupportedControl));
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
        ];
        for refusal in refusals {
            let sentence = refusal.to_string();
            assert!(sentence.len() > 20, "{sentence}");
        }
    }
}
