//! The one options type.
//!
//! There is one options type and the CLI's `run` verb is a `derive` on it (design D11): a flag
//! the library cannot express cannot be added, and an option the CLI cannot express cannot be
//! introduced. The first statement of that rule was decorative and the document's own two
//! surfaces had already drifted apart (finding F16), which is the argument for the mechanical
//! test in `metaharness-cli` rather than against it.

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::hermetic::HermeticMode;

/// Which harness drives the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
    /// The b10x loop, which this workspace observes rather than drives.
    ///
    /// The odd one out, and the enum should say so rather than a reader discovering it: every other
    /// kind names a vendor binary metaharness puts itself in front of. This one names a loop we
    /// own, whose published toolset already *is* its policy, so the adapter for it decides nothing
    /// and contributes attestation and one wire. See `metaharness-b10x`.
    B10x,
}

impl Kind {
    /// The kind's name, as the CLI spells it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Claude => "claude",
            Kind::Codex => "codex",
            Kind::B10x => "b10x",
        }
    }
}

/// Who decides a tool call.
///
/// Two modes rather than one because a round trip per call costs latency, and an embedder that
/// answers "yes" to everything the frame already admits has bought nothing (design D5). A run in
/// `frame` mode is still fully audited: every mode emits `tool.decided` and the census counts
/// them all.
///
/// The third mode is younger than D5 and is not a shortcut for the other two — see
/// [`DecisionMode::Observe`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum DecisionMode {
    /// The adapter decides from the frame's admitted set. No round trip.
    Frame,
    /// The embedder decides. The run blocks, one round trip per call.
    Ask,
    /// **Allow every call and record every call** — the capture mode, and nothing else
    /// (design amendment a10).
    ///
    /// It exists for one job: measuring how a harness behaves when metaharness is *not* steering
    /// it, with the same instrument that measures a steered run. The recording seam is installed
    /// exactly as it is in every other mode, every call arrives at it, and every call leaves it
    /// with `tool.decided { decided_by: "observe" }` — so an unsteered run and a steered one
    /// produce the same shape of transcript and can be compared. Nothing is bypassed: the hook
    /// still fires, and what it answers is `allow`.
    ///
    /// **`allow` grants**, and that is the price of the mode rather than an oversight (design
    /// § 6, finding F8): on Claude Code's hook wire an `allow` overrides a stricter rule in the
    /// vendor's own settings. An observe run is therefore *more* permissive than a run with no
    /// hook at all, and that is stated at every point of use rather than discovered.
    ///
    /// It is refused beside a frame: a frame whose text reaches the model while nothing enforces
    /// it tells the model "strictly only these operations" and makes it false (finding F9).
    Observe,
}

impl DecisionMode {
    /// Every mode, in the order the design's own table lists them.
    pub const ALL: [DecisionMode; 3] = [
        DecisionMode::Frame,
        DecisionMode::Ask,
        DecisionMode::Observe,
    ];

    /// The mode's name, as the CLI spells it and as a record prints it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            DecisionMode::Frame => "frame",
            DecisionMode::Ask => "ask",
            DecisionMode::Observe => "observe",
        }
    }
}

/// Where the run's credential comes from.
///
/// The four sources differ in **where the live token is while the child runs**, which is the only
/// distinction the hermetic rows care about: in the scratch home ([`CredentialSource::ApiKey`],
/// [`CredentialSource::OperatorLogin`]), on metaharness's side of a socket
/// ([`CredentialSource::Loopback`]), or nowhere ([`CredentialSource::None`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "kebab-case")]
pub enum CredentialSource {
    /// The operator's own login, copied into the scratch config home immediately before each
    /// spawn. Copied rather than shared because everything else in that home must be scratch;
    /// re-copied per spawn because a snapshot ages out (design § 8.1 H6, as amended — Q13).
    OperatorLogin,
    /// An API key the run declared. Without this declaration `ANTHROPIC_API_KEY` is not in the
    /// child environment at all (design § 8.1 H4).
    ApiKey,
    /// **No credential in the child; metaharness proxies with custody.**
    ///
    /// The loopback provider (`docs/design/loopback-provider-v0.1.md`, LP-3): metaharness runs a
    /// per-run proxy on 127.0.0.1, the child is pointed at it and authenticates with a
    /// placeholder that names the run, and the operator's real bearer is attached on the way
    /// upstream from one custody. The scratch home holds no credential file at all, so H6 stops
    /// being *"credentials are one file, copied"* and becomes the strictly stronger *"no
    /// credential in the child"* — attestable from the launch values rather than asserted.
    ///
    /// Claude Code only in this milestone. The codex adapter refuses it **by name**: V-LP6
    /// verified the route is feasible and the confirming paid run has not been done, so the door
    /// is stated as unbuilt rather than degraded in silence to the copy path.
    Loopback,
    /// No credential. The run is expected to fail at the first request, and that is sometimes
    /// exactly what a test wants.
    None,
}

