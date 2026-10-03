//! Opt-in native vendor observations against an owned, credential-free fixture provider.
//! This is not hosted-provider qualification or evidence of real model token consumption.
#![cfg(unix)]

use metaharness_codex::{
    HookChannelPaths, LaunchContext, LaunchPlan, RolloutReader, config_path, hook_program,
    hook_program_path, plan_launch, render_hook_response,
};
use metaharness_protocol::{
    CredentialSource, Decision, DecisionMode, Event, Kind, RunSpec, TranscriptRef,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MODEL: &str = "gpt-5.2-codex";
const ANSWER: &str = "native-fixture-final";
static NATIVE_RUN: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
enum Mode {
    Text,
    Refusal,
    Tool { exit: i32, deny: bool },
    Patch,
}

struct Provider {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}
impl Provider {
    fn start(mode: Mode) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&requests);
        let stop = Arc::clone(&stopping);
        let worker = thread::spawn(move || {
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        let ordinal = seen.lock().unwrap().len();
                        let request = serve(stream, mode, ordinal);
                        seen.lock().unwrap().push(request);
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(e) => panic!("fixture accept: {e}"),
                }
            }
        });
        Self {
            endpoint,
            requests,
            stopping,
            worker: Some(worker),
        }
    }
}
impl Drop for Provider {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        self.worker.take().unwrap().join().unwrap();
    }
}

fn serve(mut stream: TcpStream, mode: Mode, ordinal: usize) -> Value {
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    stream
        .set_write_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut reader = BufReader::new(stream.try_clone().unwrap());
    let mut first = String::new();
    reader.read_line(&mut first).unwrap();
    let mut length = 0;
    let mut authorized = false;
    loop {
        let mut line = String::new();
        assert!(reader.read_line(&mut line).unwrap() > 0);
        if line.trim().is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                length = value.trim().parse::<usize>().unwrap();
            }
            if name.eq_ignore_ascii_case("authorization") {
                authorized = true;
            }
        }
    }
    assert!(length <= 2 * 1024 * 1024, "bounded request");
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).unwrap();
    let body: Value = serde_json::from_slice(&bytes).unwrap();
    let request = json!({"request":first.trim(),"authorization_present":authorized,"body":body});
    let (status, mime, response) = if matches!(mode, Mode::Refusal) || authorized {
        ("400 Bad Request", "application/json", json!({"error":{"message":"fixture model refusal","type":"invalid_request_error","code":"model_not_found"}}).to_string())
    } else {
        let response = if let Mode::Tool { exit, .. } = mode
            && ordinal == 0
        {
            assert!(
                body["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["name"] == "exec_command")
            );
            tool_stream(exit)
        } else if matches!(mode, Mode::Patch) && ordinal == 0 {
            assert!(
                body["tools"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|tool| tool["name"] == "apply_patch")
            );
            patch_stream()
        } else {
            success_stream()
        };
        ("200 OK", "text/event-stream", response)
    };
    let header = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        response.len()
    );
    stream.write_all(header.as_bytes()).unwrap();
    stream.write_all(response.as_bytes()).unwrap();
    request
}

