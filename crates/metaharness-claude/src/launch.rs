//! The hermetic launch, constructed and never spawned.
//!
//! [`plan_launch`] is a pure function. It reads no file, no clock and no environment of its own:
//! everything it needs arrives in [`LaunchContext`], and everything it decides leaves as a value
//! on [`LaunchPlan`]. That is design § 8.4 O7, and the reason is the one `AEP`
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
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use metaharness_protocol::{
    CredentialSource, DecisionMode, Digest, HermeticAttestation, HermeticRow, ImposedControl,
    InstalledPlugin, Kind, MarketplacePlugin, PluginContent, PluginInstall, PluginTree,
    RefusalCode, ResolvedMarketplacePlugin, RunSpec, TierStatus, ToolSurface, UnavailableControl,
    required_commands,
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

/// Where the caller must write [`LaunchPlan::mcp_config`], under `--tool-surface owned`.
///
/// Beside the settings document and outside the config home, for the same reason: a config the
/// vendor discovered on its own would be a second source of servers, and `--strict-mcp-config`
/// exists precisely to say that this file is the only one.
const MCP_CONFIG_FILE: &str = "mcp-config.json";

/// The server name the tools are published under, and the string `--allowedTools` grants.
///
/// One constant because the two must agree: a grant naming a server the config does not define
/// admits nothing, and the run then has no tools at all — with `--tools ""` having already taken
/// the vendor's own away.
const TOOL_SERVER_NAME: &str = "metaharness";

/// Where an injected plugin is copied to: `<scratch root>/plugins/<name>`.
///
/// **Deliberately outside [`CONFIG_HOME`]**, and the reason is the same one that puts the settings
/// document outside it. Two facts, of two different strengths:
///
/// * **Verified** (`claude --help`, 2.1.240): *"`--plugin-dir <path>`  Load a plugin from a
///   directory or .zip for this session only (repeatable…)"*. The vendor is told the path, so no
///   particular location is required and metaharness may choose one it owns outright.
/// * **Read from the binary, and therefore weaker than a driven call**: the 2.1.240 bundle
///   resolves a `plugins` directory of its own under the config home — `join(…, "plugins")`,
///   beside `known_marketplaces.json` and a `marketplaces` cache. A copy placed there would share
///   a directory the vendor itself writes into, so *"the plugins are exactly the declared set"*
///   (H1a) would depend on the vendor's own bookkeeping not adding to it.
///
/// Given a free choice, the launch takes the directory nobody else has an opinion about.
const PLUGIN_HOME: &str = "plugins";

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

/// Where the argv's `--mcp-config` points, for a caller that has to write the document there.
///
/// Published for the same reason as [`settings_path`]: one place decides where it lives.
#[must_use]
pub fn mcp_config_path(scratch_root: &Path) -> PathBuf {
    scratch_root.join(MCP_CONFIG_FILE)
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
    ///
    /// Under [`CredentialSource::Loopback`] the operator's file is metaharness's own custody and
    /// **never travels**: the caller opens it on its side of the socket, and this plan's copy
    /// list stays empty whatever this field says.
    pub credentials_file: Option<PathBuf>,
    /// The running loopback proxy's endpoint and placeholder, under
    /// [`CredentialSource::Loopback`].
    ///
    /// The one value in this context that **cannot** be known before the run's own machinery
    /// starts: the proxy binds an ephemeral port, so unlike the static `--model-endpoint` there
    /// is nothing for a pure function to compute. The caller starts the proxy, fills this, and
    /// only then plans — and a `credentials: loopback` run that arrives here without it is
    /// [`LaunchRefusal::LoopbackNotStarted`] rather than a child launched at no endpoint with no
    /// credential.
    pub loopback: Option<LoopbackParams>,
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
    /// The metaharness binary, so `--tool-surface owned` can name a server to start.
    ///
    /// The caller's to supply, because finding it is `current_exe()` — I/O, and this function does
    /// none. A run that asked for the owned surface and arrives without it is
    /// [`LaunchRefusal::ToolServerMissing`] rather than a child launched with `--tools ""` and
    /// nothing to replace them: that child has no tools at all, and would spend the whole run
    /// explaining that it cannot read a file.
    pub tool_server: Option<PathBuf>,
    /// Every directory `spec.plugin_dir` named, **as the caller read it** (crossing #4).
    ///
    /// The caller does the walk and the per-file digests, on the same division as
    /// [`LaunchContext::memory_ancestors`]: this function decides where a plugin goes and whether
    /// the run may proceed, from values, and reads no directory of its own. A declared directory
    /// with no tree here is a caller that forgot to look, and it is refused rather than silently
    /// planned without the plugin.
    pub plugins: Vec<PluginTree>,
    /// Every `--plugin` the run declared, **resolved** against a marketplace the operator has
    /// already fetched (amendment a16, `crate::marketplace`).
    ///
    /// The same division as [`LaunchContext::plugins`]: the caller reads the operator's registry
    /// and the tree, this function decides where the copy goes and what the registry the child
    /// sees will say. A declared plugin with no resolution here is a caller that forgot to look,
    /// and it is refused rather than silently planned without.
    pub marketplace_plugins: Vec<ResolvedMarketplacePlugin>,
}

/// One document the caller writes into the scratch tree before the child starts.
///
/// A value on the plan, on the same division as [`LaunchPlan::settings`] and
/// [`LaunchPlan::mcp_config`]: the adapter decides what is in it and where it goes, and the caller
/// performs the I/O. That is what makes an assembled plugin registry a thing a test reads **before
/// any process exists** rather than a directory somebody inspects afterwards.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScratchFile {
    /// Where it goes, absolute, inside the run's own scratch tree.
    pub path: PathBuf,
    /// What is in it.
    pub document: Value,
}