/// Whose tools the model is offered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum ToolSurface {
    /// The vendor's own tools, narrowed at the decision seam (design § 7.5 strategy A).
    Native,
    /// metaharness owns the tool surface and runs the tools itself (strategy C). Not the v0.1
    /// default: per-step re-listing depends on unverified vendor behaviour (Q1).
    Owned,
}

/// Whether a declared scope is stated in the instruction as well as bound into the tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum ScopeAnnounce {
    /// Say it up front, so no turn is spent discovering it by being refused.
    #[default]
    Stated,
    /// Bind it and say nothing. The refusal has to teach it — which is what makes a run under this
    /// a measurement of the toolset rather than of the prose.
    Silent,
}

/// Everything a run is.
///
/// The builder's `with_…` methods and the CLI's flags are two spellings of this struct, and
/// neither can grow a knob the other cannot express.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::Args))]
pub struct RunSpec {
    /// Which harness.
    #[cfg_attr(feature = "clap", arg(value_enum))]
    pub kind: Kind,

    /// How hermetic the run must be. `--hermetic` alone means `on`; `--hermetic strict` makes a
    /// gating row that is not `ok` fail the run.
    #[cfg_attr(
        feature = "clap",
        arg(
            long,
            value_enum,
            num_args = 0..=1,
            default_value = "off",
            default_missing_value = "on",
        )
    )]
    pub hermetic: HermeticMode,

    /// The prompt to start with.
    #[cfg_attr(feature = "clap", arg(short = 'p', long))]
    pub prompt: Option<String>,

    /// A frame document: a sealed `metaharness.frame/1` file.
    ///
    /// A path and not a [`crate::Frame`], because resolving it is the library's job and parsing
    /// it in the binary would be protocol logic in the CLI — which D11 exists to forbid (design
    /// § 9.3, correction 3). The format landed as amendment a5: the library reads the file at
    /// start and refuses by name a document that is unreadable, untagged, misshapen or whose
    /// digest does not describe its contents.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub frame: Option<PathBuf>,

    /// Who decides a tool call: the frame, the embedder, or nobody (`observe` allows every call
    /// and records it).
    ///
    /// The default is `frame` and stays `frame`. `observe` allows everything, so a run that
    /// drifted into it by default would be a run whose control had been switched off by an
    /// omission; it is reached by asking for it and by nothing else. See [`DecisionMode`].
    #[cfg_attr(feature = "clap", arg(long, value_enum, default_value = "frame"))]
    pub decisions: DecisionMode,

    /// Who the run's writes are made *as*, carried to the child as `AEP_ACTOR`.
    ///
    /// **Declared, never inherited, and that distinction is the whole of it.** The subject's store
    /// stamps `human:$USER` on an `artifact move` unless something tells it otherwise, so a driven
    /// session's writes are indistinguishable from a person's. The obvious fix — adding `AEP_ACTOR`
    /// to the adapter's inherit list — is the wrong one: that list is an allowlist precisely
    /// because a denylist is a list of the leaks somebody thought of (design § 8.1 H3), and a run
    /// that inherited this would be journalled as whatever the operator happened to have exported.
    /// Provenance that can be set by the surrounding shell is not provenance.
    ///
    /// So the caller states it, the adapter sets it on a constructed environment, and the
    /// attestation records it as imposed. `None` sets nothing and the subject keeps its own
    /// default, which is the honest answer for a run nobody attributed.
    #[cfg_attr(feature = "clap", arg(long, value_name = "ACTOR"))]
    pub actor: Option<String>,

    /// The operator's own programs, consulted at each call — the native loop's `--hooks` file.
    ///
    /// **The only content-level refusal this arm has.** The vendor arms answer every call through
    /// this seam, so an embedder decides them here; the native loop decides in-process and consults
    /// programs instead, and without one its whole enforcement is which tools the catalogue
    /// published. That is a path-and-existence answer: it can withhold `file_write` entirely, and
    /// it cannot say *this* write, to a file the step legitimately needs, changes a field the
    /// caller owns. A driven run measured with no hook is measured with one tier switched off.
    ///
    /// Named and never discovered, exactly as the loop requires: a hook found in the workspace
    /// would be a program the repository runs on this machine.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub hooks: Option<PathBuf>,

    /// A program on this host the confined run may execute, staged and mounted read-only.
    ///
    /// **A different question from [`Self::allow_program`], and the difference is the whole
    /// point.** The allow-list says what a `run` may *name*; this says what the sandbox
    /// *contains*. A confined run reaches `/usr`, `/bin`, `/lib`, `/lib64` and its workspace, so a
    /// program allow-listed by absolute host path is admitted and then dies at `ENOENT` — which a
    /// model reads as a wrong command rather than a missing file, and which left a driven run
    /// writing a planning store's files directly because the CLI it was told to use was not there.
    ///
    /// The harness stages the one file and adds its mounted path to its own allow-list, so naming
    /// it here is the whole declaration. Native arm only: a vendor harness runs the operator's own
    /// machine and has nothing to mount into.
    #[cfg_attr(feature = "clap", arg(long, value_name = "PATH"))]
    pub driver: Option<PathBuf>,

    /// Whose tools the model is offered.
    #[cfg_attr(feature = "clap", arg(long, value_enum, default_value = "native"))]
    pub tool_surface: ToolSurface,

    /// A program the **served** `run` tool may start. Repeatable; empty publishes no `run` at all.
    ///
    /// Only under [`ToolSurface::Owned`], where metaharness supplies the tools: an argv whose
    /// program could be anything is the shell that surface exists to remove, and a set nobody
    /// named means nobody wanted one. Under `native` the vendor's own shell is on the tool list
    /// and this says nothing about it.
    #[cfg_attr(feature = "clap", arg(long, value_name = "PROGRAM"))]
    pub allow_program: Vec<String>,

    /// Where the credential comes from.
    #[cfg_attr(
        feature = "clap",
        arg(long, value_enum, default_value = "operator-login")
    )]
    pub credentials: CredentialSource,

    /// The model to ask for. Passed through to the vendor, which resolves it.
    #[cfg_attr(feature = "clap", arg(long))]
    pub model: Option<String>,

    /// A model gateway to point the harness at, as the gateway's **root** URL (no `/v1`).
    ///
    /// The generic model adapter (design `model-adapter-v0.1.md`): each harness reaches its own
    /// native dialect under this root — Claude Code speaks Anthropic messages at
    /// `{root}/v1/messages`, codex the `OpenAI` Responses wire at `{root}/v1/responses`. Requires
    /// `--credentials none`, because a child pointed at a foreign endpoint must hold no operator
    /// credential: Claude Code is given a placeholder key (the gateway sees `x-api-key:` with
    /// the placeholder), and codex sends no auth header at all for a provider that names no
    /// `env_key` — both verified against the pins (MA-V1, MA-V2).
    #[cfg_attr(feature = "clap", arg(long, value_name = "BASE_URL"))]
    pub model_endpoint: Option<String>,

    /// Which model API the harness speaks under `--model-endpoint`, when the harness has a choice.
    ///
    /// Only the b10x loop has one: `openai-responses` and `anthropic-messages` are two different
    /// endpoints under the same root, and the loop cannot infer which from the URL. The vendor
    /// harnesses each speak exactly one dialect and are refused this flag by name, rather than
    /// accepting it and ignoring it.
    #[cfg_attr(feature = "clap", arg(long, value_name = "WIRE"))]
    pub model_wire: Option<String>,

    /// A file holding a subscription token, for a route that takes one instead of an API key.
    ///
    /// **Not the same credential as [`CredentialSource::ApiKey`], and not interchangeable with
    /// it:** a subscription token and a key issued to a program travel under different header
    /// names, so a route given the wrong one answers 401 about authentication and says nothing
    /// about the header. Named here and never read: metaharness passes the path into an argv and
    /// the secret does not enter this process.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub subscription_token_file: Option<String>,

    /// A variable holding a subscription token. The name, never the value.
    #[cfg_attr(feature = "clap", arg(long, value_name = "NAME"))]
    pub subscription_token_env: Option<String>,

    /// A JSON pointer to the token inside the named source, when that source is a JSON document.
    ///
    /// Absent means the whole source is the token. Named rather than known: which field a given
    /// credential store puts its access token in is that store's business.
    #[cfg_attr(feature = "clap", arg(long, value_name = "POINTER"))]
    pub subscription_token_pointer: Option<String>,

    /// The reasoning effort to ask of the model, in the vendor's own vocabulary.
    ///
    /// Claude Code takes it as `--effort`; codex reads `model_reasoning_effort` from the
    /// scratch config. The value is passed through and validated by whoever serves the model —
    /// a run option rather than a hardcoded default, because an endpoint may support a
    /// different vocabulary than the vendor's own service (one gateway was observed refusing
    /// Claude Code's default `high` while accepting `medium`, `low` and `xhigh`).
    #[cfg_attr(feature = "clap", arg(long, value_name = "LEVEL"))]
    pub effort: Option<String>,

    /// A ceiling on turns.
    #[cfg_attr(feature = "clap", arg(long))]
    pub max_turns: Option<u32>,

    /// Plugin directories to load, and only these.
    ///
    /// Each one is **copied into the run's scratch tree before the child starts** and digested on
    /// the way in, so the plugin the run had is a snapshot metaharness holds rather than a
    /// directory the operator can edit mid-run — the same argument H10 makes for the copied input
    /// tree. The copy list and the digest are values on the launch plan, readable before any
    /// process exists, and the digest reaches `session.started` through the attestation. A
    /// directory that is not there, or that holds no file, is refused by name at plan time.
    #[cfg_attr(feature = "clap", arg(long, value_name = "DIR"))]
    pub plugin_dir: Vec<PathBuf>,

    /// A **marketplace** plugin to install into the scratch config home, pinned.
    ///
    /// `<marketplace-repo>@<name>@<version-or-commit>`, repeatable. Amendment a16, and the
    /// deliberate opposite of *"no ambient plugins"* — H1a says plugins are exactly the **declared
    /// set**, never that there are none, and this adds to that set by name.
    ///
    /// Three things about it are decisions rather than implementation
    /// (`docs/design/runs-side-by-side-v0.1.md` § 3):
    ///
    /// * **An unpinned spelling is refused**, at parse, before anything is spawned. A plugin that
    ///   can change between two runs that both name it makes a comparison of those two runs
    ///   meaningless.
    /// * **The run reaches no network.** Resolution is against a marketplace the operator has
    ///   already fetched with `claude plugin marketplace add` and `claude plugin install`; a
    ///   launch-time fetch would be unpinnable (neither verb takes a ref at 2.1.258) and would
    ///   reach out from inside the boundary § 8 exists to draw.
    /// * **Placement is into the scratch config home**, in the layout read from a real one, and
    ///   `--plugin-dir` is *not* also passed for it — two mechanisms loading one plugin would
    ///   report it twice, under two different sources.
    ///
    /// Claude Code only. Refused by name on the other kinds rather than accepted and ignored.
    #[cfg_attr(feature = "clap", arg(long, value_name = "REPO@NAME@PIN"))]
    pub plugin: Vec<crate::plugin::MarketplacePlugin>,

    /// An operator-named working directory for the child, instead of a scratch one.
    ///
    /// The declaration that trades two hermetic rows for real work (amendment a6): the child
    /// runs **in this tree**, so H7 ("the working directory is ours") and H11 ("no memory file
    /// outside the copied tree") stop being impositions and are attested unavailable, naming
    /// this directory. `--hermetic strict` therefore refuses such a run, `--hermetic` reports
    /// it honestly, and the embedder that wants a governed run over a real repository — the
    /// driven case — accepts exactly that trade. `--add-dir` stays denied either way.
    ///
    /// **The child can write to that tree** (amendment a6.1): the adapter widens the vendor's own
    /// sandbox over the named directory, because a trade that hands back two hermetic rows for a
    /// repository the child may only read hands back nothing — which is what a paid codex run
    /// found on 2026-08-23. The grant is stated in the H7 attestation row, so it is readable off
    /// the run's own record rather than off a scratch config file that no longer exists.
    #[cfg_attr(feature = "clap", arg(long, value_name = "DIR"))]
    pub cwd: Option<PathBuf>,

    /// Copy the run's raw vendor wire into this directory when the run ends.
    ///
    /// The retained transcript or rollout and every raw hook input the vendor wrote are copied
    /// out of the scratch root before it is deleted — those files and nothing else, so a
    /// credential copied into the scratch home never travels. This is the capture surface the
    /// adapter contract's golden samples come from (CT-2): recorded real wire otherwise dies
    /// with the scratch directory that held it, and capture is a per-pin cost worth a flag.
    #[cfg_attr(feature = "clap", arg(long, value_name = "DIR"))]
    pub retain_dir: Option<PathBuf>,

    /// Refuse before the run when the installed vendor version is outside the adapter's pin,
    /// instead of warning (design § 8.4 O1).
    #[cfg_attr(feature = "clap", arg(long))]
    pub strict_version: bool,

    /// Judge the run: the built-in hermetic floor always, the external auditor when `--spec` is
    /// given (design D12).
    #[cfg_attr(feature = "clap", arg(long))]
    pub audit: bool,

    /// The expectation document the external auditor is pointed at. A `--spec` with no
    /// `--auditor` is a refusal, not a skip: a specification nobody checked reads exactly like a
    /// specification that passed.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub spec: Option<PathBuf>,

    /// substrate's daemon socket, so the run may write and execute inside a confined workspace.
    /// **`b10x` only.**
    ///
    /// The vendor harnesses bring their own tools and their own containment story; this is the
    /// declaration that gives *our* loop one. Without it — or [`Self::substrate_embedded`] — the
    /// catalogue behind the three verbs is read-only, which is a fact about the machine rather
    /// than a setting, and it is why an arm launched with neither cannot attempt a task that has
    /// to change a file.
    #[cfg_attr(feature = "clap", arg(long, value_name = "SOCKET"))]
    pub substrate: Option<PathBuf>,

    /// Hold substrate's driver in the run's own process instead of reaching a daemon.
    /// **`b10x` only**, and never beside [`Self::substrate`].
    ///
    /// The same confinement — guarded IO, `openat2` containment, cgroups and namespaces around an
    /// exec. What a socket adds and this does not is an authenticated subject derived from kernel
    /// peer credentials; embedded there is no peer. Right for a run on the operator's own machine,
    /// wrong for anything multi-tenant.
    #[cfg_attr(feature = "clap", arg(long, conflicts_with = "substrate"))]
    pub substrate_embedded: bool,

    /// A delegated cgroup subtree, so a confined run may start a process. **`b10x` only.**
    ///
    /// Without one substrate reports no exec facts and no `run` entry is published — correct, and
    /// also why a test-first task cannot be attempted: a run that may not execute its suite cannot
    /// see a test fail before writing the code, so it will not write the code. The subtree must be
    /// delegated to this user, hold `cpu`, `memory` and `pids`, and be free of processes.
    #[cfg_attr(feature = "clap", arg(long, value_name = "DIR"))]
    pub cgroup_root: Option<PathBuf>,

    /// A build toolchain admitted read-only inside the confined workspace. **`b10x` only.**
    ///
    /// Without one a confined run can execute anything whose implementation lives under `/usr` —
    /// an interpreter — and nothing whose compilers live in the operator's home, which is every
    /// build tool. The directories are mounted read-only and reported in the run's observation
    /// (substrate ADR 0010); the network stays unshared.
    ///
    /// Declared rather than implied, because it is the one place a confined run is given something
    /// substrate did not verify. Refused for the vendor kinds, which reach the filesystem through
    /// their own tools and would have nothing to do with it.
    #[cfg_attr(feature = "clap", arg(long, value_name = "NAME"))]
    pub toolchain: Option<String>,

    /// A rate card, so the run's record states what it cost. **`b10x` only.**
    ///
    /// Claude Code and codex price their own runs from a catalogue their service delivers, and
    /// metaharness reads the figure they report rather than computing one — a cost multiplied out
    /// here would be a second number that disagrees with the invoice the first time a rate moves
    /// (design § 4.1, D4). Nothing changes about that.
    ///
    /// The b10x loop has no such catalogue behind it: the `OpenAI` Responses wire returns token
    /// counts and no price, and the codex model cache carries no rates either. So the rates are
    /// declared and the **loop** multiplies them out, exactly as Claude Code's own client does
    /// — the figure that reaches `session.ended.total_cost_usd` is still the harness's own, not
    /// metaharness's. Passed through to `--prices` and validated there.
    ///
    /// Refused by name on the other kinds rather than ignored: a card handed to a run that cannot
    /// use one would leave the operator believing a figure was declared when the vendor's was used.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub prices: Option<PathBuf>,

    /// Where this run may write, as `<glob>=<allowed|partial-only|denied>`. **`b10x` only.**
    ///
    /// Ordered, first match wins, and a path no rule names is unrestricted — this declares where
    /// writing is *bounded*, and a scope nobody wrote bounds nothing.
    ///
    /// `partial-only` is the word that earns the shape: the path may be changed in part and never
    /// replaced whole. No set of operations can say that, because `file.write` and `file.edit` are
    /// both writes, and it is exactly the rule a planning store whose frontmatter is owned by a CLI
    /// needs.
    ///
    /// The **operation is refused, never the run** — a run killed for reaching once makes the scope
    /// a trap rather than a boundary, and the reaching is what a denial is for.
    ///
    /// Refused for the vendor kinds, and not because they should be unbounded: they carry the same
    /// rule as `Frame.subjects`, sealed into the frame's digest and adjudicated at their hook seam.
    /// The b10x loop has no seam at all — its published toolset *is* its policy — so for that kind
    /// the scope has to travel to the tools.
    #[cfg_attr(feature = "clap", arg(long, value_name = "GLOB=SCOPE"))]
    pub write_scope: Vec<String>,

    /// Whether the declared scope is also stated in the instruction. **`b10x` only.**
    ///
    /// `silent` is an experiment control. A run told the rule and a run refused the rule both end
    /// with the rule kept, and only the second shows that the **toolset** kept it. Stating it is
    /// cheaper, so a real run states it.
    #[cfg_attr(
        feature = "clap",
        arg(long, value_name = "MODE", default_value = "stated")
    )]
    pub scope_announce: ScopeAnnounce,

    /// A file the run is given before it starts, instead of discovering it. **`b10x` only.**
    ///
    /// A stateless loop replays its conversation, so this is paid on every turn rather than once.
    /// It is still usually a saving: what it replaces is a read, a turn, *and* a result that joins
    /// the same replay. Bounded deliberately — a handful of named files, never a directory.
    ///
    /// A file that is not there refuses the run. A run given a smaller context than it was declared
    /// to have is one nobody can reproduce from the declaration.
    #[cfg_attr(feature = "clap", arg(long, value_name = "FILE"))]
    pub context: Vec<PathBuf>,

    /// The external auditor, as an **argv prefix**. A single-word program name is a degenerate
    /// prefix and a two-word subcommand is not a special case (design § 9.4, finding F2).
    #[cfg_attr(feature = "clap", arg(long, value_name = "PREFIX"))]
    pub auditor: Option<String>,

    /// Arguments passed through to the auditor after everything metaharness adds.
    #[cfg_attr(feature = "clap", arg(last = true, value_name = "AUDITOR_ARGS"))]
    pub auditor_args: Vec<String>,
}

