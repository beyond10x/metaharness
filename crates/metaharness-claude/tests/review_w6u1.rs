//! Security review pass 1, wave 6 unit 1 (`story:claude-seam-fails-closed`, amendment a41).
//!
//! Drives the public surface only: `hook_program`, `HookChannelPaths`, `render_hook_response`.
//! No vendor binary, no credential, no network. `/bin/sh` is the only process started.
//!
//! Amendment a41 § 5 states two rules for a non-empty answer: it is forwarded only when it is a
//! **single line**, and only when that line **ends with `}}`**. The first two cases hold one rule
//! each with an answer that satisfies every other rule, so removing that rule alone turns the case
//! red. The third case is the other half of § 5: every answer the run loop's writer can produce
//! still reaches the vendor byte for byte.

use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::Duration;

use metaharness_claude::{HookChannelPaths, hook_program, render_hook_response};
use metaharness_protocol::Decision;
use serde_json::{Value, json};

/// What the vendor hands the hook on stdin.
const INPUT: &[u8] = br#"{"hook_event_name":"PreToolUse","tool_use_id":"toolu_review"}"#;

/// How every allow and every deny the renderer writes opens (a41 § 5).
const ALLOW_OPENING: &str =
    r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow""#;
const DENY_OPENING: &str =
    r#"{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"deny""#;

/// A channel whose two directories exist, under a fresh scratch root.
fn ready_channel(base: &Path) -> HookChannelPaths {
    let channel = HookChannelPaths::under(&base.join("run"));
    std::fs::create_dir_all(&channel.requests).expect("requests");
    std::fs::create_dir_all(&channel.responses).expect("responses");
    channel
}

/// The request the program published, once it is there under its final name.
fn published_request(requests: &Path, child: &mut Child) -> Option<PathBuf> {
    for _ in 0..200 {
        let found = std::fs::read_dir(requests).ok().and_then(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .find(|path| {
                    path.extension().is_some_and(|x| x == "json")
                        && !path
                            .file_name()
                            .is_some_and(|name| name.to_string_lossy().starts_with('.'))
                })
        });
        if found.is_some() {
            return found;
        }
        if child.try_wait().ok().flatten().is_some() {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    None
}

/// Run the real program as the vendor would and answer its request with `reply`, published under
/// one rename as metaharness's writer does (`crates/metaharness/src/spawn.rs`, `HookChannel::answer`).
fn drive(reply: &[u8]) -> Output {
    let base = tempfile::tempdir().expect("a directory");
    let channel = ready_channel(base.path());
    let program = hook_program(&channel).expect("a UTF-8 channel");
    let script = base.path().join("pretooluse");
    std::fs::write(&script, program).expect("written");
    let mut child = Command::new("/bin/sh")
        .arg(&script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the hook runs");
    child
        .stdin
        .take()
        .expect("stdin")
        .write_all(INPUT)
        .expect("input written");
    let request =
        published_request(&channel.requests, &mut child).expect("the program published a request");
    let name = request.file_name().expect("a file name").to_owned();
    let pending = channel
        .responses
        .join(format!(".writing.{}", name.to_string_lossy()));
    std::fs::write(&pending, reply).expect("written");
    std::fs::rename(&pending, channel.responses.join(name)).expect("answered");
    child.wait_with_output().expect("the hook ends")
}

/// The program refused the answer: exit `0`, one line, a deny carrying the unreadable-answer
/// reason, and never the answer itself.
fn assert_refused(output: &Output, reply: &[u8], case: &str) {
    assert!(output.status.success(), "{case}: {output:?}");
    assert_ne!(
        output.stdout, reply,
        "{case}: the program forwarded an answer a41 § 5 says it refuses"
    );
    let printed: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "{case}: printed no single decision ({error}); stdout {:?}",
            String::from_utf8_lossy(&output.stdout)
        )
    });
    let decision = &printed["hookSpecificOutput"];
    assert_eq!(decision["hookEventName"], "PreToolUse", "{case}: {printed}");
    assert_eq!(decision["permissionDecision"], "deny", "{case}: {printed}");
    let reason = decision["permissionDecisionReason"]
        .as_str()
        .unwrap_or_else(|| panic!("{case}: a deny without a reason: {printed}"));
    assert!(reason.contains("not a hook decision"), "{case}: {reason}");
    let line = output
        .stdout
        .strip_suffix(b"\n")
        .unwrap_or_else(|| panic!("{case}: the deny does not end its line"));
    assert!(!line.contains(&b'\n'), "{case}: more than one line");
}

