//! The codex spawn: a real `codex exec`, the record it writes to a file, and the hook that
//! answers over a channel.
//!
//! [`crate::SpawnRunner`] drives Claude Code, whose record and whose calls arrive down one pipe.
//! Codex splits them, so this runner is a second implementation of the same trait rather than a
//! flag on the first:
//!
//! | | Claude Code 2.1.239 | codex-cli 0.145.0 |
//! |---|---|---|
//! | the record metaharness reads | the child's **stdout** (`--output-format stream-json`) | a **file** the child writes: `$CODEX_HOME/sessions/…/rollout-*.jsonl` |
//! | what stdout carries | everything | a thin thread/turn/item stream with **no timestamps, no durations, no cost** |
//! | how a call is decided | a `PreToolUse` process over a directory channel | the same |
//!
//! So this runner does three things at once and hands the run loop one line stream:
//!
//! 1. **Tails the rollout.** The session file does not exist at spawn and its name is a `UUIDv7` the
//!    child chooses, so it is discovered under the scratch `CODEX_HOME` and then followed. Every
//!    line is written to the retained transcript as it is read (design § 8.4 O8) and handed on.
//! 2. **Retains stdout anyway.** It is thin, but it is the child's own account of the run and it
//!    is the only thing that exists when a run dies before it opens a session — which is exactly
//!    when somebody needs it.
//! 3. **Serves the hook channel.** A blocked `PreToolUse` process publishes its stdin; this runner
//!    turns that into a line the codex seam reads as a live call, and routes the answer back under
//!    the rendezvous name that process minted.

use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::Value;

use crate::process::{HarnessProcess, LaunchPlanView, ProcessRunner, copy_credentials};

/// How long one poll of the child's stream waits before the channel is looked at again.
const POLL: Duration = Duration::from_millis(20);

/// How many idle polls pass before an unanswered hook request is reported as a block.
const BLOCK_GRACE_POLLS: u32 = 100;

/// How many lines may sit between the reader threads and the run loop before they wait.
const STREAM_QUEUE: usize = 256;

/// How often the rollout tail looks for new bytes, and for the session file before it exists.
const TAIL_POLL: Duration = Duration::from_millis(25);

/// Where a scratch `CODEX_HOME` keeps session records.
///
/// `$CODEX_HOME/sessions/YYYY/MM/DD/rollout-<ISO8601>-<uuid7>.jsonl`, verified against 2,437 local
/// files. The runner walks the tree rather than reconstructing today's date, because a run that
/// crosses midnight would otherwise look for its own record in yesterday's directory.
const SESSIONS_DIR: &str = "sessions";

/// One hook process, waiting.
#[derive(Debug, Clone)]
struct HookRequest {
    /// The rendezvous name the hook process chose for itself, and the response file's stem.
    key: String,
    /// Whether the answer has been written.
    answered: bool,
    /// How many idle polls this request has been waiting through.
    waited: u32,
}

/// The decision channel, as metaharness's half of it.
#[derive(Debug, Clone)]
pub struct CodexHookChannel {
    root: PathBuf,
    requests: PathBuf,
    responses: PathBuf,
}

impl CodexHookChannel {
    /// Create the channel's directories under this scratch root.
    ///
    /// # Errors
    ///
    /// Whatever the filesystem said. A channel that could not be created is a run whose seam would
    /// never be consulted, so it is a refusal and never a warning.
    pub fn create(scratch_root: &Path) -> std::io::Result<Self> {
        Self::create_at(&metaharness_codex::HookChannelPaths::under(scratch_root).root)
    }

    /// Create the channel's directories at a root that is already decided.
    ///
    /// # Errors
    ///
    /// Whatever the filesystem said.
    pub fn create_at(root: &Path) -> std::io::Result<Self> {
        let channel = Self::at(root);
        std::fs::create_dir_all(&channel.requests)?;
        std::fs::create_dir_all(&channel.responses)?;
        Ok(channel)
    }

    /// The channel at this root, whose directories somebody else created.
    #[must_use]
    pub fn at(root: &Path) -> Self {
        let paths = metaharness_codex::HookChannelPaths::at_root(root);
        Self {
            root: paths.root,
            requests: paths.requests,
            responses: paths.responses,
        }
    }