/// What a started loopback proxy tells the launch, and the whole of it.
///
/// Two strings and no handle: this crate plans launches and owns no threads, so the proxy itself
/// stays on the library side and only its two addressable facts cross the seam. Both are
/// per-run — an ephemeral port and a nonce-bearing placeholder — which is what stops one run
/// reaching another's endpoint by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackParams {
    /// What the child's `ANTHROPIC_BASE_URL` is set to: `http://127.0.0.1:<port>`.
    pub base_url: String,
    /// The placeholder the child authenticates with, `mh-run-<id>-<nonce>`.
    ///
    /// Worthless anywhere but that port, which is the point of putting it in the child instead
    /// of the operator's token.
    pub placeholder: String,
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
    /// What to copy into the scratch tree **once**, before the child starts: one entry per
    /// declared plugin directory, each carrying the digest of what was read (crossing #4).
    ///
    /// A value on the plan, like everything else here, so *"the copy list and the digest are
    /// readable before any process exists"* is a property a test asserts rather than a sentence
    /// in a document. The argv's `--plugin-dir` names each entry's `to`, never the operator's own
    /// directory, so what the vendor loads is the snapshot this plan digested.
    pub plugin_installs: Vec<PluginInstall>,
    /// What to copy into the scratch **config home** once, before the child starts: one entry per
    /// declared `--plugin`, at the path Claude Code's own plugin cache uses (amendment a16).
    ///
    /// Separate from [`LaunchPlan::plugin_installs`] because the two are loaded by different
    /// mechanisms and only one of them is verified: `--plugin-dir` names its copy in the argv with
    /// the vendor's own flag, and this one is read out of the registry the config home carries.
    /// **The argv never names these**, because two mechanisms loading one plugin would report it
    /// twice under two different sources and H1a's *exactly the declared set* would have to be
    /// widened to accommodate a duplicate metaharness created itself.
    pub marketplace_installs: Vec<PluginInstall>,
    /// Documents the caller writes into the scratch tree before the child starts.
    ///
    /// Empty unless the run declared a `--plugin`, in which case it is the two registry documents
    /// Claude Code reads under the config home and the marketplace manifest beside them.
    pub scratch_files: Vec<ScratchFile>,
    /// The settings document the argv's `--settings` names. The caller writes it; this crate
    /// only decides what is in it.
    pub settings: Value,
    /// The MCP configuration the argv's `--mcp-config` names, under `--tool-surface owned`.
    ///
    /// `None` under every other surface, and the flag is then absent too. A value on the plan
    /// rather than a file this crate writes, on the same division as [`LaunchPlan::settings`]: the
    /// adapter decides what is in it and the caller performs the I/O.
    pub mcp_config: Option<Value>,
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
    /// `--tool-surface owned` was asked for and the caller named no metaharness binary.
    ///
    /// A refusal and not a fallback to the vendor's tools: the argv would carry `--tools ""` with
    /// nothing to replace them, and a child with no tools at all spends the whole run explaining
    /// that it cannot read a file — which reads, from the outside, exactly like a model that
    /// would not do the task.
    ToolServerMissing,
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
    /// The run declared `credentials: loopback` and no proxy was started for it.
    ///
    /// Refused rather than planned, because the alternative is the worst of both worlds: a child
    /// with no credential *and* no endpoint, which fails at its first request with a vendor error
    /// about authentication and tells nobody that metaharness never started the thing that was
    /// supposed to hold the credential.
    LoopbackNotStarted,
    /// A loopback proxy was started for a run that did not declare `credentials: loopback`.
    ///
    /// The mirror of the row above, and the dangerous direction: an api-key run whose base URL
    /// pointed at a local proxy would send the operator's real key to it. Refused rather than
    /// resolved by precedence.
    LoopbackProxyUndeclared {
        /// The endpoint that would have been imposed on the child.
        base_url: String,
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
    /// A declared plugin directory cannot be installed (crossing #4).
    ///
    /// Refused at plan time and never warned about, because the two cases it covers are the two
    /// ways an injection silently does nothing: a path that is not there — a typo, a plugin that
    /// was never built — and a directory that exists and holds no file, which is what a typo
    /// looks like after somebody "fixed" it by creating the directory. Either way the run would
    /// be the treatment-free arm wearing the treated arm's label.
    PluginDirUnusable {
        /// The directory the run named.
        directory: PathBuf,
        /// Which of the two, in the words that say what to do about it.
        why: String,
    },
    /// A declared `--plugin` cannot be installed into the scratch config home (amendment a16).
    ///
    /// The same refusal `--plugin-dir` gets and for the same reason: a run that installed no
    /// plugin and reported one would be the untreated run wearing the treated run's label. What
    /// differs is the causes — an unresolved declaration, an empty tree, an unreadable one.
    MarketplacePluginUnusable {
        /// The plugin, as the run spelled it.
        plugin: String,
        /// Why, in the words that say what to do about it.
        why: String,
    },
    /// The run asked for a decision mode this adapter has not driven.
    ///
    /// Design § 8.4 O4, applied to the mode table: an embedder that requires an unverified
    /// mechanism gets a refusal rather than a silent no-op. A mode the descriptor declares
    /// [`TierStatus::Unverified`] is refused here, so the descriptor and the behaviour cannot
    /// drift apart.
    DecisionModeUnverified {
        /// The mode that was asked for.
        mode: DecisionMode,
        /// What the adapter declares about it.
        status: TierStatus,
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
            LaunchRefusal::DecisionModeUnverified { .. } => Some(RefusalCode::UnsupportedControl),
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
            LaunchRefusal::ToolServerMissing => f.write_str(
                "--tool-surface owned needs the metaharness binary to serve the tools, and the \
                 caller named none. Without it the child gets --tools \"\" and nothing in their \
                 place",
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
                "the run's credential source needs the operator's credential file and none was \
                 named: an operator login has nothing to copy into the scratch home, and a \
                 credentials: loopback run has nothing to put in custody behind the proxy",
            ),
            LaunchRefusal::ApiKeyMissing => f.write_str(
                "the run declared credentials: api-key and ANTHROPIC_API_KEY was not in the \
                 caller's environment",
            ),
            LaunchRefusal::EndpointWithCredential { endpoint } => write!(
                f,
                "the run declared a model endpoint ({endpoint}) together with a real credential \
                 source; a child pointed at a foreign endpoint must hold no operator credential, \
                 so declare credentials: none — the child gets a placeholder key instead — or \
                 credentials: loopback, where the endpoint becomes the proxy's upstream and the \
                 child still holds nothing"
            ),
            LaunchRefusal::LoopbackNotStarted => f.write_str(
                "the run declared credentials: loopback and the launch context carries no proxy; \
                 the base URL is an ephemeral port and is therefore only known after the proxy \
                 starts, so the caller starts it and fills LaunchContext.loopback before planning",
            ),
            LaunchRefusal::LoopbackProxyUndeclared { base_url } => write!(
                f,
                "a loopback proxy at {base_url} was started for a run that did not declare \
                 credentials: loopback; pointing a credentialed child at a local proxy would send \
                 the operator's own key to it, so the two are refused rather than composed"
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
            LaunchRefusal::PluginDirUnusable { directory, why } => write!(
                f,
                "--plugin-dir {} cannot be installed: {why}. It is refused rather than skipped, \
                 because a run that installed no plugin and reported one would be the untreated \
                 run wearing the treated run's label",
                directory.display()
            ),
            LaunchRefusal::MarketplacePluginUnusable { plugin, why } => write!(
                f,
                "--plugin {plugin} cannot be installed: {why}. It is refused rather than skipped, \
                 because a run that installed no plugin and reported one would be the untreated \
                 run wearing the treated run's label"
            ),
            LaunchRefusal::DecisionModeUnverified { mode, status } => write!(
                f,
                "the run asked for --decisions {} and the {ADAPTER_ID} adapter declares that mode \
                 {}; an embedder that requires a mechanism nobody drove is refused rather than \
                 quietly served (design § 8.4 O4)",
                mode.as_str(),
                match status {
                    TierStatus::Unverified =>
                        "unverified — the mechanism is on the vendor's surface and no run here has \
                         driven it",
                    TierStatus::Absent => "absent — this vendor has no such mechanism",
                    TierStatus::Delivered =>
                        "delivered, which is not a refusal and is a defect here",
                }
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
    guard_decision_mode(spec)?;
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

    // `loopback` is exempt: under it the declared endpoint is not the child's base URL at all —
    // it is the **proxy's upstream**, one hop further out — and the child holds a placeholder
    // either way. The row exists to stop an operator credential travelling to a foreign host, and
    // a loopback run has no credential in the child to travel.
    if let Some(endpoint) = &spec.model_endpoint
        && !matches!(
            spec.credentials,
            CredentialSource::None | CredentialSource::Loopback
        )
    {
        return Err(LaunchRefusal::EndpointWithCredential {
            endpoint: endpoint.clone(),
        });
    }
    guard_loopback(spec, context)?;

    let config_home = context.scratch_root.join(CONFIG_HOME);
    let credential_copies = credential_copies(spec, context, &config_home)?;
    let plugin_installs = plugin_installs(spec, context)?;
    let marketplace_installs = marketplace_installs(spec, context, &config_home)?;
    let scratch_files = scratch_registry_files(spec, context, &config_home)?;
    let mcp_config = build_mcp_config(spec, context)?;
    let prompt = spec.with_agent_execution_context(prompt);
    let args = build_args(
        spec,
        &prompt,
        &context.scratch_root.join(SETTINGS_FILE),
        &plugin_installs,
        &marketplace_installs,
        mcp_config
            .is_some()
            .then(|| mcp_config_path(&context.scratch_root)),
    );
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
        mcp_config,
        hook,
        attestation: attest(
            spec,
            context,
            &config_home,
            &plugin_installs,
            &marketplace_installs,
        ),
        plugin_installs,
        marketplace_installs,
        scratch_files,
    })
}

/// The copy list for every declared `--plugin`, into the scratch config home.
///
/// The digest is computed **before** the copy, over the operator's own tree, exactly as
/// `--plugin-dir`'s is: that is what the attestation is a claim about.
fn marketplace_installs(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
) -> Result<Vec<PluginInstall>, LaunchRefusal> {
    let mut installs = Vec::new();
    for declared in &spec.plugin {
        let resolved = resolution(context, declared)?;
        let digest = match &resolved.tree.content {
            PluginContent::Files { digest, .. } => digest.clone(),
            PluginContent::Empty => {
                return Err(LaunchRefusal::MarketplacePluginUnusable {
                    plugin: declared.to_string(),
                    why: "the resolved directory holds no file at all, so the run would install \
                          nothing and report a plugin"
                        .to_string(),
                });
            }
            PluginContent::Unreadable { detail } => {
                return Err(LaunchRefusal::MarketplacePluginUnusable {
                    plugin: declared.to_string(),
                    why: format!("the resolved directory could not be read: {detail}"),
                });
            }
        };
        installs.push(PluginInstall {
            from: resolved.tree.source.clone(),
            to: plugin_cache_path(config_home, resolved),
            digest,
        });
    }
    Ok(installs)
}

/// Where one resolved plugin's tree goes: `<config home>/plugins/cache/<mkt>/<name>/<version>`.
fn plugin_cache_path(config_home: &Path, resolved: &ResolvedMarketplacePlugin) -> PathBuf {
    config_home
        .join(crate::marketplace::PLUGIN_CACHE_HOME)
        .join(&resolved.marketplace)
        .join(&resolved.requested.name)
        .join(&resolved.version)
}

/// Where one marketplace's checkout goes inside the scratch home.
fn marketplace_path(config_home: &Path, marketplace: &str) -> PathBuf {
    config_home
        .join(crate::marketplace::MARKETPLACES_HOME)
        .join(marketplace)
}

/// The resolution the caller was supposed to supply for a declared plugin.
fn resolution<'a>(
    context: &'a LaunchContext,
    declared: &MarketplacePlugin,
) -> Result<&'a ResolvedMarketplacePlugin, LaunchRefusal> {
    context
        .marketplace_plugins
        .iter()
        .find(|resolved| resolved.requested == *declared)
        .ok_or_else(|| LaunchRefusal::MarketplacePluginUnusable {
            plugin: declared.to_string(),
            why: "the caller planned a launch without resolving it, so nothing digested it and \
                  there is nothing to copy"
                .to_string(),
        })
}

