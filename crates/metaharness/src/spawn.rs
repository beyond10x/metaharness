//! The real spawn: an actual vendor process, its stream, and a seam that reaches it.
//!
//! M1 put the spawn behind [`crate::ProcessRunner`] and refused with `Refusal::NoSpawner`, so the
//! whole spec → plan → transcript → events → audit path could be exercised without a vendor
//! binary. This module is the other implementation of that trait, and it carries the one thing a
//! scripted process cannot have: **a control seam that a separate hook process actually answers
//! over**.
//!
//! # How a decision reaches a call
//!
//! The seam is the on-disk `PreToolUse` command hook (design § 7.3, *"the default seam"*). A hook
//! is a separate process, so a decision cannot be a function call and it cannot be a line on the
//! child's stdin either — the vendor reads a hook's answer from **that hook's own stdout**. So:
//!
//! | # | who | what |
//! |---|---|---|
//! | 1 | vendor | writes the assistant record carrying the `tool_use` block to stdout |
//! | 2 | metaharness | reads it, emits `tool.requested`, and — in `frame` mode — decides at once |
//! | 3 | vendor | runs the hook and **waits for its exit** |
//! | 4 | hook | publishes its stdin under a name only it holds, then waits for a file |
//! | 5 | metaharness | matches the request's `tool_use_id` to the call, writes the answer |
//! | 6 | hook | prints the answer and exits `0`; the vendor honours it |
//!
//! Steps 2 and 4 race, and **neither order is a problem**: a decision that arrives first is
//! parked until its request appears, and a request that arrives first is answered as soon as the
//! decision is made. That is deliberate, because the ordering is a vendor behaviour rather than a
//! guarantee — although it was measured, and step 1 does precede step 3 (row **V23**).
//!
//! # Why the child's stdin is `/dev/null`
//!
//! Nothing metaharness sends this adapter travels on stdin. Decisions go over the hook channel,
//! and this adapter's kill tier is *"delivered by terminating the child"* rather than by the
//! `interrupt` control request, which is verified present and undriven (design § 7.3). Leaving
//! stdin open would buy nothing and cost a stall: 2.1.239 waits for stdin data and reports
//! *"no stdin data received in 3s, proceeding without it"* on every run.
//!
//! # What it captures, and why each one
//!
//! * **stdout** — every line goes to the transcript file as it is read, because § 8.4 O8's raw
//!   bytes are what § 9.4's auditor reads and what the § 4.4 cross-check compares.
//! * **stderr** — retained whole. A child that dies before its terminal record leaves exit `3`
//!   (*nobody found out*), and the only thing that says **why** is what it printed on the way out.

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
///
/// A grace period rather than an immediate report, and the reason is [`HarnessProcess::next_line`]'s
/// own contract: `WouldBlock` says *the child is waiting on a decision metaharness has not yet
/// written*, and in `frame` mode metaharness writes that decision microseconds after reading the
/// call. Reporting the gap between those two events as a block would hand the run loop a state it
/// is right to treat as exceptional, several times a second, on a run where nothing is wrong.
const BLOCK_GRACE_POLLS: u32 = 100;

/// How many lines may sit between the reader thread and the run loop before the thread waits.
///
/// Bounded on purpose: an unbounded queue turns a run loop that has stopped reading into memory
/// growth instead of back-pressure, and the vendor is perfectly happy to produce faster than a
/// decision policy consumes.
const STREAM_QUEUE: usize = 256;

/// One hook process, waiting.
#[derive(Debug, Clone)]
struct HookRequest {
    /// The rendezvous name the hook process chose for itself, and the response file's stem.
    key: String,
    /// The call this hook is holding, when the input carried one (row **V22**).
    tool_use_id: Option<String>,
    /// Whether the answer has been written.
    answered: bool,
    /// How many idle polls this request has been waiting through.
    waited: u32,
}

/// The decision channel, as metaharness's half of it.
///
/// A directory pair rather than a socket, for the reason the hook program gives: the hook is a
/// shell script with no interpreter to lean on, and a file that appears under a name it already
/// knows is the one rendezvous every shell can do. It also survives being answered out of order
/// and answered before it was asked, both of which happen.
#[derive(Debug, Clone)]
pub struct HookChannel {
    root: PathBuf,
    requests: PathBuf,
    responses: PathBuf,
}