    /// The channel root, which is what a [`LaunchPlanView`] carries.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Every request published since the last look, oldest name first, with its raw stdin.
    fn collect(
        &self,
        known: &BTreeMap<String, HookRequest>,
    ) -> std::io::Result<Vec<(String, String)>> {
        let mut found = Vec::new();
        let entries = match std::fs::read_dir(&self.requests) {
            Ok(entries) => entries,
            // The run's own scratch root is removed when the run is dropped, and a read after that
            // is not news.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(error),
        };
        for entry in entries {
            let path = entry?.path();
            let Some(key) = path
                .file_name()
                .and_then(|name| name.to_str())
                .and_then(|name| name.strip_suffix(".json"))
            else {
                continue;
            };
            if known.contains_key(key) {
                continue;
            }
            // The hook publishes under one rename, so a readable file is a complete one. What it
            // holds is the vendor's payload **verbatim**, unparsed: the adapter decides what it
            // means, this runner only carries it.
            let raw = std::fs::read_to_string(&path)?;
            found.push((key.to_string(), raw));
        }
        found.sort_by(|left, right| left.0.cmp(&right.0));
        Ok(found)
    }

    /// Write one answer, under one rename so a hook never reads a half-written file.
    ///
    /// `None` writes an **empty** file, which is `abstain`: no bytes, no `permissionDecision`, and
    /// the vendor's own approval policy decides. That is not the same as `allow`.
    fn answer(&self, key: &str, body: Option<&Value>) -> std::io::Result<()> {
        let pending = self.responses.join(format!(".writing.{key}"));
        let final_path = self.responses.join(format!("{key}.json"));
        {
            let mut file = std::fs::File::create(&pending)?;
            if let Some(body) = body {
                file.write_all(body.to_string().as_bytes())?;
                file.write_all(b"\n")?;
            }
            file.flush()?;
        }
        std::fs::rename(&pending, &final_path)
    }
}

/// Spawn the planned `codex` child for real.
#[derive(Debug, Default)]
pub struct CodexSpawnRunner {
    spawns: u32,
    credential_copies: u32,
}

impl CodexSpawnRunner {
    /// A runner that has started nothing yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// How many children this runner has started.
    #[must_use]
    pub fn spawns(&self) -> u32 {
        self.spawns
    }

    /// How many credential files this runner has copied, across every spawn.
    ///
    /// Counted so H6's *"re-copied immediately before every spawn"* is a tested number rather than
    /// a comment.
    #[must_use]
    pub fn credential_copies(&self) -> u32 {
        self.credential_copies
    }
}

impl ProcessRunner for CodexSpawnRunner {
    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        // H6, and the order is the content of it: the copy happens at the spawn, and again at the
        // next one. `auth.json` is a token snapshot the same way Claude Code's credential file is.
        copy_credentials(plan.credential_copies)?;
        self.credential_copies += u32::try_from(plan.credential_copies.len()).unwrap_or(u32::MAX);

        std::fs::create_dir_all(plan.cwd)?;
        let channel = CodexHookChannel::create_at(plan.decision_channel)?;
        if let Some(parent) = plan.transcript.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let codex_home = plan
            .env
            .get("CODEX_HOME")
            .map(PathBuf::from)
            .ok_or_else(|| {
                std::io::Error::other(
                    "the codex launch plan named no CODEX_HOME, so the run would write its \
                     session record into the operator's own home",
                )
            })?;

        let mut command = Command::new(plan.program);
        command
            .args(plan.args)
            .current_dir(plan.cwd)
            // H3: constructed, not inherited. `env_clear` first, so a variable absent from the
            // plan is absent from the child however it got into this process.
            .env_clear()
            .envs(plan.env.iter())
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        self.spawns += 1;

        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::other("the child was spawned with no stdout pipe to read")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            std::io::Error::other("the child was spawned with no stderr pipe to read")
        })?;

        let (sender, lines) = sync_channel::<String>(STREAM_QUEUE);
        let stream_error = Arc::new(Mutex::new(None::<String>));

        // The thin stream, retained and not read for events. It is the only account of a run that
        // died before it opened a session, which is the case with no rollout to read.
        let stdout_path = stdout_path(plan.transcript);
        spawn_stdout_retainer(stdout, stdout_path.clone(), Arc::clone(&stream_error));

        let captured = Arc::new(Mutex::new(String::new()));
        spawn_stderr_reader(stderr, Arc::clone(&captured));

        // The record. Discovered rather than named: the session file's UUIDv7 is the child's to
        // choose and does not exist yet.
        let done = Arc::new(Mutex::new(false));
        let rollout = Arc::new(Mutex::new(None::<PathBuf>));
        spawn_rollout_tail(RolloutTail {
            sessions: codex_home.join(SESSIONS_DIR),
            transcript: plan.transcript.to_path_buf(),
            sender,
            error_slot: Arc::clone(&stream_error),
            done: Arc::clone(&done),
            found: Arc::clone(&rollout),
        });

        Ok(Box::new(CodexProcess {
            child: Some(child),
            lines,
            stream_error,
            stderr: captured,
            channel,
            requests: BTreeMap::new(),
            parked: BTreeMap::new(),
            pending_lines: Vec::new(),
            transcript: plan.transcript.to_path_buf(),
            stdout_path,
            rollout,
            tail_done: done,
            stream_ended: false,
            exit: Exited::NotYet,
        }))
    }
}

