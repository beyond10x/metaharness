//! One options type, two spellings.
//!
//! **The builder is a face on one value, not a second configuration surface** (design D11).
//! Every `with_…` below sets one field of [`RunSpec`], `start` consumes it, and the CLI's `run`
//! flags are a `derive` on the same struct — so a flag the library cannot express cannot be
//! added, and an option the CLI cannot express cannot be introduced. The first statement of that
//! rule was decorative and the design's own two surfaces had already drifted (finding F16),
//! which is why `metaharness-cli` carries a mechanical test rather than a paragraph.
//!
//! One value is deliberately **not** a spec field: [`Metaharness::with_frame`] takes an
//! in-memory [`Frame`]. `RunSpec.frame` stays a `PathBuf` because resolving a path is the
//! library's job and parsing a frame document in the binary would be protocol logic in the CLI
//! (design § 9.3, correction 3). Since amendment a5 the path resolves to a sealed
//! `metaharness.frame/1` document; giving both spellings at once is refused by name.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use metaharness_protocol::{
    Capabilities, CredentialSource, DecisionMode, Digest, EventStream, Frame, HermeticMode, Kind,
    PluginContent, PluginInstall, PluginTree, Refused, RunId, RunSpec, Seam, ToolSurface,
    TranscriptRef, tree_digest,
};

use crate::clock::{Clock, SystemClock};
use crate::custody::CredentialCustody;
use crate::loopback::{LoopbackHandle, LoopbackProxy};
use crate::process::{CredentialCopyView, LaunchPlanView, ProcessRunner};
use crate::refusal::Refusal;
use crate::run::{LaunchFacts, Run, RunParts, vendor_hook_timeout_ms};
use metaharness_protocol::SeamFactory;

/// Where a loopback proxy forwards when the run named no gateway of its own.
///
/// The vendor's own host, stated here rather than inherited from the child's environment: the
/// whole point of the provider is that metaharness decides where the traffic goes, and a default
/// read out of an ambient `ANTHROPIC_BASE_URL` would put that decision back in the environment
/// H3 exists to ignore.
const VENDOR_UPSTREAM: &str = "https://api.anthropic.com";

/// The same for codex, whose API-key traffic goes to `api.openai.com`.
///
/// `https://api.openai.com/v1/responses` is a literal in the pinned 0.145.0 binary, and the
/// provider entry this launch writes names `{proxy}/v1` — so a child request to `/v1/responses`
/// arrives here as the same path on this host. The **subscription** host is deliberately not
/// listed: that half of the door is refused by name (V-LP6), and a default nobody routes to would
/// be a claim wearing a constant's clothes.
const CODEX_VENDOR_UPSTREAM: &str = "https://api.openai.com";

/// Where this run's proxy forwards when the run named no gateway of its own.
fn vendor_upstream(kind: Kind) -> &'static str {
    match kind {
        Kind::Claude => VENDOR_UPSTREAM,
        Kind::Codex => CODEX_VENDOR_UPSTREAM,
        // **There is no vendor upstream for a loop we own.** The other two kinds name a vendor's
        // API because metaharness may stand a credential proxy in front of it; `b10x-harness` is
        // pointed at a `--base-url` the caller names and reads a credential file the caller names,
        // so there is nothing here to forward to and nothing to intercept. The empty string is the
        // honest answer and `loopback_for` never asks for it on this kind.
        Kind::B10x => "",
    }
}

/// What a run starts with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Input {
    /// Start with this prompt. Sets [`RunSpec::prompt`], so the spec stays the one place a run
    /// is described.
    Prompt(String),
    /// Start with whatever prompt the spec already carries — the driven case, where the caller
    /// built the whole spec (design § 9.1).
    FromSpec,
}

/// The library face.
#[derive(Debug, Clone, PartialEq)]
pub struct Metaharness {
    spec: RunSpec,
    frame: Option<Frame>,
}

impl Metaharness {
    /// A run of this kind and nothing else asked for.
    #[must_use]
    pub fn new(kind: Kind) -> Self {
        Self {
            spec: RunSpec::new(kind),
            frame: None,
        }
    }

    /// A run from a spec the caller already has, skipping the builder entirely.
    #[must_use]
    pub fn from_spec(spec: RunSpec) -> Self {
        Self { spec, frame: None }
    }

    /// How hermetic the run must be.
    #[must_use]
    pub fn with_hermetic(mut self, mode: HermeticMode) -> Self {
        self.spec.hermetic = mode;
        self
    }

    /// The prompt to start with.
    #[must_use]
    pub fn with_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.spec.prompt = Some(prompt.into());
        self
    }

    /// The frame in force, as an in-memory value.
    ///
    /// Not a spec field: the spec's `frame` is a path, resolved at start. A frame given here is
    /// sealed on the way in, so the digest an event cites always describes the contents that
    /// were actually in force.
    #[must_use]
    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame.seal());
        self
    }

    /// A frame **document**: a sealed `metaharness.frame/1` file, resolved at start.
    ///
    /// The same field the CLI's `--frame` sets, so both faces resolve — and refuse — the same
    /// way. Unreadable, unsealed or malformed documents are refusals by name, and giving this
    /// together with [`Metaharness::with_frame`] is refused rather than resolved by precedence.
    #[must_use]
    pub fn with_frame_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.spec.frame = Some(path.into());
        self
    }

    /// Who decides a tool call.
    #[must_use]
    pub fn with_decisions(mut self, mode: DecisionMode) -> Self {
        self.spec.decisions = mode;
        self
    }

    /// Whose tools the model is offered.
    #[must_use]
    pub fn with_tool_surface(mut self, surface: ToolSurface) -> Self {
        self.spec.tool_surface = surface;
        self
    }

    /// Where the credential comes from.
    #[must_use]
    pub fn with_credentials(mut self, source: CredentialSource) -> Self {
        self.spec.credentials = source;
        self
    }

    /// The model to ask for.
    #[must_use]
    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.spec.model = Some(model.into());
        self
    }

    /// A model gateway to point the harness at, as the gateway's root URL (no `/v1`).
    ///
    /// The generic model adapter: Claude Code reaches `{root}/v1/messages`, codex
    /// `{root}/v1/responses`. Requires `credentials: none` — the adapters refuse the
    /// combination with an operator credential by name.
    #[must_use]
    pub fn with_model_endpoint(mut self, base_url: impl Into<String>) -> Self {
        self.spec.model_endpoint = Some(base_url.into());
        self
    }

    /// The reasoning effort to ask of the model, in the vendor's own vocabulary.
    #[must_use]
    pub fn with_effort(mut self, level: impl Into<String>) -> Self {
        self.spec.effort = Some(level.into());
        self
    }

    /// A ceiling on turns.
    #[must_use]
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.spec.max_turns = Some(turns);
        self
    }

    /// One more plugin directory to load, and only these.
    ///
    /// The directory is read, digested and **copied into the run's scratch tree** at launch, so
    /// the plugin the run had is a snapshot metaharness holds; a directory that is not there or
    /// holds no file is refused by name before anything is spawned.
    #[must_use]
    pub fn with_plugin_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.spec.plugin_dir.push(directory.into());
        self
    }

    /// An operator-named working directory for the child, instead of a scratch one.
    ///
    /// The driven case's declaration (amendment a6): the child works in a real tree, and H7 and
    /// H11 are attested unavailable rather than claimed — so `--hermetic strict` refuses the run
    /// and `--hermetic` reports the trade by name.
    #[must_use]
    pub fn with_cwd(mut self, directory: impl Into<PathBuf>) -> Self {
        self.spec.cwd = Some(directory.into());
        self
    }

    /// Copy the run's raw vendor wire into this directory when the run ends.
    ///
    /// The capture surface the adapter contract's golden samples come from (CT-2): the retained
    /// transcript or rollout and the raw hook inputs, and never a credential.
    #[must_use]
    pub fn with_retain_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.spec.retain_dir = Some(directory.into());
        self
    }

    /// Refuse before the run when the installed vendor version is outside the adapter's pin.
    #[must_use]
    pub fn with_strict_version(mut self, strict: bool) -> Self {
        self.spec.strict_version = strict;
        self
    }

    /// Judge the run.
    #[must_use]
    pub fn with_audit(mut self, audit: bool) -> Self {
        self.spec.audit = audit;
        self
    }

    /// The expectation document the external auditor is pointed at.
    #[must_use]
    pub fn with_spec_file(mut self, path: impl Into<PathBuf>) -> Self {
        self.spec.spec = Some(path.into());
        self
    }

    /// substrate's daemon socket, so the run may write and execute. `b10x` only.
    #[must_use]
    pub fn with_substrate(mut self, socket: impl Into<PathBuf>) -> Self {
        self.spec.substrate = Some(socket.into());
        self
    }

    /// Hold substrate's driver in the run's own process instead. `b10x` only.
    #[must_use]
    pub fn with_substrate_embedded(mut self, embedded: bool) -> Self {
        self.spec.substrate_embedded = embedded;
        self
    }

    /// The delegated cgroup subtree a confined run may start a process inside. `b10x` only.
    #[must_use]
    pub fn with_cgroup_root(mut self, root: impl Into<PathBuf>) -> Self {
        self.spec.cgroup_root = Some(root.into());
        self
    }

    /// The rate card this run is priced at. `b10x` only — see [`RunSpec::prices`].
    #[must_use]
    pub fn with_prices(mut self, path: impl Into<PathBuf>) -> Self {
        self.spec.prices = Some(path.into());
        self
    }

    /// The external auditor, as an argv prefix.
    #[must_use]
    pub fn with_auditor(mut self, prefix: impl Into<String>) -> Self {
        self.spec.auditor = Some(prefix.into());
        self
    }

    /// One more argument passed through to the auditor after everything metaharness adds.
    #[must_use]
    pub fn with_auditor_arg(mut self, argument: impl Into<String>) -> Self {
        self.spec.auditor_args.push(argument.into());
        self
    }

    /// The spec this builder has composed.
    #[must_use]
    pub fn spec(&self) -> &RunSpec {
        &self.spec
    }

    /// The frame this builder holds, if any.
    #[must_use]
    pub fn frame(&self) -> Option<&Frame> {
        self.frame.as_ref()
    }

    /// Start against the real vendor binary.
    ///
    /// The default face: it spawns `claude`, installs the `PreToolUse` seam, and answers that
    /// seam's calls over the channel [`crate::HookChannel`] describes. A caller that wants a
    /// different process — the scripted fake every C3 vector runs against — uses
    /// [`Metaharness::start_with`], which is the same function with the runner supplied.
    ///
    /// # Errors
    ///
    /// Every refusal a start can raise, checked in the order that tells a caller the most
    /// useful thing first: the spec's own faults before the machine's. A vendor binary that is
    /// absent or unrunnable arrives here as [`Refusal::Io`], which is exit `2` — metaharness
    /// could not do its job, never a verdict about the run.
    pub fn start(self, input: Input) -> Result<Run, Refusal> {
        // The runner and the seam are the kind's, because the two vendors do not put their record
        // and their calls in the same place: Claude Code writes both down one pipe, codex writes
        // its record to a session file and its calls to a hook channel beside it.
        match self.spec.kind {
            Kind::Claude => self.start_with(
                input,
                &mut crate::spawn::SpawnRunner::new(),
                &mut metaharness_claude::ClaudeSeams,
            ),
            Kind::Codex => self.start_with(
                input,
                &mut crate::spawn_codex::CodexSpawnRunner::new(),
                &mut metaharness_codex::CodexSeams,
            ),
            // The loop writes its whole record down one pipe, like Claude Code, so the plain
            // spawn runner reads it. What differs is that nothing decides its calls, which is the
            // seam's business rather than the runner's.
            Kind::B10x => self.start_with(
                input,
                &mut crate::spawn::SpawnRunner::new(),
                &mut metaharness_b10x::B10xSeams::new(None, None, None),
            ),
        }
    }

    /// Start against a runner the caller supplies, with the real clock.
    ///
    /// # Errors
    ///
    /// Every refusal in [`Refusal`] that a start can raise: an unknown kind, a frame document
    /// that is unreadable, unsealed or in conflict with an in-memory frame, an owned tool
    /// surface, a control the adapter cannot honour, and whatever the adapter said when it
    /// planned the launch.
    pub fn start_with(
        self,
        input: Input,
        runner: &mut dyn ProcessRunner,
        seams: &mut dyn SeamFactory,
    ) -> Result<Run, Refusal> {
        self.start_with_clock(input, runner, seams, Box::new(SystemClock::new()))
    }

    /// Start against a runner and a clock the caller supplies.
    ///
    /// The clock is a seam because § 7.7 rule 2's deadline has to expire in a test, and a vector
    /// that slept for real would buy a slow suite and prove the same thing.
    ///
    /// # Errors
    ///
    /// As [`Metaharness::start_with`].
    pub fn start_with_clock(
        self,
        input: Input,
        runner: &mut dyn ProcessRunner,
        seams: &mut dyn SeamFactory,
        clock: Box<dyn Clock>,
    ) -> Result<Run, Refusal> {
        let credentials = credentials_file(&self.spec);
        self.start_resolved(input, runner, seams, clock, credentials)
    }

    /// The same start, with the operator's credential file already resolved.
    ///
    /// Private, and the seam the loopback vectors drive: [`credentials_file`] answers out of
    /// `HOME`, and a vector that took that answer would open the operator's **real**
    /// `.credentials.json` in order to prove that a run does not copy it. Resolving one step
    /// higher lets a test name a file it fabricated and keeps the operator's own out of the
    /// suite entirely.
    fn start_resolved(
        self,
        input: Input,
        runner: &mut dyn ProcessRunner,
        seams: &mut dyn SeamFactory,
        clock: Box<dyn Clock>,
        credentials: Option<PathBuf>,
    ) -> Result<Run, Refusal> {
        let frame = self.frame.clone();
        let spec = self.applied(input);
        check_spec(&spec)?;
        let frame = resolve_frame(frame, &spec)?;
        // After the resolution and not before it, because there are two spellings of a frame —
        // an in-memory value and a document path — and the combination has to be refused in both.
        if frame.is_some() && spec.decisions == DecisionMode::Observe {
            return Err(Refusal::ObserveWithFrame);
        }

        // One `match` and no trait. The two adapters' launch plans are different types with
        // different fields — one carries a settings document and a `--settings` path, the other a
        // `hooks.json` and a `config.toml` — and a trait to unify them would be an abstraction
        // invented for two implementations. A third adapter is when it earns its keep. What is
        // shared is what is genuinely neutral and already factored: the spec check, the frame
        // resolution, the scratch root, the cwd, the ancestor walk and the control refusals.
        match spec.kind {
            Kind::Claude => start_claude(spec, credentials, frame, runner, seams, clock),
            Kind::Codex => start_codex(spec, credentials, frame, runner, seams, clock),
            Kind::B10x => start_b10x(spec, frame, runner, seams, clock),
        }
    }

    fn applied(mut self, input: Input) -> RunSpec {
        if let Input::Prompt(prompt) = input {
            self.spec.prompt = Some(prompt);
        }
        self.spec
    }
}

