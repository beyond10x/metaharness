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
    /// Whether this runner starts a real executable and needs installation checks.
    ///
    /// Defaults to true for every real or custom runner. Synthetic replay explicitly
    /// opts out; it must not report an observed vendor version without a process.
    fn requires_executable(&self) -> bool {
        true
    }

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

/// One bounded process-read observation. Silence is not EOF.
#[derive(Debug, PartialEq, Eq)]
pub enum ProcessPoll {
    /// A complete record arrived.
    Line(String),
    /// No record arrived during this poll; the stream remains open.
    Idle,
    /// The record stream ended.
    Ended,
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

    /// Poll once, retaining `WouldBlock` for an unanswered decision.
    ///
    /// Native runners bound this to one receive interval. The compatibility
    /// default may block: custom runners needing responsive steering must
    /// override it as well as implementing `next_line`.
    ///
    /// # Errors
    /// As [`HarnessProcess::next_line`].
    fn poll_line(&mut self) -> std::io::Result<ProcessPoll> {
        self.next_line()
            .map(|line| line.map_or(ProcessPoll::Ended, ProcessPoll::Line))
    }

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

    /// Measured native termination after waiting. Replay/custom runners default to unknown.
    /// Implementations must retain the actual OS status, not infer it from terminal output.
    fn termination(&self) -> metaharness_protocol::ProcessTermination {
        metaharness_protocol::ProcessTermination::Unknown
    }

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

    /// The process group the child leads, when it was started in one of its own (a named run,
    /// design amendment a72, review corrections pass 2).
    ///
    /// Such a group does not receive a terminal's `SIGINT`, so an embedder that ends on a signal
    /// forwards it through [`ProcessGroup::signal`], which sends nothing once the leader has been
    /// reaped (review corrections pass 3). `None` for a child in the embedder's own group, and by
    /// default.
    fn process_group(&self) -> Option<ProcessGroup> {
        None
    }
}

/// The process group a named run's vendor child leads, shared between the process that reaps the
/// child and whoever signals the group (design amendment a72 § 5, review corrections pass 3).
///
/// A group id is a pid. While the leader has not been reaped (running, or a zombie), it holds that
/// id and the group can only be this one. Once the leader is reaped and the group is empty, the
/// kernel can hand the id to a later process that leads a group of its own, and a signal sent to
/// the old id would reach that process instead. So the group is signalled only while the leader
/// is unreaped: the reaping takes the same lock, kills the group, reaps the leader and forgets
/// the id, and a signal sent after that finds no group and sends nothing.
///
/// Cloned freely and `Send`; an unnamed run has none.
#[derive(Clone, Debug, Default)]
pub struct ProcessGroup(std::sync::Arc<std::sync::Mutex<Option<i32>>>);

impl ProcessGroup {
    /// A group led by the child `pid`, or none when `own` is false.
    pub(crate) fn led_by(pid: u32, own: bool) -> Self {
        let id = if own { i32::try_from(pid).ok() } else { None };
        Self(std::sync::Arc::new(std::sync::Mutex::new(id)))
    }

    fn held(&self) -> std::sync::MutexGuard<'_, Option<i32>> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    /// The group id, while its leader has not been reaped.
    #[must_use]
    pub fn id(&self) -> Option<i32> {
        *self.held()
    }

    /// Send the signal numbered `signal` to the whole group, while its leader has not been
    /// reaped. Nothing is sent once the leader was reaped or for a number that is not a signal;
    /// a group whose members are all gone is no error.
    pub fn signal(&self, signal: i32) {
        let held = self.held();
        if let (Some(group), Some(signal)) = (
            held.and_then(rustix::process::Pid::from_raw),
            rustix::process::Signal::from_named_raw(signal),
        ) {
            let _ = rustix::process::kill_process_group(group, signal);
        }
    }
}

/// Whether the child `pid` has exited and is still waitable, without reaping it or waiting.
/// `None` when it is no longer this process's to wait for (already reaped).
fn exited_unreaped(pid: rustix::process::Pid, block: bool) -> std::io::Result<Option<bool>> {
    use rustix::process::{WaitId, WaitIdOptions};

    let mut options = WaitIdOptions::EXITED | WaitIdOptions::NOWAIT;
    if !block {
        options |= WaitIdOptions::NOHANG;
    }
    loop {
        match rustix::process::waitid(WaitId::Pid(pid), options) {
            Ok(found) => return Ok(Some(found.is_some())),
            Err(rustix::io::Errno::INTR) => {}
            Err(rustix::io::Errno::CHILD) => return Ok(None),
            Err(errno) => return Err(errno.into()),
        }
    }
}