/// Where the thin `--json` stream is retained, beside the record it is not.
#[must_use]
fn stdout_path(transcript: &Path) -> PathBuf {
    transcript.with_extension("stdout.jsonl")
}

/// Everything the rollout tail needs, gathered so the thread takes one argument.
struct RolloutTail {
    sessions: PathBuf,
    transcript: PathBuf,
    sender: SyncSender<String>,
    error_slot: Arc<Mutex<Option<String>>>,
    done: Arc<Mutex<bool>>,
    found: Arc<Mutex<Option<PathBuf>>>,
}

/// Follow the session rollout, retaining every line and handing every line on.
///
/// The file does not exist at spawn, is named after a `UUIDv7` the child picks, and is appended to
/// while it is read — so this is a poll and not a stream. It stops when the process says the child
/// is gone **and** the file has stopped growing, because a child that has exited may still have
/// its last records in flight.
fn spawn_rollout_tail(tail: RolloutTail) {
    std::thread::spawn(move || {
        let mut file = match std::fs::File::create(&tail.transcript) {
            Ok(file) => Some(file),
            Err(error) => {
                record(
                    &tail.error_slot,
                    format!("the transcript could not be opened: {error}"),
                );
                None
            }
        };
        let mut reader: Option<BufReader<std::fs::File>> = None;
        let mut idle_after_exit = 0_u32;
        loop {
            if reader.is_none()
                && let Some(path) = newest_rollout(&tail.sessions)
                && let Ok(opened) = std::fs::File::open(&path)
            {
                if let Ok(mut slot) = tail.found.lock() {
                    *slot = Some(path);
                }
                reader = Some(BufReader::new(opened));
            }
            let mut moved = false;
            if let Some(open) = reader.as_mut() {
                let mut line = String::new();
                loop {
                    line.clear();
                    match open.read_line(&mut line) {
                        Ok(0) => break,
                        Ok(_) => {
                            // A partial last line is a line the child has not finished writing.
                            // Waiting for its newline is the only way to hand on a whole record.
                            if !line.ends_with('\n') {
                                seek_back(open, line.len());
                                break;
                            }
                            moved = true;
                            let complete = line.trim_end().to_string();
                            // O8: the retained bytes are written before the line is handed on, so
                            // a run that dies between the two leaves the evidence, not the report.
                            if let Some(file) = file.as_mut()
                                && let Err(error) = writeln!(file, "{complete}")
                            {
                                record(
                                    &tail.error_slot,
                                    format!("the transcript could not be written: {error}"),
                                );
                            }
                            if tail.sender.send(complete).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            record(
                                &tail.error_slot,
                                format!("the session rollout could not be read: {error}"),
                            );
                            break;
                        }
                    }
                }
            }
            let finished = tail.done.lock().map_or(true, |held| *held);
            if finished && !moved {
                idle_after_exit += 1;
                // Two quiet polls after the child is gone. One would race the last flush.
                if idle_after_exit >= 2 {
                    break;
                }
            } else {
                idle_after_exit = 0;
            }
            std::thread::sleep(TAIL_POLL);
        }
        if let Some(mut file) = file {
            let _ = file.flush();
        }
    });
}

/// Put back the bytes of a line the child has not finished writing.
///
/// A partial last line is a record still being appended to. Rewinding is the only way to hand on
/// whole records: a reader that took the fragment would emit half a JSON object and the adapter
/// would preserve it as `opaque` forever.
fn seek_back(reader: &mut BufReader<std::fs::File>, bytes: usize) {
    use std::io::{Seek, SeekFrom};
    let Ok(bytes) = i64::try_from(bytes) else {
        return;
    };
    let _ = reader.seek(SeekFrom::Current(-bytes));
}

