//! The hermetic launch, constructed and never spawned.
//!
//! [`plan_launch`] is a pure function. It reads no file, no clock and no environment of its own:
//! everything it needs arrives in [`LaunchContext`], and everything it decides leaves as a value
//! on [`LaunchPlan`]. That is design § 8.4 O7, and the reason is the one `engineering-protocols`
//! gives for asserting three properties of its own argv rather than leaving them as notes —
//! *"because every one of the failures would be silent"*.
//!
//! # What the caller still has to do
//!
//! The plan names four pieces of I/O and performs none of them, because a pure function that
//! wrote to a disk would be a pure function nobody could test:
//!
//! | value | what the caller does with it |
//! |---|---|
//! | [`LaunchPlan::credential_copies`] | copies each `from` to each `to` **immediately before every spawn** |
//! | [`LaunchPlan::settings`] | writes it to the path the argv's `--settings` names |
//! | [`LaunchPlan::hook`] | is already inside `settings`; the executable it names is the caller's to place |
//! | [`LaunchPlan::config_home`], [`LaunchPlan::cwd`] | creates them, empty, before the spawn |

use std::collections::BTreeMap;
use std::fmt;
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
/// less** than the vendor's hook timeout, and a deadline set from a number nobody could read
/// would be a guarantee that depends on two files agreeing by memory.
///
/// What the vendor does when a `type: command` hook exceeds this is **Q10 and not V7** — V7's
/// fail-closed string is the SDK hook-*callback* path, not this one (finding F7). metaharness
/// closes the window from its own side regardless.
pub const HOOK_TIMEOUT_SECONDS: u64 = 60;

/// The directory under the scratch root that becomes `CLAUDE_CONFIG_DIR`.
const CONFIG_HOME: &str = "claude-home";

/// The directory under the scratch root that becomes `TMPDIR`.
///
/// Never `/tmp`: the machine's tmpfs drops writes under pressure (design § 2.1).
const TMP_DIR: &str = "tmp";

/// Where the caller must write [`LaunchPlan::settings`].
///
/// Deliberately **outside** [`LaunchPlan::config_home`]: a settings document written to
/// `<config home>/settings.json` would be the *user* source, and `--setting-sources` with an
/// empty value has just switched that source off — so the hook would silently not load.
const SETTINGS_FILE: &str = "claude-settings.json";

/// Where the caller must place the executable the `PreToolUse` hook runs.
const HOOK_FILE: &str = "hooks/pretooluse";

/// Where the argv's `--settings` points, for a caller that has to write the document there.
///
/// Published as a function of the scratch root rather than left for the caller to reconstruct
/// from the argv, because a caller that rebuilt the path by string-matching `--settings` would
/// be a second place that decides where the settings file lives — and the one thing that must
/// not vary about it is that it is **outside** [`LaunchPlan::config_home`] (see [`SETTINGS_FILE`]).
#[must_use]
pub fn settings_path(scratch_root: &Path) -> PathBuf {
    scratch_root.join(SETTINGS_FILE)
}

/// Where the caller must place the `PreToolUse` executable, which is the path the hook
/// definition already names.
///
/// The program itself is [`crate::hook_program`]; this is only where it goes.
#[must_use]
pub fn hook_program_path(scratch_root: &Path) -> PathBuf {
    scratch_root.join(HOOK_FILE)
}

/// The environment variables copied from the caller's own, when present.
///
/// An allowlist and not a denylist, because a denylist is a list of the leaks somebody thought
/// of (design § 8.1 H3). `SHELL` is deliberately absent: the vendor's shell tool would inherit
/// the operator's login shell and its startup files with it.
const INHERITED_KEYS: [&str; 7] = ["HOME", "USER", "LOGNAME", "LANG", "LC_ALL", "TERM", "TZ"];

/// The stated `PATH` the child gets, before the operator's own `~/.local/bin` is appended.
const BASE_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Arguments this adapter must never construct.
///
/// A guard over the value, not a spelling check on the source: `--bare` *"skips hooks"* — which
/// on this design is the control seam — and `--safe-mode` performs the same deletion and sets
/// `CLAUDE_CODE_SAFE_MODE=1` besides (design § 8.1 H8, finding F5).
/// `--dangerously-skip-permissions` and `--add-dir` are here for the same reason one row up: the
/// first auto-approves every call before the seam is consulted (V4), the second widens the
/// working directory H7 says is ours.
const DENIED_ARGUMENTS: [&str; 4] = [
    "--bare",
    "--safe-mode",
    "--dangerously-skip-permissions",
    "--add-dir",
];

/// Environment variables that must never reach the child, named by H8 rather than by H3's
/// prefix scrub, so a failure here is attributed to the row that forbids it.
const DENIED_ENVIRONMENT: [&str; 2] = ["CLAUDE_CODE_SAFE_MODE", "CLAUDE_CODE_SIMPLE"];