/// a41 § 5, the closing rule. Each answer is one line that opens exactly as an allow or a deny
/// and stops before it closes, so it fails no rule but that one. The vendor's parser
/// (2.1.304/2.1.305, `JZ`: starts with `{`, does not parse, does not end with `}`) reads each as
/// plain text, which is success with no decision.
#[test]
fn a_single_line_that_opens_as_a_decision_and_never_closes_is_refused() {
    let cases = [
        (
            "an allow cut after its opening",
            format!("{ALLOW_OPENING}\n"),
        ),
        (
            "an allow cut inside its updatedInput",
            format!("{ALLOW_OPENING},\"updatedInput\":{{\"command\":\"ls -la\"\n"),
        ),
        (
            "a deny cut inside its reason",
            format!("{DENY_OPENING},\"permissionDecisionReason\":\"not th\n"),
        ),
    ];
    for (case, reply) in cases {
        assert!(!reply.contains("}}"), "{case}: the case must not close");
        let output = drive(reply.as_bytes());
        assert_refused(&output, reply.as_bytes(), case);
    }
}

/// a41 § 5, the single-line rule. Each answer opens as an allow and ends with `}}`, so it fails
/// no rule but that one. The first is the shape the vendor's parser reads as several JSON
/// documents (`eFr`: two lines, each parsing and each failing the hook schema or empty) and so as
/// plain text with no decision; the second is two decisions in one answer.
#[test]
fn an_answer_of_two_lines_is_refused_even_when_it_opens_and_closes_as_a_decision() {
    let cases = [
        (
            "an allow followed by a second document",
            format!("{ALLOW_OPENING},\"updatedInput\":5}}}}\n{{\"x\":{{}}}}\n"),
        ),
        (
            "an allow followed by a deny",
            format!(
                "{ALLOW_OPENING}}}}}\n{DENY_OPENING},\"permissionDecisionReason\":\"no\"}}}}\n"
            ),
        ),
    ];
    for (case, reply) in cases {
        assert!(
            reply.starts_with(ALLOW_OPENING) && reply.trim_end_matches('\n').ends_with("}}"),
            "{case}: the case must satisfy the opening and closing rules"
        );
        let output = drive(reply.as_bytes());
        assert_refused(&output, reply.as_bytes(), case);
    }
}

/// What metaharness's writer puts in the answer file for `decision`: the decision line the seam
/// writes (`ClaudeSeam::decision_line`), parsed back as the spawn runner parses it
/// (`write_line`), then written as `to_string()` and one newline, or nothing for `Null`.
fn written_by_metaharness(decision: &Decision) -> Vec<u8> {
    let line = json!({
        "call_id": "toolu_review",
        "response": render_hook_response(decision),
    })
    .to_string();
    let parsed: Value = serde_json::from_str(&line).expect("the decision line parses");
    match parsed.get("response") {
        None | Some(Value::Null) => Vec::new(),
        Some(body) => format!("{body}\n").into_bytes(),
    }
}

/// a41 § 5, the other half, over a wider set than the unit's own test: every answer the writer
/// can produce reaches the vendor byte for byte. Deny reasons carry every ASCII byte, the
/// Unicode line separators, `}}` at the end and a reason that is blank; replacements carry every
/// JSON type, `}}` inside strings and a large value.
#[test]
fn every_answer_the_writer_can_produce_reaches_the_vendor_byte_for_byte() {
    let every_ascii: String = (0u8..=0x7f).map(char::from).collect();
    let mut decisions = vec![
        Decision::Allow,
        Decision::Abstain,
        Decision::Deny {
            reason: String::new(),
        },
        Decision::Deny {
            reason: " \n\t ".to_string(),
        },
        Decision::Deny {
            reason: every_ascii.clone(),
        },
        Decision::Deny {
            reason: "}}".to_string(),
        },
        Decision::Deny {
            reason: "ends in }}\n{}\n".to_string(),
        },
        Decision::Deny {
            reason: "\u{2028}\u{2029}\u{feff} it's \"q\" $(id) `id` %s %d \\ \\n".to_string(),
        },
    ];
    for input in [
        Value::Null,
        json!(true),
        json!(0),
        json!(-1.5e300),
        json!(""),
        json!("}}"),
        json!([]),
        json!({}),
        json!([[[[]]]]),
        json!({"a": {"b": {"c": "}}"}}}),
        json!({"command": every_ascii}),
        json!({"file_path": "/w/a", "content": "x".repeat(300_000)}),
    ] {
        decisions.push(Decision::Replace { input });
    }
    for decision in decisions {
        let written = written_by_metaharness(&decision);
        let output = drive(&written);
        assert!(output.status.success(), "{decision:?}: {output:?}");
        assert!(
            output.stdout == written,
            "{decision:?}: the vendor would read {} bytes where metaharness wrote {}: {:?}",
            output.stdout.len(),
            written.len(),
            String::from_utf8_lossy(output.stdout.get(..400).unwrap_or(output.stdout.as_slice()))
        );
    }
}