/// The newest `rollout-*.jsonl` anywhere under a sessions directory.
///
/// A walk rather than today's `YYYY/MM/DD`, because a run that crosses midnight would otherwise
/// look for its own record in yesterday's directory. The home is scratch and holds exactly one
/// session, so "newest" is "the one this run made".
fn newest_rollout(sessions: &Path) -> Option<PathBuf> {
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    let mut stack = vec![sessions.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else {
                continue;
            };
            if kind.is_dir() {
                stack.push(path);
                continue;
            }
            let is_rollout = path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("rollout-"))
                && path.extension().is_some_and(|kind| kind == "jsonl");
            if !is_rollout {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|data| data.modified())
                .unwrap_or(std::time::UNIX_EPOCH);
            if best.as_ref().is_none_or(|(when, _)| modified >= *when) {
                best = Some((modified, path));
            }
        }
    }
    best.map(|(_, path)| path)
}

/// Retain the child's thin `--json` stdout whole.
fn spawn_stdout_retainer(
    stdout: std::process::ChildStdout,
    path: PathBuf,
    error_slot: Arc<Mutex<Option<String>>>,
) {
    std::thread::spawn(move || {
        let mut file = match std::fs::File::create(&path) {
            Ok(file) => file,
            Err(error) => {
                record(
                    &error_slot,
                    format!("the stdout record could not be opened: {error}"),
                );
                return;
            }
        };
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            let _ = writeln!(file, "{line}");
        }
        let _ = file.flush();
    });
}

/// Retain the child's stderr whole.
fn spawn_stderr_reader(stderr: std::process::ChildStderr, into: Arc<Mutex<String>>) {
    std::thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else { break };
            if let Ok(mut held) = into.lock() {
                held.push_str(&line);
                held.push('\n');
            }
        }
    });
}

fn record(slot: &Arc<Mutex<Option<String>>>, message: String) {
    if let Ok(mut held) = slot.lock()
        && held.is_none()
    {
        *held = Some(message);
    }
}

/// A live `codex` process, its tailed record, and the hook channel that answers its calls.
pub struct CodexProcess {
    child: Option<Child>,
    lines: Receiver<String>,
    stream_error: Arc<Mutex<Option<String>>>,
    stderr: Arc<Mutex<String>>,
    channel: CodexHookChannel,
    requests: BTreeMap<String, HookRequest>,
    parked: BTreeMap<String, Option<Value>>,
    /// Hook-request lines the channel produced and [`HarnessProcess::next_line`] has not handed
    /// over yet. A queue because one poll can find several.
    pending_lines: Vec<String>,
    transcript: PathBuf,
    stdout_path: PathBuf,
    rollout: Arc<Mutex<Option<PathBuf>>>,
    tail_done: Arc<Mutex<bool>>,
    stream_ended: bool,
    exit: Exited,
}

/// Whether the child has been waited for, and what it said.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exited {
    /// Nobody has waited for it yet.
    NotYet,
    /// It exited, with this code, or on a signal when `None`.
    With(Option<i32>),
}

impl std::fmt::Debug for CodexProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodexProcess")
            .field("hook_requests", &self.requests.len())
            .field("parked_decisions", &self.parked.len())
            .field("stream_ended", &self.stream_ended)
            .finish_non_exhaustive()
    }
}

