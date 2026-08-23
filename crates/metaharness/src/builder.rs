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
use std::path::PathBuf;

use metaharness_protocol::{
    Capabilities, CredentialSource, DecisionMode, EventStream, Frame, HermeticMode, Kind, Refused,
    RunId, RunSpec, Seam, ToolSurface, TranscriptRef,
};

use crate::clock::{Clock, SystemClock};
use crate::process::{CredentialCopyView, LaunchPlanView, ProcessRunner};
use crate::refusal::Refusal;
use crate::run::{LaunchFacts, Run, RunParts, vendor_hook_timeout_ms};
use metaharness_protocol::SeamFactory;

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

    /// A ceiling on turns.
    #[must_use]
    pub fn with_max_turns(mut self, turns: u32) -> Self {
        self.spec.max_turns = Some(turns);
        self
    }

    /// One more plugin directory to load, and only these.
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
        let frame = self.frame.clone();
        let spec = self.applied(input);
        check_spec(&spec)?;
        let frame = resolve_frame(frame, &spec)?;

        // One `match` and no trait. The two adapters' launch plans are different types with
        // different fields — one carries a settings document and a `--settings` path, the other a
        // `hooks.json` and a `config.toml` — and a trait to unify them would be an abstraction
        // invented for two implementations. A third adapter is when it earns its keep. What is
        // shared is what is genuinely neutral and already factored: the spec check, the frame
        // resolution, the scratch root, the cwd, the ancestor walk and the control refusals.
        match spec.kind {
            Kind::Claude => start_claude(spec, frame, runner, seams, clock),
            Kind::Codex => start_codex(spec, frame, runner, seams, clock),
        }
    }

    fn applied(mut self, input: Input) -> RunSpec {
        if let Input::Prompt(prompt) = input {
            self.spec.prompt = Some(prompt);
        }
        self.spec
    }
}

/// Start a Claude Code run: the M2 path, unchanged.
fn start_claude(
    spec: RunSpec,
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

        let context = metaharness_claude::LaunchContext {
            scratch_root: scratch.path().to_path_buf(),
            cwd: cwd.clone(),
            credentials_file: credentials_file(&spec),
            inherited_env: std::env::vars().collect::<BTreeMap<String, String>>(),
            memory_ancestors: memory_ancestors(&cwd),
            inputs_digest: None,
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
            declared_plugins: spec
                .plugin_dir
                .iter()
                .filter_map(|directory| {
                    directory
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                })
                .collect(),
            pinned_versions: metaharness_claude::PINNED_VERSIONS
                .iter()
                .map(ToString::to_string)
                .collect(),
            transcript,
        };

        Ok(Run::new(RunParts {
            stream: EventStream::new(RunId::new(format!(
                "{}-{}",
                spec.kind.as_str(),
                std::process::id()
            ))),
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
        }))
    }
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
fn start_codex(
    spec: RunSpec,
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

    let context = metaharness_codex::LaunchContext {
        scratch_root: scratch.path().to_path_buf(),
        cwd: cwd.clone(),
        credentials_file: credentials_file(&spec),
        inherited_env: std::env::vars().collect::<BTreeMap<String, String>>(),
        memory_ancestors: memory_ancestors(&cwd),
        inputs_digest: None,
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
        // codex loads plugins from its own config and marketplace snapshots, and a scratch home
        // has neither — the launch plan refuses `--plugin-dir` outright rather than pretending.
        declared_plugins: Vec::new(),
        pinned_versions: metaharness_codex::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
        transcript,
    };

    Ok(Run::new(RunParts {
        stream: EventStream::new(RunId::new(format!(
            "{}-{}",
            spec.kind.as_str(),
            std::process::id()
        ))),
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
    match &spec.cwd {
        Some(directory) if directory.is_dir() => Ok(directory.clone()),
        Some(directory) => Err(Refusal::Io {
            detail: format!(
                "the operator-named working directory {} does not exist or is not a directory",
                directory.display()
            ),
        }),
        None => {
            let work = scratch_root.join("work");
            std::fs::create_dir_all(&work)?;
            Ok(work)
        }
    }
}

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
    if spec.tool_surface == ToolSurface::Owned {
        return Err(Refusal::ToolSurfaceOwned);
    }
    Ok(())
}

/// The operator's credential file, **named and not read**.
///
/// Whether it exists is the spawn's problem, because the copy happens immediately before every
/// spawn and not once per run: a copied operator-login token is a snapshot with a lifetime, and
/// a governed run on 2026-08-22 died an hour in on an OAuth session that could not be refreshed
/// (Q13).
///
/// Each vendor keeps its login in its own place and under its own name — `~/.claude/.credentials.json`,
/// `~/.codex/auth.json` — and this is the one line that knows both, because the copy is the
/// library's I/O and the adapters are pure.
fn credentials_file(spec: &RunSpec) -> Option<PathBuf> {
    let (directory, file) = match spec.kind {
        Kind::Claude => (".claude", ".credentials.json"),
        Kind::Codex => (".codex", "auth.json"),
    };
    match spec.credentials {
        CredentialSource::OperatorLogin => {
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
