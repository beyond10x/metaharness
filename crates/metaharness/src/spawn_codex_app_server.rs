//! The codex app-server spawn: a real `codex app-server`, spoken to over its stdin and stdout, the
//! record it writes to a file, and a caller's catalogue served in process (design amendment a66).
//!
//! [`crate::CodexSpawnRunner`] drives `codex exec`, whose calls reach metaharness through a
//! `PreToolUse` hook. A run with a caller's catalogue is launched as `codex app-server` instead,
//! and this runner is its client:
//!
//! | | `codex exec` | `codex app-server` |
//! |---|---|---|
//! | the record metaharness reads | the session rollout | the same rollout |
//! | the child's stdin | `/dev/null` | the client's half of the JSON-RPC conversation |
//! | the child's stdout | a thin `--json` stream, retained | the server's half, retained and read |
//! | how a call is decided | a hook process over a directory channel | an `item/tool/call` request, answered on stdin |
//! | who performs the call | the vendor | this runner, through the catalogue's server |
//!
//! The conversation itself (which message follows which) is the adapter's
//! [`metaharness_codex::AppServerSession`], a state machine with no I/O. This runner carries its
//! messages, hands each `item/tool/call` to the run loop as the line the codex seam reads, and
//! answers it with what the decision allows: the catalogue's server performs an `allow`, an
//! `abstain` and a `replace`, and a `deny` performs nothing.

use std::collections::{BTreeMap, VecDeque};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError, channel, sync_channel};
use std::sync::{Arc, Mutex};

use metaharness_codex::{AppServerOpening, AppServerSession, SessionAction};
use metaharness_protocol::Decision;
use metaharness_tools::{CallerCatalogue, Catalogue, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

use crate::process::{HarnessProcess, LaunchPlanView, ProcessRunner, copy_credentials};
use crate::spawn_codex::{
    BLOCK_GRACE_POLLS, Exited, POLL, RolloutTail, SESSIONS_DIR, STREAM_QUEUE, record,
    spawn_rollout_tail, spawn_stderr_reader, stdout_path,
};

/// Where the client's half of the conversation is retained, beside the server's.
#[must_use]
pub(crate) fn stdin_path(transcript: &Path) -> PathBuf {
    transcript.with_extension("stdin.jsonl")
}

/// Spawn the planned `codex app-server` child for real.
#[derive(Debug, Default)]
pub struct CodexAppServerRunner {
    spawns: u32,
    credential_copies: u32,
}

impl CodexAppServerRunner {
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
    #[must_use]
    pub fn credential_copies(&self) -> u32 {
        self.credential_copies
    }
}

impl ProcessRunner for CodexAppServerRunner {
    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        // H6, as on the exec route: the copy happens at the spawn.
        copy_credentials(plan.credential_copies)?;
        self.credential_copies += u32::try_from(plan.credential_copies.len()).unwrap_or(u32::MAX);

        std::fs::create_dir_all(plan.cwd)?;
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

        // What the launch wrote for this runner: the opening and the catalogue's checked copy.
        let opening = read_opening(plan.decision_channel)?;
        let catalogue = CallerCatalogue::read(&metaharness_codex::catalogue_copy_path(
            plan.decision_channel,
        ))
        .map_err(|error| {
            std::io::Error::other(format!(
                "the catalogue's scratch copy could not be read: {}: {}",
                error.code(),
                error.problems().join("; ")
            ))
        })?;
        // The operations the Claude route's `mcp-serve --writable` performs (amendment a66 § 4):
        // the run's cwd, and no program for a `run` the catalogue does not publish.
        let operations =
            LocalOperations::unconfined(plan.cwd, Vec::new()).map_err(std::io::Error::other)?;
        let server = Server::with_catalogue(Verbs::new(Catalogue::of(operations)), catalogue);

        let mut command = Command::new(plan.program);
        command
            .args(plan.args)
            .current_dir(plan.cwd)
            // H3: constructed, not inherited.
            .env_clear()
            .envs(plan.env.iter())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        self.spawns += 1;

        let stdin = child.stdin.take().ok_or_else(|| {
            std::io::Error::other("the child was spawned with no stdin pipe to write")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            std::io::Error::other("the child was spawned with no stdout pipe to read")
        })?;
        let stderr = child.stderr.take().ok_or_else(|| {
            std::io::Error::other("the child was spawned with no stderr pipe to read")
        })?;

        let stream_error = Arc::new(Mutex::new(None::<String>));
        let (message_sender, messages) = channel::<Value>();
        spawn_stdout_reader(
            stdout,
            stdout_path(plan.transcript),
            message_sender,
            Arc::clone(&stream_error),
        );
        let captured = Arc::new(Mutex::new(String::new()));
        spawn_stderr_reader(stderr, Arc::clone(&captured));

        let (sender, lines) = sync_channel::<String>(STREAM_QUEUE);
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

        let session = AppServerSession::new(opening);
        let opening_messages = session.start();
        let mut process = AppServerProcess {
            child: Some(child),
            stdin: Some(stdin),
            stdin_log: std::fs::File::create(stdin_path(plan.transcript)).ok(),
            messages,
            lines,
            stream_error,
            stderr: captured,
            rollout,
            tail_done: done,
            session,
            server,
            calls: BTreeMap::new(),
            pending_lines: VecDeque::new(),
            stream_ended: false,
            exit: Exited::NotYet,
        };
        process.apply(opening_messages);
        Ok(Box::new(process))
    }
}