impl HookChannel {
    /// Create the channel's directories under this scratch root.
    ///
    /// # Errors
    ///
    /// Whatever the filesystem said. A channel that could not be created is a run whose seam
    /// would never be consulted, so it is a refusal and never a warning.
    pub fn create(scratch_root: &Path) -> std::io::Result<Self> {
        Self::create_at(&metaharness_claude::HookChannelPaths::under(scratch_root).root)
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
        let paths = metaharness_claude::HookChannelPaths::at_root(root);
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

    /// Every request published since the last look, oldest name first.
    fn collect(&self, known: &BTreeMap<String, HookRequest>) -> std::io::Result<Vec<HookRequest>> {
        let mut found = Vec::new();
        let entries = match std::fs::read_dir(&self.requests) {
            Ok(entries) => entries,
            // A channel directory that is gone is not a request that arrived. The run's own
            // scratch root is removed when the run is dropped, and a read after that is not news.
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
            let raw = std::fs::read_to_string(&path)?;
            // The hook publishes under one rename, so a readable file is a complete one. An
            // input that will not parse is still a hook that is waiting: it gets a request with
            // no correlation key, is never matched, and its own backstop denies it — which is
            // the fail-closed outcome, reached without metaharness inventing a call id.
            let tool_use_id = metaharness_claude::parse_hook_input(&raw)
                .ok()
                .and_then(|input| input.tool_use_id);
            found.push(HookRequest {
                key: key.to_string(),
                tool_use_id,
                answered: false,
                waited: 0,
            });
        }
        found.sort_by(|left, right| left.key.cmp(&right.key));
        Ok(found)
    }

    /// Write one answer, under one rename so a hook never reads a half-written file.
    ///
    /// `None` writes an **empty** file, which is `abstain`: no bytes, no `permissionDecision`,
    /// and the vendor's own permission pipeline decides. That is not the same as `allow`, which
    /// grants and overrides a stricter rule elsewhere in the vendor's settings (design § 6).
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

/// Spawn the planned child for real.
///
/// One runner may start several children — the relaunch-per-step strategy (design § 7.5 B) is a
/// sequence of spawns — and [`SpawnRunner::spawns`] counts them, because H6's *"re-copied
/// immediately before every spawn"* is a claim about that number and nothing else can check it.
#[derive(Debug, Default)]
pub struct SpawnRunner {
    spawns: u32,
    credential_copies: u32,
}

impl SpawnRunner {
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
    /// Counted so amendment a1 is a tested claim rather than a comment: a copy taken once at the
    /// top of a run and reused for an hour is exactly the failure the amendment exists for.
    #[must_use]
    pub fn credential_copies(&self) -> u32 {
        self.credential_copies
    }
}

impl ProcessRunner for SpawnRunner {
    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        // Amendment a1, and the order is the whole content of it: the copy happens here, at the
        // spawn, and again at the next one. A token copied once and relaunched against for an
        // hour is a token that expires mid-run with nothing to refresh against (Q13).
        copy_credentials(plan.credential_copies)?;
        self.credential_copies += u32::try_from(plan.credential_copies.len()).unwrap_or(u32::MAX);

        std::fs::create_dir_all(plan.cwd)?;
        let channel = HookChannel::create_at(plan.decision_channel)?;
        if let Some(parent) = plan.transcript.parent() {
            std::fs::create_dir_all(parent)?;
        }

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
        let transcript = plan.transcript.to_path_buf();
        let stream_error = Arc::new(Mutex::new(None::<String>));
        spawn_stdout_reader(stdout, sender, transcript, Arc::clone(&stream_error));

        let captured = Arc::new(Mutex::new(String::new()));
        spawn_stderr_reader(stderr, Arc::clone(&captured));

        Ok(Box::new(SpawnedProcess {
            child: Some(child),
            lines,
            stream_error,
            stderr: captured,
            channel,
            requests: BTreeMap::new(),
            parked: BTreeMap::new(),
            transcript: plan.transcript.to_path_buf(),
            stream_ended: false,
            exit: Exited::NotYet,
        }))
    }
}

/// Read the child's stdout, retaining every byte and handing every line on.
///
/// A thread rather than a poll on the pipe, because [`HarnessProcess::next_line`] has to be able
/// to say `WouldBlock` while the child is quiet — and a blocking read on a pipe cannot.
fn spawn_stdout_reader(
    stdout: std::process::ChildStdout,
    sender: SyncSender<String>,
    transcript: PathBuf,
    error_slot: Arc<Mutex<Option<String>>>,
) {
    std::thread::spawn(move || {
        let mut file = match std::fs::File::create(&transcript) {
            Ok(file) => Some(file),
            Err(error) => {
                record(
                    &error_slot,
                    format!("the transcript could not be opened: {error}"),
                );
                None
            }
        };
        for line in BufReader::new(stdout).lines() {
            let line = match line {
                Ok(line) => line,
                Err(error) => {
                    record(
                        &error_slot,
                        format!("the child's stdout could not be read: {error}"),
                    );
                    break;
                }
            };
            // O8: the retained bytes are written before the line is handed on, so a run that
            // dies between the two leaves the evidence rather than the report.
            if let Some(file) = file.as_mut()
                && let Err(error) = writeln!(file, "{line}")
            {
                record(
                    &error_slot,
                    format!("the transcript could not be written: {error}"),
                );
            }
            if sender.send(line).is_err() {
                break;
            }
        }
        if let Some(mut file) = file {
            let _ = file.flush();
        }
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

/// A live vendor process, its retained stream, and the hook channel that answers its calls.
pub struct SpawnedProcess {
    child: Option<Child>,
    lines: Receiver<String>,
    stream_error: Arc<Mutex<Option<String>>>,
    stderr: Arc<Mutex<String>>,
    channel: HookChannel,
    requests: BTreeMap<String, HookRequest>,
    parked: BTreeMap<String, Option<Value>>,
    transcript: PathBuf,
    stream_ended: bool,
    exit: Exited,
}

/// Whether the child has been waited for, and what it said.
///
/// A named three-state rather than an `Option<Option<i32>>`, because the two `None`s mean
/// entirely different things — *nobody has waited yet* and *a signal took it, so there is no
/// code* — and a reader that mixed them up would report a killed child as one that is still
/// running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Exited {
    /// Nobody has waited for it yet.
    NotYet,
    /// It exited, with this code, or on a signal when `None`.
    With(Option<i32>),
}

impl std::fmt::Debug for SpawnedProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SpawnedProcess")
            .field("hook_requests", &self.requests.len())
            .field("parked_decisions", &self.parked.len())
            .field("stream_ended", &self.stream_ended)
            .finish_non_exhaustive()
    }
}

impl SpawnedProcess {
    /// Everything the child wrote to stderr.
    ///
    /// The only account of why a run that produced no terminal record ended, which is the
    /// difference between exit `3` meaning *nobody found out* and exit `3` meaning nothing at all.
    #[must_use]
    pub fn stderr(&self) -> String {
        self.stderr
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    /// Where the raw vendor bytes were retained (design § 8.4 O8).
    #[must_use]
    pub fn transcript_path(&self) -> &Path {
        &self.transcript
    }

    /// How many hook processes have asked, and how many are still waiting.
    ///
    /// The § 7.8 coverage question in a number: a run whose seam was never consulted and a run in
    /// which nothing was attempted look identical in the event stream, and this tells them apart.
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
        for request in self.channel.collect(&self.requests)? {
            self.requests.insert(request.key.clone(), request);
        }
        self.flush()
    }

