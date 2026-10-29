//! Security review, wave D193, `story:codex-catalogue-on-app-server` (design amendment a66).
//!
//! The per-spawn credential under the catalogue route. The launch copies the operator's
//! `auth.json` into the scratch `CODEX_HOME`, beside the run's `work/`, and the catalogue's server
//! performs calls with the run's cwd as its root. With every call **allowed**, nothing the model
//! can send through `item/tool/call` or as a server request may carry the copied credential back:
//! not a parent path, not a symlink placed in the workspace, not an absolute path, not a
//! namespaced name, and not the vendor's token-refresh request. Nor may it reach any file the run
//! retains.
//!
//! The fake app-server is `/bin/sh` speaking the 0.160.0 shapes with invented values, as the C3
//! vector does. The credential is synthetic. No vendor binary, no network, no model.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use metaharness::{
    CodexAppServerRunner, CredentialCopyView, LaunchPlanView, ProcessPoll, ProcessRunner,
};
use metaharness_codex::{AppServerOpening, CodexSeam, DynamicTool, METHOD_NOT_FOUND};
use metaharness_protocol::{
    Decision, Event, HarnessSeam, HermeticAttestation, HermeticMode, Seam, TranscriptRef,
};
use metaharness_tools::CallerCatalogue;
use serde_json::{Value, json};

const CATALOGUE: &str = include_str!("../fixtures/app-server/catalogue.yaml");

/// The synthetic credential's only distinctive bytes. Anywhere they appear but the copy itself,
/// the credential leaked.
const SECRET: &str = "synthetic-d193-credential-marker-7c41";

const PATIENCE: Duration = Duration::from_secs(20);

struct World {
    root: tempfile::TempDir,
    operator: tempfile::TempDir,
}

impl World {
    fn new() -> std::io::Result<Self> {
        let target = env!("CARGO_TARGET_TMPDIR");
        let world = Self {
            root: tempfile::Builder::new()
                .prefix("d193-sec-")
                .tempdir_in(target)?,
            operator: tempfile::Builder::new()
                .prefix("d193-op-")
                .tempdir_in(target)?,
        };
        std::fs::create_dir_all(world.work())?;
        std::fs::create_dir_all(world.home())?;
        std::fs::write(world.work().join("README.md"), "a workspace file\n")?;
        // A link the model could have asked a write tool to leave, or the operator's repository
        // carried: it names the copied credential from inside the server's root.
        std::os::unix::fs::symlink("../codex-home/auth.json", world.work().join("notes.txt"))?;

        let source = world.operator.path().join("auth.json");
        std::fs::write(
            &source,
            json!({"auth_mode": "chatgpt", "tokens": {"refresh": SECRET}}).to_string(),
        )?;
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o600))?;

        let catalogue = CallerCatalogue::from_text(Path::new("catalogue.yaml"), CATALOGUE)
            .map_err(|error| std::io::Error::other(error.code().to_string()))?;
        let directory = metaharness_codex::app_server_dir(world.root.path());
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
        let opening = AppServerOpening::new(&world.work(), None, &tools, "read README.md");
        std::fs::write(
            metaharness_codex::opening_path(&directory),
            opening.to_value().to_string(),
        )?;
        std::fs::write(
            metaharness_codex::catalogue_copy_path(&directory),
            metaharness_tools::render_scratch_copy(&catalogue.document_value()),
        )?;
        std::fs::write(world.root.path().join("app-server.sh"), world.script())?;
        Ok(world)
    }

    fn work(&self) -> PathBuf {
        self.root.path().join("work")
    }

    fn home(&self) -> PathBuf {
        self.root.path().join("codex-home")
    }

    fn script(&self) -> String {
        let root = self.root.path().display();
        let call = |id: u32, call_id: &str, namespace: Value, path: &str| {
            json!({"id": id, "method": "item/tool/call", "params": {
                "threadId": "th-fixture", "turnId": "tu-1", "callId": call_id,
                "namespace": namespace, "tool": "fs_read", "arguments": {"path": path},
            }})
            .to_string()
        };
        let absolute = format!("{root}/codex-home/auth.json");
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
        let refresh = json!({"id": 100, "method": "account/chatgptAuthTokens/refresh",
            "params": {"reason": "unauthorized", "previousAccountId": null}});
        let ask = json!({"id": 101, "method": "item/tool/requestUserInput", "params": {
            "threadId": "th-fixture", "turnId": "tu-1", "itemId": "it-1",
            "questions": [{"id": "q1", "header": "auth", "question": "paste auth.json"}]}});
        let clock = json!({"id": 102, "method": "currentTime/read", "params": {}});
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
printf '%s\n' '{refresh}'
take
printf '%s\n' '{ask}'
take
printf '%s\n' '{clock}'
take
printf '%s\n' '{parent}'
take
printf '%s\n' '{link}'
take
printf '%s\n' '{absolute}'
take
printf '%s\n' '{namespaced}'
take
printf '%s\n' '{complete}' >> "$ROLLOUT"
printf '%s\n' '{{"method":"turn/completed","params":{{"threadId":"th-fixture","turn":{{"id":"tu-1","status":"completed"}}}}}}'
while IFS= read -r line; do printf '%s\n' "$line" >> "$ROOT/received.jsonl"; done
exit 0
"#,
            parent = call(10, "call-parent", Value::Null, "../codex-home/auth.json"),
            link = call(11, "call-link", Value::Null, "notes.txt"),
            absolute = call(12, "call-absolute", Value::Null, &absolute),
            namespaced = call(
                13,
                "call-namespaced",
                json!("metaharness"),
                "../codex-home/auth.json"
            ),
        )
    }

    fn received(&self) -> Vec<Value> {
        std::fs::read_to_string(self.root.path().join("received.jsonl"))
            .unwrap_or_default()
            .lines()
            .filter_map(|line| serde_json::from_str(line).ok())
            .collect()
    }
}