fn read_opening(directory: &Path) -> std::io::Result<AppServerOpening> {
    let path = metaharness_codex::opening_path(directory);
    let text = std::fs::read_to_string(&path)?;
    let value: Value = serde_json::from_str(&text).map_err(|error| {
        std::io::Error::other(format!("the app-server opening is not JSON: {error}"))
    })?;
    AppServerOpening::from_value(&value)
        .ok_or_else(|| std::io::Error::other("the app-server opening is missing a part"))
}

/// Retain the server's half of the conversation whole, and hand on every line that is JSON.
///
/// The bytes are written before the message is handed on (design § 8.4 O8). A line that is not
/// JSON is retained and not handed on: the session has nothing to do with it.
fn spawn_stdout_reader(
    stdout: std::process::ChildStdout,
    path: PathBuf,
    sender: Sender<Value>,
    error_slot: Arc<Mutex<Option<String>>>,
) {
    std::thread::spawn(move || {
        let mut file = match std::fs::File::create(&path) {
            Ok(file) => Some(file),
            Err(error) => {
                record(
                    &error_slot,
                    format!("the app-server stdout record could not be opened: {error}"),
                );
                None
            }
        };
        for line in BufReader::new(stdout).lines() {
            let Ok(line) = line else { break };
            if let Some(file) = file.as_mut() {
                let _ = writeln!(file, "{line}");
            }
            if let Ok(message) = serde_json::from_str::<Value>(&line)
                && sender.send(message).is_err()
            {
                break;
            }
        }
        if let Some(mut file) = file {
            let _ = file.flush();
        }
    });
}

/// One `item/tool/call` the app-server is waiting on.
#[derive(Debug, Clone)]
struct ToolCall {
    request_id: Value,
    tool: String,
    arguments: Value,
    answered: bool,
    waited: u32,
}

/// A live `codex app-server` process, its conversation, its tailed record and the server that
/// performs its client tools.
pub struct AppServerProcess {
    child: Option<Child>,
    /// `None` once closed: after `turn/completed`, after a failed opening, or at a kill.
    stdin: Option<ChildStdin>,
    stdin_log: Option<std::fs::File>,
    messages: Receiver<Value>,
    lines: Receiver<String>,
    stream_error: Arc<Mutex<Option<String>>>,
    stderr: Arc<Mutex<String>>,
    rollout: Arc<Mutex<Option<PathBuf>>>,
    tail_done: Arc<Mutex<bool>>,
    session: AppServerSession,
    server: Server,
    /// Every client tool call, under the key its decision is routed by.
    calls: BTreeMap<String, ToolCall>,
    /// Call lines the run loop has not been handed yet.
    pending_lines: VecDeque<String>,
    stream_ended: bool,
    exit: Exited,
}

impl std::fmt::Debug for AppServerProcess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppServerProcess")
            .field("tool_calls", &self.calls.len())
            .field("stdin_open", &self.stdin.is_some())
            .field("stream_ended", &self.stream_ended)
            .finish_non_exhaustive()
    }
}

