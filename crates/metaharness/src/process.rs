//! The spawn seam.
//!
//! Everything between "metaharness decided what to launch" and "a line came back" is behind
//! this trait pair. It began as the seam that let M1 exercise the whole
//! spec → plan → transcript → events → audit path with no vendor binary; it stays because the
//! two implementations are worth having side by side. [`crate::SpawnRunner`] starts the real
//! process, [`crate::ScriptedRunner`] plays a recorded one, and **every C3 control vector runs
//! against the second** — which is what keeps the safety argument free of a model, a network and
//! a credential.
//!
//! The view is borrowed and carries no adapter type, so a second adapter's plan can be launched
//! by the same runner without this trait learning either adapter's name.

use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

use metaharness_protocol::{
    EnvelopeAssessment, HermeticMode, ProcessEnvelopeMeasurement, SealedProcessEnvelope,
    assess_envelope,
};

/// One credential file the runner must copy before it spawns.
///
/// **Copied immediately before every spawn, never once per run.** A copied operator-login token
/// is a snapshot with a lifetime: a governed run on 2026-08-22 died an hour in on an OAuth
/// session that could not be refreshed, and a file copied at run start cannot refresh itself.
/// Sharing the live file by hardlink is not taken here and is the open half of Q13.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CredentialCopyView<'a> {
    /// The operator's own file.
    pub from: &'a Path,
    /// Where it goes inside the scratch config home.
    pub to: &'a Path,
}

/// What a runner needs to start the planned child, borrowed from the adapter's plan.
///
/// A borrowed view rather than the adapter's own plan type, so [`ProcessRunner`] does not depend
/// on which adapter produced it.
#[derive(Debug, Clone, Copy)]
pub struct LaunchPlanView<'a> {
    /// The program to run.
    pub program: &'a str,
    /// Its arguments, in order, exactly as the adapter constructed them.
    pub args: &'a [String],
    /// The child's whole environment. Constructed, not inherited — everything absent here was
    /// dropped on purpose (design § 8.1 H3).
    pub env: &'a BTreeMap<String, String>,
    /// The working directory, which is a directory metaharness created (H7).
    pub cwd: &'a Path,
    /// The credential copies to perform **at this spawn**.
    pub credential_copies: &'a [CredentialCopyView<'a>],
    /// Where the child's control seam meets metaharness.
    ///
    /// Named here rather than derived from [`LaunchPlanView::cwd`] because a runner that
    /// reconstructed it by walking up from the working directory would be a second place that
    /// decides where the channel lives, and the two could disagree by one directory without
    /// anything failing loudly — the seam would simply never be consulted, which is the failure
    /// § 7.8 exists for.
    ///
    /// The layout under it is the adapter's; nothing in this crate reads its contents through
    /// this field.
    pub decision_channel: &'a Path,
    /// Where the raw vendor bytes are retained as they are read (design § 8.4 O8).
    ///
    /// The runner writes them, because it is the only party that sees the stream: three things
    /// depend on the file existing and none of them works without it — `transcript_digest` and
    /// `source_line` in the projection, the § 4.4 cross-check, and § 9.4's auditor, which reads
    /// a transcript and not an event stream.
    pub transcript: &'a Path,
}

/// Start the planned child.
pub trait ProcessRunner {
    /// Start the planned child and give back its line stream and its stdin.
    ///
    /// An implementation performs [`LaunchPlanView::credential_copies`] here — at the spawn, not
    /// at run start.
    ///
    /// # Errors
    ///
    /// Whatever the platform said. A refusal to spawn is exit `2`: metaharness could not do its
    /// job.
    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>>;
}

/// A confinement provider supplied by the embedder.
///
/// The provider implements the mechanism outside this repository. It receives the same launch
/// plan as a plain runner plus the immutable envelope request, and returns measurements taken at
/// the child boundary.
pub trait ProcessEnvelope {
    /// Start the child inside the sealed envelope.
    ///
    /// # Errors
    ///
    /// Whatever the platform or confinement provider said.
    fn start(
        &mut self,
        envelope: &SealedProcessEnvelope,
        plan: &LaunchPlanView,
    ) -> std::io::Result<EnvelopeLaunch>;
}

/// The child and facts returned by a [`ProcessEnvelope`] provider.
pub struct EnvelopeLaunch {
    /// The started child.
    pub process: Box<dyn HarnessProcess>,
    /// Facts measured at the child boundary. `None` means nobody found out.
    pub measurement: Option<ProcessEnvelopeMeasurement>,
}

/// A child admitted after its envelope evidence was assessed.
pub struct EnvelopeStarted {
    /// The started child.
    pub process: Box<dyn HarnessProcess>,
    /// The comparison retained for the run record and audit.
    pub assessment: EnvelopeAssessment,
}

