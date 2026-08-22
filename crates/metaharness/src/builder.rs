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
        self.start_with(
            input,
            &mut crate::spawn::SpawnRunner::new(),
            &mut metaharness_claude::ClaudeSeams,
        )
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
            scratch: Some(scratch),
        }))
    }

    fn applied(mut self, input: Input) -> RunSpec {
        if let Input::Prompt(prompt) = input {
            self.spec.prompt = Some(prompt);
        }
        self.spec
    }
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
    if spec.kind == Kind::Codex {
        // CX-M1: the codex adapter reads rollouts and declares its capabilities; nothing spawns.
        return Err(Refusal::NotInThisMilestone {
            verb: "run codex",
            missing: "a driven codex spawn (CX-M2): the adapter's rollout reader and \
                      capabilities exist, and no live codex process has answered the seam yet",
        });
    }
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
fn credentials_file(spec: &RunSpec) -> Option<PathBuf> {
    match spec.credentials {
        CredentialSource::OperatorLogin => std::env::var_os("HOME").map(|home| {
            PathBuf::from(home)
                .join(".claude")
                .join(".credentials.json")
        }),
        CredentialSource::ApiKey | CredentialSource::None => None,
    }
}

/// Every memory file discoverable above the scratch working directory (design § 8.1 H11).
///
/// Walked and handed to the adapter rather than checked here, because H11's verdict is the
/// adapter's launch assertion: auto-discovery is on in every run that is not `--bare`, and H8
/// forbids `--bare`, so a `CLAUDE.md` in any ancestor enters the context of a run this design
/// calls hermetic.
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