impl AppServerProcess {
    /// Everything the child wrote to stderr.
    #[must_use]
    pub fn stderr(&self) -> String {
        self.stderr
            .lock()
            .map(|held| held.clone())
            .unwrap_or_default()
    }

    /// The session rollout this run read, once the child has opened one.
    #[must_use]
    pub fn rollout_path(&self) -> Option<PathBuf> {
        self.rollout.lock().ok().and_then(|held| held.clone())
    }

    /// Read every message the server has written, and do what each asks.
    fn pump(&mut self) {
        loop {
            match self.messages.try_recv() {
                Ok(message) => {
                    let actions = self.session.receive(&message);
                    self.apply(actions);
                }
                Err(TryRecvError::Empty) => return,
                // The server's stdout is closed, so nothing it could still read would be
                // answered: close its stdin, so a child that is still running sees the end.
                Err(TryRecvError::Disconnected) => {
                    self.stdin = None;
                    return;
                }
            }
        }
    }

    fn apply(&mut self, actions: Vec<SessionAction>) {
        for action in actions {
            match action {
                SessionAction::Send(message) => self.send(&message),
                SessionAction::ToolCall {
                    key,
                    request_id,
                    tool,
                    arguments,
                    line,
                } => {
                    self.calls.insert(
                        key,
                        ToolCall {
                            request_id,
                            tool,
                            arguments,
                            answered: false,
                            waited: 0,
                        },
                    );
                    self.pending_lines.push_back(line);
                }
                SessionAction::CloseInput => self.stdin = None,
                SessionAction::Failed(reason) => {
                    record(&self.stream_error, reason);
                    self.stdin = None;
                }
            }
        }
    }

    /// Write one message to the child, and retain it. A child whose stdin is closed or broken has
    /// stopped reading; what it did instead is on its stdout and in its exit status.
    fn send(&mut self, message: &Value) {
        let line = message.to_string();
        if let Some(log) = self.stdin_log.as_mut() {
            let _ = writeln!(log, "{line}");
        }
        let Some(stdin) = self.stdin.as_mut() else {
            return;
        };
        let written = writeln!(stdin, "{line}").and_then(|()| stdin.flush());
        if written.is_err() {
            self.stdin = None;
        }
    }

    /// Answer one call as the decision allows.
    fn answer(&mut self, key: &str, decision: &Decision) {
        let Some(call) = self.calls.get(key).cloned() else {
            return;
        };
        if call.answered {
            return;
        }
        let (text, success) = match decision {
            Decision::Allow | Decision::Abstain => self.perform(&call.tool, &call.arguments),
            Decision::Replace { input } => self.perform(&call.tool, input),
            Decision::Deny { reason } => (
                format!("metaharness denied this call, so nothing was done: {reason}"),
                false,
            ),
        };
        self.send(&metaharness_codex::tool_call_answer(
            &call.request_id,
            &text,
            success,
        ));
        if let Some(held) = self.calls.get_mut(key) {
            held.answered = true;
        }
    }

    /// Perform one call through the catalogue's server: its result text, and whether it succeeded.
    fn perform(&mut self, tool: &str, arguments: &Value) -> (String, bool) {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 0,
            "method": "tools/call",
            "params": {"name": tool, "arguments": arguments},
        });
        let Some(response) = self.server.handle(&request) else {
            return ("the catalogue's server gave no answer".to_owned(), false);
        };
        if let Some(error) = response.get("error") {
            let message = error["message"]
                .as_str()
                .unwrap_or("an error with no message");
            return (message.to_owned(), false);
        }
        let result = &response["result"];
        let text = result["content"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(|item| item["text"].as_str())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .unwrap_or_default();
        let failed = result["isError"].as_bool().unwrap_or(false);
        (text, !failed)
    }

    fn waiting_past_grace(&self) -> bool {
        self.calls
            .values()
            .any(|call| !call.answered && call.waited >= BLOCK_GRACE_POLLS)
    }

    fn tick_waiting(&mut self) {
        for call in self.calls.values_mut() {
            if !call.answered {
                call.waited += 1;
            }
        }
    }

    fn take_stream_error(&self) -> Option<String> {
        self.stream_error
            .lock()
            .ok()
            .and_then(|held| held.as_ref().cloned())
    }

    fn mark_child_gone(&self) {
        if let Ok(mut held) = self.tail_done.lock() {
            *held = true;
        }
    }

    fn child_finished(&mut self) -> bool {
        if matches!(self.exit, Exited::With(_)) {
            return true;
        }
        let Some(child) = self.child.as_mut() else {
            return true;
        };
        match child.try_wait() {
            Ok(Some(status)) => {
                self.exit = Exited::With(status);
                true
            }
            Ok(None) => false,
            Err(_) => true,
        }
    }
}