impl CodexProcess {
    /// Everything the child wrote to stderr.
    #[must_use]
    pub fn stderr(&self) -> String {
        self.stderr
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    /// Where the raw rollout bytes were retained (design § 8.4 O8).
    #[must_use]
    pub fn transcript_path(&self) -> &Path {
        &self.transcript
    }

    /// Where the thin `--json` stdout was retained.
    ///
    /// Not the transcript, and named separately so nobody reads one for the other: it carries no
    /// timestamps, no durations and no cost, which is the whole reason the rollout is the record.
    #[must_use]
    pub fn stdout_path(&self) -> &Path {
        &self.stdout_path
    }

    /// The session rollout this run actually read, once the child has opened one.
    ///
    /// The evidence that the reader consumed the real record rather than an empty file.
    #[must_use]
    pub fn rollout_path(&self) -> Option<PathBuf> {
        self.rollout.lock().ok().and_then(|held| held.clone())
    }

    /// How many hook processes have asked, and how many are still waiting.
    ///
    /// A run whose seam was never consulted and a run in which nothing was attempted look
    /// identical in the event stream; this tells them apart (design § 7.8).
    #[must_use]
    pub fn hook_requests(&self) -> (usize, usize) {
        let waiting = self
            .requests
            .values()
            .filter(|request| !request.answered)
            .count();
        (self.requests.len(), waiting)
    }

    /// Ingest whatever the hooks have published, then answer everything that can be answered.
    fn pump(&mut self) -> std::io::Result<()> {
        for (key, raw) in self.channel.collect(&self.requests)? {
            self.pending_lines
                .push(metaharness_codex::hook_request_line(&key, &raw));
            self.requests.insert(
                key.clone(),
                HookRequest {
                    key,
                    answered: false,
                    waited: 0,
                },
            );
        }
        self.flush()
    }

    /// Write every parked decision whose hook is still waiting.
    fn flush(&mut self) -> std::io::Result<()> {
        for request in self.requests.values_mut() {
            if request.answered {
                continue;
            }
            let Some(body) = self.parked.get(&request.key) else {
                continue;
            };
            self.channel.answer(&request.key, body.as_ref())?;
            request.answered = true;
        }
        Ok(())
    }

    /// Whether a hook is waiting on a decision metaharness has not written.
    fn waiting_past_grace(&self) -> bool {
        self.requests
            .values()
            .any(|request| !request.answered && request.waited >= BLOCK_GRACE_POLLS)
    }

    fn tick_waiting(&mut self) {
        for request in self.requests.values_mut() {
            if !request.answered {
                request.waited += 1;
            }
        }
    }

    fn take_stream_error(&self) -> Option<String> {
        self.stream_error
            .lock()
            .ok()
            .and_then(|held| held.as_ref().cloned())
    }

    /// Tell the rollout tail the child is gone, so it can stop after the last flush.
    fn mark_child_gone(&self) {
        if let Ok(mut held) = self.tail_done.lock() {
            *held = true;
        }
    }

    /// Whether the child has exited, without blocking on it.
    fn child_finished(&mut self) -> bool {
        if matches!(self.exit, Exited::With(_)) {
            return true;
        }
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.exit = Exited::With(status.code());
                true
            }
            Ok(None) => false,
            Err(_) => true,
        }
    }
}