/// Everything the launch needs that this crate will not go and find for itself.
///
/// The caller does the I/O — the ancestor walk, the digest of the copied tree, reading its own
/// environment — so [`plan_launch`] stays a function of its arguments and a test can hand it a
/// world.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchContext {
    /// The run's scratch root. The config home, the temporary directory and the settings
    /// document all live under it, so one directory is the whole of what the run may write
    /// outside its working tree.
    pub scratch_root: PathBuf,
    /// The working directory the child is spawned in — the evidence for H7, and the directory
    /// [`LaunchContext::memory_ancestors`] was walked upward from.
    pub cwd: PathBuf,
    /// The operator's credential file, when the run declared
    /// [`CredentialSource::OperatorLogin`]. Ignored under any other credential source: there is
    /// then no login to copy, and copying one anyway would put a second credential in a home
    /// H6 says holds exactly one.
    pub credentials_file: Option<PathBuf>,
    /// The caller's own environment. Read here and **not** inherited: the child's environment is
    /// constructed from [`INHERITED_KEYS`], so a variable absent from that list cannot reach the
    /// child however it got into this map (design § 8.1 H3).
    pub inherited_env: BTreeMap<String, String>,
    /// What an ancestor walk from [`LaunchContext::cwd`] found. A non-empty walk is a **refusal
    /// before the run** and not a warning: `CLAUDE.md` auto-discovery is on in every run that is
    /// not `--bare`, and H8 forbids `--bare`, so a memory file in any ancestor enters the
    /// context of a run this design calls hermetic (design § 8.1 H11, finding F14).
    pub memory_ancestors: Vec<PathBuf>,
    /// The digest of the copied input tree, carried into `session.started.inputs_digest` — the
    /// evidence for H10. `None` means the caller pinned nothing, and the attestation says so
    /// rather than leaving the row silent.
    pub inputs_digest: Option<Digest>,
}

/// One file to copy into the scratch config home, and the only one.
///
/// **This copy is performed immediately before every spawn, never once per run.** A copied
/// operator-login token is a snapshot with a lifetime: a governed run observed on 2026-08-22 died
/// an hour in with the vendor reporting an expired session that could not be refreshed, because
/// a copied file cannot refresh itself. The coordinator records this as **Q13** in design § 12
/// and as an amendment to § 8.1 H6.
///
/// Option (b) of that amendment — sharing the live file by hardlink or bind mount so the
/// harness's own refresh writes back to the operator's real credential — is **not taken in M1**
/// and is Q13's open question: it hands a run write access to the operator's credential, which
/// is a custody change and not an implementation detail.
///
/// An expiry observed mid-run becomes [`metaharness_protocol::Event::AuthExpired`], which is
/// clause (c) of the same amendment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CredentialCopy {
    /// The operator's file.
    pub from: PathBuf,
    /// Where it goes in the scratch config home.
    pub to: PathBuf,
}

/// The launch, as a value.
///
/// Every field is something a test reads before anything is spawned, because every one of the
/// failures it prevents would otherwise be silent (design § 8.4 O7).
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchPlan {
    /// The program to run.
    pub program: String,
    /// Its arguments, in the order they are passed.
    pub args: Vec<String>,
    /// The child's whole environment. **Constructed, not inherited** — a key absent here is
    /// absent from the child (design § 8.1 H3).
    pub env: BTreeMap<String, String>,
    /// The directory the child is spawned in.
    pub cwd: PathBuf,
    /// The scratch `CLAUDE_CONFIG_DIR`. Fresh per run, which is what stops five foreign plugins
    /// and the operator's output style appearing in the opening record (design § 2.1).
    pub config_home: PathBuf,
    /// What to copy into that home before **every** spawn — see [`CredentialCopy`]. Exactly one
    /// entry under an operator login, and none otherwise; nothing else is ever copied (H6).
    pub credential_copies: Vec<CredentialCopy>,
    /// The settings document the argv's `--settings` names. The caller writes it; this crate
    /// only decides what is in it.
    pub settings: Value,
    /// The `PreToolUse` hook definition, as a value, so its two dangerous absences are testable:
    /// **neither `async` nor `asyncRewake` is set** (V7b, finding F6, design § 7.8). A hook that
    /// matches every tool and does not block is a guard that has already stopped guarding.
    pub hook: Value,
    /// What metaharness imposed and what it could not.
    ///
    /// **Not evidence.** It is metaharness's own claim about its own actions; the independent
    /// evidence is the vendor's opening record — the plugin list, the MCP list, the credential
    /// source, the cwd, the version. The block exists so a reader can see the intent beside the
    /// outcome and notice when they disagree (design § 8.3).
    pub attestation: HermeticAttestation,
}

impl LaunchPlan {
    /// The credential copies to perform **before this spawn**, and before the next one too.
    ///
    /// Named for when it is called rather than for what it returns, because the failure it
    /// exists to prevent is a caller that copies once at the top of a run and relaunches for an
    /// hour off a token that has since expired (Q13).
    #[must_use]
    pub fn credential_copies_for_next_spawn(&self) -> &[CredentialCopy] {
        &self.credential_copies
    }
}

/// Why a launch was refused before anything was spawned.
///
/// Every variant is a refusal the design asks for by name. None of them is a warning: a control
/// that is reported and then proceeds is a control that has already stopped controlling
/// (design § 7.1).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LaunchRefusal {
    /// The spec asks for a harness this adapter is not.
    WrongKind {
        /// What it asked for.
        asked_for: Kind,
    },
    /// The run has no prompt. `-p` needs one, and a headless session with nothing to do is a
    /// paid call for no observation.
    NoPrompt,
    /// The working directory is not under the scratch root, so H7's *"a directory metaharness
    /// created"* cannot be claimed for it.
    CwdOutsideScratch {
        /// The directory asked for.
        cwd: PathBuf,
        /// The root it must be under.
        scratch_root: PathBuf,
    },
    /// The ancestor walk found memory files above the scratch working directory (H11). A
    /// refusal and not a warning, because those files enter the context of a run this design
    /// calls hermetic.
    MemoryAncestorsFound {
        /// What the walk found, in the order it found them.
        found: Vec<PathBuf>,
    },
    /// The run declared an operator login and named no credential file to copy.
    CredentialFileMissing,
    /// The run declared `credentials: api-key` and no `ANTHROPIC_API_KEY` was in the caller's
    /// environment, so the child would have started with neither credential.
    ApiKeyMissing,
    /// The run declared a model endpoint together with a real credential source.
    ///
    /// Refused rather than composed, because the binary prefers a login over an exported key
    /// where both are present — so an operator credential in the scratch home beside a foreign
    /// base URL is the operator's token travelling to a host that is not the vendor's.
    EndpointWithCredential {
        /// The endpoint that was declared.
        endpoint: String,
    },
    /// The constructed argv carries an argument the denylist forbids (H8).
    DeniedArgument {
        /// Which one.
        argument: String,
    },
    /// The constructed environment carries a variable the denylist forbids (H8), or one H3's
    /// scrub should have dropped.
    DeniedEnvironment {
        /// Which key.
        key: String,
        /// Which row forbids it.
        row: HermeticRow,
    },
    /// The run needs a call-level seam and the launch would also carry a bare `--allowedTools`
    /// entry, which *"auto-approve[s] the whole tool before the callback is consulted"* (V4, the
    /// vendor's own string). Refused rather than served with a seam another layer overrides
    /// (design § 6.1 `SHADOWED`).
    Shadowed {
        /// The bare entries that would have shadowed it.
        entries: Vec<String>,
    },
}