/// Start a Claude Code run: the M2 path, plus LP-3's proxy.
///
/// One step is out of order compared with every other credential source, and deliberately: the
/// loopback proxy is **started before the launch is planned**. Its base URL is an ephemeral port,
/// so unlike `--model-endpoint` — a static string the spec already carries into `plan_launch` —
/// there is nothing for a pure function to compute until something has bound a socket. So: start
/// the proxy, put its two facts in the context, then plan.
/// Launching a loop we own.
///
/// # Why this is a fifth the size of the other two
///
/// Not because it is unfinished. The five things that cost `start_claude` and `start_codex` their
/// hundred lines each — a scratch `HOME` so ambient config cannot leak in, a copied plugin tree, a
/// hook channel the seam answers on, retrieval of a transcript the vendor wrote somewhere else, and
/// custody of a credential this process must hold and proxy — are all answered by what
/// `b10x-harness` already is. It reads no config file, has no plugin mechanism, decides in-process,
/// writes its record to stdout, and reads a credential file the caller named.
///
/// What is left is an argv and a pipe.
///
/// # No loopback, and that is a property rather than a gap
///
/// The other kinds may stand a credential proxy between the vendor and its API so a subscription
/// login never leaves this machine. There is nothing to intercept here: the loop is pointed at a
/// `--base-url` and a key file, both named by the caller, and metaharness never sees the
/// credential. `credentials_file` answers `None` on this kind for the same reason.
fn start_b10x(
    spec: RunSpec,
    frame: Option<Frame>,
    runner: &mut dyn ProcessRunner,
    seams: &mut dyn SeamFactory,
    clock: Box<dyn Clock>,
) -> Result<Run, Refusal> {
    let capabilities = metaharness_b10x::capabilities();
    let refusals = start_refusals(&capabilities, &spec);
    if !refusals.is_empty() {
        return Err(Refusal::Control { refusals });
    }

    let scratch = tempfile::TempDir::new()?;
    let confined = spec.substrate.is_some() || spec.substrate_embedded;
    let cwd = resolve_cwd_named(
        &spec,
        scratch.path(),
        if confined {
            B10X_SCRATCH_WORKSPACE
        } else {
            "work"
        },
    )?;
    if confined && !adoptable(&cwd) {
        // Refused rather than degraded, because the degradation is invisible: substrate would
        // decline to represent the workspace, the catalogue would come back read-only, and the run
        // would go on to report that it could not find a way to change the file it was asked to
        // change. That reads as a model failure and is a naming rule.
        return Err(Refusal::Launch {
            detail: format!(
                "confinement was asked for and the working directory {} cannot be adopted: \
                 substrate represents a workspace only when its directory name starts with \
                 `{SUBSTRATE_WORKSPACE_PREFIX}`. Rename it, or drop --cwd and let the run use a \
                 scratch one",
                cwd.display()
            ),
        });
    }
    let transcript_path = scratch.path().join("transcript.jsonl");
    let run_id = run_id(&spec);

    let transcript = TranscriptRef {
        path: Some(transcript_path.display().to_string()),
        digest: None,
        bytes: None,
    };
    // `Seam::None` whatever the tool surface says. There is no seam: no registration, no hook, no
    // control request. Naming one would put a word in the record for a thing that does not exist,
    // and every `tool.requested` this adapter emits says `nobody adjudicated this` in the same
    // breath.
    let seam = Seam::None;
    // The attestation is honest about being empty: metaharness imposed nothing on this launch
    // because there was nothing to impose. An attestation claiming controls it did not apply is
    // the one document a reader has no way to check.
    let attestation = metaharness_protocol::HermeticAttestation::none(spec.hermetic);
    let bridge = seams.build(transcript.clone(), attestation, seam);

    let mut argv = metaharness_b10x::argv(&b10x_launch(&spec, &cwd)?);
    // Resolved against the `PATH` the child is given rather than the operator's, so `doctor` and
    // the spawn agree on which install answered (CT-3), and so the record names the file that ran.
    let named = argv.remove(0);
    let child_path = metaharness_b10x::child_path(std::env::var("HOME").ok().as_deref());
    let program = metaharness_b10x::resolve_program(&named, &child_path)
        .ok_or_else(|| Refusal::Launch {
            detail: format!(
                "`{named}` is not on the PATH this run gives its child ({child_path}). The child's \
                 environment is constructed rather than inherited (H3), so a binary the operator \
                 can run is not automatically one the run can: install it there, or name it by \
                 absolute path"
            ),
        })?
        .display()
        .to_string();
    let args = argv;
    // Constructed, never inherited. `PATH` is the one variable the loop always needs and the one
    // whose absence made every launch of this arm fail before it read an argument.
    let mut env: BTreeMap<String, String> = BTreeMap::from([("PATH".to_owned(), child_path)]);
    if spec.credentials == CredentialSource::ApiKey {
        // The argv names the variable; this puts it in the child's environment, and without both
        // halves `--api-key-env` points at something that is not there. Absent from the operator's
        // environment is refused by name rather than launched unauthenticated: a run that silently
        // dropped its credential would fail at the first request with the endpoint's word for it,
        // which is a worse explanation than this one. The Claude adapter refuses the same way for
        // `ANTHROPIC_API_KEY`.
        let key = std::env::var(B10X_API_KEY_VARIABLE).map_err(|_| Refusal::Launch {
            detail: format!(
                "the run declared credentials: api-key and {B10X_API_KEY_VARIABLE} was not in the \
                 caller's environment. Set it, or use --credentials none for an endpoint that \
                 authenticates nobody"
            ),
        })?;
        env.insert(B10X_API_KEY_VARIABLE.to_owned(), key);
    }
    let view = LaunchPlanView {
        program: &program,
        args: &args,
        env: &env,
        cwd: &cwd,
        credential_copies: &[],
        // There is no decision channel. The scratch root stands in as a path the runner can hold,
        // and nothing is ever written to it: a run that answered a decision here would be a run
        // this adapter had decided something in.
        decision_channel: scratch.path(),
        transcript: &transcript_path,
    };
    let process = runner.start(&view)?;

    let launch = LaunchFacts {
        planned_cwd: Some(cwd.display().to_string()),
        declared_plugins: Vec::new(),
        // The b10x loop binds its tools in-process; there is no MCP server to configure and
        // nothing for one to be compared against.
        planned_mcp_servers: Vec::new(),
        pinned_versions: metaharness_b10x::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
        transcript,
    };

    Ok(Run::new(RunParts {
        stream: EventStream::new(RunId::new(run_id)),
        spec,
        bridge,
        process,
        clock,
        capabilities,
        frame,
        seam,
        // Zero, because there is no vendor deadline to wait out: nothing asks this adapter for a
        // decision, so no call is ever left hanging on one.
        vendor_timeout_ms: 0,
        launch,
        wire: vec![transcript_path],
        scratch: Some(scratch),
        loopback: None,
    }))
}

fn start_claude(
    spec: RunSpec,
    credentials: Option<PathBuf>,
    frame: Option<Frame>,
    runner: &mut dyn ProcessRunner,
    seams: &mut dyn SeamFactory,
    clock: Box<dyn Clock>,
) -> Result<Run, Refusal> {
    {
        let capabilities = metaharness_claude::capabilities();
        let refusals = start_refusals(&capabilities, &spec);
        if !refusals.is_empty() {
            return Err(Refusal::Control { refusals });
        }

        let scratch = tempfile::TempDir::new()?;
        let cwd = resolve_cwd(&spec, scratch.path())?;
        let transcript_path = scratch.path().join("transcript.jsonl");
        let run_id = run_id(&spec);

        // Held in a local, so every `?` below this line drops it — and `LoopbackHandle::drop`
        // stops the accept thread. A launch refused after the proxy started must not leave a port
        // open on the operator's machine holding their live credential behind it.
        let loopback = loopback_for(&spec, credentials.as_deref(), &run_id)?;

        let context = metaharness_claude::LaunchContext {
            scratch_root: scratch.path().to_path_buf(),
            cwd: cwd.clone(),
            credentials_file: credentials,
            inherited_env: std::env::vars().collect::<BTreeMap<String, String>>(),
            memory_ancestors: memory_ancestors(&cwd),
            inputs_digest: None,
            plugins: plugin_trees(&spec),
            loopback: loopback_params(loopback.as_ref()),
            tool_server: tool_server(),
        };
        let plan = metaharness_claude::plan_launch(&spec, &context).map_err(|refusal| {
            Refusal::Launch {
                detail: refusal.to_string(),
            }
        })?;

        // The plan names four pieces of I/O and performs none of them, because a pure function
        // that wrote to a disk would be a pure function nobody could test. This is where they
        // happen, once, before any runner sees the plan — so the scripted process and the real
        // one are handed a world that was built the same way.
        let channel = crate::spawn::HookChannel::create(scratch.path())?;
        materialise(&plan, scratch.path(), &channel)?;

        let transcript = TranscriptRef {
            path: Some(transcript_path.display().to_string()),
            digest: None,
            bytes: None,
        };
        let seam = match spec.tool_surface {
            ToolSurface::Owned => Seam::OwnedTool,
            ToolSurface::Native => Seam::Hook,
        };
        let bridge = seams.build(transcript.clone(), plan.attestation.clone(), seam);

        let copies: Vec<CredentialCopyView<'_>> = plan
            .credential_copies
            .iter()
            .map(|copy| CredentialCopyView {
                from: copy.from.as_path(),
                to: copy.to.as_path(),
            })
            .collect();
        let view = LaunchPlanView {
            program: &plan.program,
            args: &plan.args,
            env: &plan.env,
            cwd: &plan.cwd,
            credential_copies: &copies,
            decision_channel: channel.root(),
            transcript: &transcript_path,
        };
        let process = runner.start(&view)?;

        let launch = LaunchFacts {
            planned_cwd: Some(plan.cwd.display().to_string()),
            // Read off the **attestation** and not off the spec: H1a compares the vendor's own
            // plugin list against what metaharness says it installed, and a declared set taken
            // from the spec would still name a plugin whose copy never happened.
            declared_plugins: declared_plugins(&plan.attestation),
            // Read off the document the launch actually wrote, so H5 compares the record against
            // the file the vendor was handed rather than against a second guess at its contents.
            planned_mcp_servers: configured_mcp_servers(plan.mcp_config.as_ref()),
            pinned_versions: metaharness_claude::PINNED_VERSIONS
                .iter()
                .map(ToString::to_string)
                .collect(),
            transcript,
        };

        Ok(Run::new(RunParts {
            stream: EventStream::new(RunId::new(run_id)),
            spec,
            bridge,
            process,
            clock,
            capabilities,
            frame,
            seam,
            vendor_timeout_ms: vendor_hook_timeout_ms(&plan.hook),
            launch,
            // The raw wire, named file by file — never the scratch root, which also holds the
            // copied credential (H6). What `--retain-dir` captures is exactly this list.
            wire: vec![
                transcript_path,
                metaharness_claude::HookChannelPaths::under(scratch.path()).requests,
            ],
            scratch: Some(scratch),
            // Handed over rather than kept here: the run is what the port is scoped to, so the
            // thing that ends the run closes it.
            loopback: loopback.map(|started| started.handle),
        }))
    }
}

