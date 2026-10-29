//! C3 — the codex app-server round trip (design amendment a66): the real runner, a real child
//! speaking the app-server's JSON-RPC on its stdin and stdout, a real session file, and still no
//! model.
//!
//! The fake is `/bin/sh` and plays the server's half in the shapes the 0.160.0 binary wrote, with
//! values invented here. It reads what the runner sends, writes a rollout under the scratch
//! `CODEX_HOME`, sends one request the route does not serve and three client tool calls, and keeps
//! every line it read. What only a real process shows:
//!
//! | what | why nothing cheaper shows it |
//! |---|---|
//! | the opening reaches the child in order, with `environments: []` and the catalogue's tools | the conversation is two pipes, and a state machine test has none |
//! | an allowed call is performed by the catalogue's server and its result reaches the child | the answer is written to the child's stdin, from the run loop's decision |
//! | a denied call and a call outside the catalogue are answered `success: false` and perform nothing | the negative half: a vector with only the positive half would pass while every call ran |
//! | `turn/completed` closes the child's stdin, so the app-server exits | a client that kept stdin open would leave the child running |
//!
//! **Free, and it must stay free**: `/bin/sh`, no network, no credential, no model.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use metaharness_codex::{AppServerOpening, CodexSeam, DynamicTool};
use metaharness_protocol::{
    ConformanceTier, Decision, Event, HarnessSeam, HermeticAttestation, HermeticMode, Seam,
    TranscriptRef, VectorOutcome,
};
use metaharness_tools::CallerCatalogue;
use serde_json::{Value, json};

use crate::process::{LaunchPlanView, ProcessRunner};
use crate::spawn_codex_app_server::CodexAppServerRunner;

/// A synthetic `tool-contract/1` catalogue: `fs_read`, which this build performs, and `fs_stat`.
const CATALOGUE: &str = include_str!("../fixtures/app-server/catalogue.yaml");

/// The bytes of the file the allowed call reads. Only a performed read can put them in an answer.
const MARKER: &str = "the README the allowed call read";

/// How long the vector waits for the fake before it kills it. The fake answers at once; this is
/// only so a broken runner fails the vector instead of hanging the gate.
const PATIENCE: Duration = Duration::from_secs(20);

/// The app-server vectors of design § 8.5's C3 tier.
#[must_use]
pub fn spawn_vectors() -> Vec<VectorOutcome> {
    vec![vector_app_server_round_trip()]
}

/// One prepared fake app-server and the world it runs in.
struct Fake {
    root: tempfile::TempDir,
}