impl LaunchRefusal {
    /// The protocol refusal code this maps to, where one applies.
    ///
    /// `None` for the rows that are launch conditions rather than commands: design § 6.1's codes
    /// answer *"what happened to this command"*, and inventing one for *"the ancestor walk found
    /// a `CLAUDE.md`"* would put a wrong word on a right refusal.
    #[must_use]
    pub fn code(&self) -> Option<RefusalCode> {
        match self {
            LaunchRefusal::Shadowed { .. } => Some(RefusalCode::Shadowed),
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
                "the working directory {} is not under the scratch root {}, so H7 cannot claim \
                 it is a directory metaharness created",
                cwd.display(),
                scratch_root.display()
            ),
            LaunchRefusal::MemoryAncestorsFound { found } => write!(
                f,
                "H11: the ancestor walk found {}, and memory-file discovery is on in every run \
                 that is not --bare",
                join_paths(found)
            ),
            LaunchRefusal::CredentialFileMissing => f.write_str(
                "the run declared an operator login and named no credential file, so the scratch \
                 home would hold no credential at all",
            ),
            LaunchRefusal::ApiKeyMissing => f.write_str(
                "the run declared credentials: api-key and ANTHROPIC_API_KEY was not in the \
                 caller's environment",
            ),
            LaunchRefusal::EndpointWithCredential { endpoint } => write!(
                f,
                "the run declared a model endpoint ({endpoint}) together with a real credential \
                 source; a child pointed at a foreign endpoint must hold no operator credential, \
                 so declare credentials: none — the child gets a placeholder key instead"
            ),
            LaunchRefusal::DeniedArgument { argument } => write!(
                f,
                "H8: the constructed argv carries {argument}, which deletes the control seam"
            ),
            LaunchRefusal::DeniedEnvironment { key, row } => write!(
                f,
                "{}: the constructed child environment carries {key}",
                row.id()
            ),
            LaunchRefusal::Shadowed { entries } => write!(
                f,
                "the run needs a call-level seam and the argv would carry the bare --allowedTools \
                 {}, which auto-approves the whole tool before the seam is consulted (V4)",
                entries.join(", ")
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
/// Nothing is spawned, nothing is written and nothing is read from the machine this runs on. The
/// twelve rows of design § 8.1 that can be imposed before a process exists are imposed here, and
/// the four launch-asserted ones (H2, H3, H8, H11) are checked against the values this function
/// just built rather than against the source that built them.
///
/// # Errors
///
/// Every variant of [`LaunchRefusal`], each of which is a refusal the design asks for by name.
/// The order is deliberate: the conditions that make the run wrong (kind, prompt, frame, cwd,
/// memory ancestors, credentials) are checked before the argv exists, and the guards over the
/// constructed argv and environment run last, because a guard can only read a value that has
/// been built.
pub fn plan_launch(spec: &RunSpec, context: &LaunchContext) -> Result<LaunchPlan, LaunchRefusal> {
    if spec.kind != Kind::Claude {
        return Err(LaunchRefusal::WrongKind {
            asked_for: spec.kind,
        });
    }
    // `spec.frame` is deliberately not checked here: since amendment a5 the library resolves the
    // document to an in-memory frame before any launch is planned, so by the time this function
    // runs the path has already been read, parsed and digest-verified — or refused by name.
    let Some(prompt) = &spec.prompt else {
        return Err(LaunchRefusal::NoPrompt);
    };
    // An operator-named cwd (amendment a6) is a declaration, not a defect: the two refusals
    // below exist to keep a *scratch* cwd honest, and a run the operator pointed at a real tree
    // instead loses rows H7 and H11 in the attestation rather than being refused here.
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
    let args = build_args(spec, prompt, &context.scratch_root.join(SETTINGS_FILE));
    guard_arguments(&args)?;
    guard_shadowing(spec, &args)?;
    let env = build_env(spec, context, &config_home)?;
    let hook = build_hook(&context.scratch_root.join(HOOK_FILE));

    Ok(LaunchPlan {
        program: ADAPTER_ID.to_string(),
        args,
        env,
        cwd: context.cwd.clone(),
        config_home: config_home.clone(),
        credential_copies,
        settings: build_settings(&hook),
        hook,
        attestation: attest(spec, context, &config_home),
    })
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
                to: config_home.join(".credentials.json"),
            }])
        }
        CredentialSource::ApiKey => {
            if context.inherited_env.contains_key("ANTHROPIC_API_KEY") {
                Ok(Vec::new())
            } else {
                Err(LaunchRefusal::ApiKeyMissing)
            }
        }
        CredentialSource::None => Ok(Vec::new()),
    }
}

