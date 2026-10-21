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
    CredentialSource, DecisionMode, DeclaredContextFile, Digest, HermeticAttestation, HermeticRow,
    ImposedControl, InstalledPlugin, Kind, PluginContent, PluginInstall, PluginTree, RefusalCode,
    RunSpec, TierStatus, ToolSurface, UnavailableControl, argument_too_long,
    context_matches_declaration, oversized_argument, required_commands, with_context_files,
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

/// The `personality` the scratch `config.toml` sets, and the value H1b expects the rollout's first
/// `turn_context.personality` to record (amendment a34).
///
/// It is the 0.153.4 binary's own default: a scratch home whose config names none records
/// `"pragmatic"`, so writing it changes no recorded setting and makes the row comparable with the
/// record. **The recorded value is the session's record of the setting, not proof of what the
/// model was told**: a managed `features.personality = false` still recorded `"pragmatic"`, and
/// the fixture provider's requests carried none of the binary's pragmatic template. Observed
/// credential-free in `docs/research/2026-10-21-codex-0.153.4-record-surface.md`, with the layers
/// that change the recorded value: an explicit key here wins over `/etc/codex/config.toml`, and
/// `/etc/codex/managed_config.toml` or a project's `.codex/config.toml` wins over it — a trusted
/// project's under `read-only`, and under the `workspace-write` sandbox every `--cwd` launch uses,
/// an untrusted one's too, with no trust entry. Because a layer can change the instructions without
/// changing the record, the launch claims H1b only when no such file layer exists
/// ([`CONFIG_LAYER_PATHS`], `LaunchContext::config_layers`); the cloud layer sources the binary
/// names (`cloud_requirements`, `cloud_managed_config`) are no file, are not checked, and what they
/// set under a ChatGPT-plan login is unverified.
pub const PERSONALITY: &str = "pragmatic";

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
/// Chosen from strings in the binary, and then **driven once**:
///
/// * the binary resolves `plugins/cache` and `plugins/data` **under the Codex home**, so
///   `$CODEX_HOME/plugins` is the vendor's own neighbourhood rather than an invention here;
/// * a marketplace's plugin entries are `./plugins/<plugin-name>` relative to a marketplace root,
///   which is the same shape this constant produces with the scratch home as that root.
///
/// # What one live run observed (2026-08-23, run `codex-2139643`)
///
/// A directed probe copied `integrations/codex` to this placement, digest `154857db…`, and asked
/// the model to answer **from its runtime context only, using no tools**. It answered *"Available
/// skills catalog — `## Skills`"* with **zero tool calls** — the census read `0/0/0/0` and no
/// `tool.requested` was emitted at all — so the catalog could not have been read off disk. **The
/// vendor surfaced the injected plugin's skills into the model's context from this path**, with no
/// `[marketplaces]` table and no `codex plugin add` behind it.
///
/// Two limits travel with that, and they are why `loaded_by` still does not claim the row outright:
///
/// * **The child was 0.144.0**, the binary this machine's constructed `PATH` resolves, against a
///   pin of 0.145.0 (Q18/a8). The observation is a fact about that binary and an inference about
///   the pin.
/// * **The vendor's opening record still lists no plugins** — `session.started.plugins` was
///   `null` — so **H1a still reads `unk`** on this vendor. What was observed is the plugin's
///   *content* reaching the model, which is what the treated arm of an evaluation needs; it is not
///   the vendor enumerating what it loaded, which is what H1a asks for.
///
/// This launch still writes **no** `[marketplaces]` table, deliberately: an unrecognised key under
/// a table this binary reads is dropped without failing the config load (see [`CONFIG_FILE`]), a
/// *malformed* one could fail it outright — which on this vendor is a run with no seam — and the
/// probe shows the copy alone is enough. See `docs/design/metaharness-protocol-v0.1.md` **Q19**.
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
    ///
    /// Under [`CredentialSource::Loopback`] the same file is metaharness's own custody and
    /// **never travels**: the caller opens it on its side of the socket, and this plan's copy
    /// list stays empty whatever this field says.
    pub credentials_file: Option<PathBuf>,
    /// The running loopback proxy's endpoint, placeholder and the login class behind it, under
    /// [`CredentialSource::Loopback`] (LP-4).
    ///
    /// The one value in this context that **cannot** be known before the run's own machinery
    /// starts: the proxy binds an ephemeral port, so unlike the static `--model-endpoint` there is
    /// nothing for a pure function to compute. The caller starts the proxy, fills this, and only
    /// then plans — and a `credentials: loopback` run that arrives here without it is
    /// [`LaunchRefusal::LoopbackNotStarted`] rather than a child launched at no endpoint with no
    /// credential.
    pub loopback: Option<LoopbackParams>,
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
    /// The config layers the binary reads outside the scratch `CODEX_HOME` that exist for this
    /// run, as the caller found them (amendment a34): the machine paths in [`CONFIG_LAYER_PATHS`]
    /// and a project `.codex/config.toml` in a named cwd's tree. Read there, decided here — the
    /// same split as the Claude launch's managed settings (a33). Non-empty attests H2 and H1b
    /// unavailable, naming them.
    pub config_layers: Vec<PathBuf>,
    /// Every `--context` file, **as the caller read it** beneath the `--cwd` tree (design
    /// amendment a35).
    ///
    /// Rendered inline ahead of the task, verbatim inside a delimiter each file does not contain
    /// ([`metaharness_protocol::render_context_files`]): `codex exec` 0.153.4 does not expand an
    /// `@path` mention (measured 2026-10-20: no mention grew the input or reached the model). A
    /// list that is not exactly the declared one is [`LaunchRefusal::ContextFilesUnread`].
    pub context_files: Vec<DeclaredContextFile>,
}

/// The machine-level config layers codex-cli 0.153.4 names on Linux, read from the binary's
/// strings (amendment a34). The scratch `CODEX_HOME` reaches none of them: the first is read
/// beneath the scratch `config.toml`, the second above it — both observed for `personality` — and
/// the third constrains what the others may set (not driven here).
pub const CONFIG_LAYER_PATHS: [&str; 3] = [
    "/etc/codex/config.toml",
    "/etc/codex/managed_config.toml",
    "/etc/codex/requirements.toml",
];

/// What the H1b and H2 claims say about the layers no file check sees (amendment a34): the binary
/// names the layer sources `cloud_requirements` and `cloud_managed_config`, which are not files.
const CLOUD_LAYERS_UNCHECKED: &str = "the file layers are what this launch checks: the cloud \
                                      layer sources the binary names, cloud_requirements and \
                                      cloud_managed_config, are no file, are not checked, and what \
                                      they set under a ChatGPT login is unverified";

/// What a started loopback proxy tells the launch, and the whole of it.
///
/// Two strings and one classification: this crate plans launches and owns no threads, so the proxy
/// itself stays on the library side and only its addressable facts cross the seam. Both strings are
/// per-run — an ephemeral port and a nonce-bearing placeholder — which is what stops one run
/// reaching another's endpoint by accident.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopbackParams {
    /// What the child's provider `base_url` is built from: `http://127.0.0.1:<port>`.
    pub base_url: String,
    /// The placeholder the child authenticates with, `mh-run-<id>-<nonce>`.
    ///
    /// Worthless anywhere but that port, which is the point of putting it in the child instead of
    /// the operator's token. It reaches the child in the environment variable the provider entry's
    /// `env_key` names ([`LOOPBACK_ENV_KEY`]), never in a file.
    pub placeholder: String,
    /// Which login the custody behind the proxy holds, as the caller read it off `auth.json`.
    ///
    /// Carried here rather than discovered here, because this function reads no file — and carried
    /// **at all** because the two classes are not interchangeable on this vendor: see
    /// [`CodexLogin`].
    pub login: CodexLogin,
}

