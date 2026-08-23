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
}

impl Kind {
    /// The kind's name, as the CLI spells it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Kind::Claude => "claude",
            Kind::Codex => "codex",
        }
    }
}

/// Who decides a tool call.
///
/// Two modes rather than one because a round trip per call costs latency, and an embedder that
/// answers "yes" to everything the frame already admits has bought nothing (design D5). A run in
/// `frame` mode is still fully audited: both modes emit `tool.decided` and the census counts
/// both.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum DecisionMode {
    /// The adapter decides from the frame's admitted set. No round trip.
    Frame,
    /// The embedder decides. The run blocks, one round trip per call.
    Ask,
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

    /// Who decides a tool call.
    #[cfg_attr(feature = "clap", arg(long, value_enum, default_value = "frame"))]
    pub decisions: DecisionMode,

    /// Whose tools the model is offered.
    #[cfg_attr(feature = "clap", arg(long, value_enum, default_value = "native"))]
    pub tool_surface: ToolSurface,

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
    #[cfg_attr(feature = "clap", arg(long, value_name = "DIR"))]
    pub plugin_dir: Vec<PathBuf>,

    /// An operator-named working directory for the child, instead of a scratch one.
    ///
    /// The declaration that trades two hermetic rows for real work (amendment a6): the child
    /// runs **in this tree**, so H7 ("the working directory is ours") and H11 ("no memory file
    /// outside the copied tree") stop being impositions and are attested unavailable, naming
    /// this directory. `--hermetic strict` therefore refuses such a run, `--hermetic` reports
    /// it honestly, and the embedder that wants a governed run over a real repository — the
    /// driven case — accepts exactly that trade. `--add-dir` stays denied either way.
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
            tool_surface: ToolSurface::Native,
            credentials: CredentialSource::OperatorLogin,
            model: None,
            model_endpoint: None,
            effort: None,
            max_turns: None,
            plugin_dir: Vec::new(),
            cwd: None,
            retain_dir: None,
            strict_version: false,
            audit: false,
            spec: None,
            auditor: None,
            auditor_args: Vec::new(),
        }
    }
}