/// The run's id, which under loopback is also what the placeholder names.
///
/// Computed before the proxy starts rather than at the end, because the placeholder carries it —
/// `mh-run-<id>-<nonce>` — and a request arriving at the port can then be attributed to a run
/// without a session table.
fn run_id(spec: &RunSpec) -> String {
    format!("{}-{}", spec.kind.as_str(), std::process::id())
}

/// The proxy this run needs, or none because it declared another credential source.
///
/// A function rather than an inline `match` so the one condition that starts a listening socket
/// on the operator's machine is a single readable line, in one place, with a name.
fn loopback_for(
    spec: &RunSpec,
    credentials: Option<&Path>,
    run_id: &str,
) -> Result<Option<StartedLoopback>, Refusal> {
    match spec.credentials {
        CredentialSource::Loopback => Ok(Some(start_loopback(spec, credentials, run_id)?)),
        CredentialSource::OperatorLogin | CredentialSource::ApiKey | CredentialSource::None => {
            Ok(None)
        }
    }
}

/// A running proxy and the one thing about the custody behind it a launch plan may know.
///
/// The class travels beside the handle rather than inside it because it is a fact about the
/// **operator's file**, not about the socket: the proxy is the same object either way, and it is
/// the codex launch plan that refuses one of the two by name (V-LP6).
struct StartedLoopback {
    handle: LoopbackHandle,
    login: Option<metaharness_codex::CodexLogin>,
}

/// The two facts a started proxy contributes to a Claude Code launch plan, and nothing else.
fn loopback_params(
    started: Option<&StartedLoopback>,
) -> Option<metaharness_claude::LoopbackParams> {
    started.map(|started| metaharness_claude::LoopbackParams {
        base_url: started.handle.base_url(),
        placeholder: started.handle.placeholder().to_string(),
    })
}

/// The same for codex, plus the login class its door is opened or refused on.
///
/// A subscription custody produces a `LoopbackParams` all the same, and the **plan** refuses it —
/// rather than the builder refusing first — so the refusal is the adapter's own, is carried by a
/// C1 vector, and reads identically whether a run reached it through this library or through a
/// caller planning a launch directly.
fn codex_loopback_params(
    started: Option<&StartedLoopback>,
) -> Option<metaharness_codex::LoopbackParams> {
    started.map(|started| metaharness_codex::LoopbackParams {
        base_url: started.handle.base_url(),
        placeholder: started.handle.placeholder().to_string(),
        // A codex custody always classifies; `None` cannot arise here, and `Subscription` is the
        // conservative reading of a class that somehow did not, because it is the one that
        // refuses.
        login: started
            .login
            .unwrap_or(metaharness_codex::CodexLogin::Subscription),
    })
}

/// Open custody on the operator's credential and put a proxy in front of it.
///
/// Every failure here is a **launch refusal**, never a panic: the operator asked for a run and
/// what they get back is a reason, on the same path as every other thing that can be wrong with a
/// spec. Three of them are distinguishable and each sends the reader somewhere different — no
/// file was named, the named file is not there, and the file is there but is not a credential.
fn start_loopback(
    spec: &RunSpec,
    credentials: Option<&Path>,
    run_id: &str,
) -> Result<StartedLoopback, Refusal> {
    let missing = metaharness_claude::LaunchRefusal::CredentialFileMissing.to_string();
    let Some(path) = credentials else {
        return Err(Refusal::Launch { detail: missing });
    };
    let custody = CredentialCustody::open(spec.kind, path).map_err(|error| Refusal::Launch {
        detail: if error.kind() == std::io::ErrorKind::NotFound {
            format!("{missing} ({error})")
        } else {
            format!(
                "the loopback proxy has no custody to hold: {error}. metaharness never writes \
                 this file — it only reads it under a lock — so the fix is the vendor's own login"
            )
        },
    })?;
    let login = custody.login();
    // A gateway the run named becomes the **proxy's** upstream, one hop further out than it would
    // be without the proxy; with no gateway named it is the vendor's own host. Either way the
    // child sees only the loopback port, which is what makes the hop inspectable.
    let upstream = spec
        .model_endpoint
        .as_deref()
        .unwrap_or_else(|| vendor_upstream(spec.kind));
    let handle = LoopbackProxy::start(upstream, Arc::new(custody), run_id).map_err(|error| {
        Refusal::Launch {
            detail: format!(
                "the loopback proxy could not start in front of {upstream}: {error}. The run is \
                 refused rather than started without it, because a child pointed at a port nothing \
                 is listening on fails with a vendor error about the network and names none of this"
            ),
        }
    })?;
    Ok(StartedLoopback { handle, login })
}

/// Start a codex run: CX-M2.
///
/// The same seven steps as [`start_claude`] and two of them land differently, because this vendor
/// puts its record and its calls in different places:
///
/// * **the transcript is a file the child writes**, not its stdout, so the path handed to the
///   runner is where the runner *copies the rollout to* rather than where it dumps a pipe; and
/// * **the hook config lives inside the scratch `CODEX_HOME`**, because that is where codex
///   declares a hook and there is no `--setting-sources` to switch that source off.
///
/// The loopback proxy is started here too (LP-4), in the same out-of-order step as
/// [`start_claude`]: its base URL is an ephemeral port, so the proxy binds first and the plan is
/// made second. **Where the two vendors differ is what the child is told** — Claude Code takes a
/// base URL from its environment, codex takes a `model_providers` entry from the scratch
/// `config.toml` — and **which logins are routed**: a ChatGPT-plan custody is refused by the codex
/// launch plan by name, because V-LP6 is unanswered (see `docs/design/loopback-provider-v0.1.md`).
fn start_codex(
    spec: RunSpec,
    credentials: Option<PathBuf>,
    frame: Option<Frame>,
    runner: &mut dyn ProcessRunner,
    seams: &mut dyn SeamFactory,
    clock: Box<dyn Clock>,
) -> Result<Run, Refusal> {
    let capabilities = metaharness_codex::capabilities();
    let refusals = start_refusals(&capabilities, &spec);
    if !refusals.is_empty() {
        return Err(Refusal::Control { refusals });
    }

    let scratch = tempfile::TempDir::new()?;
    let cwd = resolve_cwd(&spec, scratch.path())?;
    let transcript_path = scratch.path().join("rollout.jsonl");
    let run_id = run_id(&spec);

    // Held in a local, so every `?` below this line drops it — and `LoopbackHandle::drop` stops
    // the accept thread. A launch refused after the proxy started (a subscription custody is the
    // real case) must not leave a port open on the operator's machine with their credential
    // behind it.
    let loopback = loopback_for(&spec, credentials.as_deref(), &run_id)?;

    let context = metaharness_codex::LaunchContext {
        scratch_root: scratch.path().to_path_buf(),
        cwd: cwd.clone(),
        credentials_file: credentials,
        inherited_env: std::env::vars().collect::<BTreeMap<String, String>>(),
        memory_ancestors: memory_ancestors(&cwd),
        inputs_digest: None,
        plugins: plugin_trees(&spec),
        loopback: codex_loopback_params(loopback.as_ref()),
    };
    let plan =
        metaharness_codex::plan_launch(&spec, &context).map_err(|refusal| Refusal::Launch {
            detail: refusal.to_string(),
        })?;

    let channel = crate::spawn_codex::CodexHookChannel::create(scratch.path())?;
    materialise_codex(&plan, scratch.path(), &channel)?;

    let transcript = TranscriptRef {
        path: Some(transcript_path.display().to_string()),
        digest: None,
        bytes: None,
    };
    // Always the hook: `--tool-surface owned` is refused by the codex launch plan by name, so
    // there is no second value this could take.
    let seam = Seam::Hook;
    let bridge = seams.build(transcript.clone(), plan.attestation.clone(), seam);

    let copies: Vec<CredentialCopyView<'_>> = plan
        .credential_copies
        .iter()
        .map(|copy| CredentialCopyView {
            from: copy.from.as_path(),
            to: copy.to.as_path(),
        })
        .collect();
    let view = LaunchPlanView {
        program: &plan.program,
        args: &plan.args,
        env: &plan.env,
        cwd: &plan.cwd,
        credential_copies: &copies,
        decision_channel: channel.root(),
        transcript: &transcript_path,
    };
    let process = runner.start(&view)?;

    let launch = LaunchFacts {
        planned_cwd: Some(plan.cwd.display().to_string()),
        // The same source as the claude path's, and the same reason. What differs on this vendor
        // is how much the attestation's `loaded_by` claims: the placement is undriven here, so a
        // plugin list the record does not carry leaves H1a `unk` — which is the honest verdict
        // for an installation nobody has watched the vendor pick up.
        declared_plugins: declared_plugins(&plan.attestation),
        // The codex config writes an empty `[mcp_servers]` table by name, so the launch gives one
        // and H5 compares against nothing — which is what the table says.
        planned_mcp_servers: Vec::new(),
        pinned_versions: metaharness_codex::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
        transcript,
    };

    Ok(Run::new(RunParts {
        stream: EventStream::new(RunId::new(run_id)),
        spec,
        bridge,
        process,
        clock,
        capabilities,
        frame,
        seam,
        vendor_timeout_ms: vendor_hook_timeout_ms(&plan.hook),
        launch,
        // The raw wire: the copied rollout, the thin `--json` stdout retained beside it, and
        // the hook inputs. Named file by file for the same reason as the claude list — the
        // scratch `CODEX_HOME` holds a copied `auth.json` that must never travel.
        wire: vec![
            crate::spawn_codex::stdout_path(&transcript_path),
            transcript_path,
            metaharness_codex::HookChannelPaths::under(scratch.path()).requests,
        ],
        scratch: Some(scratch),
        // Handed over rather than kept here: the run is what the port is scoped to, so the thing
        // that ends the run closes it (LP-4).
        loopback: loopback.map(|started| started.handle),
    }))
}

/// Perform the I/O the codex launch plan names and deliberately does not do.
///
/// | what | why it is here and not in the plan |
/// |---|---|
/// | the scratch `CODEX_HOME` and the temporary directory, empty | H1a's scratch home is a directory, and a directory has to be made |
/// | `$CODEX_HOME/config.toml`, **the seam included** | the plan decides its contents; writing it is I/O. 0.145.0 reads its hooks out of `[hooks]` in this file and reads no standalone `hooks.json` at all |
/// | the `PreToolUse` executable at the path the hook definition names | **the definition without the file is a seam that is never consulted** |
/// | its executable bit | a hook the vendor cannot execute fails as a hook that did not fire, which looks exactly like a run where nothing was attempted |
fn materialise_codex(
    plan: &metaharness_codex::LaunchPlan,
    scratch_root: &std::path::Path,
    channel: &crate::spawn_codex::CodexHookChannel,
) -> Result<(), Refusal> {
    install_plugins(&plan.plugin_installs)?;
    std::fs::create_dir_all(&plan.config_home)?;
    // Deliberately a subdirectory of the scratch root and not the operator's own: codex refuses
    // to create its helper shims when `CODEX_HOME` sits under the process's temporary directory,
    // and the child's `TMPDIR` is this directory, so the config home is beside it rather than
    // inside it.
    std::fs::create_dir_all(scratch_root.join("tmp"))?;

    std::fs::write(
        metaharness_codex::config_path(scratch_root),
        plan.config.as_bytes(),
    )?;

    let program_path = metaharness_codex::hook_program_path(scratch_root);
    if let Some(parent) = program_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let program = metaharness_codex::hook_program(&metaharness_codex::HookChannelPaths::at_root(
        channel.root(),
    ));
    std::fs::write(&program_path, program.as_bytes())?;
    make_executable(&program_path)?;
    Ok(())
}