    /// Write every parked decision whose hook has since arrived.
    fn flush(&mut self) -> std::io::Result<()> {
        for request in self.requests.values_mut() {
            if request.answered {
                continue;
            }
            let Some(call_id) = request.tool_use_id.as_ref() else {
                continue;
            };
            let Some(body) = self.parked.get(call_id) else {
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
}

impl HarnessProcess for SpawnedProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        loop {
            self.pump()?;
            if self.stream_ended {
                return Ok(None);
            }
            match self.lines.recv_timeout(POLL) {
                Ok(line) => return Ok(Some(line)),
                Err(RecvTimeoutError::Timeout) => {
                    self.tick_waiting();
                    if self.waiting_past_grace() {
                        // The contract's own words: the stream has not ended, and the child is
                        // blocked on a decision metaharness holds. Reported rather than waited
                        // out here, because the budget for that decision belongs to the run loop.
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::WouldBlock,
                            "a PreToolUse hook is waiting on a decision metaharness has not \
                             written",
                        ));
                    }
                }
                Err(RecvTimeoutError::Disconnected) => {
                    self.stream_ended = true;
                    // One last look: a hook may have published while the stream was closing, and
                    // a request left unanswered is a child that will sit out its own backstop.
                    self.pump()?;
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
            // This adapter's kill tier is delivered by terminating the child, not by the
            // `interrupt` control request — which is verified *present* and undriven, and a
            // guarantee should not rest on a string (design § 7.3). Writing the control line to
            // a `/dev/null` stdin would be the silent weakening § 7.1 forbids.
            return self.kill();
        }

        Err(std::io::Error::other(format!(
            "this adapter has no channel for the control line {line}; it is refused rather than \
             dropped, because a control that appears to work and does not is worse than one that \
             is absent"
        )))
    }

    fn kill(&mut self) -> std::io::Result<()> {
        if let Some(child) = self.child.as_mut() {
            match child.kill() {
                Ok(()) => Ok(()),
                // Already gone is the outcome `kill` was asked for.
                Err(error) if error.kind() == std::io::ErrorKind::InvalidInput => Ok(()),
                Err(error) => Err(error),
            }
        } else {
            Ok(())
        }
    }

    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        if let Exited::With(code) = self.exit {
            return Ok(code);
        }
        let Some(child) = self.child.as_mut() else {
            return Ok(None);
        };
        let code = child.wait()?.code();
        self.exit = Exited::With(code);
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
        let channel = HookChannel::create(root.path()).expect("channel");
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

    /// The half-written file the hook must never read.
    #[test]
    fn an_answer_is_published_under_one_rename_and_leaves_nothing_behind() {
        let root = scratch();
        let channel = HookChannel::create(root.path()).expect("channel");
        channel
            .answer("k1", Some(&json!({"a": 1})))
            .expect("answer");
        let leftovers: Vec<String> = std::fs::read_dir(&channel.responses)
            .expect("readable")
            .filter_map(std::result::Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(".writing."))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    /// A request published before any decision exists is remembered, and answered the moment the
    /// decision arrives. This is the ordering the design could not assume (**Q16**).
    #[test]
    fn a_hook_that_asks_before_the_decision_exists_is_answered_when_it_does() {
        let root = scratch();
        let channel = HookChannel::create(root.path()).expect("channel");
        std::fs::write(
            channel.requests.join("key-1.json"),
            r#"{"hook_event_name":"PreToolUse","tool_name":"Bash",
                "tool_input":{"command":"echo hi"},"tool_use_id":"toolu_1"}"#,
        )
        .expect("the hook publishes");

        let found = channel.collect(&BTreeMap::new()).expect("collected");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tool_use_id.as_deref(), Some("toolu_1"));
        assert_eq!(found[0].key, "key-1");
        assert!(!found[0].answered);
    }

