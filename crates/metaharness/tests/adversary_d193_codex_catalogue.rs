//! Adversary cases, wave D193 pass 1, unit `story:codex-catalogue-on-app-server` (design
//! amendment a66).
//!
//! The runner cases drive the real `CodexAppServerRunner` against a `/bin/sh` app-server, as the
//! unit's own C3 vector does: no model, no network, no credential.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use metaharness::protocol::{
    CredentialSource, Decision, Event, HermeticAttestation, HermeticMode, Kind, Seam, TranscriptRef,
};
use metaharness::{
    CodexAppServerRunner, CodexSeam, HarnessProcess, HarnessSeam, Input, LaunchPlanView,
    Metaharness, ProcessPoll, ProcessRunner, ScriptedLog, ScriptedRunner, ScriptedSeams,
};
use metaharness_codex::{AppServerOpening, DynamicTool};
use metaharness_tools::CallerCatalogue;
use serde_json::{Value, json};

const CATALOGUE: &str = include_str!("../fixtures/app-server/catalogue.yaml");
const README: &str = "readme-marker-d193";
const SECRET: &str = "secret-marker-d193";
const PATIENCE: Duration = Duration::from_secs(20);

struct Fake {
    root: tempfile::TempDir,
}

impl Fake {
    /// The world the runner reads, laid out as `start_codex` lays it out, and `body` as the
    /// server's half after the opening.
    fn new(body: &str) -> Self {
        let checked = CallerCatalogue::from_text(Path::new("catalogue.yaml"), CATALOGUE)
            .expect("the fixture catalogue reads");
        let root = tempfile::TempDir::new().expect("a scratch root");
        let fake = Self { root };
        std::fs::create_dir_all(fake.work()).expect("work");
        std::fs::create_dir_all(fake.path().join("codex-home")).expect("codex home");
        std::fs::write(fake.work().join("README.md"), format!("{README}\n")).expect("seed");
        std::fs::write(fake.work().join("secret.txt"), format!("{SECRET}\n")).expect("seed");
        let directory = metaharness_codex::app_server_dir(fake.path());
        std::fs::create_dir_all(&directory).expect("app-server dir");
        let tools: Vec<DynamicTool> = checked
            .entries()
            .iter()
            .map(|entry| DynamicTool {
                name: entry.published.clone(),
                description: entry.description.clone(),
                input_schema: entry.input_schema.clone(),
            })
            .collect();
        let opening = AppServerOpening::new(&fake.work(), None, &tools, "read README.md");
        std::fs::write(
            metaharness_codex::opening_path(&directory),
            opening.to_value().to_string(),
        )
        .expect("opening");
        std::fs::write(
            metaharness_codex::catalogue_copy_path(&directory),
            metaharness_tools::render_scratch_copy(&checked.document_value()),
        )
        .expect("catalogue copy");
        std::fs::write(fake.path().join("app-server.sh"), fake.script(body)).expect("script");
        fake
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn work(&self) -> PathBuf {
        self.path().join("work")
    }

    fn transcript(&self) -> PathBuf {
        self.path().join("rollout.jsonl")
    }

    fn script(&self, body: &str) -> String {
        let root = self.path().display();
        let meta = json!({"timestamp": "2026-10-29T10:00:00.000Z", "type": "session_meta",
        "payload": {"id": "th-fixture", "cli_version": "0.160.0", "cwd": format!("{root}/work"),
            "dynamic_tools": [
                {"type": "function", "name": "fs_read", "description": "Read.", "inputSchema": {"type": "object"}},
            ]}});
        let turn = json!({"timestamp": "2026-10-29T10:00:01.000Z", "type": "turn_context",
            "payload": {"turn_id": "tu-1", "model": "a-model", "personality": "pragmatic"}});
        format!(
            r#"#!/bin/sh
set -u
ROOT='{root}'
SESSION="$ROOT/codex-home/sessions/2026/10/29"
mkdir -p "$SESSION"
ROLLOUT="$SESSION/rollout-2026-10-29T10-00-00-fixture.jsonl"
take() {{ IFS= read -r line || exit 3; printf '%s\n' "$line" >> "$ROOT/received.jsonl"; }}
take
printf '%s\n' '{{"id":1,"result":{{"userAgent":"fixture/0.160.0"}}}}'
take
take
printf '%s\n' '{meta}' > "$ROLLOUT"
printf '%s\n' '{{"id":2,"result":{{"thread":{{"id":"th-fixture"}}}}}}'
take
printf '%s\n' '{turn}' >> "$ROLLOUT"
printf '%s\n' '{{"id":3,"result":{{"turn":{{"id":"tu-1"}}}}}}'
{body}
"#
        )
    }

    fn received(&self) -> Vec<Value> {
        std::fs::read_to_string(self.path().join("received.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    /// The client's half as the runner retained it beside the record.
    fn retained_client_half(&self) -> Vec<Value> {
        std::fs::read_to_string(self.transcript().with_extension("stdin.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }

    fn start(&self) -> Box<dyn HarnessProcess> {
        let args = vec![self.path().join("app-server.sh").display().to_string()];
        let env = BTreeMap::from([
            ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
            (
                "CODEX_HOME".to_owned(),
                self.path().join("codex-home").display().to_string(),
            ),
        ]);
        let cwd = self.work();
        let directory = metaharness_codex::app_server_dir(self.path());
        let transcript = self.transcript();
        let view = LaunchPlanView {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: &cwd,
            credential_copies: &[],
            decision_channel: &directory,
            transcript: &transcript,
        };
        CodexAppServerRunner::new()
            .start(&view)
            .expect("the fake starts")
    }
}

fn call(id: u64, call_id: &str, path: &str) -> String {
    json!({"id": id, "method": "item/tool/call", "params": {
        "threadId": "th-fixture", "turnId": "tu-1", "callId": call_id,
        "namespace": null, "tool": "fs_read", "arguments": {"path": path},
    }})
    .to_string()
}

fn seam() -> CodexSeam {
    CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::OwnedTool,
    )
}

/// Read the process until `stop_after` client calls have been presented (or the stream ends),
/// wait `before_deciding`, then decide each by `decide`, in the order presented. Returns the call
/// ids presented.
fn drive(
    process: &mut Box<dyn HarnessProcess>,
    decide: &dyn Fn(&str) -> Decision,
    before_deciding: Duration,
    stop_after: usize,
) -> Vec<String> {
    let mut seam = seam();
    let mut presented = Vec::new();
    let started = Instant::now();
    while presented.len() < stop_after {
        assert!(started.elapsed() < PATIENCE, "the fake did not finish");
        let line = match process.poll_line() {
            Ok(ProcessPoll::Line(line)) => line,
            Ok(ProcessPoll::Idle) => continue,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Ok(ProcessPoll::Ended) | Err(_) => break,
        };
        for emission in seam.push_line(&line) {
            if let Event::ToolRequested {
                call_id,
                decision_required: true,
                seam: Seam::OwnedTool,
                ..
            } = &emission.event
            {
                presented.push(call_id.clone());
            }
        }
    }
    std::thread::sleep(before_deciding);
    for call_id in &presented {
        process
            .write_line(&seam.decision_line(call_id, &decide(call_id)))
            .expect("the decision is routed");
    }
    presented
}

/// The child dies with a client tool call outstanding. The decision that follows cannot reach it,
/// yet the retained client half (the wire evidence of what metaharness answered) records an answer
/// as sent: `send` writes the log before it checks that the child's stdin is still open.
#[test]
fn an_answer_to_a_child_that_died_mid_call_is_not_retained_as_sent() {
    let fake = Fake::new(&format!(
        "printf '%s\\n' '{}'\nexit 0",
        call(7, "call-orphan", "README.md")
    ));
    let mut process = fake.start();
    let presented = drive(
        &mut process,
        &|_| Decision::Allow,
        // Long enough for /bin/sh to have exited after writing the call.
        Duration::from_millis(500),
        1,
    );
    assert_eq!(presented, ["call-orphan"]);
    let _ = process.wait();

    assert!(
        !fake
            .received()
            .iter()
            .any(|message| message["id"] == 7 && message.get("result").is_some()),
        "precondition: the child never read an answer"
    );
    let claimed: Vec<Value> = fake
        .retained_client_half()
        .into_iter()
        .filter(|message| message["id"] == 7 && message.get("result").is_some())
        .collect();
    assert!(
        claimed.is_empty(),
        "the retained client half says call 7 was answered, and the child was gone: {claimed:?}"
    );
}

/// Two outstanding `item/tool/call` requests under one JSON-RPC id. The runner keys its calls by
/// the id alone, so the second overwrites the first: the decision the frame gave `call-a` (allow,
/// README.md) is performed with `call-b`'s arguments (secret.txt, denied), and `call-b`'s own
/// deny is then dropped as already answered.
#[test]
fn a_reused_request_id_never_performs_a_denied_calls_arguments() {
    let body = format!(
        "printf '%s\\n' '{}'\nprintf '%s\\n' '{}'\nwhile IFS= read -r line; do printf '%s\\n' \"$line\" >> \"$ROOT/received.jsonl\"; done\nexit 0",
        call(4, "call-a", "README.md"),
        call(4, "call-b", "secret.txt"),
    );
    let fake = Fake::new(&body);
    let mut process = fake.start();
    let presented = drive(
        &mut process,
        &|call_id| match call_id {
            "call-a" => Decision::Allow,
            _ => Decision::Deny {
                reason: "secret.txt is outside this step".to_owned(),
            },
        },
        Duration::ZERO,
        2,
    );
    assert_eq!(presented, ["call-a", "call-b"]);
    std::thread::sleep(Duration::from_millis(300));
    let _ = process.wait();

    let answers: Vec<String> = fake
        .received()
        .iter()
        .filter(|message| message["id"] == 4 && message.get("result").is_some())
        .map(|message| message["result"].to_string())
        .collect();
    assert!(
        !answers.iter().any(|answer| answer.contains(SECRET)),
        "the denied call's arguments were performed under the allowed call's decision: {answers:?}"
    );
}

/// a66 § 3 says the vendor's `validate_dynamic_tools` admits every rendering a40 § 3 admits,
/// "never prefixed `mcp`". a40's check admits `mcp`, `mcp__…` and two entries with one
/// rendering; 0.160.0 refuses all three at `thread/start`
/// (`app-server/src/request_processors/thread_processor.rs:362-371`), after the spawn and the
/// credential copy. Each is a catalogue Codex cannot carry, so it is refused before the spawn.
#[test]
fn a_catalogue_codex_refuses_at_thread_start_is_refused_before_the_spawn() {
    let dir = tempfile::tempdir().expect("a directory");
    let mut started_anyway = Vec::new();
    for (name, text) in [
        (
            "reserved-prefix.yaml",
            CATALOGUE.replace("mcp: fs_read", "mcp: mcp__fs_read"),
        ),
        (
            "reserved-name.yaml",
            CATALOGUE.replace("mcp: fs_read", "mcp: mcp"),
        ),
        (
            "duplicate.yaml",
            CATALOGUE.replace("mcp: fs_stat", "mcp: fs_read"),
        ),
    ] {
        assert_ne!(text, CATALOGUE, "{name}: the fixture changed");
        let path = dir.path().join(name);
        std::fs::write(&path, text).expect("a catalogue file");
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::of_lines(
            [
                r#"{"emit":"session.started","harness_version":"0.0.0"}"#,
                r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#,
            ],
            log.clone(),
        );
        let started = Metaharness::new(Kind::Codex)
            .with_credentials(CredentialSource::None)
            .with_catalogue(&path)
            .start_with(
                Input::Prompt("read the readme".to_owned()),
                &mut runner,
                &mut ScriptedSeams,
            );
        if started.is_ok() || log.spawns() != 0 {
            started_anyway.push(format!("{name}: {} spawn(s)", log.spawns()));
        }
    }
    assert!(
        started_anyway.is_empty(),
        "started, and spawned a child the vendor would refuse at thread/start: {started_anyway:?}"
    );
}