/// Why an enveloped child was not admitted.
#[derive(Debug)]
pub enum EnvelopeStartError {
    /// The sealed request was changed after sealing.
    InvalidSeal,
    /// Strict mode requires a complete matching measurement.
    StrictEvidence {
        /// The mismatch or absence that caused the refusal.
        assessment: EnvelopeAssessment,
    },
    /// The confinement provider could not start the child.
    Io(std::io::Error),
}

impl fmt::Display for EnvelopeStartError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EnvelopeStartError::InvalidSeal => {
                formatter.write_str("process envelope digest does not match its request")
            }
            EnvelopeStartError::StrictEvidence { assessment } => write!(
                formatter,
                "strict process envelope refused non-matching evidence: {assessment:?}"
            ),
            EnvelopeStartError::Io(error) => write!(formatter, "process envelope failed: {error}"),
        }
    }
}

impl std::error::Error for EnvelopeStartError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EnvelopeStartError::Io(error) => Some(error),
            EnvelopeStartError::InvalidSeal | EnvelopeStartError::StrictEvidence { .. } => None,
        }
    }
}

impl From<std::io::Error> for EnvelopeStartError {
    fn from(error: std::io::Error) -> Self {
        EnvelopeStartError::Io(error)
    }
}

/// Start through an envelope and apply the strict evidence rule in one place.
///
/// A strict run kills a child whose measurements are absent or disagree before returning a named
/// refusal. Other modes retain the gap or unknown assessment for the audit.
///
/// # Errors
///
/// Refuses an invalid seal, a provider failure, or non-matching evidence in strict mode.
pub fn start_in_envelope(
    port: &mut dyn ProcessEnvelope,
    envelope: &SealedProcessEnvelope,
    plan: &LaunchPlanView,
    mode: HermeticMode,
) -> Result<EnvelopeStarted, EnvelopeStartError> {
    if !envelope.digest_intact() {
        return Err(EnvelopeStartError::InvalidSeal);
    }
    let mut launch = port.start(envelope, plan)?;
    let assessment = assess_envelope(envelope, launch.measurement.as_ref());
    if mode == HermeticMode::Strict && assessment != EnvelopeAssessment::Matched {
        launch.process.kill()?;
        return Err(EnvelopeStartError::StrictEvidence { assessment });
    }
    Ok(EnvelopeStarted {
        process: launch.process,
        assessment,
    })
}

/// A started child, as a line stream and a line sink.
pub trait HarnessProcess {
    /// The next line the child wrote, or `None` at end of stream.
    ///
    /// **`None` means the stream ended and nothing else.** A child that is blocked waiting on a
    /// decision metaharness has not yet written returns
    /// [`std::io::ErrorKind::WouldBlock`] instead, because the two are different facts and a
    /// reader that conflated them would end the run every time the seam did its job.
    ///
    /// # Errors
    ///
    /// Whatever the platform said, plus `WouldBlock` for the case above.
    fn next_line(&mut self) -> std::io::Result<Option<String>>;

    /// Write one line to the child.
    ///
    /// # Errors
    ///
    /// Whatever the platform said.
    fn write_line(&mut self, line: &str) -> std::io::Result<()>;

    /// Stop the child.
    ///
    /// # Errors
    ///
    /// Whatever the platform said.
    fn kill(&mut self) -> std::io::Result<()>;

    /// Wait for the child and report its exit code, or `None` when a signal took it.
    ///
    /// # Errors
    ///
    /// Whatever the platform said.
    fn wait(&mut self) -> std::io::Result<Option<i32>>;

    /// Everything the child wrote to stderr, where the runner retained it.
    ///
    /// **The only thing that says why a run produced no records at all.** A child that dies on its
    /// own argument parsing writes one sentence here and nothing to stdout, so without this the
    /// run ends at exit 3 — *nobody found out* — with both streams empty and no way to tell a
    /// vendor that refused a flag from one that was never installed. That is exactly what a b10x
    /// launch did, and the stderr the spawner had been retaining all along had no reader.
    ///
    /// Empty by default: a scripted runner has no child and inventing one would be a claim.
    fn stderr(&self) -> String {
        String::new()
    }
}

/// Perform the plan's credential copies.
///
/// Offered so a runner does not reimplement the one rule that matters: the parent directory is
/// created first, because the scratch config home is fresh at every spawn and a copy into a
/// directory that does not exist is the failure that looks like "no credentials".
///
/// # Errors
///
/// The first copy that failed, naming the file.
pub fn copy_credentials(copies: &[CredentialCopyView<'_>]) -> std::io::Result<()> {
    for copy in copies {
        if let Some(parent) = copy.to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(copy.from, copy.to)?;
    }
    Ok(())
}