    /// A request whose input carries no correlation key is never matched and never invented: the
    /// hook's own backstop denies it, which is the fail-closed outcome.
    #[test]
    fn a_request_without_a_correlation_key_is_left_for_the_hooks_own_backstop() {
        let root = scratch();
        let channel = HookChannel::create(root.path()).expect("channel");
        std::fs::write(channel.requests.join("key-2.json"), "not json at all")
            .expect("the hook publishes");
        let found = channel.collect(&BTreeMap::new()).expect("collected");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].tool_use_id, None);
    }

    #[test]
    fn a_request_already_known_is_not_collected_twice() {
        let root = scratch();
        let channel = HookChannel::create(root.path()).expect("channel");
        std::fs::write(
            channel.requests.join("key-3.json"),
            r#"{"tool_use_id":"toolu_3"}"#,
        )
        .expect("published");
        let first = channel.collect(&BTreeMap::new()).expect("collected");
        let known: BTreeMap<String, HookRequest> = first
            .into_iter()
            .map(|request| (request.key.clone(), request))
            .collect();
        assert!(channel.collect(&known).expect("collected").is_empty());
    }

    #[test]
    fn a_channel_directory_that_is_gone_is_not_a_request_that_arrived() {
        let root = scratch();
        let channel = HookChannel::at(&root.path().join("never-created"));
        assert!(
            channel
                .collect(&BTreeMap::new())
                .expect("no error")
                .is_empty()
        );
    }
}