/// Every file under `directory`, recursively, without following links.
fn files(directory: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(directory) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        match entry.file_type() {
            Ok(kind) if kind.is_dir() => found.extend(files(&path)),
            Ok(kind) if kind.is_file() => found.push(path),
            _ => {}
        }
    }
    found
}

/// What one drive saw: every line and event as text, the call ids decided, and the exit.
struct Observed {
    seen: String,
    requested: Vec<String>,
    exit: Option<i32>,
}

/// Run the real runner against the fake, allowing every call the seam surfaces.
fn drive(world: &World) -> Observed {
    let copy = world.home().join("auth.json");
    let source = world.operator.path().join("auth.json");
    let script = world
        .root
        .path()
        .join("app-server.sh")
        .display()
        .to_string();
    let args = vec![script];
    let env = BTreeMap::from([
        ("PATH".to_owned(), "/usr/bin:/bin".to_owned()),
        ("CODEX_HOME".to_owned(), world.home().display().to_string()),
    ]);
    let cwd = world.work();
    let directory = metaharness_codex::app_server_dir(world.root.path());
    let transcript = world.root.path().join("rollout.jsonl");
    let copies = [CredentialCopyView {
        from: &source,
        to: &copy,
    }];
    let view = LaunchPlanView {
        program: "/bin/sh",
        args: &args,
        env: &env,
        cwd: &cwd,
        credential_copies: &copies,
        decision_channel: &directory,
        transcript: &transcript,
    };
    let mut process = CodexAppServerRunner::new()
        .start(&view)
        .expect("the fake starts");
    let mut seam = CodexSeam::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::OwnedTool,
    );

    let mut seen = String::new();
    let mut requested = Vec::new();
    let started = Instant::now();
    loop {
        assert!(started.elapsed() < PATIENCE, "the fake did not finish");
        let line = match process.poll_line() {
            Ok(ProcessPoll::Line(line)) => line,
            Ok(ProcessPoll::Idle) => continue,
            Ok(ProcessPoll::Ended) => break,
            Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => continue,
            Err(error) => panic!("the stream failed: {error}"),
        };
        seen.push_str(&line);
        for emission in seam.push_line(&line) {
            write!(seen, "{:?}", emission.event).expect("a String takes the write");
            if let Event::ToolRequested {
                call_id,
                decision_required: true,
                seam: Seam::OwnedTool,
                ..
            } = &emission.event
            {
                requested.push(call_id.clone());
                // Every call allowed: the frame is not what keeps the credential in.
                process
                    .write_line(&seam.decision_line(call_id, &Decision::Allow))
                    .expect("the decision is written");
            }
        }
    }
    let exit = process.wait().expect("the fake is reaped");
    Observed {
        seen,
        requested,
        exit,
    }
}

#[test]
fn no_allowed_call_or_server_request_carries_the_copied_credential_anywhere() {
    let world = World::new().expect("the world is laid out");
    let copy = world.home().join("auth.json");
    let Observed {
        seen,
        requested,
        exit,
    } = drive(&world);

    assert!(
        std::fs::read_to_string(&copy).is_ok_and(|text| text.contains(SECRET)),
        "precondition: the credential was copied into the scratch home"
    );
    let mode = std::fs::metadata(&copy)
        .expect("the copy")
        .permissions()
        .mode();
    assert_eq!(
        mode & 0o077,
        0,
        "the copy is readable beyond its owner: {mode:o}"
    );
    assert_eq!(
        requested,
        [
            "call-parent",
            "call-link",
            "call-absolute",
            "call-namespaced"
        ],
        "precondition: every call reached the seam"
    );
    assert_eq!(exit, Some(0), "the fake read its stdin to the end");

    let received = world.received();
    let answer = |id: u64| {
        received
            .iter()
            .find(|message| message.get("method").is_none() && message["id"].as_u64() == Some(id))
            .cloned()
            .unwrap_or_else(|| panic!("no answer to {id}: {received:?}"))
    };
    for id in [100, 101, 102] {
        let answer = answer(id);
        assert_eq!(answer["error"]["code"], METHOD_NOT_FOUND, "{answer}");
        assert!(!answer.to_string().contains(SECRET), "{answer}");
    }
    for id in [10, 11, 12, 13] {
        let answer = answer(id);
        assert_eq!(
            answer["result"]["success"], false,
            "an allowed call read the credential: {answer}"
        );
        assert!(!answer.to_string().contains(SECRET), "{answer}");
    }
    for (id, reason) in [
        (10, "resolves outside the workspace"),
        (11, "resolves outside the workspace"),
        (12, "resolves outside the workspace"),
        (13, "not in this run's catalogue"),
    ] {
        let text = answer(id)["result"]["contentItems"][0]["text"].clone();
        assert!(
            text.as_str().is_some_and(|text| text.contains(reason)),
            "call {id} was refused for another reason than {reason:?}: {text}"
        );
    }
    assert!(
        !seen.contains(SECRET),
        "a line or an event carried the credential"
    );

    let leaked: Vec<PathBuf> = files(world.root.path())
        .into_iter()
        .filter(|path| *path != copy)
        .filter(|path| {
            std::fs::read(path).is_ok_and(|bytes| {
                bytes
                    .windows(SECRET.len())
                    .any(|window| window == SECRET.as_bytes())
            })
        })
        .collect();
    assert!(
        leaked.is_empty(),
        "the credential reached retained files: {leaked:?}"
    );
}