/// The MCP servers a launch's own configuration document names, for H5.
///
/// Read off the document rather than recomputed from the spec: H5 asks whether the surface the
/// vendor reported is the surface the launch *gave*, and a list derived from the spec would still
/// name a server whose configuration was never written.
fn configured_mcp_servers(mcp_config: Option<&serde_json::Value>) -> Vec<String> {
    mcp_config
        .and_then(|config| config.get("mcpServers"))
        .and_then(serde_json::Value::as_object)
        .map(|servers| servers.keys().cloned().collect())
        .unwrap_or_default()
}

/// This binary, so `--tool-surface owned` can name a server for the child to start.
///
/// `current_exe` and not a configured path: the server is `metaharness mcp-serve`, so the program
/// that serves the tools is by construction the same build as the one that planned the run. A
/// configured path could name an older one, and the tools a run was judged on would then be a
/// different set from the tools it declared.
///
/// `None` when the kernel will not say, which is a refusal at the launch rather than a guess here.
fn tool_server() -> Option<std::path::PathBuf> {
    std::env::current_exe().ok()
}

/// Perform the four pieces of I/O the launch plan names and deliberately does not do.
///
/// | what | why it is here and not in the plan |
/// |---|---|
/// | the config home and the temporary directory, empty | H1a's scratch home is a directory, and a directory has to be made |
/// | the settings document at the path the argv's `--settings` names | the plan decides its contents; writing it is I/O |
/// | the `PreToolUse` executable at the path the hook definition names | the definition is a value; the program is a file, and **the definition without the file is a seam that is never consulted** |
/// | the hook program's executable bit | a hook the vendor cannot execute fails as a hook that did not fire, which looks exactly like a run where nothing was attempted |
///
/// The settings document goes **outside** the config home, which is the placement the adapter
/// chose so it would not have to know the answer to Q14 — and the answer, read from a live run
/// on 2.1.239, is that the hook does fire from there under `--setting-sources ""`.
fn materialise(
    plan: &metaharness_claude::LaunchPlan,
    scratch_root: &std::path::Path,
    channel: &crate::spawn::HookChannel,
) -> Result<(), Refusal> {
    install_plugins(&plan.plugin_installs)?;
    std::fs::create_dir_all(&plan.config_home)?;
    std::fs::create_dir_all(scratch_root.join("tmp"))?;

    std::fs::write(
        metaharness_claude::settings_path(scratch_root),
        serde_json::to_string_pretty(&plan.settings)
            .map_err(|error| Refusal::Io {
                detail: format!("the settings document could not be rendered: {error}"),
            })?
            .as_bytes(),
    )?;

    // Written only when the plan carries one, so a `native` run leaves no configuration lying in
    // the scratch root claiming servers it never had.
    if let Some(mcp_config) = &plan.mcp_config {
        std::fs::write(
            metaharness_claude::mcp_config_path(scratch_root),
            serde_json::to_string_pretty(mcp_config)
                .map_err(|error| Refusal::Io {
                    detail: format!("the MCP configuration could not be rendered: {error}"),
                })?
                .as_bytes(),
        )?;
    }

    let program_path = metaharness_claude::hook_program_path(scratch_root);
    if let Some(parent) = program_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let program = metaharness_claude::hook_program(&metaharness_claude::HookChannelPaths::at_root(
        channel.root(),
    ));
    std::fs::write(&program_path, program.as_bytes())?;
    make_executable(&program_path)?;
    Ok(())
}

/// Every declared plugin directory, **read**, so a pure `plan_launch` can decide about it.
///
/// The I/O half of crossing #4, on this side of § 8.4 O7's line for the same reason the ancestor
/// walk and the inputs digest are: the adapter decides where a plugin goes and whether the run may
/// proceed, and it does that from values rather than from a filesystem it went and looked at.
///
/// Nothing is refused here. A directory that is missing or empty comes back as
/// [`PluginContent::Unreadable`] or [`PluginContent::Empty`] and is refused **by name, by the
/// launch plan** — because a refusal raised here would be a refusal the adapter's own vectors
/// could not reach.
fn plugin_trees(spec: &RunSpec) -> Vec<PluginTree> {
    spec.plugin_dir
        .iter()
        .map(|source| PluginTree {
            source: source.clone(),
            content: read_plugin_tree(source),
        })
        .collect()
}

/// What one plugin directory holds, as a digest over its files.
fn read_plugin_tree(source: &Path) -> PluginContent {
    let mut files = BTreeMap::new();
    match walk_plugin_tree(source, source, &mut files) {
        Err(error) => PluginContent::Unreadable {
            detail: error.to_string(),
        },
        Ok(()) if files.is_empty() => PluginContent::Empty,
        Ok(()) => PluginContent::Files {
            count: files.len(),
            digest: tree_digest(&files),
        },
    }
}

/// Every regular file under `directory`, keyed by its path relative to `root`.
///
/// **A symlink is neither a directory nor a file here** and is therefore skipped, because
/// `DirEntry::file_type` does not follow one. That is deliberate and it is the same rule
/// [`copy_tree`] uses: what is digested and what is copied must be the same set, or the
/// attestation cites a digest of something other than what the run got. A plugin that needs a
/// symlink is a plugin whose contents metaharness cannot pin, and it will digest and install
/// without it rather than follow a link out of the tree it was pointed at.
fn walk_plugin_tree(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, Digest>,
) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let path = entry.path();
        if kind.is_dir() {
            walk_plugin_tree(root, &path, files)?;
        } else if kind.is_file() {
            let relative = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .into_owned();
            files.insert(relative, Digest::of(&std::fs::read(&path)?));
        }
    }
    Ok(())
}

/// Copy each planned plugin into the run's scratch tree.
///
/// Performed **once**, here, before the child exists — unlike a credential copy, which happens
/// immediately before every spawn because a token ages out (H6, Q13). A plugin does not age; what
/// it must not do is change under a run that has already digested it, and a copy is what stops it.
fn install_plugins(installs: &[PluginInstall]) -> Result<(), Refusal> {
    for install in installs {
        copy_tree(&install.from, &install.to).map_err(|error| Refusal::Io {
            detail: format!(
                "the plugin {} could not be installed at {}: {error}",
                install.from.display(),
                install.to.display()
            ),
        })?;
    }
    Ok(())
}

/// One directory into another, files and real subdirectories, skipping exactly what
/// [`walk_plugin_tree`] skipped.
fn copy_tree(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        let target = to.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &target)?;
        } else if kind.is_file() {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// The plugin names metaharness says it installed, for H1a's comparison against the record.
fn declared_plugins(attestation: &metaharness_protocol::HermeticAttestation) -> Vec<String> {
    attestation
        .installed_plugins
        .iter()
        .map(|plugin| plugin.name.clone())
        .collect()
}

/// Give the hook program its executable bit.
///
/// Separated because it is the one step with a platform in it, and because forgetting it is a
/// silent failure: the vendor reports a hook that would not run the same way it reports a hook
/// that had nothing to say.
#[cfg(unix)]
fn make_executable(path: &std::path::Path) -> Result<(), Refusal> {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = std::fs::metadata(path)?.permissions();
    permissions.set_mode(0o700);
    std::fs::set_permissions(path, permissions)?;
    Ok(())
}

/// The same, where there is no such bit.
#[cfg(not(unix))]
fn make_executable(_path: &std::path::Path) -> Result<(), Refusal> {
    Ok(())
}

/// Every command this run's configuration will need that the adapter refuses.
///
/// Computed at **run start** and not at the call, so a run that will fail on control fails
/// before it spends money (design § 6.1). A separate function so the mapping from the adapter's
/// capability table to a named refusal is one thing a test can read.
#[must_use]
pub fn start_refusals(capabilities: &Capabilities, spec: &RunSpec) -> Vec<(String, Refused)> {
    capabilities
        .refusals_for(spec)
        .into_iter()
        .map(|(name, code)| {
            (
                name.to_string(),
                Refused::new(
                    code,
                    format!(
                        "this run's configuration needs {name} and the {} adapter refuses it",
                        capabilities.adapter.id
                    ),
                ),
            )
        })
        .collect()
}

/// The child's working directory: the operator's, or a scratch one made here.
///
/// The operator's directory is used, never created: a typo that silently became an empty
/// directory would be a run over nothing reporting success.
fn resolve_cwd(spec: &RunSpec, scratch_root: &std::path::Path) -> Result<PathBuf, Refusal> {
    resolve_cwd_named(spec, scratch_root, "work")
}

/// The same, with the scratch directory's own name.
///
/// Named rather than fixed because substrate will only represent a workspace whose directory starts
/// with `ws_`, and a b10x run that means to write needs one it can adopt. A scratch directory called
/// `work` leaves that run **silently read-only** — the tools it publishes are what the machine can
/// confine, so the write entries simply do not appear and nothing says why.
fn resolve_cwd_named(
    spec: &RunSpec,
    scratch_root: &std::path::Path,
    scratch_name: &str,
) -> Result<PathBuf, Refusal> {
    match &spec.cwd {
        Some(directory) if directory.is_dir() => Ok(directory.clone()),
        Some(directory) => Err(Refusal::Io {
            detail: format!(
                "the operator-named working directory {} does not exist or is not a directory",
                directory.display()
            ),
        }),
        None => {
            let work = scratch_root.join(scratch_name);
            std::fs::create_dir_all(&work)?;
            Ok(work)
        }
    }
}

/// The prefix substrate requires of a workspace directory it will represent.
const SUBSTRATE_WORKSPACE_PREFIX: &str = "ws_";

/// The scratch working directory a confined b10x run gets.
const B10X_SCRATCH_WORKSPACE: &str = "ws_run";

/// The frame in force, from whichever of the two spellings this run used.
///
/// Resolving the path is the library's job (D11): the CLI carries it, this reads it. Done before
/// any I/O toward a spawn, so a bad document is a free refusal, never a paid one.
fn resolve_frame(in_memory: Option<Frame>, spec: &RunSpec) -> Result<Option<Frame>, Refusal> {
    let Some(path) = &spec.frame else {
        return Ok(in_memory);
    };
    if in_memory.is_some() {
        return Err(Refusal::FrameConflict { path: path.clone() });
    }
    let text = std::fs::read_to_string(path).map_err(|error| Refusal::FrameUnreadable {
        path: path.clone(),
        detail: error.to_string(),
    })?;
    let frame = Frame::parse_document(&text).map_err(|error| Refusal::FrameInvalid {
        path: path.clone(),
        detail: error.to_string(),
    })?;
    Ok(Some(frame))
}

/// Everything a spec can be refused for, in one place, so both faces refuse identically.
///
/// `spec.frame` is deliberately not here: since amendment a5 a frame document is resolved, not
/// refused, and its failures ([`Refusal::FrameUnreadable`], [`Refusal::FrameInvalid`]) can only
/// be raised by the resolution itself.
///
/// # Errors
///
/// [`Refusal::NoAdapter`] or [`Refusal::ToolSurfaceOwned`].
pub fn check_spec(spec: &RunSpec) -> Result<(), Refusal> {
    // Strategy C is built (`metaharness mcp-serve`), so what is left is a question about the
    // *vendor*: can its built-in tools be taken away and ours put in their place? Claude Code can
    // (`--tools ""` plus `--mcp-config`). Codex cannot — `dynamicTools` is an app-server surface
    // `codex exec` does not expose, which its own launch already explains. And b10x publishes this
    // very catalogue in-process, so there is nothing to replace and the flag would mean nothing.
    if spec.tool_surface == ToolSurface::Owned && spec.kind != Kind::Claude {
        return Err(Refusal::ToolSurfaceOwned { kind: spec.kind });
    }
    // A rate card is only meaningful where nothing else prices the run. See the variant.
    if spec.prices.is_some() && spec.kind != Kind::B10x {
        return Err(Refusal::PricesUnsupported { kind: spec.kind });
    }
    // substrate confines the tools **we** publish. The vendor harnesses bring their own and reach
    // the filesystem through them, so a socket here would be configured and never consulted.
    let confinement_asked_for =
        spec.substrate.is_some() || spec.substrate_embedded || spec.cgroup_root.is_some();
    if confinement_asked_for && spec.kind != Kind::B10x {
        return Err(Refusal::ConfinementUnsupported { kind: spec.kind });
    }
    Ok(())
}

/// The spec, as the argv `b10x-harness run` is actually spawned with.
///
/// # Why this replaced an argv written here
///
/// This function used to be four words inline — `run --json --input <prompt>` — while
/// [`metaharness_b10x::argv`] sat beside it, exported, unit-tested and called by nothing. The
/// inline version named no endpoint, no model and no credential, so **every launch of this arm
/// died on the loop's own argument parsing before it reached a model**, and the adapter's tests
/// went on passing because they tested the argv nobody used. One builder now, and the tests below
/// it are tests of what runs.
///
/// # What is refused, and why each is refused rather than defaulted
///
/// A missing endpoint or model could be defaulted, and a default would silently point an
/// evaluation arm at somebody's production API on the run where the operator forgot the flag.
///
/// The credential is the interesting one. `b10x-harness` holds no vendor login — there is no
/// `~/.b10x` to copy — so `--credentials operator-login`, which is the *default*, names nothing at
/// all here. Refusing it makes the arm say so once; accepting it would launch a run with no
/// credential under a flag that claims one, which is the failure this whole codebase is built to
/// avoid. `none` and `api-key` are the two that mean something.
fn b10x_launch(
    spec: &RunSpec,
    cwd: &std::path::Path,
) -> Result<metaharness_b10x::B10xLaunch, Refusal> {
    let refuse = |detail: &str| Refusal::Launch {
        detail: detail.to_owned(),
    };
    let base_url = spec.model_endpoint.as_ref().ok_or_else(|| {
        refuse(
            "b10x needs --model-endpoint: the loop is pointed at an endpoint by the caller and \
             has no service of its own to fall back on. A default here would aim an evaluation \
             arm at somebody's production API the first time the flag was forgotten",
        )
    })?;
    let model = spec.model.as_ref().ok_or_else(|| {
        refuse("b10x needs --model: the endpoint serves several and the loop picks none")
    })?;

    let mut launch = metaharness_b10x::B10xLaunch::new(
        base_url.clone(),
        model.clone(),
        cwd,
        spec.prompt.clone().unwrap_or_default(),
    );
    launch = match spec.credentials {
        // The designed shape for a foreign endpoint: no credential in the child, and the far end
        // decides. `model_endpoint`'s own documentation requires exactly this.
        CredentialSource::None => launch,
        CredentialSource::ApiKey => launch.from_environment(B10X_API_KEY_VARIABLE),
        CredentialSource::OperatorLogin => {
            return Err(refuse(
                "b10x has no operator login to copy — it reads a credential the caller names and \
                 keeps none of its own. Use --credentials none for a gateway that authenticates \
                 nobody, or --credentials api-key to pass one through",
            ));
        }
        CredentialSource::Loopback => {
            return Err(refuse(
                "the loopback provider is Claude Code only in this milestone (LP-3), and this \
                 loop already keeps the credential out of metaharness by construction: it is \
                 pointed at a file or a variable the caller named and this process never sees it",
            ));
        }
    };
    // The programs `run` may start, and where it may start them. An empty set publishes no `run`
    // at all, which is the right answer to nobody having named one.
    let programs = spec.allow_program.clone();
    launch = match (&spec.substrate, spec.substrate_embedded) {
        (Some(socket), _) => launch.confined(socket, programs),
        // The root substrate serves is the workspace's parent, because the workspace is adopted
        // rather than created. A run whose working directory has no parent has no tree to serve.
        (None, true) => match cwd.parent() {
            Some(root) => launch.confined_in_process(root, programs),
            None => {
                return Err(refuse(
                    "--substrate-embedded serves the working directory's parent, and this working \
                     directory has none",
                ));
            }
        },
        (None, false) => launch,
    };
    if let Some(root) = &spec.cgroup_root {
        launch = launch.with_cgroup_root(root);
    }
    if let Some(card) = &spec.prices {
        launch = launch.with_prices(card);
    }
    if let Some(turns) = spec.max_turns {
        launch = launch.with_max_turns(turns);
    }
    Ok(launch)
}

/// `true` when substrate will represent a workspace at this path.
fn adoptable(cwd: &std::path::Path) -> bool {
    cwd.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with(SUBSTRATE_WORKSPACE_PREFIX))
}