impl HarnessProcess for CodexProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        loop {
            self.pump()?;
            if !self.pending_lines.is_empty() {
                return Ok(Some(self.pending_lines.remove(0)));
            }
            if self.stream_ended {
                return Ok(None);
            }
            if self.child_finished() {
                self.mark_child_gone();
            }
            match self.lines.recv_timeout(POLL) {
                Ok(line) => return Ok(Some(line)),
                Err(RecvTimeoutError::Timeout) => {
                    self.tick_waiting();
                    if self.waiting_past_grace() {
                        // The contract's own words: the stream has not ended, and the child is
                        // blocked on a decision metaharness holds. Reported rather than waited out
                        // here, because the budget for that decision belongs to the run loop.
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WouldBlock,
                            "a PreToolUse hook is waiting on a decision metaharness has not \
                             written",
                        ));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.stream_ended = true;
                    // One last look: a hook may have published while the tail was closing, and a
                    // request left unanswered is a child that will sit out its own backstop.
                    self.pump()?;
                    if !self.pending_lines.is_empty() {
                        return Ok(Some(self.pending_lines.remove(0)));
                    }
                    if let Some(error) = self.take_stream_error() {
                        return Err(std::io::Error::other(error));
                    }
                    return Ok(None);
                }
            }
        }
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            std::io::Error::other(format!("metaharness wrote a line it cannot route: {error}"))
        })?;

        if let Some(call_id) = value.get("call_id").and_then(Value::as_str) {
            // `Value::Null` is `abstain` and is not the absence of a decision: it means write an
            // empty answer, so the hook passes the call through claiming nothing.
            let body = match value.get("response") {
                None | Some(Value::Null) => None,
                Some(body) => Some(body.clone()),
            };
            self.parked.insert(call_id.to_string(), body);
            return self.flush();
        }

        let subtype = value
            .get("request")
            .and_then(|request| request.get("subtype"))
            .and_then(Value::as_str);
        if subtype == Some("interrupt") {
            // This adapter's kill tier is delivered by terminating the child. `turn/interrupt` is
            // verified present on the **app-server** surface (V14) and `codex exec` has no such
            // channel, so writing a control line to a `/dev/null` stdin would be the silent
            // weakening design § 7.1 forbids.
            return self.kill();
        }

        Err(std::io::Error::other(format!(
            "this adapter has no channel for the control line {line}; it is refused rather than \
             dropped, because a control that appears to work and does not is worse than one that \
             is absent"
        )))
    }

    fn kill(&mut self) -> std::io::Result<()> {
        let outcome = if let Some(child) = self.child.as_mut() {
            match child.kill() {
                Ok(()) => Ok(()),
                // Already gone is the outcome `kill` was asked for.
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
                Err(error) => Err(error),
            }
        } else {
            Ok(())
        };
        self.mark_child_gone();
        outcome
    }

    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        if let Exited::With(code) = self.exit {
            self.mark_child_gone();
            return Ok(code);
        }
        let Some(child) = self.child.as_mut() else {
            self.mark_child_gone();
            return Ok(None);
        };
        let code = child.wait()?.code();
        self.exit = Exited::With(code);
        self.mark_child_gone();
        Ok(code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn scratch() -> tempfile::TempDir {
        tempfile::TempDir::new().expect("a scratch root")
    }

    #[test]
    fn an_abstain_is_an_empty_answer_and_an_allow_is_not() {
        let root = scratch();
        let channel = CodexHookChannel::create(root.path()).expect("channel");
        channel.answer("k1", None).expect("abstain");
        channel
            .answer(
                "k2",
                Some(&json!({"hookSpecificOutput": {"permissionDecision": "allow"}})),
            )
            .expect("allow");

        let abstained =
            std::fs::read_to_string(channel.responses.join("k1.json")).expect("the file exists");
        assert!(
            abstained.is_empty(),
            "abstain writes no bytes, so the hook prints none: {abstained:?}"
        );
        let allowed =
            std::fs::read_to_string(channel.responses.join("k2.json")).expect("the file exists");
        assert!(allowed.contains("\"allow\""));
    }

    /// A request published before any decision exists is remembered under the name the hook
    /// process minted, whatever its payload turns out to carry.
    #[test]
    fn a_request_is_collected_under_the_rendezvous_name_and_carries_its_stdin_verbatim() {
        let root = scratch();
        let channel = CodexHookChannel::create(root.path()).expect("channel");
        std::fs::write(
            channel.requests.join("key-1.json"),
            r#"{"hook_event_name":"PreToolUse","tool_name":"exec"}"#,
        )
        .expect("the hook publishes");
        let found = channel.collect(&BTreeMap::new()).expect("collected");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, "key-1");
        assert!(found[0].1.contains("\"exec\""));
    }

    #[test]
    fn a_request_already_known_is_not_collected_twice() {
        let root = scratch();
        let channel = CodexHookChannel::create(root.path()).expect("channel");
        std::fs::write(channel.requests.join("key-3.json"), "{}").expect("published");
        let known = BTreeMap::from([(
            "key-3".to_string(),
            HookRequest {
                key: "key-3".to_string(),
                answered: false,
                waited: 0,
            },
        )]);
        assert!(channel.collect(&known).expect("collected").is_empty());
    }

    #[test]
    fn a_channel_directory_that_is_gone_is_not_a_request_that_arrived() {
        let root = scratch();
        let channel = CodexHookChannel::at(&root.path().join("never-created"));
        assert!(
            channel
                .collect(&BTreeMap::new())
                .expect("no error")
                .is_empty()
        );
    }

    /// The record is found by walking, not by reconstructing today's date: a run that crossed
    /// midnight would otherwise look for its own session in yesterday's directory.
    #[test]
    fn the_newest_rollout_is_found_by_walking_the_sessions_tree() {
        let root = scratch();
        let sessions = root.path().join("sessions/2026/08/22");
        std::fs::create_dir_all(&sessions).expect("laid out");
        let path = sessions.join("rollout-2026-08-22T10-00-00-0199.jsonl");
        std::fs::write(&path, "{}\n").expect("written");
        std::fs::write(sessions.join("not-a-rollout.jsonl"), "{}\n").expect("written");
        assert_eq!(
            newest_rollout(&root.path().join("sessions")),
            Some(path.clone())
        );
        assert_eq!(newest_rollout(&root.path().join("absent")), None);
    }

    /// The thin stream is retained **beside** the record and never as it: reading one for the
    /// other would be reading a stream with no timestamps as the one that has them.
    #[test]
    fn the_thin_stdout_stream_is_retained_beside_the_record_and_not_as_it() {
        let stdout = stdout_path(Path::new("/scratch/run-1/transcript.jsonl"));
        assert_eq!(
            stdout,
            PathBuf::from("/scratch/run-1/transcript.stdout.jsonl")
        );
        assert_ne!(stdout, PathBuf::from("/scratch/run-1/transcript.jsonl"));
    }
}