/// The registry documents and manifests the scratch config home needs, as values.
fn scratch_registry_files(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
) -> Result<Vec<ScratchFile>, LaunchRefusal> {
    if spec.plugin.is_empty() {
        return Ok(Vec::new());
    }
    let mut entries = Vec::new();
    for declared in &spec.plugin {
        let resolved = resolution(context, declared)?;
        entries.push(crate::marketplace::ScratchEntry {
            marketplace: resolved.marketplace.clone(),
            repo: resolved.requested.repo.clone(),
            name: resolved.requested.name.clone(),
            version: resolved.version.clone(),
            commit: resolved.commit.clone(),
            installed_at: plugin_cache_path(config_home, resolved)
                .display()
                .to_string(),
            marketplace_at: marketplace_path(config_home, &resolved.marketplace)
                .display()
                .to_string(),
        });
    }

    let (marketplaces, plugins) = crate::marketplace::scratch_registry(&entries);
    let mut files = vec![
        ScratchFile {
            path: config_home.join(crate::marketplace::KNOWN_MARKETPLACES),
            document: marketplaces,
        },
        ScratchFile {
            path: config_home.join(crate::marketplace::INSTALLED_PLUGINS),
            document: plugins,
        },
    ];
    // One manifest per marketplace, so the checkout the registry points at is a marketplace and
    // not an empty directory. Minimal on purpose: the plugin's own tree is the copy, and a
    // manifest that listed a `source` outside the scratch home would send the vendor back to the
    // operator's tree.
    let mut seen: Vec<String> = Vec::new();
    for entry in &entries {
        if seen.contains(&entry.marketplace) {
            continue;
        }
        seen.push(entry.marketplace.clone());
        let plugins_of: Vec<Value> = entries
            .iter()
            .filter(|other| other.marketplace == entry.marketplace)
            .map(|other| serde_json::json!({"name": other.name, "source": other.installed_at}))
            .collect();
        files.push(ScratchFile {
            path: marketplace_path(config_home, &entry.marketplace)
                .join(".claude-plugin")
                .join("marketplace.json"),
            document: serde_json::json!({
                "name": entry.marketplace,
                "owner": {"name": entry.repo},
                "plugins": plugins_of,
            }),
        });
    }
    Ok(files)
}

/// The mode this run asked for against the mode table this adapter publishes.
///
/// One function, reading [`crate::capabilities`], so the descriptor an embedder queries and the
/// behaviour it gets cannot disagree: a mode declared unverified there is refused here, and a mode
/// declared delivered is planned.
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

/// The plugin copy list, one entry per declared directory, or the refusal that says why not.
///
/// The digest is computed **before** the copy, over the operator's own directory, because that is
/// what the attestation is a claim about: the plugin as it stood when the run took its snapshot.
fn plugin_installs(
    spec: &RunSpec,
    context: &LaunchContext,
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
            to: context.scratch_root.join(PLUGIN_HOME).join(tree.name()),
            digest,
        });
    }
    Ok(installs)
}

/// What the attestation says about one installed plugin, and how strong the claim is.
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
                "--plugin-dir {} in the argv: the vendor's own flag, which loads a plugin from a \
                 named directory for one session (verified, claude --help 2.1.240). Whether it \
                 then appears in the session is asserted from the opening record's plugin list \
                 (H1a), never from this row",
                install.to.display()
            ),
        })
        .collect()
}

/// What the attestation says about one **marketplace** plugin, and how strong the claim is.
///
/// The `loaded_by` sentence is the honest half of amendment a16: the layout was **read from a real
/// config home** at 2.1.258 and recorded in the research note, and nobody has driven a session
/// against a config home metaharness assembled. So it says *not driven*, names the open probe, and
/// points at the record that would settle it — the opening record's plugin list, which is H1a's
/// own comparison (invariant 4, invariant 3).
fn marketplace_installed_plugins(
    spec: &RunSpec,
    context: &LaunchContext,
    installs: &[PluginInstall],
) -> Vec<InstalledPlugin> {
    spec.plugin
        .iter()
        .zip(installs)
        .map(|(declared, install)| {
            let marketplace = context
                .marketplace_plugins
                .iter()
                .find(|resolved| resolved.requested == *declared)
                .map_or_else(
                    || "unknown".to_string(),
                    |resolved| resolved.marketplace.clone(),
                );
            InstalledPlugin {
                name: declared.name.clone(),
                source: format!("{declared} (marketplace {marketplace})"),
                installed_at: install.to.display().to_string(),
                digest: install.digest.clone(),
                loaded_by: format!(
                    "placed in the scratch config home's plugin registry — the tree at {}, named by \
                     `plugins/installed_plugins.json` as `{}@{marketplace}` and pinned to `{}` — \
                     **and** named to the vendor with its own --plugin-dir flag pointing at that \
                     same copy. Both, because the registry alone loads nothing under this launch: \
                     the vendor enables an installed plugin through the user settings source, which \
                     `--setting-sources \"\"` (H2) switches off. Probe Q19 closed on 2026-09-03: a \
                     session declaring two pinned plugins opened with only the --plugin-dir one in \
                     its plugin list (docs/research/2026-09-03-claude-plugin-headless-install.md \
                     § 4). Whether the plugin appears in the session is still read from the opening \
                     record\'s plugin list (H1a), never from this row",
                    install.to.display(),
                    declared.name,
                    declared.pin,
                ),
            }
        })
        .collect()
}