impl RunSpec {
    /// A run of this kind and nothing else asked for.
    #[must_use]
    pub fn new(kind: Kind) -> Self {
        Self {
            kind,
            hermetic: HermeticMode::Off,
            prompt: None,
            frame: None,
            decisions: DecisionMode::Frame,
            actor: None,
            hooks: None,
            driver: None,
            tool_surface: ToolSurface::Native,
            allow_program: Vec::new(),
            credentials: CredentialSource::OperatorLogin,
            model: None,
            model_endpoint: None,
            model_wire: None,
            subscription_token_file: None,
            subscription_token_env: None,
            subscription_token_pointer: None,
            effort: None,
            max_turns: None,
            plugin_dir: Vec::new(),
            plugin: Vec::new(),
            cwd: None,
            retain_dir: None,
            strict_version: false,
            audit: false,
            spec: None,
            substrate: None,
            substrate_embedded: false,
            cgroup_root: None,
            toolchain: None,
            prices: None,
            write_scope: Vec::new(),
            scope_announce: ScopeAnnounce::Stated,
            context: Vec::new(),
            auditor: None,
            auditor_args: Vec::new(),
        }
    }

    /// Prefix a task with the execution boundary the agent is actually running under.
    ///
    /// This is context, never authority. The adapters still enforce their own launch plans and
    /// metaharness still owns the decision seam. Stating the split prevents an agent from assuming
    /// that the vendor's own confinement and metaharness observation are one mechanism. Metaharness
    /// does not place vendor harnesses inside a Substrate process envelope.
    #[must_use]
    pub fn with_agent_execution_context(&self, task: &str) -> String {
        let hermetic = match self.hermetic {
            HermeticMode::Off => "off",
            HermeticMode::On => "on",
            HermeticMode::Strict => "strict",
        };
        let workspace = if self.cwd.is_some() {
            "operator-named"
        } else {
            "metaharness-scratch"
        };
        let decisions = match self.decisions {
            DecisionMode::Ask => "metaharness asks the embedder before each mediated call",
            DecisionMode::Frame => "metaharness applies the sealed frame before each mediated call",
            DecisionMode::Observe => "metaharness records calls but does not narrow them",
        };
        format!(
            "<execution-context>\nexecution_path=metaharness-driven\ninner_harness={}\nworkspace={}\nhermetic={}\nconfinement=inner-harness\nsubstrate=not-used-by-metaharness\ndecision_mode={}\nimplication={}\n</execution-context>\n\n{}",
            self.kind.as_str(),
            workspace,
            hermetic,
            self.decisions.as_str(),
            decisions,
            task
        )
    }
}

#[cfg(test)]
mod execution_context_tests {
    use super::*;

    #[test]
    fn a_vendor_agent_sees_who_drives_decides_and_owns_confinement() {
        let mut spec = RunSpec::new(Kind::Codex);
        spec.decisions = DecisionMode::Ask;
        spec.hermetic = HermeticMode::Strict;
        let prompt = spec.with_agent_execution_context("do the work");

        assert!(
            prompt.contains("execution_path=metaharness-driven"),
            "{prompt}"
        );
        assert!(prompt.contains("inner_harness=codex"), "{prompt}");
        assert!(prompt.contains("confinement=inner-harness"), "{prompt}");
        assert!(
            prompt.contains("substrate=not-used-by-metaharness"),
            "{prompt}"
        );
        assert!(prompt.contains("decision_mode=ask"), "{prompt}");
        assert!(prompt.ends_with("do the work"), "{prompt}");
    }
}