/// Reap `child` if it has exited, without waiting: `Child::try_wait`, except that a leader of its
/// own group has the group sent `SIGKILL` first, while its zombie still holds the group id, so
/// nothing it started outlives it, and the id is forgotten with the reap ([`ProcessGroup`]).
pub(crate) fn try_reap(
    child: &mut std::process::Child,
    group: &ProcessGroup,
) -> std::io::Result<Option<std::process::ExitStatus>> {
    let mut held = group.held();
    if let Some(id) = *held
        && let Some(pid) = rustix::process::Pid::from_raw(id)
    {
        match exited_unreaped(pid, false)? {
            Some(false) => return Ok(None),
            Some(true) => {
                let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
            }
            None => {}
        }
    }
    let status = child.try_wait()?;
    if status.is_some() {
        *held = None;
    }
    Ok(status)
}

/// Wait for `child` and reap it: `Child::wait`, with the group of a leader of its own sent
/// `SIGKILL` between its exit and its reap, as [`try_reap`] does. The lock is not held while the
/// leader runs, so the group can still be signalled then.
pub(crate) fn reap(
    child: &mut std::process::Child,
    group: &ProcessGroup,
) -> std::io::Result<std::process::ExitStatus> {
    let id = group.id();
    if let Some(pid) = id.and_then(rustix::process::Pid::from_raw) {
        exited_unreaped(pid, true)?;
    }
    let mut held = group.held();
    if let Some(pid) = held.and_then(rustix::process::Pid::from_raw)
        && exited_unreaped(pid, false)? == Some(true)
    {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
    let status = child.wait()?;
    *held = None;
    Ok(status)
}

/// Kill and reap a spawned child nobody has reaped, as its process is dropped (design amendment
/// a72, review corrections).
///
/// A `HarnessProcess` that owns a `std::process::Child` calls this from its `Drop`, and `Run`
/// declares its process before its scratch, so a run dropped before its child ended leaves no
/// child writing by path into a root that is removed next, or, under a scratch name, into the
/// next run's root. `reaped` is whether a `wait` already took the status.
///
/// A child that leads a group of its own (a named run, a72 review corrections pass 2) has the
/// whole group sent `SIGKILL` first, because what the vendor started (a hook, a tool command, an
/// MCP server) is in that group and may still run. Only while the leader is unreaped: a leader
/// already reaped no longer holds the id, which may lead somebody else's group by now (review
/// corrections pass 3), and the reap that took it already killed the group.
pub(crate) fn reap_unreaped(
    child: Option<&mut std::process::Child>,
    reaped: bool,
    group: &ProcessGroup,
) {
    let Some(child) = child else {
        return;
    };
    let mut held = group.held();
    if !reaped
        && let Some(pid) = held.and_then(rustix::process::Pid::from_raw)
        && matches!(exited_unreaped(pid, false), Ok(Some(_)))
    {
        let _ = rustix::process::kill_process_group(pid, rustix::process::Signal::KILL);
    }
    if !(reaped || matches!(child.try_wait(), Ok(Some(_)))) {
        drop(child.kill());
        drop(child.wait());
    }
    *held = None;
}

thread_local! {
    /// Whether the launch in progress on this thread asks for a process group of its own.
    static OWN_PROCESS_GROUP: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// The request, held for one [`ProcessRunner::start`] call: a named run's child leads a process
/// group of its own (design amendment a72, review corrections pass 2).
///
/// It travels beside the [`LaunchPlanView`] and not in it, so every runner that wraps a spawn
/// runner and hands it a view of its own on the same call (a test's `/bin/sh` stand-in, an
/// embedder's wrapper) launches the child the way the run declared, without a field the wrapper
/// could forget to copy. Restored when dropped.
pub(crate) struct OwnProcessGroupScope {
    previous: bool,
}

impl Drop for OwnProcessGroupScope {
    fn drop(&mut self) {
        OWN_PROCESS_GROUP.with(|requested| requested.set(self.previous));
    }
}

/// Ask for a process group of its own for every child started on this thread until the returned
/// scope is dropped.
pub(crate) fn own_process_group_scope(requested: bool) -> OwnProcessGroupScope {
    OwnProcessGroupScope {
        previous: OWN_PROCESS_GROUP.with(|held| held.replace(requested)),
    }
}

/// Whether the launch in progress on this thread asked for a process group of its own.
pub(crate) fn own_process_group_requested() -> bool {
    OWN_PROCESS_GROUP.with(std::cell::Cell::get)
}

/// Start `command` in a process group of its own when the launch asked for one, and say whether
/// it did.
pub(crate) fn apply_process_group(command: &mut std::process::Command) -> bool {
    use std::os::unix::process::CommandExt as _;

    let own = own_process_group_requested();
    if own {
        command.process_group(0);
    }
    own
}

pub(crate) fn observed_termination(
    status: std::process::ExitStatus,
) -> metaharness_protocol::ProcessTermination {
    use metaharness_protocol::ProcessTermination;
    if let Some(code) = status.code() {
        return ProcessTermination::Exited { code };
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(signal) = status.signal() {
            return ProcessTermination::Signaled { signal };
        }
    }
    ProcessTermination::Unknown
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