/// The command line.
///
/// `--verbose` is not decoration: the vendor refuses the combination without it —
/// *"Error: When using --print, --output-format=stream-json requires --verbose"*, read from the
/// 2.1.239 binary. Without a stream there is no transcript, and without a transcript there is
/// nothing for design § 9.4's auditor to read.
fn build_args(spec: &RunSpec, prompt: &str, settings_path: &Path) -> Vec<String> {
    let mut args = vec![
        "-p".to_string(),
        prompt.to_string(),
        "--output-format".to_string(),
        "stream-json".to_string(),
        "--verbose".to_string(),
        // H5, always. Account-level MCP servers arrive with the login, over the network, and no
        // directory the runner controls excludes them (design § 2.1).
        "--strict-mcp-config".to_string(),
        // H2 / V12. The vendor's own parser reads the empty value as "no sources":
        // `function R8u(e){if(e==="")return[];…}` in the 2.1.239 binary.
        "--setting-sources".to_string(),
        String::new(),
        "--settings".to_string(),
        settings_path.display().to_string(),
    ];
    if let Some(model) = &spec.model {
        args.push("--model".to_string());
        args.push(model.clone());
    }
    if let Some(effort) = &spec.effort {
        args.push("--effort".to_string());
        args.push(effort.clone());
    }
    if let Some(max_turns) = spec.max_turns {
        args.push("--max-turns".to_string());
        args.push(max_turns.to_string());
    }
    for dir in &spec.plugin_dir {
        args.push("--plugin-dir".to_string());
        args.push(dir.display().to_string());
    }
    if spec.tool_surface == ToolSurface::Owned {
        // Strategy C (design § 7.5): `--tools ""` disables the whole built-in set (V11) and the
        // metaharness server's tools are admitted by a whole-server grant, because nothing in the
        // vendor's own settings has heard of them. That grant is *bare*, which is exactly what
        // `guard_shadowing` refuses — see its own comment.
        args.push("--tools".to_string());
        args.push(String::new());
        args.push("--allowedTools".to_string());
        args.push("mcp__metaharness".to_string());
    }
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
    env.insert(
        "CLAUDE_CONFIG_DIR".to_string(),
        config_home.display().to_string(),
    );
    if spec.credentials == CredentialSource::ApiKey
        && let Some(key) = context.inherited_env.get("ANTHROPIC_API_KEY")
    {
        env.insert("ANTHROPIC_API_KEY".to_string(), key.clone());
    }
    if let Some(endpoint) = &spec.model_endpoint {
        env.insert(
            "ANTHROPIC_BASE_URL".to_string(),
            endpoint.trim_end_matches('/').to_string(),
        );
        // The placeholder, never a real credential (the loopback design's custody rule): the
        // binary wants *something* to authenticate with, sends it as `x-api-key`, and what the
        // endpoint does with it is the endpoint's business. Verified against 2.1.240 (MA-V1):
        // with a base URL and this key set, every API request goes to the base.
        env.insert(
            "ANTHROPIC_API_KEY".to_string(),
            ENDPOINT_PLACEHOLDER_KEY.to_string(),
        );
    }
    guard_environment(&env, spec)?;
    Ok(env)
}

/// What the child authenticates with under a declared model endpoint: a marker, not a secret.
const ENDPOINT_PLACEHOLDER_KEY: &str = "metaharness-model-endpoint";

/// The stated `PATH`, and why it is not shorter.
///
/// The operator's `~/.local/bin` is on it because that is where the vendor installs `claude`,
/// and a child that cannot find its own program is not a hermetic run, it is no run.
fn reduced_path(env: &BTreeMap<String, String>) -> String {
    child_path(env.get("HOME").map(String::as_str))
}