fn tool_stream(exit: i32) -> String {
    let arguments = json!({"cmd":format!("printf native-fixture-tool; printf marker > native-fixture-marker; exit {exit}"),"yield_time_ms":1000,"max_output_tokens":128}).to_string();
    let item = json!({"id":"fixture-tool-item","type":"function_call","name":"exec_command","call_id":"fixture-call-1","arguments":arguments});
    [
        json!({"type":"response.created","response":{"id":"fixture-tool-response","status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.added","output_index":0,"item":{"id":"fixture-tool-item","type":"function_call","name":"exec_command","call_id":"fixture-call-1","arguments":""}}),
        json!({"type":"response.function_call_arguments.delta","item_id":"fixture-tool-item","output_index":0,"delta":arguments}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":{"id":"fixture-tool-response","status":"completed","output":[item],"usage":{"input_tokens":3,"output_tokens":4,"total_tokens":7}}}),
    ].iter().fold(String::new(), |mut output, event| {
        write!(output, "event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap()).unwrap();
        output
    })
}

fn patch_stream() -> String {
    let item = json!({"id":"fixture-patch-item","type":"custom_tool_call","name":"apply_patch","call_id":"fixture-patch-1","input":"*** Begin Patch\n*** Add File: native-fixture-patch\n+native-fixture-content\n*** End Patch"});
    [
        json!({"type":"response.created","response":{"id":"fixture-patch-response","status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.added","output_index":0,"item":{"id":"fixture-patch-item","type":"custom_tool_call","name":"apply_patch","call_id":"fixture-patch-1","input":""}}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":{"id":"fixture-patch-response","status":"completed","output":[item],"usage":{"input_tokens":3,"output_tokens":4,"total_tokens":7}}}),
    ].iter().fold(String::new(), |mut output, event| {
        write!(output, "event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap()).unwrap();
        output
    })
}

fn success_stream() -> String {
    let item = json!({"id":"fixture-message","type":"message","role":"assistant","status":"completed","phase":"final_answer","content":[{"type":"output_text","text":ANSWER,"annotations":[]}]});
    let response = json!({"id":"fixture-response","object":"response","created_at":1,"status":"completed","model":MODEL,"output":[item.clone()],"usage":{"input_tokens":3,"output_tokens":4,"total_tokens":7}});
    [
        json!({"type":"response.created","response":{"id":"fixture-response","status":"in_progress","output":[]}}),
        json!({"type":"response.output_item.added","output_index":0,"item":{"id":"fixture-message","type":"message","role":"assistant","content":[]}}),
        json!({"type":"response.output_text.delta","item_id":"fixture-message","output_index":0,"content_index":0,"delta":ANSWER}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":response}),
    ].iter().fold(String::new(), |mut output, event| {
        write!(output, "event: {}\ndata: {event}\n\n", event["type"].as_str().unwrap()).unwrap();
        output
    })
}

fn launch(root: &Path, endpoint: &str, mode: Mode) -> LaunchPlan {
    let mut spec = RunSpec::new(Kind::Codex);
    spec.credentials = CredentialSource::None;
    spec.decisions = DecisionMode::Observe;
    spec.model = Some(MODEL.to_owned());
    spec.model_endpoint = Some(endpoint.to_owned());
    spec.prompt = Some("Return the fixture provider's terminal answer. No tools.".to_owned());
    if matches!(mode, Mode::Tool { .. } | Mode::Patch) {
        spec.cwd = Some(root.join("work"));
        spec.prompt =
            Some("Execute the fixture's one command, then return its final answer.".to_owned());
    }
    let context = LaunchContext {
        scratch_root: root.to_path_buf(),
        cwd: root.join("work"),
        credentials_file: None,
        inherited_env: BTreeMap::from([
            ("HOME".to_owned(), root.join("home").display().to_string()),
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
        ]),
        memory_ancestors: Vec::new(),
        inputs_digest: None,
        plugins: Vec::new(),
        loopback: None,
    };
    let plan = plan_launch(&spec, &context).unwrap();
    assert!(plan.credential_copies.is_empty());
    assert!(plan.plugin_installs.is_empty());
    assert!(!plan.config.contains("env_key"));
    for path in [
        &plan.config_home,
        &plan.cwd,
        &root.join("home"),
        &root.join("tmp"),
    ] {
        fs::create_dir_all(path).unwrap();
    }
    fs::write(config_path(root), &plan.config).unwrap();
    let channel = HookChannelPaths::under(root);
    fs::create_dir_all(&channel.requests).unwrap();
    fs::create_dir_all(&channel.responses).unwrap();
    let hook = hook_program_path(root);
    fs::create_dir_all(hook.parent().unwrap()).unwrap();
    fs::write(&hook, hook_program(&channel)).unwrap();
    fs::set_permissions(hook, fs::Permissions::from_mode(0o700)).unwrap();
    plan
}

fn run_child(binary: &Path, plan: &LaunchPlan, root: &Path, mode: Mode) -> i32 {
    let mut command = Command::new(binary);
    command
        .args(&plan.args)
        .env_clear()
        .envs(&plan.env)
        .current_dir(&plan.cwd);
    run_command(command, root, mode)
}

struct NativeChild {
    child: Child,
    reaped: bool,
}

impl Drop for NativeChild {
    fn drop(&mut self) {
        if !self.reaped {
            let _ = Command::new("/bin/kill")
                .env_clear()
                .args(["-KILL", "--", &format!("-{}", self.child.id())])
                .status();
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn run_command(mut command: Command, root: &Path, mode: Mode) -> i32 {
    let child = command
        .stdin(Stdio::null())
        .stdout(fs::File::create(root.join("stdout.jsonl")).unwrap())
        .stderr(fs::File::create(root.join("stderr.log")).unwrap())
        .process_group(0)
        .spawn()
        .unwrap();
    let mut child = NativeChild {
        child,
        reaped: false,
    };
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        answer_hooks(root, mode);
        if let Some(status) = child.child.try_wait().unwrap() {
            child.reaped = true;
            return status.code().unwrap_or(-1);
        }
        assert!(
            Instant::now() < deadline,
            "native fixture exceeded 30 seconds; inspect retained evidence"
        );
        thread::sleep(Duration::from_millis(20));
    }
}

#[test]
fn child_is_reaped_when_fixture_servicing_panics() {
    let child = Command::new("/bin/sleep")
        .arg("30")
        .env_clear()
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()
        .unwrap();
    let pid = child.id();
    let result = std::panic::catch_unwind(move || {
        let _guard = NativeChild {
            child,
            reaped: false,
        };
        panic!("fixture servicing failed");
    });
    assert!(result.is_err());
    assert!(
        !Command::new("/bin/kill")
            .args(["-0", "--", &pid.to_string()])
            .env_clear()
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

fn answer_hooks(root: &Path, mode: Mode) {
    let channel = HookChannelPaths::under(root);
    if !channel.requests.exists() {
        return;
    }
    for entry in fs::read_dir(&channel.requests).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_none_or(|e| e != "json") {
            continue;
        }
        let response = channel.responses.join(path.file_name().unwrap());
        if response.exists() {
            continue;
        }
        let decision = if matches!(mode, Mode::Tool { deny: true, .. }) {
            Decision::Deny {
                reason: "fixture denial before the command effect".to_owned(),
            }
        } else {
            Decision::Allow
        };
        let temporary = response.with_extension("pending");
        fs::write(
            &temporary,
            serde_json::to_vec(&render_hook_response(&decision)).unwrap(),
        )
        .unwrap();
        fs::rename(temporary, response).unwrap();
    }
}

#[test]
#[ignore = "also requires absolute METAHARNESS_NATIVE_DRIVER built from this checkout; local fixture provider only"]
fn actual_binary_preserves_native_success_and_provider_failure_through_final_closure() {
    let _lease = NATIVE_RUN.lock().unwrap();
    let vendor = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CODEX").expect("explicit vendor binary"),
    );
    let driver = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_DRIVER").expect("explicit built metaharness binary"),
    );
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence directory"),
    );
    assert!(vendor.is_absolute() && driver.is_absolute() && evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    for failed in [false, true] {
        let root = tempfile::Builder::new()
            .prefix(if failed {
                "driver-refusal-"
            } else {
                "driver-success-"
            })
            .tempdir_in(&evidence)
            .unwrap()
            .keep();
        let home = root.join("home");
        let cwd = root.join("work");
        fs::create_dir_all(home.join(".local/bin")).unwrap();
        fs::create_dir_all(&cwd).unwrap();
        fs::create_dir_all(root.join("tmp")).unwrap();
        std::os::unix::fs::symlink(&vendor, home.join(".local/bin/codex")).unwrap();
        let mode = if failed { Mode::Refusal } else { Mode::Text };
        let provider = Provider::start(mode);
        let mut command = Command::new(&driver);
        command
            .args([
                "run",
                "codex",
                "--credentials",
                "none",
                "--decisions",
                "observe",
                "--model",
                MODEL,
                "--model-endpoint",
                &provider.endpoint,
                "--prompt",
                "Return the fixture terminal answer without tools.",
                "--cwd",
            ])
            .arg(&cwd)
            .arg("--retain-dir")
            .arg(root.join("retained"))
            .current_dir(&cwd)
            .env_clear()
            .env("HOME", &home)
            .env("PATH", "/usr/local/bin:/usr/bin:/bin")
            .env("LANG", "C.UTF-8")
            .env("TMPDIR", root.join("tmp"));
        let exit = run_command(command, &root, mode);
        let requests = provider.requests.lock().unwrap().clone();
        fs::write(
            root.join("requests.json"),
            serde_json::to_vec_pretty(&requests).unwrap(),
        )
        .unwrap();
        assert_eq!(requests.len(), 1);
        assert!(requests.iter().all(|r| r["authorization_present"] == false));
        assert_driver_events(&root, failed, exit);
    }
}

fn assert_driver_events(root: &Path, failed: bool, exit: i32) {
    let events: Vec<Value> = fs::read_to_string(root.join("stdout.jsonl"))
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "session.started"
                && event["harness_version"] == "0.153.4")
    );
    let terminal = events
        .iter()
        .find(|event| event["event"] == "session.ended")
        .unwrap();
    assert_eq!(terminal["is_error"], failed);
    assert!(terminal["total_cost_usd"].is_null());
    let closure = events.last().unwrap();
    assert_eq!(closure["event"], "stream.closed");
    assert_eq!(
        events
            .iter()
            .filter(|event| event["event"] == "stream.closed")
            .count(),
        1
    );
    assert_eq!(closure["events"], events.len() - 1);
    assert_eq!(
        closure["process"],
        json!({"kind":"exited","code":i32::from(failed)})
    );
    // The CLI classifies a failed run as 3; native exit 1 remains in the closure.
    assert_eq!(exit, if failed { 3 } else { 0 });
    if failed {
        assert!(terminal["final_answer"].is_null());
        assert_eq!(closure["reason"], "error");
    } else {
        assert_eq!(terminal["final_answer"]["text"], ANSWER);
        assert_eq!(closure["reason"], "completed");
    }
}

fn rollouts(path: &Path, found: &mut Vec<PathBuf>) {
    if !path.exists() {
        return;
    }
    for entry in fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rollouts(&path, found);
        } else if path.extension().is_some_and(|e| e == "jsonl") {
            found.push(path);
        }
    }
}

fn observe(root: &Path, plan: &LaunchPlan) -> Vec<Event> {
    let mut paths = Vec::new();
    rollouts(&plan.config_home.join("sessions"), &mut paths);
    assert_eq!(
        paths.len(),
        1,
        "one native rollout; inspect retained stderr"
    );
    let raw = fs::read_to_string(&paths[0]).unwrap();
    let records: Vec<Value> = raw
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    let meta = records
        .iter()
        .find(|record| record["type"] == "session_meta")
        .unwrap();
    assert_eq!(meta["payload"]["cli_version"], "0.153.4");
    assert_eq!(meta["payload"]["history_mode"], "paginated");
    let mut reader = RolloutReader::new(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        plan.attestation.clone(),
    );
    let mut events = Vec::new();
    for line in raw.lines() {
        events.extend(reader.push_line(line).into_iter().map(|e| e.event));
    }
    events.extend(reader.finish().into_iter().map(|e| e.event));
    fs::write(
        root.join("normalized.json"),
        serde_json::to_vec_pretty(&events).unwrap(),
    )
    .unwrap();
    assert!(!plan.config_home.join("auth.json").exists());
    events
}

#[test]
#[ignore = "requires explicit METAHARNESS_NATIVE_CODEX and private METAHARNESS_NATIVE_EVIDENCE; local fixture provider only"]
fn actual_codex_reports_terminal_answer_model_and_failure_from_fixture_provider() {
    let _lease = NATIVE_RUN.lock().unwrap();
    let binary = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CODEX").expect("explicit vendor binary"),
    );
    assert!(binary.is_absolute());
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence directory"),
    );
    assert!(evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    for failed in [false, true] {
        let root = tempfile::Builder::new()
            .prefix(if failed { "refusal-" } else { "success-" })
            .tempdir_in(&evidence)
            .unwrap()
            .keep();
        let mode = if failed { Mode::Refusal } else { Mode::Text };
        let provider = Provider::start(mode);
        let plan = launch(&root, &provider.endpoint, mode);
        let exit = run_child(&binary, &plan, &root, mode);
        let requests = provider.requests.lock().unwrap().clone();
        fs::write(
            root.join("requests.json"),
            serde_json::to_vec_pretty(&requests).unwrap(),
        )
        .unwrap();
        assert!(
            !requests.is_empty(),
            "native child reached the fixture provider"
        );
        assert!(requests.iter().all(|r| r["authorization_present"] == false));
        let events = observe(&root, &plan);
        let terminal = events
            .iter()
            .find_map(|e| {
                if let Event::SessionEnded {
                    is_error,
                    final_answer,
                    observed_models,
                    total_cost_usd,
                    ..
                } = e
                {
                    Some((is_error, final_answer, observed_models, total_cost_usd))
                } else {
                    None
                }
            })
            .expect("native terminal record");
        assert_eq!(*terminal.0, Some(failed));
        assert!(terminal.3.is_none(), "fixture tokens are not monetary cost");
        if failed {
            assert_ne!(exit, 0);
            assert!(terminal.1.is_none());
        } else {
            assert_eq!(exit, 0);
            assert_eq!(terminal.1.as_ref().unwrap().text, ANSWER);
            assert!(events.iter().any(|event| matches!(event,
                Event::Usage { model: None, usage, .. }
                if usage.input_tokens == Some(3) && usage.output_tokens == Some(4) && usage.cost_usd.is_none()
            )), "fixture usage is retained without model or price inference");
            assert!(
                terminal
                    .2
                    .as_ref()
                    .unwrap()
                    .iter()
                    .any(|model| model.model.as_deref() == Some(MODEL))
            );
        }
    }
}

#[test]
#[ignore = "requires explicit METAHARNESS_NATIVE_CODEX and private METAHARNESS_NATIVE_EVIDENCE; local fixture provider only"]
fn actual_codex_reports_command_success_failure_and_hook_denial() {
    let _lease = NATIVE_RUN.lock().unwrap();
    let binary = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CODEX").expect("explicit vendor binary"),
    );
    assert!(binary.is_absolute());
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence directory"),
    );
    assert!(evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    for (code, denied) in [(0, false), (7, false), (0, true)] {
        let root = tempfile::Builder::new()
            .prefix("tool-")
            .tempdir_in(&evidence)
            .unwrap()
            .keep();
        let mode = Mode::Tool {
            exit: code,
            deny: denied,
        };
        let provider = Provider::start(mode);
        let plan = launch(&root, &provider.endpoint, mode);
        let exit = run_child(&binary, &plan, &root, mode);
        let requests = provider.requests.lock().unwrap().clone();
        fs::write(
            root.join("requests.json"),
            serde_json::to_vec_pretty(&requests).unwrap(),
        )
        .unwrap();
        assert_eq!(
            exit, 0,
            "the fixture's final answer completes the turn after the tool"
        );
        assert_eq!(requests.len(), 2, "one tool call and one final answer");
        assert!(requests.iter().all(|r| r["authorization_present"] == false));
        assert_eq!(plan.cwd.join("native-fixture-marker").exists(), !denied);
        let channel = HookChannelPaths::under(&root);
        assert!(
            fs::read_dir(channel.responses).unwrap().count() > 0,
            "actual hook was answered"
        );
        let events = observe(&root, &plan);
        let outcome = events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::ToolResult {
                    call_id,
                    is_error,
                    exit_code,
                    ..
                } if call_id == "fixture-call-1" => Some((*is_error, *exit_code)),
                _ => None,
            })
            .expect("correlated native tool outcome");
        // This native version records hook denial without a CommandExecution completion.
        // The denied decision is retained separately above; prose must not become an outcome.
        assert_eq!(outcome.0, if denied { None } else { Some(code != 0) });
        assert_eq!(outcome.1, if denied { None } else { Some(code) });
    }
}