impl HarnessProcess for AppServerProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        loop {
            match self.poll_line()? {
                crate::ProcessPoll::Line(line) => return Ok(Some(line)),
                crate::ProcessPoll::Idle => (),
                crate::ProcessPoll::Ended => return Ok(None),
            }
        }
    }

    fn poll_line(&mut self) -> std::io::Result<crate::ProcessPoll> {
        self.pump();
        if let Some(line) = self.pending_lines.pop_front() {
            return Ok(crate::ProcessPoll::Line(line));
        }
        if self.stream_ended {
            return Ok(crate::ProcessPoll::Ended);
        }
        if self.child_finished() {
            self.stdin = None;
            self.mark_child_gone();
        }
        match self.lines.recv_timeout(POLL) {
            Ok(line) => Ok(crate::ProcessPoll::Line(line)),
            Err(RecvTimeoutError::Timeout) => {
                self.tick_waiting();
                if self.waiting_past_grace() {
                    // As on the exec route: the stream has not ended and the child is waiting on
                    // a decision metaharness holds; the run loop owns that decision's budget.
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::WouldBlock,
                        "the app-server is waiting on a client tool call metaharness has not \
                         answered",
                    ));
                }
                Ok(crate::ProcessPoll::Idle)
            }
            Err(RecvTimeoutError::Disconnected) => {
                self.stream_ended = true;
                self.pump();
                if let Some(line) = self.pending_lines.pop_front() {
                    return Ok(crate::ProcessPoll::Line(line));
                }
                if let Some(error) = self.take_stream_error() {
                    return Err(std::io::Error::other(error));
                }
                Ok(crate::ProcessPoll::Ended)
            }
        }
    }

    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        let value: Value = serde_json::from_str(line).map_err(|error| {
            std::io::Error::other(format!("metaharness wrote a line it cannot route: {error}"))
        })?;

        if let Some((key, decision)) = metaharness_codex::dynamic_decision(&value) {
            self.answer(&key, &decision);
            return Ok(());
        }
        if value.get("call_id").is_some() {
            // A decision for a call no client tool holds: no hook is installed on this route, so
            // nothing is waiting on it.
            return Ok(());
        }
        let subtype = value
            .get("request")
            .and_then(|request| request.get("subtype"))
            .and_then(Value::as_str);
        if subtype == Some("interrupt") {
            // The kill tier, as on the exec route: `turn/interrupt` has not been driven here.
            return self.kill();
        }
        Err(std::io::Error::other(format!(
            "this adapter has no channel for the control line {line}; it is refused rather than \
             dropped, because a control that appears to work and does not is worse than one that \
             is absent"
        )))
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.stdin = None;
        let outcome = if let Some(child) = self.child.as_mut() {
            match child.kill() {
                Ok(()) => Ok(()),
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
        // A child that is still reading its stdin would never exit while this waits.
        self.stdin = None;
        if let Exited::With(status) = self.exit {
            self.mark_child_gone();
            return Ok(status.code());
        }
        let Some(child) = self.child.as_mut() else {
            self.mark_child_gone();
            return Ok(None);
        };
        let status = child.wait()?;
        self.exit = Exited::With(status);
        self.mark_child_gone();
        Ok(status.code())
    }

    fn termination(&self) -> metaharness_protocol::ProcessTermination {
        match self.exit {
            Exited::With(status) => crate::process::observed_termination(status),
            Exited::NotYet => metaharness_protocol::ProcessTermination::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_client_half_is_retained_beside_the_record_and_not_as_it() {
        let stdin = stdin_path(Path::new("/scratch/run-1/rollout.jsonl"));
        assert_eq!(stdin, PathBuf::from("/scratch/run-1/rollout.stdin.jsonl"));
        assert_ne!(
            stdin,
            stdout_path(Path::new("/scratch/run-1/rollout.jsonl"))
        );
    }
}