/// The declaration and the started proxy must agree, in both directions.
///
/// Two failures rather than one, because they fail in opposite ways: without a proxy the child
/// reaches nothing, and with an undeclared proxy a credentialed child reaches a local socket
/// holding the operator's own key. Neither is a warning — a control that reports and proceeds has
/// already stopped controlling (design § 7.1).
fn guard_loopback(spec: &RunSpec, context: &LaunchContext) -> Result<(), LaunchRefusal> {
    let declared = spec.credentials == CredentialSource::Loopback;
    match (declared, &context.loopback) {
        (true, None) => Err(LaunchRefusal::LoopbackNotStarted),
        (false, Some(params)) => Err(LaunchRefusal::LoopbackProxyUndeclared {
            base_url: params.base_url.clone(),
        }),
        // Both present or both absent: the two halves agree, which is the whole of the check.
        (true, Some(_)) | (false, None) => Ok(()),
    }
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
        // Nothing travels under either. `Loopback` is the stronger of the two and says so in H6:
        // the operator's file is real and is held by metaharness's own custody, on this side of
        // the socket, so the copy list being empty is the whole claim.
        CredentialSource::Loopback | CredentialSource::None => Ok(Vec::new()),
    }
}

/// The command line.
///
/// `--verbose` is not decoration: the vendor refuses the combination without it —
/// *"Error: When using --print, --output-format=stream-json requires --verbose"*, read from the
/// 2.1.239 binary. Without a stream there is no transcript, and without a transcript there is
/// nothing for design § 9.4's auditor to read.
fn build_args(
    spec: &RunSpec,
    prompt: &str,
    settings_path: &Path,
    plugin_installs: &[PluginInstall],
    marketplace_installs: &[PluginInstall],
    mcp_config_path: Option<PathBuf>,
) -> Vec<String> {
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
    if let Some(budget) = &spec.max_budget_usd {
        // The vendor's own stop, and the only cap that acts *during* a run: a runner comparing the
        // bill against a cap between two runs holds a receipt, not a ceiling. `--max-budget-usd`
        // is documented for print mode only (`claude --help`, 2.1.259), which is the only mode this
        // adapter launches.
        args.push("--max-budget-usd".to_string());
        args.push(budget.clone());
    }
    // The **copy**, never the operator's own directory: what the vendor loads has to be the tree
    // this plan digested, and a directory outside the scratch can be edited while the run is in
    // flight — which would leave the attestation citing a digest of something that is no longer
    // there.
    for install in plugin_installs {
        args.push("--plugin-dir".to_string());
        args.push(install.to.display().to_string());
    }
    // A pinned marketplace plugin is named the same way, pointing at its copy inside the scratch
    // config home. The registry documents beside that copy are still written, but on their own they
    // load nothing here: the vendor enables an installed plugin through the *user* settings source,
    // and `--setting-sources ""` (H2) is the flag that switches that source off. Probe Q19 closed on
    // 2026-09-03 with exactly that observation — a session declaring two pinned plugins opened with
    // only the `--plugin-dir` one in its `plugins` list (`docs/research/2026-09-03-claude-plugin-
    // headless-install.md` § 4).
    for install in marketplace_installs {
        args.push("--plugin-dir".to_string());
        args.push(install.to.display().to_string());
    }
    if let Some(mcp_config_path) = mcp_config_path {
        // Strategy C (design § 7.5), and all three flags are one act. `--tools ""` disables the
        // whole built-in set (V11) — no `Bash` to deny, because there is no `Bash`. `--mcp-config`
        // names the server that replaces them. The grant is a whole-server one because nothing in
        // the vendor's own settings has heard of these tools, and it is *bare*: under this surface
        // no decision travels to the vendor per call — metaharness runs the tool itself — so there
        // is no seam for a bare grant to shadow. `guard_shadowing` still refuses the combination
        // wherever a seam *does* need to see the call, which `--decisions ask` is.
        args.push("--tools".to_string());
        args.push(String::new());
        args.push("--mcp-config".to_string());
        args.push(mcp_config_path.display().to_string());
        args.push("--allowedTools".to_string());
        args.push(format!("mcp__{TOOL_SERVER_NAME}"));
    }
    args
}