impl Fake {
    fn new(catalogue: &CallerCatalogue) -> std::io::Result<Self> {
        let root = tempfile::TempDir::new()?;
        let fake = Self { root };
        std::fs::create_dir_all(fake.work())?;
        std::fs::create_dir_all(fake.path().join("codex-home"))?;
        std::fs::write(fake.work().join("README.md"), format!("{MARKER}\n"))?;

        // What the launch writes for the runner: the opening and the catalogue's scratch copy,
        // rendered by the same calls `start_codex` makes.
        let directory = metaharness_codex::app_server_dir(fake.path());
        std::fs::create_dir_all(&directory)?;
        let tools: Vec<DynamicTool> = catalogue
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
        )?;
        std::fs::write(
            metaharness_codex::catalogue_copy_path(&directory),
            metaharness_tools::render_scratch_copy(&catalogue.document_value()),
        )?;
        std::fs::write(fake.path().join("app-server.sh"), fake.script())?;
        Ok(fake)
    }

    fn path(&self) -> &Path {
        self.root.path()
    }

    fn work(&self) -> PathBuf {
        self.path().join("work")
    }

    /// The server's half. `take` reads one line from the runner and keeps it; every message
    /// written is one the 0.160.0 binary writes, in its shape, with invented values.
    fn script(&self) -> String {
        let root = self.path().display();
        let call = |id: u32, call_id: &str, tool: &str| {
            json!({"id": id, "method": "item/tool/call", "params": {
                "threadId": "th-fixture", "turnId": "tu-1", "callId": call_id,
                "namespace": null, "tool": tool, "arguments": {"path": "README.md"},
            }})
            .to_string()
        };
        let meta = json!({"timestamp": "2026-10-29T10:00:00.000Z", "type": "session_meta",
        "payload": {"id": "th-fixture", "cli_version": "0.160.0", "cwd": format!("{root}/work"),
            "dynamic_tools": [
                {"type": "function", "name": "fs_read", "description": "Read.", "inputSchema": {"type": "object"}},
                {"type": "function", "name": "fs_stat", "description": "Stat.", "inputSchema": {"type": "object"}},
            ]}});
        let turn = json!({"timestamp": "2026-10-29T10:00:01.000Z", "type": "turn_context",
            "payload": {"turn_id": "tu-1", "model": "a-model", "personality": "pragmatic"}});
        let complete = json!({"timestamp": "2026-10-29T10:00:05.000Z", "type": "event_msg",
            "payload": {"type": "task_complete", "turn_id": "tu-1", "duration_ms": 4000}});
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
printf '%s\n' '{{"id":0,"method":"item/commandExecution/requestApproval","params":{{"threadId":"th-fixture"}}}}'
take
printf '%s\n' '{allowed}'
take
printf '%s\n' '{denied}'
take
printf '%s\n' '{outside}'
take
printf '%s\n' '{complete}' >> "$ROLLOUT"
printf '%s\n' '{{"method":"turn/completed","params":{{"threadId":"th-fixture","turn":{{"id":"tu-1","status":"completed"}}}}}}'
while IFS= read -r line; do printf '%s\n' "$line" >> "$ROOT/received.jsonl"; done
exit 0
"#,
            allowed = call(1, "call-allowed", "fs_read"),
            denied = call(2, "call-denied", "fs_read"),
            outside = call(3, "call-outside", "shell"),
        )
    }

    /// Every line the fake read from the runner, parsed.
    fn received(&self) -> Vec<Value> {
        std::fs::read_to_string(self.path().join("received.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

/// What the run loop would decide for each call, by the vendor's call id.
fn decision_for(call_id: &str) -> Decision {
    match call_id {
        "call-denied" => Decision::Deny {
            reason: "this step admits no read of README.md".to_owned(),
        },
        // The call outside the catalogue is allowed here on purpose: the server must refuse it
        // by name whatever the decision says.
        _ => Decision::Allow,
    }
}

/// What one drive observed.
struct Observed {
    /// `session.started.offered_tools`, or `None` when it was absent or never came.
    offered: Option<Vec<String>>,
    requested: Vec<String>,
    exit: Option<i32>,
}

fn drive(fake: &Fake) -> std::io::Result<Observed> {
    let script = fake.path().join("app-server.sh").display().to_string();
    let args = vec![script];
    let env = BTreeMap::from([
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        (
            "CODEX_HOME".to_owned(),
            fake.path().join("codex-home").display().to_string(),
        ),
    ]);
    let cwd = fake.work();
    let directory = metaharness_codex::app_server_dir(fake.path());
    let transcript = fake.path().join("rollout.jsonl");
    let view = LaunchPlanView {
        program: "/bin/sh",
        args: &args,
        env: &env,
        cwd: &cwd,
        credential_copies: &[],
        decision_channel: &directory,
        transcript: &transcript,
    };
    let mut process = CodexAppServerRunner::new().start(&view)?;
    let mut seam = CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::OwnedTool,
    );
    let mut observed = Observed {
        offered: None,
        requested: Vec::new(),
        exit: None,
    };
    let started = Instant::now();
    loop {
        if started.elapsed() > PATIENCE {
            let _ = process.kill();
            return Err(std::io::Error::other(
                "the fake app-server did not finish in time",
            ));
        }
        let line = match process.poll_line() {
            Ok(crate::ProcessPoll::Line(line)) => line,
            Ok(crate::ProcessPoll::Idle) => continue,
            Ok(crate::ProcessPoll::Ended) => break,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(error) => {
                let _ = process.kill();
                return Err(error);
            }
        };
        for emission in seam.push_line(&line) {
            match &emission.event {
                Event::SessionStarted { offered_tools, .. } => {
                    observed.offered.clone_from(offered_tools);
                }
                Event::ToolRequested {
                    call_id,
                    decision_required: true,
                    seam: Seam::OwnedTool,
                    ..
                } => {
                    observed.requested.push(call_id.clone());
                    let decision = decision_for(call_id);
                    process.write_line(&seam.decision_line(call_id, &decision))?;
                }
                _ => {}
            }
        }
    }
    observed.exit = process.wait()?;
    Ok(observed)
}

/// The answer the fake read for the server request with this id.
fn answer_to(received: &[Value], id: u64) -> Option<&Value> {
    received
        .iter()
        .find(|message| message.get("method").is_none() && message["id"].as_u64() == Some(id))
}

/// What is wrong with the answers the fake read: the unserved request refused, the allowed call
/// performed, and the denied call and the call outside the catalogue answered `success: false`
/// without the file's bytes, which only a performed read could carry.
fn answer_faults(received: &[Value]) -> Vec<String> {
    let mut faults = Vec::new();
    match answer_to(received, 0) {
        Some(answer) if answer["error"]["code"] == metaharness_codex::METHOD_NOT_FOUND => {}
        other => faults.push(format!("the unserved request was answered with {other:?}")),
    }
    let result = |id: u64| answer_to(received, id).map(|answer| answer["result"].clone());
    let text = |result: &Value| {
        result["contentItems"][0]["text"]
            .as_str()
            .map(str::to_owned)
    };
    match result(1) {
        Some(answer)
            if answer["success"] == true && text(&answer).is_some_and(|t| t.contains(MARKER)) => {}
        other => faults.push(format!("the allowed call was answered with {other:?}")),
    }
    match result(2) {
        Some(answer)
            if answer["success"] == false
                && text(&answer).is_some_and(|t| t.contains("denied") && !t.contains(MARKER)) => {}
        other => faults.push(format!("the denied call was answered with {other:?}")),
    }
    match result(3) {
        Some(answer)
            if answer["success"] == false
                && text(&answer).is_some_and(|t| {
                    t.contains("not in this run's catalogue") && !t.contains(MARKER)
                }) => {}
        other => faults.push(format!(
            "the call outside the catalogue was answered with {other:?}"
        )),
    }
    faults
}

/// The round trip, with the three decisions a client tool can get.
fn vector_app_server_round_trip() -> VectorOutcome {
    let id = "c3-app-server-round-trip";
    let checked = match CallerCatalogue::from_text(Path::new("catalogue.yaml"), CATALOGUE) {
        Ok(checked) => checked,
        Err(error) => return broken(id, &format!("the catalogue did not read: {}", error.code())),
    };
    let Ok(fake) = Fake::new(&checked) else {
        return broken(id, "the fake app-server could not be laid out");
    };
    let observed = match drive(&fake) {
        Ok(observed) => observed,
        Err(error) => return broken(id, &error.to_string()),
    };
    let received = fake.received();
    let mut faults = Vec::new();

    let methods: Vec<&str> = received
        .iter()
        .filter_map(|message| message["method"].as_str())
        .collect();
    if methods != ["initialize", "initialized", "thread/start", "turn/start"] {
        faults.push(format!("the client sent {methods:?}"));
    }
    let initialize = received.iter().find(|m| m["method"] == "initialize");
    if initialize.is_none_or(|m| m["params"]["capabilities"]["experimentalApi"] != true) {
        faults.push("initialize did not ask for the experimental API".to_owned());
    }
    let thread_start = received.iter().find(|m| m["method"] == "thread/start");
    let offered_names: Vec<&str> = thread_start
        .and_then(|m| m["params"]["dynamicTools"].as_array())
        .map(|tools| tools.iter().filter_map(|t| t["name"].as_str()).collect())
        .unwrap_or_default();
    if offered_names != ["fs_read", "fs_stat"] {
        faults.push(format!("thread/start offered {offered_names:?}"));
    }
    if thread_start.is_none_or(|m| m["params"]["environments"] != json!([])) {
        faults.push("thread/start did not name an empty environment list".to_owned());
    }

    faults.extend(answer_faults(&received));

    if observed.requested != ["call-allowed", "call-denied", "call-outside"] {
        faults.push(format!(
            "the seam held {:?}, not the three calls under their call ids",
            observed.requested
        ));
    }
    if observed.offered != Some(vec!["fs_read".to_owned(), "fs_stat".to_owned()]) {
        faults.push(format!("session.started offered {:?}", observed.offered));
    }
    // The fake exits 0 only after it read to the end of its stdin, so a zero is the evidence
    // that `turn/completed` closed it.
    if observed.exit != Some(0) {
        faults.push(format!("the fake exited {:?}", observed.exit));
    }
    if faults.is_empty() {
        VectorOutcome::passed(id, ConformanceTier::C3)
    } else {
        VectorOutcome::failed(id, ConformanceTier::C3, faults.join("; "))
    }
}

fn broken(id: &str, detail: &str) -> VectorOutcome {
    VectorOutcome::failed(
        id,
        ConformanceTier::C3,
        format!("the vector observed nothing: {detail}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_app_server_round_trip_passes() {
        for outcome in spawn_vectors() {
            assert!(outcome.passed, "{}: {}", outcome.id, outcome.detail);
            assert!(!outcome.tier.needs_a_model(), "{}", outcome.id);
        }
    }

    fn answers(denied_text: &str) -> Vec<Value> {
        let answer = |id: u64, text: &str, success: bool| {
            json!({"id": id, "result": {
                "contentItems": [{"type": "inputText", "text": text}],
                "success": success,
            }})
        };
        vec![
            json!({"id": 0, "error": {"code": metaharness_codex::METHOD_NOT_FOUND, "message": "refused"}}),
            answer(1, MARKER, true),
            answer(2, denied_text, false),
            answer(
                3,
                "`shell` is not in this run's catalogue, so nothing was done.",
                false,
            ),
        ]
    }

    /// The negative half can go red: a runner that performed a denied call would put the file's
    /// bytes in its answer, and the vector must say so.
    #[test]
    fn a_denied_call_that_carried_the_file_fails_the_vector() {
        assert_eq!(
            answer_faults(&answers("metaharness denied this call")),
            Vec::<String>::new()
        );
        let faults = answer_faults(&answers(&format!("denied, yet {MARKER}")));
        assert_eq!(faults.len(), 1, "{faults:?}");
        assert!(faults[0].contains("the denied call"), "{faults:?}");
    }
}