/// The variable `--credentials api-key` points the b10x loop at.
///
/// The loop speaks the `OpenAI` Responses wire, and this is that wire's conventional name. Named
/// here rather than discovered, so a run's credential source is a fact about the launch instead of
/// a fact about the shell it was started from.
const B10X_API_KEY_VARIABLE: &str = "OPENAI_API_KEY";

/// The operator's credential file, **named and not read**.
///
/// Under an operator login, whether it exists is the spawn's problem, because the copy happens
/// immediately before every spawn and not once per run: a copied operator-login token is a
/// snapshot with a lifetime, and a governed run on 2026-08-22 died an hour in on an OAuth session
/// that could not be refreshed (Q13).
///
/// Under `loopback` the same path means something different: it is the file
/// [`crate::CredentialCustody`] takes custody of, **on this side of the socket**, and it is
/// opened at start rather than at the spawn — a malformed custody refuses the run before it costs
/// anything, which is the other half of the same incident.
///
/// Each vendor keeps its login in its own place and under its own name — `~/.claude/.credentials.json`,
/// `~/.codex/auth.json` — and this is the one line that knows both, because the copy is the
/// library's I/O and the adapters are pure.
fn credentials_file(spec: &RunSpec) -> Option<PathBuf> {
    let (directory, file) = match spec.kind {
        Kind::Claude => (".claude", ".credentials.json"),
        Kind::Codex => (".codex", "auth.json"),
        // **No login to copy.** `b10x-harness` reads a credential file the caller named and has no
        // vendor login directory of its own; `CredentialSource::OperatorLogin` on this kind names
        // nothing, so this answers `None` before the match on the source below is reached.
        Kind::B10x => return None,
    };
    match spec.credentials {
        CredentialSource::OperatorLogin | CredentialSource::Loopback => {
            std::env::var_os("HOME").map(|home| PathBuf::from(home).join(directory).join(file))
        }
        CredentialSource::ApiKey | CredentialSource::None => None,
    }
}