/// Which shape of login an operator's `~/.codex/auth.json` holds.
///
/// The distinction is V-LP6's, and it decides whether the loopback door opens at all. The
/// **field names** are read from the pinned binary's own `AuthDotJson` serde metadata (0.145.0:
/// `OPENAI_API_KEY`, `auth_mode`, `tokens`, `last_refresh`, `agent_identity`,
/// `personal_access_token`, `bedrock_api_key`), so the classification is not a guess about the
/// file; what stays unverified is what the vendor *does* with a subscription token when a custom
/// provider is in force, which is exactly why one class is refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexLogin {
    /// `OPENAI_API_KEY` in `auth.json`: a key the proxy can replay upstream as a bearer.
    ApiKey,
    /// `tokens`: a ChatGPT-plan login. **V-LP6's open half** — whether subscription traffic can be
    /// routed through a `model_providers` entry at all is unanswered, and the answer needs one
    /// live turn nobody here has spent.
    Subscription,
}

/// The environment variable the loopback provider entry's `env_key` names.
///
/// Deliberately not `OPENAI_API_KEY`: that one is scrubbed under every credential source but
/// `api-key` (H3/H4), and a placeholder arriving under the operator's own variable name would be
/// indistinguishable from the leak the scrub exists to catch.
pub const LOOPBACK_ENV_KEY: &str = "METAHARNESS_LOOPBACK_KEY";

/// The provider id the loopback entry is declared under.
///
/// Prefixed, because 0.145.0 refuses a custom provider that collides with a built-in one —
/// `model_providers contains reserved built-in provider IDs: … Built-in providers cannot be
/// overridden. Rename your custom provider (for example, openai-custom)`, read verbatim from the
/// pinned binary — and a refused config on this vendor is a run with no seam.
const LOOPBACK_PROVIDER: &str = "metaharness_loopback";

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
    /// The run declared `credentials: loopback` and no proxy was started for it.
    ///
    /// Refused rather than planned, because the alternative is the worst of both worlds: a child
    /// with no credential *and* no endpoint, which fails at its first request with a vendor error
    /// about authentication and tells nobody that metaharness never started the thing that was
    /// supposed to hold the credential.
    LoopbackNotStarted,
    /// A loopback proxy was started for a run that did not declare `credentials: loopback`.
    ///
    /// The mirror of the row above, and the dangerous direction: an api-key run whose provider
    /// pointed at a local proxy would send the operator's real key to it. Refused rather than
    /// resolved by precedence.
    LoopbackProxyUndeclared {
        /// The endpoint that would have been imposed on the child.
        base_url: String,
    },
    /// The run declared `credentials: loopback` over a **subscription** login, which this adapter
    /// does not route (V-LP6's open half).
    ///
    /// **Refused by name, never degraded to the copy path.** The model-adapter rule is that there
    /// is no silent fallback between adapter classes, and the copy path is exactly the thing the
    /// loopback provider exists to replace — a run that asked for "no credential in the child"
    /// and got `auth.json` copied into the scratch home would be wrong in the direction that
    /// matters, and nothing in its record would say so.
    LoopbackSubscriptionUnverified,
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
    /// The run asked for a decision mode the descriptor does not call delivered.
    ///
    /// Design § 8.4 O4 over the mode table. Every mode the table carries today is delivered —
    /// `observe` joined on 2026-08-23 when R2.4's live run drove the `allow` half — so this fires
    /// only when a mode the table has not yet verified is asked for, and it is refused by name
    /// rather than served with a response the vendor may discard.
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
    /// The run declared `--context` files and the caller handed this plan another list (design
    /// amendment a35): a caller that planned without reading them, or read other files.
    ContextFilesUnread {
        /// How many the run declared.
        declared: usize,
        /// How many the caller read.
        read: usize,
    },
    /// An argument the launch built does not fit one argv string (`MAX_ARG_STRLEN`, design
    /// amendment a35): refused here rather than failing at `execve` with `E2BIG`.
    ArgumentTooLong {
        /// The argument, by the flag it follows or as the prompt.
        argument: String,
        /// Its size in bytes.
        bytes: usize,
        /// Every context file the run declared, with its size in bytes.
        files: Vec<(String, usize)>,
    },
}

impl LaunchRefusal {
    /// The protocol refusal code this maps to, where one applies.
    #[must_use]
    pub fn code(&self) -> Option<RefusalCode> {
        match self {
            LaunchRefusal::UnsupportedOption { .. }
            | LaunchRefusal::LoopbackSubscriptionUnverified
            | LaunchRefusal::DecisionModeUnverified { .. } => Some(RefusalCode::UnsupportedControl),
            _ => None,
        }
    }
}

