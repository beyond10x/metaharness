//! Adversary pass over `story:malformed-steering-line-answered-by-id` (amendment a44), through the
//! built binary and its real stdin.
//!
//! The unit's own cases put steering lines on the channel `drive` reads, so the thread that reads
//! stdin and fills that channel never runs in them. These cases write bytes to the process's
//! stdin and read `command.result` events off its stdout.
//!
//! No vendor binary can run. `HOME` is a scratch directory, and the child's `PATH` is built from it
//! (`$HOME/.local/bin` first, then the system directories), so the `claude` that starts is a
//! two-line `sh` script this file writes there, which only sleeps. No model is called and nothing
//! is spent.

use std::io::Write as _;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde_json::Value;

/// One test at a time in this binary: a script written while another test thread forks can be
/// refused at exec as busy, which would fail a case for a reason it does not claim.
static SERIAL: Mutex<()> = Mutex::new(());

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
    elapsed: Duration,
}

impl Outcome {
    /// Every `command.result` on stdout, as `(id, result, code)`.
    fn answers(&self) -> Vec<(String, String, Option<String>)> {
        self.events()
            .into_iter()
            .filter(|event| event["event"] == "command.result")
            .map(|event| {
                (
                    event["id"].as_str().unwrap_or("<not a string>").to_string(),
                    event["outcome"]["result"]
                        .as_str()
                        .unwrap_or_default()
                        .to_string(),
                    event["outcome"]["refused"]["code"]
                        .as_str()
                        .map(ToString::to_string),
                )
            })
            .collect()
    }

    fn events(&self) -> Vec<Value> {
        self.stdout
            .lines()
            .map(|line| serde_json::from_str(line).expect("every stdout line is JSON"))
            .collect()
    }

    fn close_reason(&self) -> Option<String> {
        self.events()
            .into_iter()
            .find(|event| event["event"] == "stream.closed")
            .and_then(|event| event["reason"].as_str().map(ToString::to_string))
    }
}

/// `metaharness run claude` against a stand-in that sleeps `seconds`, with `stdin` as its input.
fn run_with_stdin(base: &Path, seconds: u32, stdin: &[u8]) -> Outcome {
    run_against(
        base,
        &format!("exec sleep {seconds}"),
        Duration::ZERO,
        stdin,
    )
}

/// `metaharness run claude` against a stand-in whose `sh` body is `body`, writing `stdin` once
/// `delay` has passed.
fn run_against(base: &Path, body: &str, delay: Duration, stdin: &[u8]) -> Outcome {
    use std::os::unix::fs::PermissionsExt as _;
    let home = base.join("home");
    let bin = home.join(".local/bin");
    let tmp = base.join("tmp");
    std::fs::create_dir_all(&bin).expect("a scratch bin directory");
    std::fs::create_dir_all(&tmp).expect("a scratch tmp directory");
    let stub = bin.join("claude");
    std::fs::write(&stub, format!("#!/bin/sh\n{body}\n")).expect("the stand-in");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700)).expect("executable");

    let started = Instant::now();
    let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["run", "claude", "-p", "go", "--credentials", "none"])
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", &tmp)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary runs");
    std::thread::sleep(delay);
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(stdin)
        .expect("written");
    let output = child.wait_with_output().expect("it ends");
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        elapsed: started.elapsed(),
    }
}

/// A flat decision, which the decoder refuses, carrying `note` as an extra field.
fn flat_decision_with_note(note: &[u8]) -> Vec<u8> {
    let mut line = br#"{"format":"metaharness.command/1","id":"b1","command":"tool.decide","call_id":"ghost","decision":"allow","note":""#.to_vec();
    line.extend_from_slice(note);
    line.extend_from_slice(b"\"}\n");
    line
}

const HALT: &[u8] =
    br#"{"format":"metaharness.command/1","id":"h1","command":"halt","reason":"stop here"}"#;

/// Finding: a steering line that is not UTF-8 ends the stdin reader for the rest of the run.
///
/// `steering()` reads with `read_line`, which refuses bytes that are not UTF-8, and its loop treats
/// every error as the end of input. So the line itself gets no answer, and neither does any line
/// after it: here a `halt` that is never applied, so the child runs to its own end. The outcome
/// (a44) is that every non-blank line `metaharness run` reads gets exactly one `command.result`.
#[test]
fn adversary_w7u3_a_halt_after_a_line_that_is_not_utf8_is_still_answered() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let base = tempfile::tempdir().expect("a scratch directory");
    // `ö` as one Latin-1 byte, which is not UTF-8.
    let mut stdin = flat_decision_with_note(b"L\xf6sung");
    stdin.extend_from_slice(HALT);
    stdin.push(b'\n');

    let outcome = run_with_stdin(base.path(), 4, &stdin);
    let answers = outcome.answers();
    let codes: Vec<Option<&str>> = answers.iter().map(|(_, _, code)| code.as_deref()).collect();
    assert!(
        answers
            .iter()
            .any(|(id, result, _)| id == "h1" && result == "ok"),
        "the halt sent after a line that is not UTF-8 got no answer, so it was never read; \
         answers {answers:?}, close {:?}, exit {}, {:?} elapsed, stderr {:?}",
        outcome.close_reason(),
        outcome.code,
        outcome.elapsed,
        outcome.stderr
    );
    assert_eq!(
        codes,
        vec![Some("MALFORMED"), None],
        "the line that is not UTF-8 is refused MALFORMED, then the halt is answered ok"
    );
    assert_eq!(outcome.close_reason().as_deref(), Some("steer-halt"));
}