/// The MCP configuration under `--tool-surface owned`, and `None` under every other surface.
///
/// One stdio server, which is **this binary** under its own `mcp-serve` subcommand: the program
/// the vendor starts is therefore already installed, already the version this run is, and needs no
/// second artefact to be built, found or kept in step.
///
/// The workspace it serves is the child's own cwd, so the model sees the tree it was pointed at
/// and nothing above it.
fn build_mcp_config(
    spec: &RunSpec,
    context: &LaunchContext,
) -> Result<Option<Value>, LaunchRefusal> {
    if spec.tool_surface != ToolSurface::Owned {
        return Ok(None);
    }
    let Some(server) = &context.tool_server else {
        return Err(LaunchRefusal::ToolServerMissing);
    };

    let mut args = vec![
        "mcp-serve".to_string(),
        "--workspace".to_string(),
        context.cwd.display().to_string(),
        "--writable".to_string(),
    ];
    for program in &spec.allow_program {
        args.push("--allow-program".to_string());
        args.push(program.clone());
    }

    Ok(Some(json!({
        "mcpServers": {
            TOOL_SERVER_NAME: {
                "type": "stdio",
                "command": server.display().to_string(),
                "args": args,
            }
        }
    })))
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
    // **Declared by the caller, so it cannot be set by the shell that launched us.** The subject
    // stamps `human:$USER` on a store write unless told otherwise, which makes a driven session's
    // `artifact move` indistinguishable from a person's. Adding `AEP_ACTOR` to [`INHERITED_KEYS`]
    // would have carried it — and would have carried the operator's value in any run the driver did
    // not set one for, which is provenance the surrounding environment can forge. This reads the
    // spec instead, so the value is the caller's statement and the attestation can call it imposed.
    if let Some(actor) = &spec.actor {
        env.insert("AEP_ACTOR".to_string(), actor.clone());
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
    if let Some(loopback) = &context.loopback {
        loopback_env(&mut env, loopback);
    } else if let Some(endpoint) = &spec.model_endpoint {
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

/// The three variables a loopback child runs on, and the one it must not have.
///
/// Each line is a spike finding rather than a preference, and each would fail silently:
///
/// | variable | why exactly this |
/// |---|---|
/// | `ANTHROPIC_BASE_URL` | the proxy's own ephemeral port; plain HTTP and loopback-only, because the hop the operator wants to inspect must not be encrypted and never leaves the machine |
/// | `ANTHROPIC_AUTH_TOKEN` | the placeholder, delivered in **this spelling only**, which the binary sends as `Authorization: Bearer`. `ANTHROPIC_API_KEY` is deliberately **not** also set: two credential variables mean two spellings on the wire and a precedence rule nobody here owns |
/// | `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` | **without it Claude Code opens `api.anthropic.com:443` for first-party analytics whatever the base URL says** — a channel the proxy cannot see, which makes *"the proxy sees every request"* false. Verified: this variable removes it |
///
/// The absent `ANTHROPIC_API_KEY` is not merely omitted here: [`is_scrubbed`] keeps it scrubbed
/// under `loopback`, so an operator's exported key cannot arrive by the inheritance route either.
fn loopback_env(env: &mut BTreeMap<String, String>, loopback: &LoopbackParams) {
    env.insert(
        "ANTHROPIC_BASE_URL".to_string(),
        loopback.base_url.trim_end_matches('/').to_string(),
    );
    env.insert(
        "ANTHROPIC_AUTH_TOKEN".to_string(),
        loopback.placeholder.clone(),
    );
    env.insert(DISABLE_NONESSENTIAL_TRAFFIC.to_string(), "1".to_string());
}

/// What the child authenticates with under a declared model endpoint: a marker, not a secret.
const ENDPOINT_PLACEHOLDER_KEY: &str = "metaharness-model-endpoint";

/// The variable that closes Claude Code's first-party analytics channel.
///
/// Named as a constant because the whole inspection claim of the loopback provider rests on it:
/// the binary otherwise talks to `api.anthropic.com` directly, past a base URL that redirected
/// everything else, and nothing in the run's record would say so.
const DISABLE_NONESSENTIAL_TRAFFIC: &str = "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC";

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
///
/// `loopback` relaxes exactly three keys and tightens one. The three
/// ([`loopback_env`]) are the run's own values and cannot arrive from the operator's environment,
/// because that environment is read through the [`INHERITED_KEYS`] allowlist and none of them is
/// on it. The tightened one is `ANTHROPIC_API_KEY`: under `loopback` it is scrubbed
/// *unconditionally*, including beside a declared model endpoint, because the endpoint under
/// `loopback` is the **proxy's upstream** and the child authenticates with the placeholder bearer
/// alone — a second credential variable would put a second spelling on the wire for no reason.
fn is_scrubbed(key: &str, spec: &RunSpec) -> bool {
    let credentials = spec.credentials;
    let loopback = credentials == CredentialSource::Loopback;
    if key == "ANTHROPIC_API_KEY" {
        return loopback
            || (credentials != CredentialSource::ApiKey && spec.model_endpoint.is_none());
    }
    if key == "ANTHROPIC_BASE_URL" {
        return !loopback && spec.model_endpoint.is_none();
    }
    if key == "ANTHROPIC_AUTH_TOKEN" || key == DISABLE_NONESSENTIAL_TRAFFIC {
        return !loopback;
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
/// The guard reads the argv this adapter just built, which is why it is a guard rather than a
/// check on the spec. `--tool-surface owned` is the only configuration that puts a bare entry
/// there, and on its own it is **not** the trap: under strategy C metaharness runs the tool, no
/// decision travels to the vendor per call, and there is no seam for the grant to shadow.
///
/// Combine it with `--decisions ask` and there is: the operator is answering every call, and a
/// bare `--allowedTools` entry auto-approves before their answer is asked for. That is what this
/// refuses, and the golden vector `c1-shadow-refusal` is exactly that pair.
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

/// Both injection mechanisms, in **one** list, each row saying which carried it.
///
/// One list and not two, because H1a is *"plugins are exactly the declared set"* and a reader
/// comparing that set against the vendor's opening record has to be comparing the whole of it.
fn all_installed_plugins(
    spec: &RunSpec,
    context: &LaunchContext,
    plugin_installs: &[PluginInstall],
    marketplace_installs: &[PluginInstall],
) -> Vec<InstalledPlugin> {
    let mut all = installed_plugins(plugin_installs);
    all.extend(marketplace_installed_plugins(
        spec,
        context,
        marketplace_installs,
    ));
    all
}

/// What metaharness claims it imposed, and what it says it could not.
#[allow(
    clippy::too_many_lines,
    reason = "one row per hermetic control, read top to bottom against § 8.1's list; splitting it \
              would put the twelve rows in more than one place"
)]
fn attest(
    spec: &RunSpec,
    context: &LaunchContext,
    config_home: &Path,
    plugin_installs: &[PluginInstall],
    marketplace_installs: &[PluginInstall],
) -> HermeticAttestation {
    let home = config_home.display();
    let mut imposed = vec![
        control_imposed(
            HermeticRow::H1a,
            plugin_posture(&home.to_string(), plugin_installs, marketplace_installs),
        ),
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

    match spec.credentials {
        CredentialSource::OperatorLogin => imposed.push(control_imposed(
            HermeticRow::H6,
            "one credential file copied into the scratch home immediately before every spawn, \
             and nothing else (Q13)",
        )),
        // The upgrade LP-3 exists for. H6's copy row is advisory — it claims something about a
        // file this plan hands to a runner — whereas this claim is readable off the launch values
        // themselves: no entry in `credential_copies`, and an `ANTHROPIC_AUTH_TOKEN` that is
        // worth nothing anywhere but one ephemeral loopback port.
        CredentialSource::Loopback => imposed.push(control_imposed(
            HermeticRow::H6,
            format!("the scratch home holds no credential file at all: {LOOPBACK_POSTURE}"),
        )),
        CredentialSource::ApiKey | CredentialSource::None => unavailable.push(control_unavailable(
            HermeticRow::H6,
            "the run declared no operator login, so there is no credential file to copy",
        )),
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
        installed_plugins: all_installed_plugins(
            spec,
            context,
            plugin_installs,
            marketplace_installs,
        ),
    }
}

/// H1a's `how`, which now has to say two things: the home is scratch, and what was put in it.
///
/// One sentence rather than two rows, because H1a is *"plugins are exactly the declared set"* and
/// the declared set is only meaningful beside the home it is declared into. A plugin-less run says
/// so out loud — an H1a row that went quiet when nothing was injected would read as a row that
/// stopped checking.
fn plugin_posture(
    config_home: &str,
    installs: &[PluginInstall],
    marketplace_installs: &[PluginInstall],
) -> String {
    if installs.is_empty() && marketplace_installs.is_empty() {
        return format!(
            "CLAUDE_CONFIG_DIR={config_home}, and no plugin was injected: the declared set is empty"
        );
    }
    let names = |installs: &[PluginInstall]| -> Vec<String> {
        installs
            .iter()
            .map(|install| {
                install
                    .to
                    .file_name()
                    .map_or_else(String::new, |name| name.to_string_lossy().into_owned())
            })
            .collect()
    };
    let mut said = format!("CLAUDE_CONFIG_DIR={config_home}");
    if !installs.is_empty() {
        let directories = names(installs);
        let _ = write!(
            said,
            ", and the declared set includes {directories:?} — each copied into the scratch tree \
             at launch, digested before the copy, and named to the vendor with its own \
             --plugin-dir flag"
        );
    }
    // **Stated apart from the directories above, because the two arrive differently** (amendment
    // a16): a directory is copied where metaharness chooses, a pinned plugin is copied into the
    // config home's own plugin cache with the registry documents beside it. Both are then named to
    // the vendor with `--plugin-dir`, because the registry alone loads nothing while H2 switches the
    // user settings source off (probe Q19, closed 2026-09-03).
    if !marketplace_installs.is_empty() {
        let placed = names(marketplace_installs);
        let _ = write!(
            said,
            ", and it also includes the pinned marketplace plugin(s) {placed:?} — copied into this \
             config home's own plugin cache, named by the registry documents beside it, and named \
             to the vendor with its own --plugin-dir flag, because the registry alone loads nothing \
             while --setting-sources is empty (probe Q19, closed 2026-09-03). Whether the session \
             loaded them is read from the opening record's plugin list, never from this row"
        );
    }
    said
}

/// The one sentence that says what a loopback run's credential posture is.
///
/// Written once and used by both H4 and H6, because the two rows are two views of the same fact
/// and a reader who found them worded differently would have to work out whether they meant
/// different things.
const LOOPBACK_POSTURE: &str = "no operator credential in the child; a placeholder authenticates \
                                to metaharness's loopback proxy, which holds custody";

fn api_key_posture(spec: &RunSpec) -> String {
    if spec.credentials == CredentialSource::Loopback {
        // Checked before the endpoint row: under loopback a declared endpoint is the proxy's
        // upstream, and saying the child holds a placeholder *key* would name the wrong variable.
        format!("ANTHROPIC_API_KEY is absent from the child environment — {LOOPBACK_POSTURE}")
    } else if spec.model_endpoint.is_some() {
        "ANTHROPIC_API_KEY carries this plan's own placeholder for the declared model endpoint; \
         no operator credential is in the child at all"
            .to_string()
    } else if spec.credentials == CredentialSource::ApiKey {
        "ANTHROPIC_API_KEY is in the child environment because the run declared credentials: \
         api-key"
            .to_string()
    } else {
        "ANTHROPIC_API_KEY is absent from the child environment".to_string()
    }
}

/// Inputs metaharness reports and does **not** claim to have removed.
///
/// Both are named by the design rather than discovered here, and both would otherwise be read
/// out of the attestation's silence as absences.
fn ambient_inputs(spec: &RunSpec) -> Vec<String> {
    let mut inputs = vec![
        "git status: the vendor's own --exclude-dynamic-system-prompt-sections description says \
         cwd, env info, memory paths and git status are in the system prompt (design § 8.1, H11's \
         second half)"
            .to_string(),
        "network access: Claude Code's CLI carries no sandbox knob, so a hermetic run here is not \
         network-isolated (design § 8.2)"
            .to_string(),
    ];
    if spec.decisions == DecisionMode::Observe {
        // The price of the capture mode, stated where a reader of the record will meet it rather
        // than left in a document. `allow` grants on this wire: the binary carries "Hook approved
        // tool use for ${name}, bypassing permission prompt" (§ 6, finding F8), so a run that
        // answers `allow` to everything is **more** permissive than a run with no hook at all.
        inputs.push(
            "observe mode: every call is allowed at the PreToolUse seam, and an allow on this \
             wire grants — it bypasses the rest of the vendor's permission pipeline and overrides \
             a stricter rule in the vendor's own settings (finding F8). This run is therefore not \
             a run with the seam switched off; it is a run whose seam permits everything"
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
            plugins: Vec::new(),
            marketplace_plugins: Vec::new(),
            loopback: None,
            tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
        }
    }

    /// Who the run writes as is the caller's statement, and the operator's shell cannot forge it.
    ///
    /// The subject stamps `human:$USER` on a store write unless `AEP_ACTOR` says otherwise, so a
    /// driven session's `artifact move` reads as a person's. Carrying it on [`INHERITED_KEYS`] was
    /// the proposed fix and is refused here in the only way that matters: the context below has
    /// `AEP_ACTOR` set in the operator's environment, and a spec that declares nothing must still
    /// send nothing. Provenance a surrounding shell can set is not provenance.
    #[test]
    fn the_actor_a_run_writes_as_is_declared_and_never_inherited() {
        let mut world = context();
        world
            .inherited_env
            .insert("AEP_ACTOR".to_string(), "human:someone-else".to_string());

        let mut spec = RunSpec::new(Kind::Claude);
        spec.prompt = Some("do the thing".to_owned());
        let plan = plan_launch(&spec, &world).expect("plans");
        assert!(
            !plan.env.contains_key("AEP_ACTOR"),
            "an undeclared actor must not be picked up from the operator's environment: {:?}",
            plan.env
        );

        spec.actor = Some("agent:EVAL-1.1".to_owned());
        let plan = plan_launch(&spec, &world).expect("plans");
        assert_eq!(
            plan.env.get("AEP_ACTOR").map(String::as_str),
            Some("agent:EVAL-1.1"),
            "the declared actor is what reaches the child: {:?}",
            plan.env
        );
    }

    /// A plugin directory the caller has "read", with a digest over two invented files.
    fn plugin_tree() -> (PathBuf, PluginTree, Digest) {
        let source = PathBuf::from("/operator/integrations/claude-code");
        let files: std::collections::BTreeMap<String, Digest> = [
            (".claude-plugin/plugin.json".to_string(), Digest::of(b"{}")),
            ("skills/one/SKILL.md".to_string(), Digest::of(b"body")),
        ]
        .into_iter()
        .collect();
        let digest = metaharness_protocol::tree_digest(&files);
        (
            source.clone(),
            PluginTree {
                source,
                content: PluginContent::Files {
                    count: files.len(),
                    digest: digest.clone(),
                },
            },
            digest,
        )
    }

    /// A spec and a context that agree on one declared plugin, as the builder produces them.
    fn plugin_world() -> (RunSpec, LaunchContext, Digest) {
        let (source, tree, digest) = plugin_tree();
        let mut spec = spec();
        spec.plugin_dir.push(source);
        let mut context = context();
        context.plugins.push(tree);
        (spec, context, digest)
    }

    /// A spec and a context that agree on a started proxy, as the builder produces them.
    fn loopback_world() -> (RunSpec, LaunchContext) {
        let mut spec = spec();
        spec.credentials = CredentialSource::Loopback;
        let mut context = context();
        context.loopback = Some(LoopbackParams {
            base_url: "http://127.0.0.1:44321".to_string(),
            placeholder: "mh-run-claude-7-0f1e2d".to_string(),
        });
        (spec, context)
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
        assert!(
            plan.args[1].ends_with("\n\ndo the thing"),
            "{}",
            plan.args[1]
        );
        assert!(
            plan.args[1].contains("execution_path=metaharness-driven"),
            "{}",
            plan.args[1]
        );
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

    // ------------------------------------------------------------ observe mode (a10) and #4

    /// Observe mode reaches the record: the attestation names it, and the price of the mode — an
    /// `allow` on this wire **grants** (finding F8) — is stated there too, so a reader of a
    /// captured run does not have to know the design to know what they are looking at.
    #[test]
    fn an_observe_run_attests_the_mode_and_says_what_an_allow_costs() {
        let mut spec = spec();
        spec.decisions = DecisionMode::Observe;
        let plan = plan_launch(&spec, &context()).expect("observe plans");
        assert_eq!(plan.attestation.decisions, DecisionMode::Observe);
        assert!(plan.attestation.is_observing());
        let caveat = plan
            .attestation
            .ambient_inputs
            .iter()
            .find(|input| input.contains("observe mode"))
            .expect("the mode's price is reported");
        assert!(
            caveat.contains("grants") && caveat.contains("F8"),
            "{caveat}"
        );
    }

    /// **A run that did not ask for observe mode never gets it.** The polarity, asserted rather
    /// than assumed: a mode that allows every call must be reachable by asking and by nothing
    /// else, and an attestation that claimed it for a frame-mode run would be a record saying the
    /// control was off when it was on.
    #[test]
    fn a_run_that_did_not_ask_for_observe_mode_never_gets_it() {
        for mode in [DecisionMode::Frame, DecisionMode::Ask] {
            let mut spec = spec();
            spec.decisions = mode;
            let plan = plan_launch(&spec, &context()).expect("plans");
            assert_eq!(plan.attestation.decisions, mode);
            assert!(
                !plan.attestation.is_observing(),
                "--decisions {} read back as observing",
                mode.as_str()
            );
            assert!(
                !plan
                    .attestation
                    .ambient_inputs
                    .iter()
                    .any(|input| input.contains("observe mode")),
                "--decisions {} reports observe mode's caveat",
                mode.as_str()
            );
        }
        // And the default, which is what a spec nobody touched carries.
        assert_eq!(
            RunSpec::new(Kind::Claude).decisions,
            DecisionMode::Frame,
            "the default must not be the mode that allows everything"
        );
    }

    /// Observe mode needs the same per-call channel a launch-time frame needs, because it writes
    /// a decision per call. Declared through `required_commands` so an adapter that could not
    /// honour it would be refused at run start rather than at the first call.
    #[test]
    fn observe_mode_needs_the_call_seam_like_every_other_deciding_mode() {
        let mut spec = spec();
        spec.decisions = DecisionMode::Observe;
        assert!(required_commands(&spec).contains(&"tool.decide"));
        assert!(needs_call_seam(&spec));
    }

    /// The mode table and the plan-time refusal are one decision. On this adapter all three modes
    /// are delivered, so all three plan — and if the table ever said otherwise, this would fail
    /// rather than let a published capability drift from what a run gets.
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

    /// Crossing #4: **the plan is a value.** The copy list and the digest are readable before any
    /// process exists, the argv names the copy rather than the operator's own directory, and the
    /// attestation carries the digest and the source.
    #[test]
    fn a_declared_plugin_is_a_copy_list_and_a_digest_before_anything_is_spawned() {
        let (spec, context, digest) = plugin_world();
        let plan = plan_launch(&spec, &context).expect("the injection plans");

        assert_eq!(plan.plugin_installs.len(), 1);
        let install = &plan.plugin_installs[0];
        assert_eq!(
            install.from,
            PathBuf::from("/operator/integrations/claude-code")
        );
        assert_eq!(
            install.to,
            PathBuf::from("/scratch/run-1/plugins/claude-code")
        );
        assert_eq!(install.digest, digest);
        assert!(
            !install.to.starts_with(&plan.config_home),
            "the copy stays out of the directory the vendor keeps its own plugin bookkeeping in"
        );

        let named = plan
            .args
            .windows(2)
            .find(|pair| pair[0] == "--plugin-dir")
            .expect("--plugin-dir is in the argv");
        assert_eq!(named[1], "/scratch/run-1/plugins/claude-code");
        assert!(
            !plan
                .args
                .iter()
                .any(|argument| argument == "/operator/integrations/claude-code"),
            "the vendor must be pointed at the snapshot, not at a directory that can change \
             under the run: {:?}",
            plan.args
        );

        let attested = &plan.attestation.installed_plugins;
        assert_eq!(attested.len(), 1);
        assert_eq!(attested[0].name, "claude-code");
        assert_eq!(attested[0].source, "/operator/integrations/claude-code");
        assert_eq!(attested[0].digest, digest);
        assert!(attested[0].loaded_by.contains("--plugin-dir"));
        let h1a = plan
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H1a)
            .expect("H1a is imposed");
        assert!(h1a.how.contains("claude-code"), "{}", h1a.how);
    }

    /// **One edited byte in one plugin file is a different digest.** The caller reads the tree, so
    /// this asserts the plan carries what the caller computed — the mutation itself is proven
    /// against a real directory in `metaharness`'s own suite and against the rule in
    /// `metaharness-protocol`.
    #[test]
    fn an_edited_plugin_file_reaches_the_plan_as_a_different_digest() {
        let (spec, context, digest) = plugin_world();
        let before = plan_launch(&spec, &context).expect("plans");

        let mut edited = context.clone();
        let files: std::collections::BTreeMap<String, Digest> = [
            (".claude-plugin/plugin.json".to_string(), Digest::of(b"{}")),
            (
                "skills/one/SKILL.md".to_string(),
                Digest::of(b"bodz"), // one byte
            ),
        ]
        .into_iter()
        .collect();
        let mutated = metaharness_protocol::tree_digest(&files);
        edited.plugins[0].content = PluginContent::Files {
            count: files.len(),
            digest: mutated.clone(),
        };
        let after = plan_launch(&spec, &edited).expect("plans");

        assert_ne!(digest, mutated, "the mutation found its byte");
        assert_ne!(
            before.attestation.installed_plugins[0].digest,
            after.attestation.installed_plugins[0].digest
        );
        assert_eq!(after.plugin_installs[0].digest, mutated);
    }

    /// A directory that is not there, and one that is there and empty, are both refused before the
    /// spawn — with the path and the reason in the sentence.
    #[test]
    fn a_plugin_directory_that_cannot_be_read_is_refused_by_name() {
        let source = PathBuf::from("/operator/integrations/claude-code");
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
            assert!(refusal.to_string().contains("claude-code"));
        }
    }

    /// A run with no plugin says so with an empty list rather than by dropping the key.
    #[test]
    fn a_run_with_no_plugin_attests_an_empty_list_and_never_an_absent_key() {
        let plan = plan();
        assert!(plan.plugin_installs.is_empty());
        assert!(plan.attestation.installed_plugins.is_empty());
        let json = serde_json::to_string(&plan.attestation).expect("the attestation serializes");
        assert!(json.contains(r#""installed_plugins":[]"#), "{json}");
        assert!(!plan.args.iter().any(|argument| argument == "--plugin-dir"));
        let h1a = plan
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H1a)
            .expect("H1a is imposed");
        assert!(h1a.how.contains("no plugin was injected"), "{}", h1a.how);
    }

    // ------------------------------------------------------------ the loopback provider (LP-3)

    /// Vector 1. The whole of what a loopback child is given, and the two things it is not.
    ///
    /// Both absences are spike findings and both fail **silently** if they regress: a second
    /// credential variable puts a second spelling on the wire, and a missing
    /// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` lets the binary reach `api.anthropic.com`
    /// directly for first-party analytics — past the base URL, out of the proxy's sight, and with
    /// nothing in the run's record to say it happened.
    #[test]
    fn a_loopback_child_gets_the_proxy_a_placeholder_bearer_and_no_api_key() {
        let (spec, context) = loopback_world();
        let plan = plan_launch(&spec, &context).expect("the loopback launch plans");

        assert_eq!(
            plan.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("http://127.0.0.1:44321"),
            "the child must be pointed at this run's own proxy port"
        );
        assert_eq!(
            plan.env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str),
            Some("mh-run-claude-7-0f1e2d"),
            "the placeholder is delivered in the spelling the binary sends as a bearer"
        );
        assert_eq!(
            plan.env
                .get(DISABLE_NONESSENTIAL_TRAFFIC)
                .map(String::as_str),
            Some("1"),
            "without this the binary opens api.anthropic.com for analytics whatever the base URL \
             says, and \"the proxy sees every request\" stops being true"
        );
        assert!(
            !plan.env.contains_key("ANTHROPIC_API_KEY"),
            "ANTHROPIC_API_KEY must not be set beside the auth token: two credential variables \
             are two spellings on the wire and a precedence rule nobody here owns"
        );
        assert!(
            plan.credential_copies.is_empty(),
            "no credential file travels under loopback; that is the whole of the H6 upgrade"
        );

        let posture = |rows: &[String]| {
            rows.iter().any(|why| {
                why.contains("no operator credential in the child")
                    && why.contains("loopback proxy, which holds custody")
            })
        };
        let h4: Vec<String> = plan
            .attestation
            .imposed
            .iter()
            .filter(|control| control.row == HermeticRow::H4)
            .map(|control| control.how.clone())
            .collect();
        let h6: Vec<String> = plan
            .attestation
            .imposed
            .iter()
            .filter(|control| control.row == HermeticRow::H6)
            .map(|control| control.how.clone())
            .collect();
        assert!(
            posture(&h4),
            "H4 does not state the loopback posture: {h4:?}"
        );
        assert!(
            posture(&h6),
            "H6 does not state the loopback posture: {h6:?}"
        );
        assert!(
            plan.attestation.claims(HermeticRow::H6),
            "H6 is an imposition under loopback, not an unavailable row: the claim is stronger \
             than the copy row it replaces and is readable off the launch values"
        );
    }

    /// Vector 2. `loopback` is not `api-key`, so H3's scrub still deletes an exported key — the
    /// one that *"takes precedence over the claude.ai login"* and would send the operator's own
    /// account through a proxy that was about to attach a bearer anyway.
    #[test]
    fn an_inherited_api_key_does_not_survive_into_a_loopback_child() {
        let (spec, mut context) = loopback_world();
        context.inherited_env.insert(
            "ANTHROPIC_API_KEY".to_string(),
            "sk-operators-own".to_string(),
        );
        let plan = plan_launch(&spec, &context).expect("the loopback launch plans");
        assert!(
            !plan.env.contains_key("ANTHROPIC_API_KEY"),
            "an exported key reached a loopback child: {:?}",
            plan.env
        );
        assert!(
            is_scrubbed("ANTHROPIC_API_KEY", &spec),
            "the scrub must still name ANTHROPIC_API_KEY under loopback, or a later widening of \
             the inherited allowlist would let it through unnoticed"
        );
    }

    /// The base URL is an ephemeral port, so it cannot exist before the proxy does. A plan
    /// without one would be a child with no endpoint *and* no credential, failing at its first
    /// request with a vendor error that names nothing about metaharness.
    #[test]
    fn a_loopback_run_with_no_started_proxy_is_refused_by_name() {
        let mut spec = spec();
        spec.credentials = CredentialSource::Loopback;
        assert_eq!(
            plan_launch(&spec, &context()),
            Err(LaunchRefusal::LoopbackNotStarted)
        );
    }

    /// The dangerous direction: a proxy started for a run that declared a real credential would
    /// point a credentialed child at a local socket.
    #[test]
    fn a_started_proxy_without_the_loopback_declaration_is_refused_by_name() {
        let (_, context) = loopback_world();
        let mut spec = spec();
        spec.credentials = CredentialSource::ApiKey;
        assert_eq!(
            plan_launch(&spec, &context),
            Err(LaunchRefusal::LoopbackProxyUndeclared {
                base_url: "http://127.0.0.1:44321".to_string()
            })
        );
    }

    /// Loopback in front of a gateway: the declared endpoint is the **proxy's** upstream, one hop
    /// further out, so the child's base URL is still the loopback port and the composition that
    /// is refused beside a real credential is allowed here.
    #[test]
    fn a_model_endpoint_under_loopback_does_not_reach_the_child_and_is_not_refused() {
        let (mut spec, context) = loopback_world();
        spec.model_endpoint = Some("https://llmgw.example/".to_string());
        let plan = plan_launch(&spec, &context).expect("loopback in front of a gateway plans");
        assert_eq!(
            plan.env.get("ANTHROPIC_BASE_URL").map(String::as_str),
            Some("http://127.0.0.1:44321"),
            "the child talks to the proxy; the gateway is the proxy's business"
        );
        assert!(
            !plan.env.contains_key("ANTHROPIC_API_KEY"),
            "the endpoint placeholder key must not appear under loopback: the child already \
             authenticates with the bearer placeholder"
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
        // `owned` alone is no longer the trap — nothing decides per call under it. `ask` is what
        // puts an operator behind every call that the bare grant would answer before them.
        spec.decisions = DecisionMode::Ask;
        let refusal = plan_launch(&spec, &context()).expect_err("the shadow is refused");
        assert_eq!(
            refusal,
            LaunchRefusal::Shadowed {
                entries: vec!["mcp__metaharness".to_string()]
            }
        );
        assert_eq!(refusal.code(), Some(RefusalCode::Shadowed));
    }

    /// The owned surface plans, and all three flags are there: the built-ins removed, a server
    /// to replace them, and a grant that names it.
    ///
    /// The failure this pins is the one the argv had for as long as the refusal stood above it:
    /// `--tools ""` and `--allowedTools mcp__metaharness` with **no `--mcp-config`** — a run whose
    /// vendor tools are gone and whose replacement server was never configured, so the model has
    /// no tools at all and spends the run saying it cannot read a file.
    #[test]
    fn an_owned_surface_takes_the_vendors_tools_away_and_names_the_server_that_replaces_them() {
        let mut spec = spec();
        spec.tool_surface = ToolSurface::Owned;
        spec.allow_program = vec!["/usr/bin/cargo".to_string()];
        let plan = plan_launch(&spec, &context()).expect("frame mode needs no call seam");

        let window = plan.args.windows(2).collect::<Vec<_>>();
        assert!(window.contains(&&["--tools".to_string(), String::new()][..]));
        assert!(window.contains(
            &&[
                "--mcp-config".to_string(),
                "/scratch/run-1/mcp-config.json".to_string()
            ][..]
        ));
        assert!(
            window.contains(&&["--allowedTools".to_string(), "mcp__metaharness".to_string()][..])
        );
        assert!(
            !plan.args.iter().any(|arg| arg == "Bash"),
            "there is no Bash to grant or deny: {:?}",
            plan.args
        );

        let server = &plan.mcp_config.as_ref().expect("a config")["mcpServers"]["metaharness"];
        assert_eq!(server["command"], "/usr/local/bin/metaharness");
        assert_eq!(
            server["args"],
            json!([
                "mcp-serve",
                "--workspace",
                "/scratch/run-1/work",
                "--writable",
                "--allow-program",
                "/usr/bin/cargo"
            ]),
            "the server serves the child's own cwd, and starts only what the spec declared"
        );
    }

    /// `--tools ""` with nothing in their place is a run with no tools, so it is refused instead.
    #[test]
    fn an_owned_surface_with_no_server_to_serve_it_is_refused_rather_than_left_toolless() {
        let mut spec = spec();
        spec.tool_surface = ToolSurface::Owned;
        let mut context = context();
        context.tool_server = None;

        let refusal = plan_launch(&spec, &context).expect_err("no server, no run");
        assert_eq!(refusal, LaunchRefusal::ToolServerMissing);
        assert!(refusal.to_string().contains("--tool-surface owned"));
    }

    /// A `native` run carries no MCP configuration at all, so nothing is written claiming servers
    /// it never had.
    #[test]
    fn a_native_surface_plans_no_mcp_configuration_and_no_flag() {
        let plan = plan_launch(&spec(), &context()).expect("plans");
        assert!(plan.mcp_config.is_none());
        assert!(!plan.args.iter().any(|arg| arg == "--mcp-config"));
        assert!(!plan.args.iter().any(|arg| arg == "--tools"));
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
            LaunchRefusal::LoopbackNotStarted,
            LaunchRefusal::LoopbackProxyUndeclared {
                base_url: "http://127.0.0.1:44321".to_string(),
            },
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
