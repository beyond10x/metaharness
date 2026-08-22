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
//! library's job and parsing a frame document in the binary would be protocol logic in the CLI —
//! and the on-disk format is owed and not in v0.1, so that path is refused rather than shipped
//! against something undefined (design § 9.3, correction 3).

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
    /// Not a spec field: the spec's `frame` is a path and the on-disk format does not exist in
    /// v0.1. A frame given here is sealed on the way in, so the digest an event cites always
    /// describes the contents that were actually in force.
    #[must_use]
    pub fn with_frame(mut self, frame: Frame) -> Self {
        self.frame = Some(frame.seal());
        self
    }

    /// A frame **document**, which [`Metaharness::start`] refuses.
    ///
    /// Present so the builder and the CLI have the same field set, and so the refusal is the
    /// same one on both faces rather than a flag the library silently lacks.
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
    /// # Errors
    ///
    /// Always [`Refusal::NoSpawner`] in this build, after every other refusal has been checked
    /// so the caller learns about a bad spec before it learns about the missing spawner. The
    /// refusal is a tested behaviour rather than a `todo!()`, because a panic here would look
    /// like a crash and exit `2` is what "metaharness could not do its job" means.
    pub fn start(self, input: Input) -> Result<Run, Refusal> {
        let spec = self.applied(input);
        check_spec(&spec)?;
        Err(Refusal::NoSpawner)
    }

    /// Start against a runner the caller supplies, with the real clock.
    ///
    /// # Errors
    ///
    /// Every refusal in [`Refusal`] that a start can raise: an unknown kind, `--frame`, an owned
    /// tool surface, a control the adapter cannot honour, and whatever the adapter said when it
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

        let capabilities = metaharness_claude::capabilities();
        let refusals = start_refusals(&capabilities, &spec);
        if !refusals.is_empty() {
            return Err(Refusal::Control { refusals });
        }

        let scratch = tempfile::TempDir::new()?;
        let cwd = scratch.path().join("work");
        std::fs::create_dir_all(&cwd)?;
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

/// Everything a spec can be refused for, in one place, so both faces refuse identically.
///
/// # Errors
///
/// [`Refusal::NoAdapter`], [`Refusal::FrameFile`] or [`Refusal::ToolSurfaceOwned`].
pub fn check_spec(spec: &RunSpec) -> Result<(), Refusal> {
    if spec.kind == Kind::Codex {
        return Err(Refusal::NoAdapter {
            kind: Kind::Codex.as_str().to_string(),
        });
    }
    if let Some(path) = &spec.frame {
        return Err(Refusal::FrameFile { path: path.clone() });
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