/// Every memory file discoverable above the scratch working directory (design § 8.1 H11).
///
/// Walked and handed to the adapter rather than checked here, because H11's verdict is the
/// adapter's launch assertion: on Claude Code auto-discovery is on in every run that is not
/// `--bare`, and H8 forbids `--bare`; on codex the root-to-cwd `AGENTS.md` walk is native and has
/// no switch at all. Either way a memory file in an ancestor enters the context of a run this
/// design calls hermetic.
fn memory_ancestors(cwd: &std::path::Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for ancestor in cwd.ancestors().skip(1) {
        for name in ["CLAUDE.md", "AGENTS.md"] {
            let candidate = ancestor.join(name);
            if candidate.is_file() {
                found.push(candidate);
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    //! The loopback provider as the library wires it (LP-3).
    //!
    //! Free and model-free by construction: the child is a scripted process that speaks one real
    //! HTTP request at the real proxy, and the upstream is a socket this module opened. No paid
    //! call, no network, and — the rule that matters most here — **no operator credential**: the
    //! file every vector below puts in custody is one it wrote itself, holding a token no account
    //! has ever issued.

    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{Shutdown, TcpListener, TcpStream};
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    use metaharness_protocol::Kind;

    use super::{Input, Metaharness};
    use crate::clock::ManualClock;
    use crate::process::{HarnessProcess, LaunchPlanView, ProcessRunner};
    use crate::refusal::Refusal;
    use crate::scripted::{ScriptStep, ScriptedLog, ScriptedProcess, ScriptedSeams};
    use metaharness_protocol::CredentialSource;

    /// The token the fake credential holds. No account has ever issued it.
    const FAKE_TOKEN: &str = "fake-operator-token-lp3";

    /// The one line the scripted child writes, so the run has a terminal record to end on.
    const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

    /// A credential file in the vendor's shape, in a directory of the test's own.
    fn fake_credential(dir: &std::path::Path) -> PathBuf {
        let path = dir.join(".credentials.json");
        let body = serde_json::json!({
            "claudeAiOauth": {
                "accessToken": FAKE_TOKEN,
                "refreshToken": "fake-refresh-token",
                "expiresAt": 4_102_444_800_000_i64,
                "refreshTokenExpiresAt": 4_102_444_800_000_i64,
                "scopes": ["user:inference"],
                "subscriptionType": "fake",
                "rateLimitTier": "fake",
            }
        });
        std::fs::write(&path, serde_json::to_vec(&body).expect("a credential body"))
            .expect("the fake credential");
        path
    }

    /// One request the fake upstream saw, in the form the assertions need it.
    #[derive(Debug, Clone, Default)]
    struct Seen {
        target: String,
        headers: Vec<(String, String)>,
    }

    impl Seen {
        fn header(&self, name: &str) -> Option<&str> {
            self.headers
                .iter()
                .find(|(candidate, _)| candidate.eq_ignore_ascii_case(name))
                .map(|(_, value)| value.as_str())
        }
    }

    /// An upstream that answers 200 to anything and records what it was asked.
    ///
    /// Deliberately not the vendor: the point of the vector is that the proxy attached the
    /// **custody** token on the way out, and that is only observable from the far side.
    struct FakeUpstream {
        port: u16,
        seen: Arc<Mutex<Vec<Seen>>>,
        stopping: Arc<AtomicBool>,
    }

    impl FakeUpstream {
        fn serving() -> Self {
            let listener = TcpListener::bind(("127.0.0.1", 0)).expect("a fake upstream port");
            let port = listener.local_addr().expect("its address").port();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let stopping = Arc::new(AtomicBool::new(false));
            {
                let seen = Arc::clone(&seen);
                let stopping = Arc::clone(&stopping);
                std::thread::spawn(move || {
                    for incoming in listener.incoming() {
                        if stopping.load(Ordering::SeqCst) {
                            break;
                        }
                        let Ok(stream) = incoming else { break };
                        let seen = Arc::clone(&seen);
                        std::thread::spawn(move || answer(&stream, &seen));
                    }
                });
            }
            Self {
                port,
                seen,
                stopping,
            }
        }

        fn base(&self) -> String {
            format!("http://127.0.0.1:{}", self.port)
        }

        fn requests(&self) -> Vec<Seen> {
            self.seen.lock().expect("the record").clone()
        }
    }

    impl Drop for FakeUpstream {
        fn drop(&mut self) {
            self.stopping.store(true, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
        }
    }

    fn answer(stream: &TcpStream, seen: &Mutex<Vec<Seen>>) {
        let mut reader = BufReader::new(stream);
        let mut request = Seen::default();
        let mut length = 0usize;
        let mut first = true;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line).unwrap_or(0) == 0 {
                return;
            }
            let line = line.trim_end_matches(['\r', '\n']).to_string();
            if line.is_empty() {
                break;
            }
            if first {
                first = false;
                request.target = line
                    .split_whitespace()
                    .nth(1)
                    .unwrap_or_default()
                    .to_string();
                continue;
            }
            if let Some((name, value)) = line.split_once(':') {
                let (name, value) = (name.trim().to_string(), value.trim().to_string());
                if name.eq_ignore_ascii_case("content-length") {
                    length = value.parse().unwrap_or(0);
                }
                request.headers.push((name, value));
            }
        }
        let mut body = vec![0u8; length];
        let _ = reader.read_exact(&mut body);
        seen.lock().expect("the record").push(request);

        let body = r#"{"type":"message","content":[]}"#;
        let head = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n",
            body.len()
        );
        let mut out = stream;
        let _ = out.write_all(head.as_bytes());
        let _ = out.write_all(body.as_bytes());
        let _ = out.flush();
        let _ = stream.shutdown(Shutdown::Both);
    }

    /// What the scripted child was given and what came back when it used it.
    #[derive(Debug, Default, Clone)]
    struct ChildRecord {
        base_url: Option<String>,
        auth_token: Option<String>,
        disable_traffic: Option<String>,
        api_key: Option<String>,
        answer: Option<String>,
    }

    /// A runner whose child really dials the base URL it was given.
    ///
    /// The scripted runner records a plan; this one **uses** it. Without a child that speaks, the
    /// vector would prove that metaharness wrote three environment variables and nothing about
    /// whether a request made with them reaches the upstream carrying the operator's bearer.
    struct DiallingRunner {
        log: ScriptedLog,
        record: Arc<Mutex<ChildRecord>>,
    }

    impl ProcessRunner for DiallingRunner {
        fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
            let value = |key: &str| plan.env.get(key).cloned();
            let mut record = ChildRecord {
                base_url: value("ANTHROPIC_BASE_URL"),
                auth_token: value("ANTHROPIC_AUTH_TOKEN"),
                disable_traffic: value("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC"),
                api_key: value("ANTHROPIC_API_KEY"),
                answer: None,
            };
            if let (Some(base), Some(token)) = (&record.base_url, &record.auth_token) {
                record.answer = Some(dial(base, token));
            }
            *self.record.lock().expect("the record") = record;
            Ok(Box::new(ScriptedProcess::new(
                vec![ScriptStep::line(END)],
                self.log.clone(),
            )))
        }
    }

    /// One `POST /v1/messages` at the proxy, in the spelling `ANTHROPIC_AUTH_TOKEN` documents.
    fn dial(base_url: &str, token: &str) -> String {
        let port: u16 = base_url
            .rsplit(':')
            .next()
            .and_then(|text| text.parse().ok())
            .expect("a base URL ending in a port");
        let body = r#"{"model":"claude-opus-5","messages":[]}"#;
        let request = format!(
            "POST /v1/messages HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: Bearer {token}\r\n\
             anthropic-version: 2023-06-01\r\ncontent-type: application/json\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the proxy port");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a bounded wait");
        stream
            .write_all(request.as_bytes())
            .expect("the whole request");
        stream.flush().expect("the flush");
        let mut all = Vec::new();
        stream.read_to_end(&mut all).expect("the whole answer");
        String::from_utf8_lossy(&all).to_string()
    }

    /// The three variables a loopback child runs on, and the one it must not have.
    ///
    /// Each absence and each presence is a spike finding, and each would fail silently: a second
    /// credential variable puts a second spelling on the wire, and a missing
    /// `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` lets the binary reach `api.anthropic.com`
    /// directly for analytics — past the base URL and out of the proxy's sight.
    fn assert_child_env(child: &ChildRecord) {
        let base_url = child.base_url.as_deref().unwrap_or_default();
        assert!(
            base_url.starts_with("http://127.0.0.1:"),
            "the child must be pointed at a loopback port, got {base_url:?}"
        );
        assert!(
            child
                .auth_token
                .as_deref()
                .is_some_and(|token| token.starts_with("mh-run-claude-")),
            "the child must hold this run's placeholder and nothing else, got {:?}",
            child.auth_token
        );
        assert_eq!(
            child.disable_traffic.as_deref(),
            Some("1"),
            "without this the binary opens api.anthropic.com for analytics whatever the base URL \
             says, and \"the proxy sees every request\" stops being true"
        );
        assert_eq!(
            child.api_key, None,
            "no ANTHROPIC_API_KEY may travel beside the placeholder bearer"
        );
    }

    /// What the far side of the proxy must have seen: the custody token, and no placeholder.
    fn assert_upstream_saw_custody(seen: &[Seen], placeholder: &str) {
        assert_eq!(seen.len(), 1, "exactly one request reached the upstream");
        assert_eq!(seen[0].target, "/v1/messages");
        assert_eq!(
            seen[0].header("authorization"),
            Some(format!("Bearer {FAKE_TOKEN}").as_str()),
            "the upstream must see the custody token: the placeholder is worthless to it, and a \
             proxy that forwarded the placeholder would be spending nothing and reporting success"
        );
        assert!(
            !seen[0]
                .headers
                .iter()
                .any(|(_, value)| value.contains(placeholder)),
            "no header may still carry the placeholder: {:?}",
            seen[0].headers
        );
    }

    /// Whether anything is still listening there.
    fn port_accepts(port: u16) -> bool {
        TcpStream::connect_timeout(
            &std::net::SocketAddr::from(([127, 0, 0, 1], port)),
            Duration::from_millis(500),
        )
        .is_ok()
    }

    /// Whether the port is free of a listener within `patience`, polled rather than asked once.
    ///
    /// The guarantee this stands in for is **not** raced: `LoopbackHandle::shutdown` joins the
    /// accept thread, and that thread owns the `TcpListener`, so the listening socket is closed
    /// before `drain` returns. What is polled here is the difference between that guarantee and
    /// what a single `connect` can observe — **a port number is machine-wide**, and the ephemeral
    /// number this run has just released is immediately available to every other process on the
    /// box. One accepting connect therefore does not falsify "this run's proxy stopped serving";
    /// an accepting connect that persists does, which is what the bound is for.
    ///
    /// Measured, not supposed: under a synthetic bind/close load this assertion failed 3 of 25
    /// runs, and each failing probe was answered by a socket that **closed the connection at once**
    /// (42µs–872µs, never this proxy's 401) and was gone from `ss -ltn` by the next millisecond —
    /// a stranger holding the number, not a proxy outliving its run. The same shutdown path, run
    /// 27,000 times in isolation and under that load, never once left this proxy's own listener up.
    fn port_stops_accepting(port: u16, patience: Duration) -> bool {
        let deadline = std::time::Instant::now() + patience;
        loop {
            if !port_accepts(port) {
                return true;
            }
            if std::time::Instant::now() >= deadline {
                return false;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Vector 4. The whole path, end to end and free: the builder starts a proxy in front of a
    /// fabricated custody, the child is handed its port and placeholder, a real request goes
    /// through it, the fake upstream sees the **custody token** and no trace of the placeholder,
    /// and the run's wind-up closes the port behind it.
    #[test]
    fn a_loopback_run_proxies_the_childs_request_with_custody_and_closes_the_port_after() {
        let home = tempfile::TempDir::new().expect("a directory");
        let credential = fake_credential(home.path());
        let upstream = FakeUpstream::serving();
        let record = Arc::new(Mutex::new(ChildRecord::default()));
        let log = ScriptedLog::new();
        let mut runner = DiallingRunner {
            log: log.clone(),
            record: Arc::clone(&record),
        };
        let mut seams = ScriptedSeams;

        let mut run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::Loopback)
            // The gateway case: under loopback a declared endpoint is the **proxy's** upstream,
            // not the child's base URL, so this is where the fake upstream goes.
            .with_model_endpoint(upstream.base())
            .start_resolved(
                Input::Prompt("go".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
                Some(credential),
            )
            .expect("a loopback run starts");

        let child = record.lock().expect("the record").clone();
        assert_child_env(&child);
        let base_url = child.base_url.clone().expect("the child got a base URL");
        let placeholder = child.auth_token.clone().expect("a placeholder");
        assert!(
            child
                .answer
                .as_deref()
                .is_some_and(|answer| answer.starts_with("HTTP/1.1 200 OK")),
            "the child's own request must have been answered through the proxy, got {:?}",
            child.answer
        );
        assert!(
            log.credential_copies().is_empty(),
            "a loopback run copies no credential anywhere; that is the H6 upgrade"
        );
        assert_upstream_saw_custody(&upstream.requests(), &placeholder);

        let live = run.loopback_report().expect("a loopback run has a report");
        assert!(
            live.forwarded >= 1,
            "the proxy's counters must be readable while the run is going, got {live:?}"
        );
        assert_eq!(live.refused, 0, "nothing was answered 401 at the port");

        let port: u16 = base_url
            .rsplit(':')
            .next()
            .and_then(|text| text.parse().ok())
            .expect("a port");
        assert!(port_accepts(port), "the proxy is up while the run is");

        let lines = run.drain().expect("the run drains");
        assert!(
            lines
                .iter()
                .any(|line| line.event.name() == "session.ended"),
            "the run must complete: {lines:?}"
        );

        assert!(
            port_stops_accepting(port, Duration::from_secs(2)),
            "the run wound up and the loopback port is still accepting; a proxy that outlives its \
             run holds the operator's live credential behind a socket nothing is scoped to"
        );
        let after = run
            .loopback_report()
            .expect("the counters survive the shutdown for the audit that reads them");
        assert_eq!(
            after.forwarded, live.forwarded,
            "the final report is the counters as they stood at wind-up"
        );
    }

    /// Vector 5. A loopback run with nothing to put in custody is refused **before** anything is
    /// spawned, in both spellings of "nothing": no file named at all, and a named file that is
    /// not there. The second is the real one — it is what an operator who has never logged in
    /// hits — and it must not read as a network fault an hour later.
    #[test]
    fn a_loopback_run_without_a_credential_file_is_refused_before_any_spawn() {
        let empty = tempfile::TempDir::new().expect("a directory");
        let log = ScriptedLog::new();
        let mut seams = ScriptedSeams;

        for named in [None, Some(empty.path().join(".credentials.json"))] {
            let mut runner =
                crate::scripted::ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
            let refusal = Metaharness::new(Kind::Claude)
                .with_credentials(CredentialSource::Loopback)
                .start_resolved(
                    Input::Prompt("go".to_string()),
                    &mut runner,
                    &mut seams,
                    Box::new(ManualClock::new()),
                    named.clone(),
                )
                .expect_err("a loopback run with no custody is refused");
            let Refusal::Launch { detail } = &refusal else {
                panic!("expected a launch refusal, got {refusal:?}");
            };
            assert!(
                detail.contains("needs the operator's credential file")
                    && detail.contains("custody"),
                "the refusal must name what was missing and why loopback needs it, got: {detail}"
            );
        }
        assert_eq!(
            log.spawns(),
            0,
            "a refused loopback run must not have spawned a child"
        );
    }

    // ------------------------------------------------------- the codex loopback door (LP-4)

    /// A codex `auth.json` in the API-key shape — the class the door routes. The field name is the
    /// pinned binary's own (`AuthDotJson`); no account has ever been issued this key.
    fn fake_codex_credential(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("auth.json");
        let body = serde_json::json!({
            "OPENAI_API_KEY": FAKE_CODEX_KEY,
            "tokens": null,
            "last_refresh": null,
        });
        std::fs::write(&path, serde_json::to_vec(&body).expect("a credential body"))
            .expect("the fake credential");
        path
    }

    /// The same file in the ChatGPT-plan shape: opens as custody, and is refused by the launch.
    fn fake_codex_subscription(dir: &std::path::Path) -> PathBuf {
        let path = dir.join("auth.json");
        let body = serde_json::json!({
            "OPENAI_API_KEY": null,
            "tokens": {
                "access_token": "fake-chatgpt-access",
                "refresh_token": "fake-chatgpt-refresh",
                "account_id": "fake-account",
            },
            "last_refresh": "2026-08-23T00:00:00Z",
        });
        std::fs::write(&path, serde_json::to_vec(&body).expect("a credential body"))
            .expect("the fake credential");
        path
    }

    /// The key the fake codex credential holds. No account has ever issued it.
    const FAKE_CODEX_KEY: &str = "sk-fake-operator-key-lp4";

    /// What the scripted codex child was given, read from where **this vendor** is told it.
    #[derive(Debug, Default, Clone)]
    struct CodexChildRecord {
        provider_base: Option<String>,
        placeholder: Option<String>,
        api_key: Option<String>,
        auth_json_exists: bool,
        answer: Option<String>,
    }

    /// A runner whose child reads its provider out of the scratch `config.toml` and really dials
    /// it.
    ///
    /// Reading the file rather than the plan is the point: on this vendor the base URL is a config
    /// key, so a vector that asserted on the plan's value alone would pass while the document that
    /// carries it was never written — and a codex child with no `[model_providers]` entry silently
    /// talks to the vendor's own host instead.
    struct CodexDiallingRunner {
        log: ScriptedLog,
        record: Arc<Mutex<CodexChildRecord>>,
    }

    impl ProcessRunner for CodexDiallingRunner {
        fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
            let home = PathBuf::from(plan.env.get("CODEX_HOME").cloned().unwrap_or_default());
            let config = std::fs::read_to_string(home.join("config.toml")).unwrap_or_default();
            let provider_base = config.lines().find_map(|line| {
                line.strip_prefix("base_url = ")
                    .map(|value| value.trim_matches('"').to_string())
            });
            let mut record = CodexChildRecord {
                provider_base: provider_base.clone(),
                placeholder: plan.env.get(metaharness_codex::LOOPBACK_ENV_KEY).cloned(),
                api_key: plan
                    .env
                    .get("OPENAI_API_KEY")
                    .or_else(|| plan.env.get("CODEX_API_KEY"))
                    .cloned(),
                auth_json_exists: home.join("auth.json").exists(),
                answer: None,
            };
            if let (Some(base), Some(placeholder)) = (&provider_base, &record.placeholder) {
                record.answer = Some(dial_responses(base, placeholder));
            }
            *self.record.lock().expect("the record") = record;
            Ok(Box::new(ScriptedProcess::new(
                vec![ScriptStep::line(END)],
                self.log.clone(),
            )))
        }
    }

    /// One `POST {base}/responses` at the proxy, in the spelling an `env_key` provider uses.
    ///
    /// The path is the Responses wire because that is the `wire_api` the provider entry declares;
    /// what the proxy does with the path is nothing at all — it relays it verbatim, which is why
    /// this vector can dial a codex-shaped request through a proxy written for a different vendor.
    fn dial_responses(base: &str, placeholder: &str) -> String {
        let rest = base.trim_start_matches("http://");
        let (authority, path) = rest.split_once('/').unwrap_or((rest, ""));
        let port: u16 = authority
            .rsplit(':')
            .next()
            .and_then(|text| text.parse().ok())
            .expect("a provider base ending in a port");
        let body = r#"{"model":"gpt-fake","input":[]}"#;
        let request = format!(
            "POST /{path}/responses HTTP/1.1\r\nHost: 127.0.0.1\r\n\
             Authorization: Bearer {placeholder}\r\ncontent-type: application/json\r\n\
             Content-Length: {}\r\n\r\n{body}",
            body.len()
        );
        let mut stream = TcpStream::connect(("127.0.0.1", port)).expect("the proxy port");
        stream
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("a bounded wait");
        stream
            .write_all(request.as_bytes())
            .expect("the whole request");
        stream.flush().expect("the flush");
        let mut all = Vec::new();
        stream.read_to_end(&mut all).expect("the whole answer");
        String::from_utf8_lossy(&all).to_string()
    }

    /// What a loopback codex child must have been given, and what it must not. Returns the
    /// provider base it read out of the scratch `config.toml`, because the port in it is what the
    /// rest of the vector is about.
    ///
    /// Each assertion is a spike finding and each would fail silently: a provider entry that never
    /// reached the config file leaves the child talking to the vendor's own host, and a second
    /// credential variable beside the placeholder puts a second spelling on the wire.
    fn assert_codex_child(child: &CodexChildRecord) -> String {
        let base = child
            .provider_base
            .clone()
            .expect("the scratch config.toml names a provider base");
        assert!(
            base.starts_with("http://127.0.0.1:") && base.ends_with("/v1"),
            "the child's provider must be this run's loopback port, got {base:?}"
        );
        assert!(
            child
                .placeholder
                .as_deref()
                .is_some_and(|value| value.starts_with("mh-run-codex-")),
            "the child must hold this run's placeholder and nothing else, got {:?}",
            child.placeholder
        );
        assert_eq!(
            child.api_key, None,
            "no OPENAI_API_KEY or CODEX_API_KEY may travel beside the placeholder"
        );
        assert!(
            !child.auth_json_exists,
            "a loopback run writes no auth.json into the scratch CODEX_HOME; that is the H6 upgrade"
        );
        assert!(
            child
                .answer
                .as_deref()
                .is_some_and(|answer| answer.starts_with("HTTP/1.1 200 OK")),
            "the child's own request must have been answered through the proxy, got {:?}",
            child.answer
        );
        base
    }

    /// What the far side of the proxy must have seen on a codex run: the custody key, the path
    /// verbatim, and no placeholder anywhere.
    fn assert_upstream_saw_codex_custody(seen: &[Seen], placeholder: &str) {
        assert_eq!(seen.len(), 1, "exactly one request reached the upstream");
        assert_eq!(
            seen[0].target, "/v1/responses",
            "the path is relayed verbatim: the proxy curates no vendor's routes"
        );
        assert_eq!(
            seen[0].header("authorization"),
            Some(format!("Bearer {FAKE_CODEX_KEY}").as_str()),
            "the upstream must see the custody key: the placeholder is worthless to it, and a \
             proxy that forwarded the placeholder would be spending nothing and reporting success"
        );
        assert!(
            !seen[0]
                .headers
                .iter()
                .any(|(_, value)| value.contains(placeholder)),
            "no header may still carry the placeholder: {:?}",
            seen[0].headers
        );
    }

    /// LP-4, end to end and free: the builder starts a proxy in front of a fabricated codex
    /// custody, writes a `model_providers` entry pointing at it into the scratch `CODEX_HOME`, the
    /// child dials that entry with the per-run placeholder, the fake upstream sees the **custody
    /// key** and no trace of the placeholder, no `auth.json` is copied anywhere, and the run's
    /// wind-up closes the port behind it.
    ///
    /// What this does **not** prove is that `codex` itself honours the entry — no vendor binary
    /// runs here. That is the paid half of LP-4, and it is labelled as outstanding rather than
    /// implied by this vector's green.
    #[test]
    fn a_codex_loopback_run_points_the_child_at_the_proxy_and_the_upstream_sees_custody() {
        let home = tempfile::TempDir::new().expect("a directory");
        let credential = fake_codex_credential(home.path());
        let upstream = FakeUpstream::serving();
        let record = Arc::new(Mutex::new(CodexChildRecord::default()));
        let log = ScriptedLog::new();
        let mut runner = CodexDiallingRunner {
            log: log.clone(),
            record: Arc::clone(&record),
        };
        let mut seams = ScriptedSeams;

        let mut run = Metaharness::new(Kind::Codex)
            .with_credentials(CredentialSource::Loopback)
            // Under loopback a declared endpoint is the **proxy's** upstream, not the child's
            // provider, so this is where the fake upstream goes.
            .with_model_endpoint(upstream.base())
            .start_resolved(
                Input::Prompt("go".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
                Some(credential),
            )
            .expect("a codex loopback run starts");

        let child = record.lock().expect("the record").clone();
        let base = assert_codex_child(&child);
        assert!(
            log.credential_copies().is_empty(),
            "a loopback run copies no credential anywhere"
        );
        let placeholder = child.placeholder.clone().expect("a placeholder");
        assert_upstream_saw_codex_custody(&upstream.requests(), &placeholder);

        let port: u16 = base
            .trim_end_matches("/v1")
            .rsplit(':')
            .next()
            .and_then(|text| text.parse().ok())
            .expect("a port");
        assert!(port_accepts(port), "the proxy is up while the run is");

        let lines = run.drain().expect("the run drains");
        assert!(
            lines
                .iter()
                .any(|line| line.event.name() == "session.ended"),
            "the run must complete: {lines:?}"
        );
        assert!(
            port_stops_accepting(port, Duration::from_secs(2)),
            "the run wound up and the loopback port is still accepting; a proxy that outlives its \
             run holds the operator's live credential behind a socket nothing is scoped to"
        );
        let after = run
            .loopback_report()
            .expect("the counters survive the shutdown for the audit that reads them");
        assert!(after.forwarded >= 1, "{after:?}");
        assert_eq!(after.refused, 0, "nothing was answered 401 at the port");
    }

    /// V-LP6's open half at the library seam: a ChatGPT-plan custody is refused **by name**,
    /// nothing is spawned, and the port that was opened to look at the custody does not outlive
    /// the refusal.
    #[test]
    fn a_codex_loopback_run_over_a_subscription_login_is_refused_by_name_and_spawns_nothing() {
        let home = tempfile::TempDir::new().expect("a directory");
        let credential = fake_codex_subscription(home.path());
        let log = ScriptedLog::new();
        let mut runner =
            crate::scripted::ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
        let mut seams = ScriptedSeams;

        let refusal = Metaharness::new(Kind::Codex)
            .with_credentials(CredentialSource::Loopback)
            .start_resolved(
                Input::Prompt("go".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
                Some(credential),
            )
            .expect_err("the subscription half of the codex door is unbuilt");
        let Refusal::Launch { detail } = &refusal else {
            panic!("expected a launch refusal, got {refusal:?}");
        };
        assert!(
            detail.contains("LP-4") && detail.contains("V-LP6"),
            "the refusal must name the milestone and its open verification: {detail}"
        );
        assert!(
            detail.contains("refused by name rather than degraded"),
            "the refusal must say it is not a silent fallback to the copy path: {detail}"
        );
        assert_eq!(log.spawns(), 0, "nothing was spawned");
        assert!(
            log.credential_copies().is_empty(),
            "and nothing was copied either"
        );
    }

    /// A codex loopback run with nothing to put in custody is refused before any spawn, in both
    /// spellings of "nothing": no file named at all, and a named file that is not there.
    #[test]
    fn a_codex_loopback_run_without_a_credential_file_is_refused_before_any_spawn() {
        let empty = tempfile::TempDir::new().expect("a directory");
        let log = ScriptedLog::new();
        let mut seams = ScriptedSeams;

        for named in [None, Some(empty.path().join("auth.json"))] {
            let mut runner =
                crate::scripted::ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
            let refusal = Metaharness::new(Kind::Codex)
                .with_credentials(CredentialSource::Loopback)
                .start_resolved(
                    Input::Prompt("go".to_string()),
                    &mut runner,
                    &mut seams,
                    Box::new(ManualClock::new()),
                    named.clone(),
                )
                .expect_err("a loopback run with no custody is refused");
            let Refusal::Launch { detail } = &refusal else {
                panic!("expected a launch refusal, got {refusal:?}");
            };
            assert!(
                detail.contains("credential file") && detail.contains("custody"),
                "the refusal must name what was missing and why loopback needs it, got: {detail}"
            );
        }
        assert_eq!(
            log.spawns(),
            0,
            "a refused loopback run must not have spawned a child"
        );
    }
}

#[cfg(test)]
mod injection_tests {
    //! Crossing #4 and the capture mode, as the library wires them (R2.5, R2.6).
    //!
    //! These drive **real directories** — the loopback module's vectors mock the world, and the
    //! one thing that cannot be mocked here is the walk itself: the claim is that the digest
    //! describes the bytes on disk and that the copy is the same set the digest was taken over.
    //! No model, no network, no credential; the child is the scripted process.

    use std::path::{Path, PathBuf};

    use metaharness_protocol::{
        DecisionMode, Digest, Frame, Handoff, Kind, NodeRef, Operation, OperationSet,
        PluginContent, StepRef, WorkflowRef,
    };

    use super::{Input, Metaharness, read_plugin_tree};
    use crate::clock::ManualClock;
    use crate::refusal::Refusal;
    use crate::scripted::{ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams};

    /// The one line the scripted child writes, so the run has a terminal record to end on.
    const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

    /// A plugin directory on disk: a manifest and a skill one directory down.
    fn write_plugin(root: &Path, skill_body: &[u8]) -> PathBuf {
        let plugin = root.join("claude-code");
        std::fs::create_dir_all(plugin.join(".claude-plugin")).expect("the manifest directory");
        std::fs::create_dir_all(plugin.join("skills").join("planning")).expect("the skill");
        std::fs::write(
            plugin.join(".claude-plugin").join("plugin.json"),
            br#"{"name":"claude-code"}"#,
        )
        .expect("the manifest");
        std::fs::write(
            plugin.join("skills").join("planning").join("SKILL.md"),
            skill_body,
        )
        .expect("the skill body");
        plugin
    }

    fn digest_of(tree: &PluginContent) -> String {
        match tree {
            PluginContent::Files { digest, .. } => digest.to_string(),
            other => panic!("expected files, got {other:?}"),
        }
    }

    /// The mutation clause, against a real tree: **one edited byte in one plugin file is a
    /// different digest**. A digest a mutation cannot move would pin nothing, and the arm-b
    /// column of an eval matrix would be a plugin identifier that could not tell two plugins
    /// apart.
    #[test]
    fn one_edited_byte_in_one_plugin_file_is_a_different_digest() {
        let home = tempfile::TempDir::new().expect("a directory");
        let plugin = write_plugin(home.path(), b"classify the request, then route it");
        let before = read_plugin_tree(&plugin);

        std::fs::write(
            plugin.join("skills").join("planning").join("SKILL.md"),
            b"classify the request, then route it.",
        )
        .expect("the edit");
        let after = read_plugin_tree(&plugin);

        assert_ne!(
            digest_of(&before),
            digest_of(&after),
            "an edited plugin file must not digest to what it digested before"
        );
        assert_eq!(digest_of(&before).len(), 64);
    }

    /// The whole crossing, end to end and free: a declared plugin is digested, copied into the
    /// run's own scratch tree, named to the vendor as **the copy**, and reported in the
    /// attestation with the digest of what was read.
    #[test]
    fn a_declared_plugin_is_copied_into_the_scratch_and_the_child_is_pointed_at_the_copy() {
        let home = tempfile::TempDir::new().expect("a directory");
        let plugin = write_plugin(home.path(), b"classify the request, then route it");
        let expected = digest_of(&read_plugin_tree(&plugin));

        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
        let mut seams = ScriptedSeams;
        let mut run = Metaharness::new(Kind::Claude)
            .with_plugin_dir(&plugin)
            .start_with_clock(
                Input::Prompt("inject".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
            )
            .expect("the injected run starts");

        let scratch = run
            .scratch_root()
            .expect("a scripted run owns a scratch")
            .to_path_buf();
        let installed = scratch.join("plugins").join("claude-code");
        assert_eq!(
            std::fs::read(installed.join(".claude-plugin").join("plugin.json"))
                .expect("the manifest travelled"),
            br#"{"name":"claude-code"}"#
        );
        assert_eq!(
            std::fs::read(installed.join("skills").join("planning").join("SKILL.md"))
                .expect("the skill travelled"),
            b"classify the request, then route it"
        );
        assert_eq!(
            digest_of(&read_plugin_tree(&installed)),
            expected,
            "the copy must digest to what the source digested, or the attestation cites a \
             number that describes something else"
        );
        assert!(
            !installed.starts_with(scratch.join("claude-home")),
            "the copy stays out of the config home the vendor keeps its own plugin bookkeeping in"
        );

        let launched = log.launched();
        let argv = launched.first().expect("the child was started");
        let named = argv
            .windows(2)
            .find(|pair| pair[0] == "--plugin-dir")
            .expect("--plugin-dir reached the child");
        assert_eq!(named[1], installed.display().to_string());

        let lines = run.drain().expect("the run drains");
        assert!(
            lines
                .iter()
                .any(|line| line.event.name() == "session.ended"),
            "{lines:?}"
        );
    }

    /// A `--plugin-dir` that is not there is refused **before any spawn**, by name — exit `2`,
    /// and no money spent finding out.
    #[test]
    fn a_plugin_directory_that_is_not_there_is_refused_before_any_spawn() {
        let home = tempfile::TempDir::new().expect("a directory");
        let empty = home.path().join("built-by-nobody");
        std::fs::create_dir_all(&empty).expect("an empty directory");

        for (directory, expected) in [
            (home.path().join("not-there"), "could not be read"),
            (empty, "holds no file at all"),
        ] {
            let log = ScriptedLog::new();
            let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
            let mut seams = ScriptedSeams;
            let refusal = Metaharness::new(Kind::Claude)
                .with_plugin_dir(&directory)
                .start_with_clock(
                    Input::Prompt("inject".to_string()),
                    &mut runner,
                    &mut seams,
                    Box::new(ManualClock::new()),
                )
                .expect_err("an unusable plugin directory is refused");
            let Refusal::Launch { detail } = &refusal else {
                panic!("expected a launch refusal, got {refusal:?}");
            };
            assert!(
                detail.contains("--plugin-dir") && detail.contains(expected),
                "the refusal must name the flag and what was wrong: {detail}"
            );
            assert_eq!(log.spawns(), 0, "a refused injection must spawn nothing");
        }
    }

    /// Observe mode beside a frame is refused by name: the frame's text would reach the model as
    /// *"strictly only these operations"* while nothing enforced it (finding F9).
    #[test]
    fn observe_mode_beside_a_frame_is_refused_by_name() {
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::new(vec![ScriptStep::line(END)], log.clone());
        let mut seams = ScriptedSeams;
        let frame = Frame {
            workflow: WorkflowRef {
                id: "development/default".into(),
                version: "1".into(),
            },
            node: NodeRef {
                id: "implement".into(),
            },
            step: StepRef {
                workflow: "development/default".into(),
                state: "implement".into(),
                index: 1,
                attempt: 1,
            },
            prior: Vec::new(),
            obligations: Vec::new(),
            reaching: Vec::new(),
            next: Vec::new(),
            handoff: Handoff::None,
            operations: OperationSet::of([Operation::FileRead]),
            entities: None,
            digest: Digest::of(b""),
        };

        let refusal = Metaharness::new(Kind::Claude)
            .with_decisions(DecisionMode::Observe)
            .with_frame(frame)
            .start_with_clock(
                Input::Prompt("observe".to_string()),
                &mut runner,
                &mut seams,
                Box::new(ManualClock::new()),
            )
            .expect_err("the composition is refused");
        assert!(
            matches!(refusal, Refusal::ObserveWithFrame),
            "got {refusal:?}"
        );
        assert!(refusal.to_string().contains("F9"), "{refusal}");
        assert_eq!(log.spawns(), 0, "nothing was spawned");
    }
}

#[cfg(test)]
mod b10x_launch_tests {
    //! The argv arm `native` is actually spawned with.
    //!
    //! These are here rather than beside `metaharness_b10x::argv` because the bug they exist for
    //! was not in that function: it was that nothing called it, and the adapter's own tests went
    //! on passing while every launch died on the loop's argument parsing.

    use metaharness_protocol::{CredentialSource, Kind, RunSpec};

    use super::{Refusal, b10x_launch, check_spec};

    fn spec() -> RunSpec {
        let mut spec = RunSpec::new(Kind::B10x);
        spec.model_endpoint = Some("https://gw.example/v1".to_owned());
        spec.model = Some("gpt-5.6-sol".to_owned());
        spec.credentials = CredentialSource::None;
        spec.prompt = Some("do the thing".to_owned());
        spec
    }

    fn argv_of(spec: &RunSpec) -> Vec<String> {
        metaharness_b10x::argv(&b10x_launch(spec, std::path::Path::new("/work")).expect("planned"))
    }

    fn value_after(argv: &[String], flag: &str) -> Option<String> {
        argv.iter()
            .position(|word| word == flag)
            .and_then(|at| argv.get(at + 1))
            .cloned()
    }

    #[test]
    fn the_launch_names_the_endpoint_the_model_and_the_tree_it_works_in() {
        // The regression this file exists for: the argv used to be `run --json --input <prompt>`,
        // which `b10x-harness` rejects for missing required arguments before it reaches a model.
        let argv = argv_of(&spec());
        assert_eq!(argv[0], "b10x-harness");
        assert_eq!(argv[1], "run");
        assert_eq!(
            value_after(&argv, "--base-url"),
            Some("https://gw.example/v1".to_owned())
        );
        assert_eq!(
            value_after(&argv, "--model"),
            Some("gpt-5.6-sol".to_owned())
        );
        assert_eq!(value_after(&argv, "--workspace"), Some("/work".to_owned()));
        assert!(argv.contains(&"--json".to_owned()), "{argv:?}");
    }

    #[test]
    fn a_rate_card_reaches_the_loop_so_the_arm_states_what_it_cost() {
        let mut spec = spec();
        spec.prices = Some("/etc/rates.json".into());
        assert_eq!(
            value_after(&argv_of(&spec), "--prices"),
            Some("/etc/rates.json".to_owned())
        );
        assert!(
            !argv_of(&self::spec()).iter().any(|word| word == "--prices"),
            "and a run nobody priced names no card"
        );
    }

    #[test]
    fn no_credential_declared_means_no_credential_flag() {
        let argv = argv_of(&spec());
        assert!(
            !argv.iter().any(|word| word.starts_with("--api-key")),
            "{argv:?}"
        );
    }

    #[test]
    fn an_api_key_run_is_pointed_at_a_variables_name_and_never_its_value() {
        // metaharness passes a name into an argv; the secret never enters this process.
        let mut spec = spec();
        spec.credentials = CredentialSource::ApiKey;
        assert_eq!(
            value_after(&argv_of(&spec), "--api-key-env"),
            Some(super::B10X_API_KEY_VARIABLE.to_owned())
        );
    }

    #[test]
    fn the_default_credential_source_is_refused_by_name_rather_than_launching_without_one() {
        // `operator-login` is the flag's default and names nothing here: there is no `~/.b10x` to
        // copy. Accepting it would start a run with no credential under a flag claiming one.
        let mut spec = spec();
        spec.credentials = CredentialSource::OperatorLogin;
        let refusal = b10x_launch(&spec, std::path::Path::new("/work")).expect_err("refused");
        let said = refusal.to_string();
        assert!(said.contains("--credentials none"), "{said}");
        assert!(said.contains("--credentials api-key"), "{said}");
    }

    #[test]
    fn a_missing_endpoint_or_model_is_refused_rather_than_defaulted() {
        // A default endpoint would aim an evaluation arm at somebody's production API the first
        // time the flag was forgotten.
        let mut without_endpoint = spec();
        without_endpoint.model_endpoint = None;
        let said = b10x_launch(&without_endpoint, std::path::Path::new("/work"))
            .expect_err("refused")
            .to_string();
        assert!(said.contains("--model-endpoint"), "{said}");

        let mut without_model = spec();
        without_model.model = None;
        let said = b10x_launch(&without_model, std::path::Path::new("/work"))
            .expect_err("refused")
            .to_string();
        assert!(said.contains("--model"), "{said}");
    }

    #[test]
    fn a_confined_run_names_the_socket_the_programs_and_the_cgroup_subtree() {
        // Without these the catalogue behind the three verbs is read-only, so the arm cannot
        // attempt a task that changes a file — and nothing in the record says why.
        let mut spec = spec();
        spec.substrate = Some("/run/substrate.sock".into());
        spec.cgroup_root = Some("/sys/fs/cgroup/run.slice".into());
        spec.allow_program = vec!["/usr/bin/python3".to_owned()];

        let argv = argv_of(&spec);
        assert_eq!(
            value_after(&argv, "--substrate"),
            Some("/run/substrate.sock".to_owned())
        );
        assert_eq!(
            value_after(&argv, "--cgroup-root"),
            Some("/sys/fs/cgroup/run.slice".to_owned())
        );
        assert_eq!(
            value_after(&argv, "--allow-program"),
            Some("/usr/bin/python3".to_owned()),
            "the program set is what makes a `run` entry appear at all"
        );
    }

    #[test]
    fn an_embedded_run_serves_the_working_directorys_parent() {
        // The workspace is adopted rather than created, so the root is the tree above it. Naming
        // the workspace itself would ask substrate to represent a tree it is inside.
        let mut spec = spec();
        spec.substrate_embedded = true;
        let launch = b10x_launch(&spec, std::path::Path::new("/scratch/ws_run")).expect("planned");
        assert_eq!(
            value_after(&metaharness_b10x::argv(&launch), "--substrate-embedded"),
            Some("/scratch".to_owned())
        );
    }

    #[test]
    fn an_unconfined_run_names_no_socket_and_gets_a_read_only_catalogue() {
        let argv = argv_of(&spec());
        for flag in ["--substrate", "--substrate-embedded", "--cgroup-root"] {
            assert!(!argv.iter().any(|word| word == flag), "{flag}: {argv:?}");
        }
    }

    #[test]
    fn a_working_directory_substrate_cannot_adopt_is_refused_rather_than_left_read_only() {
        // The degradation this refuses is invisible: substrate declines the workspace, the write
        // entries never appear, and the run reports that it could not change the file it was asked
        // to change. That reads as a model failure and is a directory naming rule.
        assert!(super::adoptable(std::path::Path::new("/scratch/ws_run")));
        assert!(!super::adoptable(std::path::Path::new("/scratch/work")));
    }

    #[test]
    fn confinement_is_refused_for_a_kind_whose_tools_are_not_ours() {
        // A socket configured for Claude Code would be accepted and never consulted: it reaches
        // the filesystem through its own tools, so this would read as containment nobody applied.
        for kind in [Kind::Claude, Kind::Codex] {
            let mut spec = RunSpec::new(kind);
            spec.substrate = Some("/run/substrate.sock".into());
            let refusal = check_spec(&spec).expect_err("refused");
            assert!(
                matches!(refusal, Refusal::ConfinementUnsupported { .. }),
                "{kind:?}"
            );
            assert!(refusal.to_string().contains("never consulted"));
        }
        let mut confined = RunSpec::new(Kind::B10x);
        confined.substrate_embedded = true;
        assert!(check_spec(&confined).is_ok(), "b10x is the one that takes it");
    }

    #[test]
    fn a_rate_card_is_refused_for_a_kind_that_prices_its_own_runs() {
        // Claude Code and codex read rates from a catalogue their service delivers. A card handed
        // to one could only be ignored, and the operator would believe their rates were in force.
        for kind in [Kind::Claude, Kind::Codex] {
            let mut spec = RunSpec::new(kind);
            spec.prices = Some("/etc/rates.json".into());
            let refusal = check_spec(&spec).expect_err("refused");
            assert!(
                matches!(refusal, Refusal::PricesUnsupported { .. }),
                "{kind:?}"
            );
            assert!(refusal.to_string().contains("prices its own runs"));
        }
        let mut priced = RunSpec::new(Kind::B10x);
        priced.prices = Some("/etc/rates.json".into());
        assert!(
            check_spec(&priced).is_ok(),
            "b10x is the one that takes one"
        );
    }
}