/// The control for the case above: the same two lines with `ö` written as UTF-8. Only the encoding
/// differs, so the case above is red for the encoding and not for the stand-in or the halt.
#[test]
fn adversary_w7u3_control_the_same_lines_in_utf8_are_both_answered_and_the_halt_applies() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let base = tempfile::tempdir().expect("a scratch directory");
    let mut stdin = flat_decision_with_note("Lösung".as_bytes());
    stdin.extend_from_slice(HALT);
    stdin.push(b'\n');

    let outcome = run_with_stdin(base.path(), 4, &stdin);
    assert_eq!(
        outcome.answers(),
        vec![
            (
                "b1".to_string(),
                "refused".to_string(),
                Some("MALFORMED".to_string())
            ),
            ("h1".to_string(), "ok".to_string(), None),
        ],
        "stdout {}, stderr {}",
        outcome.stdout,
        outcome.stderr
    );
    assert_eq!(outcome.close_reason().as_deref(), Some("steer-halt"));
    assert!(
        outcome.elapsed < Duration::from_secs(4),
        "the halt ended the run before the stand-in's own sleep did: {:?}",
        outcome.elapsed
    );
}

/// The real stdin reader, which no unit case drives: several lines in one write, a CRLF ending, a
/// lone `\r`, a whitespace-only line and an empty one. Each non-blank line is answered exactly
/// once, in the order written, and the blank ones are answered by nothing.
#[test]
fn adversary_w7u3_lines_in_one_write_are_each_answered_once_in_order() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let base = tempfile::tempdir().expect("a scratch directory");
    let stdin = [
        r#"{"format":"metaharness.command/1","id":"a1","command":"tool.decide","call_id":"ghost","decision":{"decision":"allow"}}"#,
        "\r",
        "   \t",
        "",
        "not json",
        r#"{"format":"metaharness.command/1","id":"a1","command":"tool.decide","call_id":"ghost","decision":"allow"}"#,
        r#"{"format":"metaharness.command/1","id":7,"command":"halt","reason":"x"}"#,
        r#"{"format":"metaharness.command/1","id":"line\nbreak","command":"steer","text":"x"}"#,
    ]
    .join("\r\n");
    let stdin = format!("{stdin}\n{}\n", String::from_utf8_lossy(HALT));

    let outcome = run_with_stdin(base.path(), 4, stdin.as_bytes());
    assert_eq!(
        outcome.answers(),
        vec![
            (
                "a1".to_string(),
                "refused".to_string(),
                Some("UNKNOWN_CALL".to_string())
            ),
            (
                "-".to_string(),
                "refused".to_string(),
                Some("MALFORMED".to_string())
            ),
            (
                "a1".to_string(),
                "refused".to_string(),
                Some("MALFORMED".to_string())
            ),
            (
                "-".to_string(),
                "refused".to_string(),
                Some("MALFORMED".to_string())
            ),
            (
                "line\nbreak".to_string(),
                "refused".to_string(),
                Some("UNSUPPORTED_CONTROL".to_string())
            ),
            ("h1".to_string(), "ok".to_string(), None),
        ],
        "stdout {}, stderr {}",
        outcome.stdout,
        outcome.stderr
    );
    assert!(
        outcome.stderr.is_empty(),
        "no answer is echoed to stderr: {}",
        outcome.stderr
    );
}

/// Finding (amendment a17 against a44): a steering line that arrives while the run winds up is
/// answered **after** `stream.closed`, which a17 makes the last line of the stream.
///
/// The stand-in closes its stdout at once and lives three seconds more. The run reads the end of
/// the record and waits for the child in `wind_up`; both lines arrive during that wait. When the
/// wait ends the marker goes out, and `drive` then drains the channel, so both answers follow it.
/// The first line is the unit's new path (`MALFORMED`), the second the path `send_as` has always
/// taken (`UNKNOWN_CALL`).
#[test]
fn adversary_w7u3_a_line_read_while_the_run_winds_up_is_not_answered_after_the_marker() {
    let _serial = SERIAL
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let base = tempfile::tempdir().expect("a scratch directory");
    let mut stdin = flat_decision_with_note(b"late");
    stdin.extend_from_slice(
        br#"{"format":"metaharness.command/1","id":"g1","command":"tool.decide","call_id":"ghost","decision":{"decision":"allow"}}"#,
    );
    stdin.push(b'\n');

    let outcome = run_against(
        base.path(),
        "exec 1>&-\nexec sleep 3",
        Duration::from_secs(1),
        &stdin,
    );
    let names: Vec<String> = outcome
        .events()
        .iter()
        .map(|event| event["event"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(
        names.last().map(String::as_str),
        Some("stream.closed"),
        "stream.closed is not the last line: stdout {}",
        outcome.stdout
    );
    assert_eq!(
        outcome
            .answers()
            .into_iter()
            .map(|(id, _, code)| (id, code))
            .collect::<Vec<_>>(),
        vec![
            ("b1".to_string(), Some("MALFORMED".to_string())),
            ("g1".to_string(), Some("UNKNOWN_CALL".to_string())),
        ],
        "both lines are answered, before the marker"
    );
}