#[test]
#[ignore = "requires explicit METAHARNESS_NATIVE_CODEX and private METAHARNESS_NATIVE_EVIDENCE; local fixture provider only"]
fn actual_codex_reports_patch_outcome_without_inventing_an_exit_code() {
    let _lease = NATIVE_RUN.lock().unwrap();
    let binary = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CODEX").expect("explicit vendor binary"),
    );
    assert!(binary.is_absolute());
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence directory"),
    );
    assert!(evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    let root = tempfile::Builder::new()
        .prefix("patch-")
        .tempdir_in(&evidence)
        .unwrap()
        .keep();
    let provider = Provider::start(Mode::Patch);
    let plan = launch(&root, &provider.endpoint, Mode::Patch);
    assert_eq!(run_child(&binary, &plan, &root, Mode::Patch), 0);
    assert_eq!(
        fs::read_to_string(plan.cwd.join("native-fixture-patch")).unwrap(),
        "native-fixture-content\n"
    );
    let requests = provider.requests.lock().unwrap().clone();
    fs::write(
        root.join("requests.json"),
        serde_json::to_vec_pretty(&requests).unwrap(),
    )
    .unwrap();
    assert_eq!(requests.len(), 2);
    assert!(requests.iter().all(|r| r["authorization_present"] == false));
    let events = observe(&root, &plan);
    let outcome = events
        .iter()
        .rev()
        .find_map(|event| match event {
            Event::ToolResult {
                call_id,
                is_error,
                exit_code,
                ..
            } if call_id == "fixture-patch-1" => Some((*is_error, *exit_code)),
            _ => None,
        })
        .expect("correlated native patch completion");
    assert_eq!(outcome, (Some(false), None));
}