/// The `PATH` a spawned child gets, given the inherited `HOME`.
///
/// Public because `doctor` must resolve the vendor binary **the way the spawn will** (CT-3): a
/// machine that holds two installs can resolve them differently on the operator's `PATH` and on
/// this constructed one — the codex adapter's Q18 was exactly that — and a pre-flight that
/// blesses a binary the run never executes is not a pre-flight.
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
///
/// The scrub can only fire if a key were added to [`INHERITED_KEYS`], which is precisely the
/// silent widening it exists to catch: an allowlist that grew is an allowlist nobody re-read.
fn guard_environment(env: &BTreeMap<String, String>, spec: &RunSpec) -> Result<(), LaunchRefusal> {
    for key in env.keys() {
        if DENIED_ENVIRONMENT.contains(&key.as_str()) {
            return Err(LaunchRefusal::DeniedEnvironment {
                key: key.clone(),
                row: HermeticRow::H8,
            });
        }
        if is_scrubbed(key, spec) {
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
/// `ANTHROPIC_API_KEY` is one conditional row: absent unless the run declared
/// `credentials: api-key`, because an exported key *"takes precedence over the claude.ai login
/// and may point at an account with no credits"* (design § 2.1, H4) — or unless a model
/// endpoint is declared, in which case the key is this plan's own placeholder and never the
/// operator's. `ANTHROPIC_BASE_URL` is the other: an ambient one silently redirects the model
/// API and stays refused; a **declared** one is the whole point of `--model-endpoint`.
fn is_scrubbed(key: &str, spec: &RunSpec) -> bool {
    let credentials = spec.credentials;
    if key == "ANTHROPIC_API_KEY" {
        return credentials != CredentialSource::ApiKey && spec.model_endpoint.is_none();
    }
    if key == "ANTHROPIC_BASE_URL" {
        return spec.model_endpoint.is_none();
    }
    key.starts_with("ANTHROPIC_")
        || key.starts_with("CLAUDE_CODE_")
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

/// The `--allowedTools`-shadows-the-seam trap, refused rather than delivered.
///
/// V4 is the vendor's own string: *"Bare allowedTools entries auto-approve the whole tool before
/// the callback is consulted."* A bare entry is one with no parenthesised specifier — `Bash`
/// rather than `Bash(git status:*)` — so it grants the tool whatever the arguments are.
///
/// The guard reads the argv this adapter just built, which is why it is a guard: today the only
/// configuration that reaches it is `--tool-surface owned`, and refusing that is the honest
/// answer for M1 anyway, since per-step re-listing on this vendor depends on unverified
/// `notifications/tools/list_changed` behaviour (**Q1**).
fn guard_shadowing(spec: &RunSpec, args: &[String]) -> Result<(), LaunchRefusal> {
    if !needs_call_seam(spec) {
        return Ok(());
    }
    let entries = bare_allowed_tools(args);
    if entries.is_empty() {
        Ok(())
    } else {
        Err(LaunchRefusal::Shadowed { entries })
    }
}

/// Every bare `--allowedTools` entry in an argv, in the order they appear.
fn bare_allowed_tools(args: &[String]) -> Vec<String> {
    let mut bare = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let value = if let Some(rest) = args[index].strip_prefix("--allowedTools=") {
            index += 1;
            Some(rest.to_string())
        } else if args[index] == "--allowedTools" {
            index += 2;
            args.get(index - 1).cloned()
        } else {
            index += 1;
            None
        };
        let Some(value) = value else { continue };
        bare.extend(
            value
                .split(',')
                .map(str::trim)
                .filter(|entry| !entry.is_empty() && !entry.contains('('))
                .map(ToString::to_string),
        );
    }
    bare
}

/// The `PreToolUse` hook definition.
///
/// Matcher `""` per V8 — the vendor's own hook documentation says an empty matcher matches every
/// tool — and that is **Q11**: the measured 11-for-11 denial parity used the narrow matchers
/// `Edit|Write|NotebookEdit` and `Bash`, never `""`, and `""` changes the regime to a child
/// process per `Read`, `Glob`, `Grep`, `WebFetch`, `TodoWrite` and every MCP call.
///
/// Neither `async` nor `asyncRewake` appears, and their absence is the point: V7b records that an
/// `async` hook *"runs in background without blocking"*, so a hook that matched everything and
/// declared itself `async` would be a guard that has already stopped guarding (finding F6,
/// design § 7.8).
fn build_hook(hook_path: &Path) -> Value {
    json!({
        "matcher": "",
        "hooks": [{
            "type": "command",
            "command": hook_path.display().to_string(),
            "timeout": HOOK_TIMEOUT_SECONDS,
        }],
    })
}

/// The settings document the argv's `--settings` names.
///
/// The empty `permissions.allow` is deliberate and is H2's advisory half said out loud: V4 names
/// settings allow rules as a second thing that can shadow the seam, and this file is a source the
/// run *does* load, so it is the one place the absence can be written down rather than assumed.
fn build_settings(hook: &Value) -> Value {
    json!({
        "$schema": "https://json.schemastore.org/claude-code-settings.json",
        "permissions": { "allow": [], "deny": [] },
        "hooks": { "PreToolUse": [hook.clone()] },
    })
}

/// What metaharness claims it imposed, and what it says it could not.
fn attest(spec: &RunSpec, context: &LaunchContext, config_home: &Path) -> HermeticAttestation {
    let home = config_home.display();
    let mut imposed = vec![
        control_imposed(HermeticRow::H1a, format!("CLAUDE_CONFIG_DIR={home}")),
        control_imposed(HermeticRow::H1b, format!("CLAUDE_CONFIG_DIR={home}")),
        control_imposed(
            HermeticRow::H2,
            "--setting-sources with an empty value; the vendor's parser reads \"\" as no sources",
        ),
        control_imposed(
            HermeticRow::H3,
            format!(
                "the child environment is constructed from an allowlist of {} keys plus PATH, \
                 TMPDIR and CLAUDE_CONFIG_DIR",
                INHERITED_KEYS.len()
            ),
        ),
        control_imposed(HermeticRow::H4, api_key_posture(spec)),
        control_imposed(HermeticRow::H5, "--strict-mcp-config"),
        control_imposed(
            HermeticRow::H8,
            "neither --bare, --safe-mode nor --dangerously-skip-permissions is in the argv, and \
             neither CLAUDE_CODE_SAFE_MODE nor CLAUDE_CODE_SIMPLE is in the child environment",
        ),
    ];
    let mut unavailable = vec![control_unavailable(
        HermeticRow::H9,
        format!(
            "this plan spawns nothing and runs no doctor; the pin is {} and the version is \
             asserted from the opening record",
            PINNED_VERSIONS.join(", ")
        ),
    )];

    // H7 and H11 are impositions only over a scratch cwd. An operator-named directory
    // (amendment a6) is real work in a real tree: the rows are attested unavailable with the
    // declaration named, which is what makes `--hermetic strict` refuse such a run instead of
    // this attestation quietly claiming a directory it never made.
    if spec.cwd.is_some() {
        unavailable.push(control_unavailable(
            HermeticRow::H7,
            format!(
                "the run was pointed at the operator's directory {} (--cwd), which metaharness \
                 did not create; --add-dir is still never passed",
                context.cwd.display()
            ),
        ));
        unavailable.push(control_unavailable(
            HermeticRow::H11,
            format!(
                "the ancestor walk from the operator's directory found {} memory file(s), and \
                 the operator declared the tree as the run's context by naming it",
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
            "the ancestor walk from the scratch cwd found no CLAUDE.md and no AGENTS.md",
        ));
    }

    if spec.credentials == CredentialSource::OperatorLogin {
        imposed.push(control_imposed(
            HermeticRow::H6,
            "one credential file copied into the scratch home immediately before every spawn, \
             and nothing else (Q13)",
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
        ambient_inputs: ambient_inputs(),
    }
}

fn api_key_posture(spec: &RunSpec) -> &'static str {
    if spec.model_endpoint.is_some() {
        "ANTHROPIC_API_KEY carries this plan's own placeholder for the declared model endpoint; \
         no operator credential is in the child at all"
    } else if spec.credentials == CredentialSource::ApiKey {
        "ANTHROPIC_API_KEY is in the child environment because the run declared credentials: \
         api-key"
    } else {
        "ANTHROPIC_API_KEY is absent from the child environment"
    }
}

/// Inputs metaharness reports and does **not** claim to have removed.
///
/// Both are named by the design rather than discovered here, and both would otherwise be read
/// out of the attestation's silence as absences.
fn ambient_inputs() -> Vec<String> {
    vec![
        "git status: the vendor's own --exclude-dynamic-system-prompt-sections description says \
         cwd, env info, memory paths and git status are in the system prompt (design § 8.1, H11's \
         second half)"
            .to_string(),
        "network access: Claude Code's CLI carries no sandbox knob, so a hermetic run here is not \
         network-isolated (design § 8.2)"
            .to_string(),
    ]
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
    use metaharness_protocol::{DecisionMode, HermeticMode};

    fn context() -> LaunchContext {
        LaunchContext {
            scratch_root: PathBuf::from("/scratch/run-1"),
            cwd: PathBuf::from("/scratch/run-1/work"),
            credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
            inherited_env: [
                ("HOME", "/operator"),
                ("USER", "operator"),
                ("LANG", "C.UTF-8"),
                ("ANTHROPIC_BASE_URL", "https://example.invalid"),
                ("HTTPS_PROXY", "http://proxy.invalid:8080"),
                ("CLAUDE_CODE_SAFE_MODE", "1"),
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
        let mut spec = RunSpec::new(Kind::Claude);
        spec.hermetic = HermeticMode::Strict;
        spec.prompt = Some("do the thing".to_string());
        spec
    }

    fn plan() -> LaunchPlan {
        plan_launch(&spec(), &context()).expect("the strict run plans")
    }

    #[test]
    fn the_config_home_is_scratch_and_the_child_is_told_where_it_is() {
        let plan = plan();
        assert_eq!(
            plan.config_home,
            PathBuf::from("/scratch/run-1/claude-home")
        );
        assert_eq!(
            plan.env.get("CLAUDE_CONFIG_DIR").map(String::as_str),
            Some("/scratch/run-1/claude-home")
        );
    }

    /// The tmpfs on the machine this runs on drops writes under pressure, so a run that used
    /// `/tmp` would lose its own transcript at the worst moment (design § 2.1).
    #[test]
    fn tmpdir_is_in_the_scratch_tree_and_is_never_slash_tmp() {
        let plan = plan();
        let tmpdir = plan.env.get("TMPDIR").expect("TMPDIR is set");
        assert_eq!(tmpdir, "/scratch/run-1/tmp");
        assert!(!tmpdir.starts_with("/tmp"));
    }

    /// The child environment is constructed, so every one of these is absent however loudly it
    /// was exported (H3).
    #[test]
    fn the_child_environment_is_constructed_and_not_inherited() {
        let plan = plan();
        for key in [
            "ANTHROPIC_API_KEY",
            "ANTHROPIC_BASE_URL",
            "ANTHROPIC_MODEL",
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "http_proxy",
            "https_proxy",
            "CLAUDE_CODE_SAFE_MODE",
            "CLAUDE_CODE_SIMPLE",
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
        let plan = plan();
        assert_eq!(
            plan.env.get("PATH").map(String::as_str),
            Some("/operator/.local/bin:/usr/local/bin:/usr/bin:/bin")
        );
    }

    /// An exported key *"takes precedence over the claude.ai login and may point at an account
    /// with no credits"*, so it is in the child only when the run said so (H4).
    #[test]
    fn the_api_key_reaches_the_child_only_when_the_run_declared_it() {
        let mut spec = spec();
        spec.credentials = CredentialSource::ApiKey;
        let mut context = context();
        context
            .inherited_env
            .insert("ANTHROPIC_API_KEY".to_string(), "sk-test".to_string());
        let plan = plan_launch(&spec, &context).expect("the api-key run plans");
        assert_eq!(
            plan.env.get("ANTHROPIC_API_KEY").map(String::as_str),
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

    /// Exactly one file is copied, and it is the credential file. Anything else in that home
    /// would be an operator artefact in a directory H1a says is scratch.
    #[test]
    fn the_credential_copy_is_one_file_and_nothing_else() {
        let plan = plan();
        assert_eq!(
            plan.credential_copies,
            vec![CredentialCopy {
                from: PathBuf::from("/operator/.claude/.credentials.json"),
                to: PathBuf::from("/scratch/run-1/claude-home/.credentials.json"),
            }]
        );
        assert_eq!(plan.credential_copies_for_next_spawn().len(), 1);
    }

    #[test]
    fn the_argv_always_carries_strict_mcp_config_and_settings_and_empty_setting_sources() {
        let plan = plan();
        assert!(plan.args.contains(&"--strict-mcp-config".to_string()));
        let settings = plan
            .args
            .windows(2)
            .find(|pair| pair[0] == "--settings")
            .expect("--settings is always present");
        assert_eq!(settings[1], "/scratch/run-1/claude-settings.json");
        let sources = plan
            .args
            .windows(2)
            .find(|pair| pair[0] == "--setting-sources")
            .expect("--setting-sources is always present");
        assert_eq!(sources[1], "");
    }

    /// Without `--verbose` the vendor refuses the combination outright, so the run would produce
    /// no transcript and there would be nothing to judge.
    #[test]
    fn print_mode_asks_for_stream_json_and_the_verbose_the_vendor_requires() {
        let plan = plan();
        assert_eq!(plan.args[0], "-p");
        assert_eq!(plan.args[1], "do the thing");
        assert!(plan.args.contains(&"--output-format".to_string()));
        assert!(plan.args.contains(&"stream-json".to_string()));
        assert!(plan.args.contains(&"--verbose".to_string()));
    }

    #[test]
    fn the_argv_denylist_is_a_guard_over_the_value_and_not_a_spelling_check() {
        for argument in DENIED_ARGUMENTS {
            let args = vec!["-p".to_string(), argument.to_string()];
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
        let env = BTreeMap::from([("ANTHROPIC_BASE_URL".to_string(), "x".to_string())]);
        assert_eq!(
            guard_environment(&env, &spec()),
            Err(LaunchRefusal::DeniedEnvironment {
                key: "ANTHROPIC_BASE_URL".to_string(),
                row: HermeticRow::H3,
            })
        );
    }

    /// The model-adapter door (MA-1): a **declared** endpoint reaches the child as
    /// `ANTHROPIC_BASE_URL` plus the placeholder key — and only a declared one; the ambient
    /// variable stays scrubbed by the test above, which is the difference between an option
    /// and a leak.
    #[test]
    fn a_declared_model_endpoint_reaches_the_child_with_a_placeholder_and_no_credential() {
        let mut spec = spec();
        spec.credentials = CredentialSource::None;
        spec.model_endpoint = Some("https://llmgw.example/".to_string());
        spec.effort = Some("medium".to_string());
        let plan = plan_launch(&spec, &context()).expect("the endpoint launch plans");
        assert_eq!(
            plan.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("https://llmgw.example"),
            "the root travels, with the trailing slash gone"
        );
        assert_eq!(
            plan.env.get("ANTHROPIC_API_KEY").map(String::as_str),
            Some(ENDPOINT_PLACEHOLDER_KEY)
        );
        assert!(
            plan.credential_copies.is_empty(),
            "no credential file travels"
        );
        let effort = plan.args.iter().position(|arg| arg == "--effort");
        assert!(
            effort.is_some_and(|at| plan.args.get(at + 1).map(String::as_str) == Some("medium")),
            "{:?}",
            plan.args
        );
    }

    /// An endpoint beside an operator credential is refused by name: the binary prefers a
    /// login over an exported key, so composing them would send the operator's token to a
    /// host that is not the vendor's.
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

    /// A memory file above the scratch cwd enters the context of a run this design calls
    /// hermetic, so the walk's result is a refusal and never a warning (H11, finding F14).
    #[test]
    fn a_non_empty_ancestor_walk_is_a_refusal_before_the_run() {
        let mut context = context();
        context.memory_ancestors = vec![PathBuf::from("/scratch/CLAUDE.md")];
        let refusal = plan_launch(&spec(), &context).expect_err("H11 refuses");
        assert_eq!(
            refusal,
            LaunchRefusal::MemoryAncestorsFound {
                found: vec![PathBuf::from("/scratch/CLAUDE.md")]
            }
        );
        assert_eq!(refusal.code(), None);
    }

    /// A control that appears to work and does not is worse than one that is absent, so the run
    /// is refused rather than served with a seam `--allowedTools` overrides (V4, § 6.1).
    #[test]
    fn a_run_needing_a_call_seam_beside_a_bare_allowed_tools_entry_is_refused_shadowed() {
        let mut spec = spec();
        spec.tool_surface = ToolSurface::Owned;
        let refusal = plan_launch(&spec, &context()).expect_err("the shadow is refused");
        assert_eq!(
            refusal,
            LaunchRefusal::Shadowed {
                entries: vec!["mcp__metaharness".to_string()]
            }
        );
        assert_eq!(refusal.code(), Some(RefusalCode::Shadowed));
    }

    /// A specifier-carrying entry does not auto-approve the whole tool, so it is not the trap
    /// V4 names and must not be refused as one.
    #[test]
    fn an_allowed_tools_entry_with_a_specifier_is_not_bare() {
        let args = vec![
            "--allowedTools".to_string(),
            "Bash(git status:*),Read".to_string(),
        ];
        assert_eq!(bare_allowed_tools(&args), vec!["Read".to_string()]);
        assert!(bare_allowed_tools(&["--allowedTools=Bash(ls:*)".to_string()]).is_empty());
    }

    /// A frame-mode run needs no call-level seam, so the same argv is not shadowed for it: the
    /// refusal is about the seam being overridden, not about the flag existing.
    #[test]
    fn the_shadow_guard_only_fires_when_the_run_needs_a_call_seam() {
        let mut frame_mode = spec();
        frame_mode.decisions = DecisionMode::Frame;
        let args = vec!["--allowedTools".to_string(), "Bash".to_string()];
        assert!(guard_shadowing(&frame_mode, &args).is_ok());

        let mut ask_mode = spec();
        ask_mode.decisions = DecisionMode::Ask;
        assert!(guard_shadowing(&ask_mode, &args).is_err());
    }

    /// A hook that matches everything and does not block is a guard that has already stopped
    /// guarding, so the two absences are asserted rather than assumed (V7b, finding F6).
    #[test]
    fn the_hook_is_a_blocking_command_hook_with_neither_async_nor_async_rewake() {
        let plan = plan();
        assert_eq!(plan.hook["matcher"], json!(""));
        let entry = &plan.hook["hooks"][0];
        assert_eq!(entry["type"], json!("command"));
        assert_eq!(entry["timeout"], json!(HOOK_TIMEOUT_SECONDS));
        assert!(entry.get("async").is_none());
        assert!(entry.get("asyncRewake").is_none());
        assert_eq!(plan.settings["hooks"]["PreToolUse"][0], plan.hook);
    }

    /// The settings document goes beside the config home and not inside it: inside, it would be
    /// the user source, which `--setting-sources` has just switched off.
    #[test]
    fn the_settings_document_is_not_written_into_the_disabled_user_source() {
        let plan = plan();
        assert!(
            !PathBuf::from("/scratch/run-1/claude-settings.json").starts_with(&plan.config_home)
        );
        assert_eq!(plan.settings["permissions"]["allow"], json!([]));
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
        assert_eq!(plan.attestation.unavailable.len(), 1);
        assert_eq!(plan.attestation.unavailable[0].row, HermeticRow::H9);
    }

    /// Git status is in the system prompt by the vendor's own account, and metaharness has no
    /// way to take it out — so it is reported rather than left to read as an absence.
    #[test]
    fn git_status_is_reported_as_an_ambient_input_and_not_claimed_removed() {
        let plan = plan();
        assert!(
            plan.attestation
                .ambient_inputs
                .iter()
                .any(|input| input.contains("git status"))
        );
    }

    #[test]
    fn a_run_with_no_inputs_digest_says_so_rather_than_leaving_h10_silent() {
        let mut context = context();
        context.inputs_digest = None;
        let plan = plan_launch(&spec(), &context).expect("plans");
        assert!(!plan.attestation.claims(HermeticRow::H10));
        assert!(
            plan.attestation
                .unavailable
                .iter()
                .any(|control| control.row == HermeticRow::H10)
        );
    }

    /// The document was resolved — read, parsed, digest-verified — above this seam, so a spec
    /// that still names it plans exactly the launch a spec without it plans.
    #[test]
    fn a_spec_naming_a_frame_document_plans_the_same_launch_as_one_without() {
        let mut framed = spec();
        framed.frame = Some(PathBuf::from("/scratch/run-1/frame.json"));
        let with_frame = plan_launch(&framed, &context()).expect("plans");
        let without = plan_launch(&spec(), &context()).expect("plans");
        assert_eq!(with_frame.args, without.args);
        assert_eq!(with_frame.env, without.env);
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

    // ------------------------------------------------------------ the operator cwd (a6)

    /// The declaration that trades two rows for real work: an operator-named cwd plans, and H7
    /// and H11 move from imposed to unavailable with the trade named — which is what makes
    /// `--hermetic strict` refuse such a run instead of the attestation quietly claiming a
    /// directory metaharness never made.
    #[test]
    fn an_operator_named_cwd_plans_and_gives_up_h7_and_h11_by_name() {
        let mut spec = spec();
        spec.cwd = Some(PathBuf::from("/operator/repo"));
        let mut context = context();
        context.cwd = PathBuf::from("/operator/repo");
        let plan = plan_launch(&spec, &context).expect("an operator cwd plans");

        for row in [HermeticRow::H7, HermeticRow::H11] {
            assert!(
                !plan.attestation.claims(row),
                "{} must not be claimed",
                row.id()
            );
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

    /// A memory file above an operator-named cwd is the operator's declared context, not a
    /// refusal: the tree is theirs, and the walk's findings go into H11's reason instead.
    #[test]
    fn memory_ancestors_above_an_operator_named_cwd_do_not_refuse_the_launch() {
        let mut spec = spec();
        spec.cwd = Some(PathBuf::from("/operator/repo"));
        let mut context = context();
        context.cwd = PathBuf::from("/operator/repo");
        context.memory_ancestors = vec![PathBuf::from("/operator/repo/AGENTS.md")];
        let plan = plan_launch(&spec, &context).expect("the declared tree plans");
        let h11 = plan
            .attestation
            .unavailable
            .iter()
            .find(|control| control.row == HermeticRow::H11)
            .expect("H11 unavailable");
        assert!(h11.why.contains("1 memory file"), "{}", h11.why);
    }

    #[test]
    fn a_run_for_another_harness_is_refused_by_name() {
        let mut spec = spec();
        spec.kind = Kind::Codex;
        assert_eq!(
            plan_launch(&spec, &context()),
            Err(LaunchRefusal::WrongKind {
                asked_for: Kind::Codex
            })
        );
    }

    #[test]
    fn every_refusal_says_what_it_refused_and_why() {
        let refusals = [
            LaunchRefusal::WrongKind {
                asked_for: Kind::Codex,
            },
            LaunchRefusal::NoPrompt,
            LaunchRefusal::CwdOutsideScratch {
                cwd: PathBuf::from("/a"),
                scratch_root: PathBuf::from("/b"),
            },
            LaunchRefusal::MemoryAncestorsFound {
                found: vec![PathBuf::from("/a/CLAUDE.md")],
            },
            LaunchRefusal::CredentialFileMissing,
            LaunchRefusal::ApiKeyMissing,
            LaunchRefusal::DeniedArgument {
                argument: "--bare".to_string(),
            },
            LaunchRefusal::DeniedEnvironment {
                key: "GIT_DIR".to_string(),
                row: HermeticRow::H3,
            },
            LaunchRefusal::Shadowed {
                entries: vec!["Bash".to_string()],
            },
        ];
        for refusal in refusals {
            let sentence = refusal.to_string();
            assert!(sentence.len() > 20, "{sentence}");
        }
    }
}