/// The two `--context` refusals in words (design amendment a35).
fn context_words(refusal: &LaunchRefusal, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match refusal {
        LaunchRefusal::ContextFilesUnread { declared, read } => write!(
            f,
            "the run declared {declared} --context file(s) and the caller handed this plan {read} \
             read ones that are not that list; a run handed a smaller context than it declared is \
             one nobody can reproduce from the declaration (design amendment a35)"
        ),
        LaunchRefusal::ArgumentTooLong {
            argument,
            bytes,
            files,
        } => f.write_str(&argument_too_long(argument, *bytes, files)),
        _ => unreachable!("only the context refusals are worded here"),
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
            LaunchRefusal::LoopbackSubscriptionUnverified => f.write_str(
                "the run declared credentials: loopback over a ChatGPT-plan login, and that half \
                 of milestone LP-4 is not built: V-LP6 asks whether subscription traffic can be \
                 routed through a model_providers entry at all, and nothing here has answered it \
                 — the API-key half is built and free-tested, this one needs one live turn nobody \
                 has spent. It is refused by name rather than degraded to the credential-copy \
                 path, because a run that asked for no credential in the child and got auth.json \
                 copied into its scratch home would be wrong in the direction that matters",
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
                 {}. It is refused by name rather than served with a response this binary may \
                 discard, because a discarded decision on a run looks exactly like a decision \
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
                "the run asked for {option} and the pinned codex exec has no way to express it: \
                 {why}. It is refused rather than dropped, because an option that was set and \
                 ignored is a run that is not the one that was asked for"
            ),
            LaunchRefusal::ContextFilesUnread { .. } | LaunchRefusal::ArgumentTooLong { .. } => {
                context_words(self, f)
            }
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

    // `loopback` is exempt: under it the declared endpoint is not the child's provider base at
    // all — it is the **proxy's upstream**, one hop further out — and the child holds a
    // placeholder either way. The row exists to stop an operator credential travelling to a
    // foreign host, and a loopback run has no credential in the child to travel.
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
    let plugin_installs = plugin_installs(spec, context, &config_home)?;
    if !context_matches_declaration(&spec.context, &context.context_files) {
        return Err(LaunchRefusal::ContextFilesUnread {
            declared: spec.context.len(),
            read: context.context_files.len(),
        });
    }
    // Inline, ahead of the task, and nothing when no file was declared (amendment a35).
    let prompt =
        spec.with_agent_execution_context(&with_context_files(prompt, &context.context_files));
    let args = build_args(spec, &prompt);
    guard_arguments(&args)?;
    guard_argument_lengths(&args, &context.context_files)?;
    let env = build_env(spec, context, &config_home)?;
    let hook = build_hook(&hook_program_path(&context.scratch_root));

    Ok(LaunchPlan {
        program: ADAPTER_ID.to_string(),
        args,
        env,
        cwd: context.cwd.clone(),
        config_home: config_home.clone(),
        credential_copies,
        config: build_config(spec, context, &hook),
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
                 writes no marketplace entry. It is copied to {} because this binary keeps its own \
                 plugins under CODEX_HOME/plugins. **Driven once (Q19, 2026-08-23, run \
                 codex-2139643): the vendor surfaced the injected plugin's skills catalog into the \
                 model's context from this placement** — the model quoted its first heading with \
                 zero tool calls, so nothing was read off disk. Two limits: the child was codex \
                 0.144.0 against a 0.145.0 pin, and the opening record still lists no plugins, so \
                 H1a reads unk on this vendor rather than confirming this row",
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

/// The declaration and the started proxy must agree, in both directions — and the login behind
/// the proxy must be one this adapter routes.
///
/// Three failures rather than one, because they fail in opposite ways: without a proxy the child
/// reaches nothing; with an undeclared proxy a credentialed child reaches a local socket holding
/// the operator's own key; and over a subscription login the route itself is unverified (V-LP6).
/// None is a warning — a control that reports and proceeds has already stopped controlling
/// (design § 7.1).
fn guard_loopback(spec: &RunSpec, context: &LaunchContext) -> Result<(), LaunchRefusal> {
    let declared = spec.credentials == CredentialSource::Loopback;
    match (declared, &context.loopback) {
        (true, None) => Err(LaunchRefusal::LoopbackNotStarted),
        (false, Some(params)) => Err(LaunchRefusal::LoopbackProxyUndeclared {
            base_url: params.base_url.clone(),
        }),
        (true, Some(params)) => match params.login {
            CodexLogin::ApiKey => Ok(()),
            CodexLogin::Subscription => Err(LaunchRefusal::LoopbackSubscriptionUnverified),
        },
        // Both absent: the two halves agree, which is the whole of the check.
        (false, None) => Ok(()),
    }
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
    if spec.max_budget_usd.is_some() {
        return Err(LaunchRefusal::UnsupportedOption {
            option: "--max-budget-usd",
            why: "codex exec has no spend ceiling of its own; a cap metaharness enforced by \
                  killing the child at a price it estimated would be a different thing wearing \
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
        // Nothing travels under either, and `Loopback` is the stronger of the two: the operator's
        // `auth.json` is real and is held by metaharness's own custody, on this side of the
        // socket, so the copy list being empty is the whole H6 claim (LP-4).
        CredentialSource::Loopback | CredentialSource::None => Ok(Vec::new()),
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
    if let Some(loopback) = &context.loopback {
        // The whole of what a loopback child holds: one string, under this plan's own variable
        // name, worth nothing anywhere but one ephemeral port. The provider entry in the scratch
        // `config.toml` is what points the child at that port; **no `OPENAI_API_KEY` and no
        // `CODEX_API_KEY` travels beside it** — both stay scrubbed, because the run did not
        // declare `api-key` — and no `auth.json` is copied, which is the H6 upgrade LP-4 exists
        // for.
        env.insert(LOOPBACK_ENV_KEY.to_string(), loopback.placeholder.clone());
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
/// N1 (amendment a35, correction round 1): every argument fits one argv string, or the launch is
/// refused by name — the argument, its size and every context file with its size — rather than
/// failing at `execve` with `E2BIG`. The prompt carries the context block and the task together
/// here, so it is the argument the files can push past the limit.
fn guard_argument_lengths(
    args: &[String],
    files: &[DeclaredContextFile],
) -> Result<(), LaunchRefusal> {
    let Some((index, bytes)) = oversized_argument(args) else {
        return Ok(());
    };
    let argument = match index.checked_sub(1).map(|flag| args[flag].as_str()) {
        Some("--") => "the prompt".to_owned(),
        Some(flag) => flag.to_owned(),
        None => "an argument".to_owned(),
    };
    Err(LaunchRefusal::ArgumentTooLong {
        argument,
        bytes,
        files: files
            .iter()
            .map(|file| (file.path().to_owned(), file.text().len()))
            .collect(),
    })
}

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
///
/// At most **one** `model_provider` is ever written. The loopback door and the declared endpoint
/// are two different providers for two different reasons, and a document that carried both would
/// leave the vendor's own precedence deciding which brain answered — a fact this design says must
/// be checkable rather than resolved by a rule nobody here owns.
fn build_config(spec: &RunSpec, context: &LaunchContext, hook: &Value) -> String {
    let mut document = String::from(
        "# metaharness scratch CODEX_HOME. Generated per run; edited by nobody.\n\n\
         # The seam is the only thing that may refuse a call. `codex exec` on 0.145.0 has no\n\
         # --ask-for-approval flag, so the posture is a config key or nothing, and the operator\'s\n\
         # own default is `on-request` — which in a headless run means a call can be turned away\n\
         # by a prompt nobody is there to answer, before the hook ever sees it. `never` makes the\n\
         # hook the one thing that decides, so a denial is attributable to metaharness.\n\
         approval_policy = \"never\"\n",
    );
    let _ = writeln!(
        document,
        "\n# The vendor's own floor beneath the seam, and Claude Code has no counterpart for it\n\
         # (design § 7.4, V17). It is **not** a constant: it is what the run's cwd declaration\n\
         # decides, because on this vendor the two are one setting.\n\
         #\n\
         # {}\n\
         sandbox_mode = {}",
        sandbox_reason(spec),
        quote(sandbox_mode(spec))
    );
    // Top-level keys must precede the first table header, or TOML files them under it. The
    // personality is H1b's: written so the record's turn_context can be compared with it
    // (amendment a34, [`PERSONALITY`]).
    let _ = writeln!(document, "personality = {}", quote(PERSONALITY));
    if let Some(effort) = &spec.effort {
        let _ = writeln!(document, "\nmodel_reasoning_effort = {}", quote(effort));
    }
    if let Some(loopback) = &context.loopback {
        // The loopback door (LP-4). The child's provider is metaharness's own port, and the key it
        // authenticates with is the placeholder in [`LOOPBACK_ENV_KEY`] — so `env_key` is named
        // here where the endpoint provider deliberately names none. The upstream the proxy
        // forwards to (the vendor, or a gateway the run declared) is on the far side of that port
        // and never appears in this document: the child cannot address it, which is the whole
        // inspection claim.
        let base = format!("{}/v1", loopback.base_url.trim_end_matches('/'));
        let _ = writeln!(
            document,
            "\n# metaharness is this run's model provider (loopback design, LP-4). The child holds\n\
             # no credential: {LOOPBACK_ENV_KEY} carries a per-run placeholder, and the real token\n\
             # is attached by the proxy from one custody on the other side of this port.\n\
             model_provider = {provider}\n\n\
             [model_providers.{LOOPBACK_PROVIDER}]\n\
             name = {provider}\n\
             base_url = {base}\n\
             wire_api = \"responses\"\n\
             env_key = {key}",
            provider = quote(LOOPBACK_PROVIDER),
            base = quote(&base),
            key = quote(LOOPBACK_ENV_KEY),
        );
    } else if let Some(endpoint) = &spec.model_endpoint {
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

/// The vendor sandbox a **scratch-cwd** run gets: the child may read, and write nothing.
///
/// Unchanged since CX-M2, and the posture every run had until 2026-08-23.
const SANDBOX_READ_ONLY: &str = "read-only";

/// The vendor sandbox an **operator-named-cwd** run gets (amendment a6).
///
/// The value is the vendor's own, in the vendor's own spelling: `SandboxMode` deserialises
/// `read-only`, `workspace-write` and `danger-full-access` — kebab-case, read from the pinned
/// 0.145.0 binary's serde variant list, where the snake-case trio beside it belongs to a different
/// type entirely. A guessed spelling would be an unrecognised value in a file this vendor parses
/// leniently, which is the silent failure this module is written against.
const SANDBOX_WORKSPACE_WRITE: &str = "workspace-write";

/// Which vendor sandbox this run gets, and the whole of the rule.
///
/// **This is the fix for what a paid run found (2026-08-23, run codex-1982431).** A run with
/// `--cwd <a real repository>` spawned, worked, and could not write one file: the child reported
/// *"this workspace is mounted read-only"* and the vendor's own stream said *"the workspace is
/// read-only, so the planning-store patch was rejected."* The cause was this function's absence —
/// the scratch config named `read-only` for every run, so amendment a6's trade (give up H7 and H11,
/// get real work in a real tree) bought a repository the child could only look at.
///
/// The grant is exactly the a6 case and nothing wider. The vendor's own description of the value,
/// verbatim from the binary: *"`sandbox_mode` is `workspace-write`: The sandbox permits reading
/// files, and editing files in `cwd` and `writable_roots`. Editing files in other directories
/// requires approval."* The child is spawned **in** the named tree, so `cwd` is that tree and no
/// `writable_roots` entry is needed — naming one would widen the grant beyond what was declared.
///
/// What this does **not** do: `--add-dir` is still never passed, `danger-full-access` is never
/// written, and no `[sandbox_workspace_write]` table is emitted — so `network_access` keeps
/// whatever default this vendor applies, which is **undriven here** and therefore claimed nowhere.
/// A filesystem grant is not a network grant, and this function does not quietly make it one.
fn sandbox_mode(spec: &RunSpec) -> &'static str {
    if spec.cwd.is_some() {
        SANDBOX_WORKSPACE_WRITE
    } else {
        SANDBOX_READ_ONLY
    }
}

/// The comment the config carries beside its sandbox line, so the file says why it is what it is.
///
/// Wrapped by hand at the width the rest of this document uses: a comment that runs off the side
/// of a terminal is one an operator reading a retained scratch home will not read.
fn sandbox_reason(spec: &RunSpec) -> &'static str {
    if spec.scratch_cwd && spec.cwd.is_some() {
        "This run was pointed at the embedder's declared scratch copy (--cwd --scratch-cwd,\n\
         # amendment a27), so the child can edit files in it: the copy's changes are the proposal\n\
         # the embedder rechecks. The grant is the same as under --cwd alone (a6.1); what differs\n\
         # is the attestation, where H7 reads declared instead of unavailable. --add-dir is still\n\
         # never passed, and no writable_roots widens it past this copy."
    } else if spec.cwd.is_some() {
        "This run was pointed at the operator's own tree (--cwd, amendment a6), so the child\n\
         # can edit files in it — that is what a6 trades H7 and H11 for. Without this grant the\n\
         # trade buys a repository nobody can write to, which is what a paid run found on\n\
         # 2026-08-23: \"the workspace is read-only, so the planning-store patch was rejected\".\n\
         # --add-dir is still never passed, and no writable_roots widens it past this tree."
    } else {
        "The cwd is scratch, so the child can write nothing at all: it reads, and the seam\n\
         # decides the rest."
    }
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
    attest_config_sources(
        &home.to_string(),
        &context.config_layers,
        &mut imposed,
        &mut unavailable,
    );

    let mut declared = Vec::new();
    attest_cwd(spec, context, &mut imposed, &mut unavailable, &mut declared);

    match spec.credentials {
        CredentialSource::OperatorLogin => imposed.push(control_imposed(
            HermeticRow::H6,
            "one auth.json copied into the scratch home immediately before every spawn, and \
             nothing else (Q13)",
        )),
        // The upgrade LP-4 exists for. H6's copy row is advisory — it claims something about a
        // file this plan hands to a runner — whereas this claim is readable off the launch values
        // themselves: no entry in `credential_copies`, and a provider key that is worth nothing
        // anywhere but one ephemeral loopback port.
        CredentialSource::Loopback => imposed.push(control_imposed(
            HermeticRow::H6,
            format!("the scratch CODEX_HOME holds no auth.json at all: {LOOPBACK_POSTURE}"),
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
        declared,
        ambient_inputs: ambient_inputs(spec),
        installed_plugins: installed_plugins(plugin_installs),
    }
}

/// H1b and H2, which this launch can claim only while no config file layer outside the scratch home
/// exists for the run (amendment a34, the precedent of a33's managed settings).
///
/// The launch writes one config source and the binary reads more: `/etc/codex/config.toml`
/// beneath it, `/etc/codex/managed_config.toml` above it, `/etc/codex/requirements.toml` over
/// what either may set, and a project's `.codex/config.toml` above it — trusted, or under the
/// `workspace-write` sandbox every `--cwd` launch uses, untrusted too. When none exists the
/// scratch file is the whole of the *file* layers and both rows are claimed. When any exists
/// neither is: H2 was not excluded, and H1b cannot be read from the record, because such a layer
/// can change what the model is told while the recorded personality stays the one the launch
/// wrote — a managed `features.personality = false` still recorded `"pragmatic"` (observed,
/// 0.153.4). The binary also names the layer sources `cloud_requirements` and
/// `cloud_managed_config`, which are no file: no check here sees them, and what they set under a
/// ChatGPT-plan login is unverified. Both claims say so.
fn attest_config_sources(
    home: &str,
    layers: &[PathBuf],
    imposed: &mut Vec<ImposedControl>,
    unavailable: &mut Vec<UnavailableControl>,
) {
    if layers.is_empty() {
        imposed.push(control_imposed(
            HermeticRow::H1b,
            format!(
                "CODEX_HOME={home}, and its config.toml sets personality = \"{PERSONALITY}\", the \
                 pinned binary's own default; the row compares that with the personality the \
                 rollout's first turn_context records, which is the session's record of the \
                 setting and not proof of what the model was told; no config file layer outside \
                 the scratch home exists for this run ({CLOUD_LAYERS_UNCHECKED})"
            ),
        ));
        imposed.push(control_imposed(
            HermeticRow::H2,
            format!(
                "the one config source this launch writes is {home}/config.toml, in a scratch \
                 CODEX_HOME that holds no operator config, and none of the file layers \
                 {CONFIG_LAYER_PATHS:?} or a project .codex/config.toml exists for this run \
                 ({CLOUD_LAYERS_UNCHECKED}); --ignore-user-config is never passed, because it \
                 would switch off the file the seam is declared in"
            ),
        ));
        return;
    }
    let named: Vec<String> = layers
        .iter()
        .map(|layer| layer.display().to_string())
        .collect();
    unavailable.push(control_unavailable(
        HermeticRow::H1b,
        format!(
            "the scratch config.toml sets personality = \"{PERSONALITY}\", and the binary also \
             reads {named:?}, which this launch neither writes nor switches off: such a layer can \
             change what the model is told while the recorded personality stays \"{PERSONALITY}\" \
             (a managed features.personality = false still recorded it), so the record cannot \
             show this row"
        ),
    ));
    unavailable.push(control_unavailable(
        HermeticRow::H2,
        format!(
            "this launch writes {home}/config.toml, and the binary also reads {named:?}, which a \
             scratch CODEX_HOME does not reach and this launch does not switch off"
        ),
    ));
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
         binary has no --plugin-dir and this launch writes no marketplace entry. One live run \
         (Q19) observed the vendor surfacing an injected plugin's skills catalog into the model's \
         context from this placement, with zero tool calls; the opening record still lists no \
         plugins, so this row is answered from the record and reads unk when there is none"
    )
}

/// H7 and H11, which are impositions only over a scratch working directory.
///
/// An operator-named directory (amendment a6) is real work in a real tree: the rows are attested
/// unavailable with the declaration named, which is what makes `--hermetic strict` refuse such a
/// run instead of this attestation quietly claiming a directory metaharness never made.
///
/// **H7's sentence also carries what the vendor sandbox was set to**, in both directions, because
/// that is the difference between a run that could change the operator's repository and one that
/// could not — and a reader must be able to see it in the run's own record without diffing two
/// scratch config files that no longer exist. It is stated in the row that names the a6 trade
/// because it *is* the trade: [`sandbox_mode`] grants it, and this says so.
fn attest_cwd(
    spec: &RunSpec,
    context: &LaunchContext,
    imposed: &mut Vec<ImposedControl>,
    unavailable: &mut Vec<UnavailableControl>,
    declared: &mut Vec<metaharness_protocol::DeclaredControl>,
) {
    if spec.scratch_cwd && spec.cwd.is_some() {
        // Amendment a27: an embedder's declared scratch copy. H7 is stated by the party that made
        // the directory, never claimed here; the sandbox grant is the same as under `--cwd` alone
        // and is stated in the same row for the same reason as a6.1. H11 is measured by the walk a
        // scratch cwd gets, because the declaration covers the copy and nothing above it.
        declared.push(metaharness_protocol::DeclaredControl {
            row: HermeticRow::H7,
            declaration: format!(
                "the embedder declared {} as its disposable scratch copy (--cwd --scratch-cwd); \
                 metaharness did not create it and --add-dir is never passed. THE CHILD COULD WRITE \
                 TO THAT COPY: sandbox_mode = \"{SANDBOX_WORKSPACE_WRITE}\" for this run, as under \
                 --cwd alone (a6.1), because the copy's changes are the proposal the embedder \
                 rechecks",
                context.cwd.display()
            ),
        });
        if context.memory_ancestors.is_empty() {
            imposed.push(control_imposed(
                HermeticRow::H11,
                format!(
                    "the ancestor walk from the declared scratch copy {} found no AGENTS.md and no \
                     CLAUDE.md above it; a memory file at the copy's root is the copy's own",
                    context.cwd.display()
                ),
            ));
        } else {
            unavailable.push(control_unavailable(
                HermeticRow::H11,
                format!(
                    "the ancestor walk from the declared scratch copy found {} memory file(s) \
                     above it, which the declaration does not cover: the copy is the embedder's, \
                     the directories above it are not",
                    context.memory_ancestors.len()
                ),
            ));
        }
    } else if spec.cwd.is_some() {
        unavailable.push(control_unavailable(
            HermeticRow::H7,
            format!(
                "the run was pointed at the operator's directory {} (--cwd), which metaharness did \
                 not create; --add-dir is still never passed. THE CHILD COULD WRITE TO THAT TREE: \
                 the vendor sandbox was widened to it for this run — sandbox_mode = \
                 \"{SANDBOX_WORKSPACE_WRITE}\", which permits editing files in the cwd — because \
                 a6 trades this row for real work and a tree the child cannot write to is not \
                 that. A scratch-cwd run keeps sandbox_mode = \"{SANDBOX_READ_ONLY}\"",
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
                "cwd {} is under the scratch root, --add-dir is never passed, and the vendor \
                 sandbox stays sandbox_mode = \"{SANDBOX_READ_ONLY}\": the child writes nothing, \
                 anywhere",
                context.cwd.display()
            ),
        ));
        imposed.push(control_imposed(
            HermeticRow::H11,
            "the ancestor walk from the scratch cwd found no AGENTS.md and no CLAUDE.md",
        ));
    }
}

/// The one sentence that says what a loopback run's credential posture is.
///
/// Written once and used by both H4 and H6, because the two rows are two views of the same fact
/// and a reader who found them worded differently would have to work out whether they meant
/// different things.
const LOOPBACK_POSTURE: &str = "no operator credential in the child; a placeholder in \
                                METAHARNESS_LOOPBACK_KEY authenticates to metaharness's loopback \
                                proxy, which holds custody";

fn api_key_posture(credentials: CredentialSource) -> String {
    match credentials {
        CredentialSource::ApiKey => {
            "an API key is in the child environment because the run declared credentials: api-key"
                .to_string()
        }
        CredentialSource::Loopback => format!(
            "neither OPENAI_API_KEY nor CODEX_API_KEY is in the child environment — \
             {LOOPBACK_POSTURE}"
        ),
        CredentialSource::OperatorLogin | CredentialSource::None => {
            "neither OPENAI_API_KEY nor CODEX_API_KEY is in the child environment".to_string()
        }
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
            context_files: Vec::new(),
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
            // No proxy: every case below is a pure plan against a synthetic world, and a loopback
            // endpoint is by construction a port something really bound.
            loopback: None,
            config_layers: Vec::new(),
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

    /// A spec and a context for a loopback run over an API-key login: the half LP-4 builds.
    fn loopback_world() -> (RunSpec, LaunchContext) {
        let mut spec = spec();
        spec.credentials = CredentialSource::Loopback;
        let mut context = context();
        context.loopback = Some(LoopbackParams {
            base_url: "http://127.0.0.1:45999".to_string(),
            placeholder: "mh-run-codex-7-nonce".to_string(),
            login: CodexLogin::ApiKey,
        });
        (spec, context)
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

    /// Amendment a35: a plan handed a read list that is not the declared one refuses, whichever
    /// way it differs — missing, extra or another file — rather than rendering a smaller context.
    #[test]
    fn a_context_list_that_is_not_the_declared_one_is_refused() {
        let read =
            |path: &str| DeclaredContextFile::new(path.to_owned(), b"x\n".to_vec()).expect("text");
        for (declared, handed) in [
            (vec!["a.py"], vec![]),
            (vec![], vec![read("a.py")]),
            (vec!["a.py"], vec![read("b.py")]),
        ] {
            let mut spec = spec();
            spec.context = declared.iter().map(PathBuf::from).collect();
            let mut world = context();
            world.context_files = handed;
            assert!(
                matches!(
                    plan_launch(&spec, &world),
                    Err(LaunchRefusal::ContextFilesUnread { .. })
                ),
                "{declared:?}"
            );
        }
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
        assert_eq!(plan.credential_copies, []);
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
        let prompt = &plan.args[plan.args.len() - 1];
        assert!(prompt.ends_with("\n\ndo the thing"), "{prompt}");
        assert!(
            prompt.contains("execution_path=metaharness-driven"),
            "{prompt}"
        );
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

    /// H1b is read from the record against what the launch wrote (amendment a34), so the launch
    /// writes it: a top-level `personality` key — before the first table header, or TOML files it
    /// under that table and the vendor never sees it — carrying exactly [`PERSONALITY`], on every
    /// credential door.
    #[test]
    fn the_scratch_config_writes_the_personality_the_floor_compares_the_record_with() {
        let line = format!("personality = \"{PERSONALITY}\"");
        let (loopback_spec, loopback_context) = loopback_world();
        let loopback = plan_launch(&loopback_spec, &loopback_context).expect("plans");
        for config in [plan().config, named_cwd_plan().config, loopback.config] {
            let at = config
                .lines()
                .position(|candidate| candidate == line)
                .unwrap_or_else(|| panic!("no {line:?} in\n{config}"));
            let first_table = config
                .lines()
                .position(|candidate| candidate.starts_with('['))
                .expect("the config has tables");
            assert!(
                at < first_table,
                "{line:?} is filed under a table:\n{config}"
            );
            assert_eq!(config.matches("personality").count(), 1, "{config}");
        }
    }

    /// The H1b imposition says what the launch wrote and where the row reads it, so the
    /// attestation and the record can be put side by side (design § 8.3).
    #[test]
    fn the_h1b_attestation_names_the_personality_it_wrote() {
        let plan = plan();
        let h1b = plan
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H1b)
            .expect("H1b is imposed");
        assert!(
            h1b.how
                .contains(&format!("personality = \"{PERSONALITY}\"")),
            "{}",
            h1b.how
        );
        assert!(h1b.how.contains("turn_context"), "{}", h1b.how);
    }

    /// H2 names only what this launch writes. It once named `{home}/hooks.json` as a source
    /// "written by this launch", and no launch writes one: the seam is in `config.toml`, and a
    /// `hooks.json` is a plugin manifest's file (see [`CONFIG_FILE`]).
    #[test]
    fn the_h2_attestation_names_only_the_source_this_launch_writes() {
        let plan = plan();
        let h2 = plan
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H2)
            .expect("H2 is imposed");
        assert!(!h2.how.contains("hooks.json"), "{}", h2.how);
        assert!(
            h2.how.contains(
                &config_path(Path::new("/scratch/run-1"))
                    .display()
                    .to_string()
            ),
            "{}",
            h2.how
        );
    }

    /// A config layer the binary reads and the scratch home does not reach is a source the launch
    /// did not exclude and a value it does not control (amendment a34): H2 and H1b are attested
    /// unavailable, each naming every layer found, and neither is claimed. The paths are handed
    /// in, so no test reads a real `/etc`.
    #[test]
    fn a_config_layer_outside_the_scratch_home_makes_h1b_and_h2_unavailable_by_name() {
        let layers = vec![
            PathBuf::from("/etc/codex/managed_config.toml"),
            PathBuf::from("/work/repo/.codex/config.toml"),
        ];
        let mut layered = context();
        layered.config_layers.clone_from(&layers);
        let plan = plan_launch(&spec(), &layered).expect("a layer is attested, not refused");
        for row in [HermeticRow::H1b, HermeticRow::H2] {
            assert!(
                !plan.attestation.claims(row),
                "{} is still claimed",
                row.id()
            );
            let Some(why) = plan
                .attestation
                .unavailable
                .iter()
                .find(|control| control.row == row)
                .map(|control| control.why.clone())
            else {
                panic!("{} is not attested unavailable", row.id());
            };
            for layer in &layers {
                assert!(
                    why.contains(&layer.display().to_string()),
                    "{}: {why}",
                    row.id()
                );
            }
        }
        // The launch still writes its personality: the comparison is unchanged, only what the
        // record can show is.
        assert!(
            plan.config
                .contains(&format!("personality = \"{PERSONALITY}\""))
        );
        // With no layer, both rows are claimed as before.
        let clean = plan_launch(&spec(), &context()).expect("plans");
        assert!(clean.attestation.claims(HermeticRow::H1b));
        assert!(clean.attestation.claims(HermeticRow::H2));
        assert!(
            clean
                .attestation
                .unavailable
                .iter()
                .all(|control| control.row != HermeticRow::H1b && control.row != HermeticRow::H2)
        );
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

    // ------------------------------------- the a6 trade is a writable tree, or it is nothing

    /// A plan for a run pointed at the operator's own tree.
    fn named_cwd_plan() -> LaunchPlan {
        let mut spec = spec();
        spec.cwd = Some(PathBuf::from("/operator/repo"));
        let mut context = context();
        context.cwd = PathBuf::from("/operator/repo");
        context.memory_ancestors = vec![PathBuf::from("/operator/repo/AGENTS.md")];
        plan_launch(&spec, &context).expect("an operator cwd plans")
    }

    /// Whether a config document grants the child write access to the tree it runs in.
    ///
    /// A function rather than an inline assertion so the mutation test below can run **the same
    /// check** against a document with the grant taken out — a check that cannot go red is not a
    /// check.
    fn grants_the_named_tree(config: &str) -> bool {
        config.contains(r#"sandbox_mode = "workspace-write""#)
    }

    /// What the paid run of 2026-08-23 found, as a test: with `--cwd` the child must be able to
    /// **write** to the tree it was pointed at. Before this, the scratch config said `read-only`
    /// for every run and the vendor rejected every patch — *"the workspace is read-only, so the
    /// planning-store patch was rejected"* — so a6's trade bought a repository nobody could
    /// change.
    #[test]
    fn an_operator_named_cwd_grants_the_child_write_access_to_that_tree() {
        let config = named_cwd_plan().config;
        assert!(
            grants_the_named_tree(&config),
            "a --cwd run must widen the vendor sandbox to the named tree, or the a6 trade is \
             meaningless on this vendor:\n{config}"
        );
        assert!(
            !config.contains(r#"sandbox_mode = "read-only""#),
            "one sandbox_mode is written, not two:\n{config}"
        );
        // The grant is exactly the declared tree: no extra writable root, and never the value that
        // switches the sandbox off altogether. Asserted on the **keys**, not on the words — the
        // comment above them says `writable_roots` on purpose, and a substring check would be
        // testing prose.
        assert!(!config.contains("writable_roots ="), "{config}");
        assert!(!config.contains("danger-full-access\""), "{config}");
        // A filesystem grant is not a network grant: no [sandbox_workspace_write] table is written,
        // so nothing here claims or changes this vendor's network default.
        assert!(!config.contains("[sandbox_workspace_write]"), "{config}");
        assert!(!config.contains("network_access ="), "{config}");
    }

    /// The other half of the polarity, and the one that keeps every existing run honest: a scratch
    /// cwd is still `read-only`. This is the posture every run had before the fix, and it must not
    /// have moved.
    #[test]
    fn a_scratch_cwd_run_still_writes_nothing_anywhere() {
        let config = plan().config;
        assert!(config.contains(r#"sandbox_mode = "read-only""#), "{config}");
        assert!(
            !grants_the_named_tree(&config),
            "a scratch run must not be granted write access to anything:\n{config}"
        );
    }

    /// The mutation: take the grant out of a named-cwd config and the check must go red. A test
    /// that passes on a document with the fix removed would be describing nothing.
    #[test]
    fn a_config_with_the_grant_stripped_fails_the_same_check() {
        let config = named_cwd_plan().config;
        assert!(grants_the_named_tree(&config), "the real plan grants it");
        let stripped = config.replace(
            r#"sandbox_mode = "workspace-write""#,
            r#"sandbox_mode = "read-only""#,
        );
        assert_ne!(stripped, config, "the mutation found its line");
        assert!(
            !grants_the_named_tree(&stripped),
            "the check passed on a config that grants nothing, so it is decoration"
        );
    }

    /// The grant is **visible in the run's own record**, not only in a file that lives for the
    /// length of a spawn: H7's row says the tree was writable, in words a reader does not have to
    /// diff two configs to understand — and the scratch case says the opposite just as plainly.
    #[test]
    fn the_attestation_says_the_operators_tree_was_writable_without_reading_a_config() {
        let named = named_cwd_plan();
        let row = named
            .attestation
            .unavailable
            .iter()
            .find(|control| control.row == HermeticRow::H7)
            .expect("H7 is attested unavailable under a named cwd");
        assert!(
            row.why.contains("COULD WRITE TO THAT TREE"),
            "a reader must see the grant stated, not implied: {}",
            row.why
        );
        assert!(row.why.contains("workspace-write"), "{}", row.why);
        assert!(row.why.contains("/operator/repo"), "{}", row.why);

        let scratch = plan();
        let row = scratch
            .attestation
            .imposed
            .iter()
            .find(|control| control.row == HermeticRow::H7)
            .expect("H7 is imposed under a scratch cwd");
        assert!(
            row.how.contains("read-only") && row.how.contains("writes nothing"),
            "the scratch case must state its posture just as plainly: {}",
            row.how
        );
    }

    // ------------------------------------------------------- the declared scratch copy (a27)

    /// A plan for a run in the embedder's declared scratch copy.
    fn declared_copy_plan(memory_ancestors: Vec<PathBuf>) -> LaunchPlan {
        let mut spec = spec();
        spec.cwd = Some(PathBuf::from("/embedder/copy"));
        spec.scratch_cwd = true;
        let mut context = context();
        context.cwd = PathBuf::from("/embedder/copy");
        context.memory_ancestors = memory_ancestors;
        plan_launch(&spec, &context).expect("a declared copy plans")
    }

    /// The embedder's declared copy: H7 is `declared` — naming the copy and the `workspace-write`
    /// grant, a6.1's rule — and neither imposed nor unavailable; H11 is imposed by the same walk a
    /// scratch cwd gets; the scratch config still grants the tree.
    #[test]
    fn a_declared_scratch_copy_declares_h7_and_measures_h11() {
        let plan = declared_copy_plan(Vec::new());
        let declared = plan
            .attestation
            .declaration(HermeticRow::H7)
            .expect("H7 is declared");
        assert!(
            declared.declaration.contains("/embedder/copy"),
            "{}",
            declared.declaration
        );
        assert!(
            declared.declaration.contains("workspace-write"),
            "{}",
            declared.declaration
        );
        assert!(!plan.attestation.claims(HermeticRow::H7));
        assert!(
            plan.attestation
                .unavailable
                .iter()
                .all(|control| control.row != HermeticRow::H7)
        );
        assert!(plan.attestation.claims(HermeticRow::H11));
        assert!(
            plan.attestation
                .declared
                .iter()
                .all(|control| control.row.admits_declaration()),
            "only H7 may be declared: {:?}",
            plan.attestation.declared
        );
        assert!(grants_the_named_tree(&plan.config), "{}", plan.config);
        assert!(plan.config.contains("amendment a27"), "{}", plan.config);
    }

    /// A memory file above the declared copy is measured, not declared away: H11 goes unavailable
    /// with the count named, H7 stays declared, and the launch is not refused.
    #[test]
    fn a_memory_file_above_the_declared_copy_is_attested_unavailable_not_refused() {
        let plan = declared_copy_plan(vec![PathBuf::from("/embedder/AGENTS.md")]);
        assert!(plan.attestation.declares(HermeticRow::H7));
        assert!(!plan.attestation.claims(HermeticRow::H11));
        let h11 = plan
            .attestation
            .unavailable
            .iter()
            .find(|control| control.row == HermeticRow::H11)
            .expect("H11 unavailable");
        assert!(h11.why.contains("1 memory file"), "{}", h11.why);
    }

    /// The declaration without `--cwd` is the library's refusal (`check_spec`), not this plan's:
    /// over a scratch cwd the flag changes nothing here, and nothing is declared.
    #[test]
    fn a_declaration_without_a_directory_changes_nothing_in_the_plan() {
        let mut spec = spec();
        spec.scratch_cwd = true;
        let plan = plan_launch(&spec, &context()).expect("a scratch cwd plans");
        assert_eq!(
            plan.attestation.declared,
            Vec::<metaharness_protocol::DeclaredControl>::new()
        );
        assert!(plan.attestation.claims(HermeticRow::H7));
        assert!(plan.attestation.claims(HermeticRow::H11));
        assert!(!grants_the_named_tree(&plan.config), "{}", plan.config);
    }

    /// `--hermetic strict` must still refuse a named-cwd run: the grant changes what the child may
    /// do, **not** what the attestation claims. H7 and H11 stay unavailable, which is what the
    /// strict floor reads.
    #[test]
    fn granting_the_tree_does_not_make_a_named_cwd_run_hermetically_clean() {
        let plan = named_cwd_plan();
        for row in [HermeticRow::H7, HermeticRow::H11] {
            assert!(
                !plan.attestation.claims(row),
                "{} is claimed on a run that writes to the operator's tree",
                row.id()
            );
        }
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

    /// Observe mode is the **allow** half of this vendor's decision wire, and that half was
    /// driven live on 2026-08-23 (R2.4): the hook held a real `Bash` call, metaharness answered
    /// `allow`, and the rollout's own `custom_tool_call_output` carried the command's output. So
    /// an observe run plans, and the attestation says which posture decided it — the refusal this
    /// test used to pin moved with the evidence, not before it.
    #[test]
    fn a_codex_observe_run_plans_now_that_the_allow_half_is_driven() {
        let mut spec = spec();
        spec.decisions = DecisionMode::Observe;
        let plan = plan_launch(&spec, &context()).expect("observe plans since R2.4's live run");
        assert_eq!(plan.attestation.decisions, DecisionMode::Observe);
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
        // The record carries the observation **and both of its limits**. A row that had been
        // upgraded to a bare "it loads" would be claiming the vendor's plugin list (which is
        // still absent) and the pinned binary (which is not the one that was driven).
        let loaded_by = &attested[0].loaded_by;
        assert!(loaded_by.contains("Q19"), "{loaded_by}");
        assert!(
            loaded_by.contains("Driven once") && loaded_by.contains("skills catalog"),
            "the record must say what was observed, not imply it: {loaded_by}"
        );
        assert!(
            loaded_by.contains("0.144.0") && loaded_by.contains("unk"),
            "the observation's two limits — the off-pin binary and the still-absent plugin list — \
             travel with it or the row overclaims: {loaded_by}"
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
        assert_eq!(plan.plugin_installs, []);
        assert_eq!(plan.attestation.installed_plugins, []);
        let json = serde_json::to_string(&plan.attestation).expect("the attestation serializes");
        assert!(json.contains(r#""installed_plugins":[]"#), "{json}");
        assert!(json.contains(r#""decisions":"frame""#), "{json}");
    }

    // ------------------------------------------------------------ the loopback door (LP-4)

    /// LP-4 vector 1. The whole of what a loopback child is given, and the three things it is not:
    /// no `auth.json` copy, no `OPENAI_API_KEY`, no `CODEX_API_KEY`. The provider entry is where
    /// the port arrives, because on this vendor a base URL is a config key and not an environment
    /// variable — so a launch vector that read only the environment would pin nothing about it.
    #[test]
    fn a_loopback_child_gets_a_provider_at_the_proxy_a_placeholder_key_and_no_auth_json() {
        let (spec, context) = loopback_world();
        let plan = plan_launch(&spec, &context).expect("the loopback launch plans");

        assert!(
            plan.credential_copies.is_empty(),
            "no auth.json travels under loopback; that is the whole of the H6 upgrade"
        );
        assert_eq!(
            plan.env.get(LOOPBACK_ENV_KEY).map(String::as_str),
            Some("mh-run-codex-7-nonce"),
            "the placeholder reaches the child under the name the provider's env_key gives it"
        );
        for absent in ["OPENAI_API_KEY", "CODEX_API_KEY"] {
            assert!(
                !plan.env.contains_key(absent),
                "{absent} must not travel beside the placeholder: two credential variables mean \
                 two spellings on the wire and a precedence rule nobody here owns"
            );
        }
        for line in [
            "model_provider = \"metaharness_loopback\"",
            "[model_providers.metaharness_loopback]",
            "base_url = \"http://127.0.0.1:45999/v1\"",
            "wire_api = \"responses\"",
            "env_key = \"METAHARNESS_LOOPBACK_KEY\"",
        ] {
            assert!(
                plan.config.contains(line),
                "missing {line} in:\n{}",
                plan.config
            );
        }
        // The seam survives the provider injection: a config that lost its hook would be a run
        // with no guard, and on this vendor that failure is silent.
        assert!(
            plan.config.contains("[[hooks.PreToolUse]]"),
            "{}",
            plan.config
        );
    }

    /// LP-4 vector 2. The two rows the attestation must state differently under loopback: H6 is an
    /// **imposition** here rather than an unavailable row, because "no credential in the child at
    /// all" is stronger than "one file, copied" — and it is readable off the launch values.
    #[test]
    fn a_loopback_run_attests_the_stronger_h6_rather_than_giving_the_row_up() {
        let (spec, context) = loopback_world();
        let plan = plan_launch(&spec, &context).expect("the loopback launch plans");
        assert!(
            plan.attestation.claims(HermeticRow::H6),
            "H6 is an imposition under loopback: {:?}",
            plan.attestation
        );
        for row in [HermeticRow::H4, HermeticRow::H6] {
            let control = plan
                .attestation
                .imposed
                .iter()
                .find(|control| control.row == row)
                .unwrap_or_else(|| panic!("{} must be imposed", row.id()));
            assert!(
                control.how.contains("loopback proxy, which holds custody"),
                "{} does not state the loopback posture: {}",
                row.id(),
                control.how
            );
        }
    }

    /// LP-4 vector 3. `loopback` is not `api-key`, so H3's scrub still deletes an exported key:
    /// the child authenticates with the placeholder and nothing else, whatever the operator's own
    /// environment holds.
    #[test]
    fn a_loopback_run_still_scrubs_an_exported_api_key_from_the_child() {
        let (spec, mut context) = loopback_world();
        context
            .inherited_env
            .insert("OPENAI_API_KEY".to_string(), "sk-operators-own".to_string());
        let plan = plan_launch(&spec, &context).expect("the loopback launch plans");
        assert!(!plan.env.contains_key("OPENAI_API_KEY"), "{:?}", plan.env);
        assert!(is_scrubbed("OPENAI_API_KEY", CredentialSource::Loopback));
    }

    /// LP-4 vector 4. Under loopback a declared endpoint is the **proxy's upstream**, one hop
    /// further out, so the two compose instead of refusing — and the child's own provider is still
    /// the loopback port, because a document naming both would leave the vendor's precedence
    /// deciding which brain answered.
    #[test]
    fn a_declared_endpoint_under_loopback_is_the_proxys_upstream_and_never_the_childs_provider() {
        let (mut spec, context) = loopback_world();
        spec.model_endpoint = Some("https://llmgw.example".to_string());
        let plan = plan_launch(&spec, &context).expect("endpoint + loopback compose");
        assert!(
            !plan.config.contains("llmgw.example"),
            "the upstream must not be addressable by the child: {}",
            plan.config
        );
        assert_eq!(
            plan.config.matches("model_provider = ").count(),
            1,
            "exactly one provider is ever selected: {}",
            plan.config
        );
    }

    /// LP-4 vector 5. **V-LP6's open half, refused by name.** A ChatGPT-plan login is not routed
    /// through a `model_providers` entry by this adapter, because nobody here has verified that
    /// the vendor honours one for subscription traffic — and the alternative, degrading to the
    /// credential-copy path, is precisely what the loopback design exists to replace.
    #[test]
    fn a_loopback_run_over_a_subscription_login_is_refused_by_name_and_never_degraded() {
        let (spec, mut context) = loopback_world();
        if let Some(loopback) = context.loopback.as_mut() {
            loopback.login = CodexLogin::Subscription;
        }
        let refusal = plan_launch(&spec, &context).expect_err("the subscription half is unbuilt");
        assert_eq!(refusal, LaunchRefusal::LoopbackSubscriptionUnverified);
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

    /// LP-4 vector 6. The declaration and the started proxy must agree in **both** directions, and
    /// the dangerous direction is the second: a credentialed child pointed at a local port would
    /// send the operator's own key to whatever is listening on it.
    #[test]
    fn a_loopback_declaration_and_a_started_proxy_must_agree_in_both_directions() {
        let mut declared_only = spec();
        declared_only.credentials = CredentialSource::Loopback;
        assert_eq!(
            plan_launch(&declared_only, &context()),
            Err(LaunchRefusal::LoopbackNotStarted)
        );

        let (_, context) = loopback_world();
        // An api-key run that somehow got a proxy: refused, never composed.
        let mut undeclared = spec();
        undeclared.credentials = CredentialSource::ApiKey;
        let mut context = context;
        context
            .inherited_env
            .insert("OPENAI_API_KEY".to_string(), "sk-test".to_string());
        assert_eq!(
            plan_launch(&undeclared, &context),
            Err(LaunchRefusal::LoopbackProxyUndeclared {
                base_url: "http://127.0.0.1:45999".to_string()
            })
        );
    }

    /// The copy list is empty under loopback rather than refusing: the custody is real and is held
    /// on metaharness's side of the socket, so "copies nothing" is the claim, not an omission.
    #[test]
    fn the_copy_list_is_empty_under_loopback_because_custody_stays_on_this_side() {
        let (spec, context) = loopback_world();
        assert_eq!(
            credential_copies(&spec, &context, Path::new("/scratch/run-1/codex-home")),
            Ok(Vec::new())
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
            LaunchRefusal::LoopbackNotStarted,
            LaunchRefusal::LoopbackProxyUndeclared {
                base_url: "http://127.0.0.1:1".to_string(),
            },
            LaunchRefusal::LoopbackSubscriptionUnverified,
        ];
        for refusal in refusals {
            let sentence = refusal.to_string();
            assert!(sentence.len() > 20, "{sentence}");
        }
    }
}
