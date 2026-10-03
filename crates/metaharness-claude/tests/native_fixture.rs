//! Opt-in actual Claude observations through the production CLI and an owned Messages fixture.
//! Run the entire test process in a network namespace containing only loopback. Fixture token
//! counts and vendor-computed prices are synthetic inputs, never evidence of paid model usage.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

const MODEL: &str = "claude-sonnet-4-5-20250929";
const ANSWER: &str = "native-claude-fixture-final";
static NATIVE_RUN: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
enum Mode {
    Text,
    Refusal,
    Tool { deny: bool },
    RepeatTool,
    Hang { command: &'static str },
}

struct Provider {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    errors: Arc<Mutex<Vec<String>>>,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Provider {
    fn start(mode: Mode) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&requests);
        let problems = Arc::clone(&errors);
        let stop = Arc::clone(&stopping);
        let worker = thread::spawn(move || {
            let mut turn = 0;
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Err(error) = serve(stream, mode, &mut turn, &seen, &stop) {
                            problems.lock().unwrap().push(error.to_string());
                        }
                    }
                    Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(10));
                    }
                    Err(error) => {
                        problems.lock().unwrap().push(error.to_string());
                        return;
                    }
                }
            }
        });
        Self {
            endpoint,
            requests,
            errors,
            stopping,
            worker: Some(worker),
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn invalid(message: &str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}

fn request(stream: &TcpStream) -> io::Result<Value> {
    let deadline = Instant::now() + Duration::from_secs(3);
    stream.set_write_timeout(Some(Duration::from_secs(3)))?;
    let mut reader = BufReader::new(stream.try_clone()?);
    let mut headers = Vec::new();
    let mut header_bytes = 0;
    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| invalid("header deadline"))?;
        reader.get_mut().set_read_timeout(Some(remaining))?;
        let mut line = String::new();
        let count = reader.by_ref().take(16 * 1024).read_line(&mut line)?;
        header_bytes += count;
        if count == 0 || header_bytes > 16 * 1024 {
            return Err(invalid("bounded HTTP headers"));
        }
        if line == "\r\n" {
            break;
        }
        headers.push(line);
    }
    let request = headers
        .first()
        .ok_or_else(|| invalid("request line"))?
        .trim();
    let mut length = 0;
    let mut placeholder = false;
    let mut key_present = false;
    let mut authorization = false;
    for line in &headers[1..] {
        if let Some((name, value)) = line.split_once(':') {
            match name.to_ascii_lowercase().as_str() {
                "content-length" => {
                    length = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| invalid("content length"))?;
                }
                "x-api-key" => {
                    key_present = true;
                    placeholder = value.trim() == "metaharness-model-endpoint";
                }
                "authorization" => authorization = true,
                _ => {}
            }
        }
    }
    if length > 2 * 1024 * 1024 {
        return Err(invalid("bounded request body"));
    }
    let mut bytes = vec![0; length];
    reader.get_mut().set_read_timeout(Some(
        deadline
            .checked_duration_since(Instant::now())
            .ok_or_else(|| invalid("body deadline"))?,
    ))?;
    let mut read = 0;
    while read < length {
        reader.get_mut().set_read_timeout(Some(
            deadline
                .checked_duration_since(Instant::now())
                .ok_or_else(|| invalid("body deadline"))?,
        ))?;
        let count = reader.read(&mut bytes[read..])?;
        if count == 0 {
            return Err(invalid("incomplete request body"));
        }
        read += count;
    }
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).map_err(io::Error::other)?
    };
    // Retain only credential predicates, never an unexpected header's value.
    Ok(
        json!({"request": request, "placeholder": placeholder, "key_present": key_present, "authorization_present": authorization, "body": body}),
    )
}

fn serve(
    mut stream: TcpStream,
    mode: Mode,
    turn: &mut usize,
    seen: &Mutex<Vec<Value>>,
    stopping: &AtomicBool,
) -> io::Result<()> {
    let observed = request(&stream)?;
    seen.lock().unwrap().push(observed.clone());
    let request = observed["request"].as_str().unwrap();
    let placeholder = observed["placeholder"] == true;
    let authorization = observed["authorization_present"] == true;
    let body = &observed["body"];
    if request == "HEAD /api/hello HTTP/1.1" && observed["key_present"] == false && !authorization {
        return reply(&mut stream, "200 OK", "application/json", "");
    }
    if !placeholder || authorization {
        return Err(io::Error::other(format!(
            "unexpected credential source on {request}"
        )));
    }
    if request.contains("/count_tokens") {
        return reply(
            &mut stream,
            "200 OK",
            "application/json",
            &json!({"input_tokens":3}).to_string(),
        );
    }
    if !request.starts_with("POST /v1/messages") {
        return Err(io::Error::other(format!("unexpected endpoint: {request}")));
    }
    if matches!(mode, Mode::Hang { .. }) {
        let hang_deadline = Instant::now() + Duration::from_secs(40);
        while !stopping.load(Ordering::SeqCst) && Instant::now() < hang_deadline {
            thread::sleep(Duration::from_millis(10));
        }
        return Ok(());
    }
    if matches!(mode, Mode::Refusal) {
        return reply(&mut stream, "400 Bad Request", "application/json", &json!({"type":"error","error":{"type":"invalid_request_error","message":"fixture model refusal"}}).to_string());
    }
    let tool =
        matches!(mode, Mode::RepeatTool) || (matches!(mode, Mode::Tool { .. }) && *turn == 0);
    if tool
        && !body["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|t| t["name"] == "Bash"))
    {
        return Err(invalid("native request did not offer Bash"));
    }
    *turn += 1;
    respond_message(&mut stream, body, tool, *turn)
}

fn respond_message(
    stream: &mut TcpStream,
    body: &Value,
    tool: bool,
    turn: usize,
) -> io::Result<()> {
    let content = if tool {
        json!({"type":"tool_use","id":format!("fixture-call-{turn}"),"name":"Bash","input":{"command":"printf marker > native-fixture-marker; printf native-fixture-tool","description":"Write the owned fixture marker"}})
    } else {
        json!({"type":"text","text":ANSWER})
    };
    let reason = if tool { "tool_use" } else { "end_turn" };
    let response = json!({"id":format!("fixture-message-{turn}"),"type":"message","role":"assistant","model":MODEL,"content":[content],"stop_reason":reason,"stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":4}});
    if body["stream"] != true {
        return reply(stream, "200 OK", "application/json", &response.to_string());
    }
    let start = if tool {
        json!({"type":"tool_use","id":content["id"],"name":"Bash","input":{}})
    } else {
        json!({"type":"text","text":""})
    };
    let delta = if tool {
        json!({"type":"input_json_delta","partial_json":content["input"].to_string()})
    } else {
        json!({"type":"text_delta","text":ANSWER})
    };
    let events = [
        json!({"type":"message_start","message":{"id":response["id"],"type":"message","role":"assistant","model":MODEL,"content":[],"stop_reason":null,"stop_sequence":null,"usage":{"input_tokens":3,"output_tokens":0}}}),
        json!({"type":"content_block_start","index":0,"content_block":start}),
        json!({"type":"content_block_delta","index":0,"delta":delta}),
        json!({"type":"content_block_stop","index":0}),
        json!({"type":"message_delta","delta":{"stop_reason":reason,"stop_sequence":null},"usage":{"output_tokens":4}}),
        json!({"type":"message_stop"}),
    ];
    let mut wire = String::new();
    for event in events {
        writeln!(
            wire,
            "event: {}\ndata: {event}\n",
            event["type"].as_str().unwrap()
        )
        .unwrap();
    }
    reply(stream, "200 OK", "text/event-stream", &wire)
}

fn reply(stream: &mut TcpStream, status: &str, mime: &str, body: &str) -> io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status}\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
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

fn require_isolation() {
    assert_eq!(
        std::env::var("METAHARNESS_NATIVE_NETNS").as_deref(),
        Ok("loopback-only"),
        "run in the documented isolated network namespace"
    );
    let interfaces: Vec<String> = fs::read_to_string("/proc/net/dev")
        .unwrap()
        .lines()
        .skip(2)
        .map(|line| line.split_once(':').unwrap().0.trim().to_owned())
        .collect();
    assert_eq!(
        interfaces,
        ["lo"],
        "the fixture refuses an externally connected namespace"
    );
}

#[test]
fn fixture_child_is_reaped_during_unwind() {
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

fn event_lines(root: &Path) -> Vec<Value> {
    // An in-flight final line can be partial; finalized output is checked strictly below.
    fs::read_to_string(root.join("events.jsonl"))
        .unwrap()
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn run(mode: Mode) -> (PathBuf, Vec<Value>, i32) {
    require_isolation();
    let vendor = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CLAUDE").expect("explicit installed vendor"),
    );
    let driver = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_DRIVER").expect("explicit production driver"),
    );
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence root"),
    );
    assert!(vendor.is_absolute() && driver.is_absolute() && evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    let root = tempfile::Builder::new()
        .prefix("claude-")
        .tempdir_in(&evidence)
        .unwrap()
        .keep();
    let home = root.join("home");
    let cwd = root.join("work");
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(root.join("tmp")).unwrap();
    std::os::unix::fs::symlink(vendor, home.join(".local/bin/claude")).unwrap();
    let provider = Provider::start(mode);
    let mut command = driver_command(&driver, &root, &provider.endpoint, mode);
    let mut child = NativeChild {
        child: command.spawn().unwrap(),
        reaped: false,
    };
    let mut input = child.child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(45);
    let mut decisions = BTreeSet::new();
    let mut cancelled = false;
    let mut cancel_deadline = None;
    let mut started = false;
    let status = loop {
        for event in event_lines(&root) {
            started |= event["event"] == "session.started";
            if event["event"] == "tool.requested" && event["decision_required"] == true {
                let call = event["call_id"].as_str().unwrap();
                if decisions.insert(call.to_owned()) {
                    let decision = if matches!(mode, Mode::Tool { deny: true }) {
                        json!({"decision":"deny","reason":"fixture refuses the marker effect"})
                    } else {
                        json!({"decision":"allow"})
                    };
                    writeln!(input, "{}", json!({"format":"metaharness.command/1","id":format!("decision-{call}"),"command":"tool.decide","call_id":call,"decision":decision})).unwrap();
                }
            }
        }
        if let Mode::Hang { command } = mode
            && !cancelled
            && started
            && provider.requests.lock().unwrap().iter().any(|r| {
                r["request"]
                    .as_str()
                    .unwrap()
                    .starts_with("POST /v1/messages")
                    && !r["request"].as_str().unwrap().contains("count_tokens")
            })
        {
            writeln!(input, "{}", json!({"format":"metaharness.command/1","id":"fixture-cancel","command":command,"reason":"owned fixture cancellation"})).unwrap();
            cancelled = true;
            cancel_deadline = Some(Instant::now() + Duration::from_secs(2));
        }
        if let Some(status) = child.child.try_wait().unwrap() {
            child.reaped = true;
            break status;
        }
        if cancel_deadline.is_some_and(|limit| Instant::now() >= limit) {
            retain_provider(&root, &provider);
            panic!(
                "steering did not close the native process within two seconds: {}",
                root.display()
            );
        }
        assert!(
            Instant::now() < deadline,
            "native fixture exceeded 45 seconds; private evidence: {}",
            root.display()
        );
        thread::sleep(Duration::from_millis(20));
    };
    let events = collect_evidence(&root, &provider);
    (root, events, status.code().unwrap_or(-1))
}

fn driver_command(driver: &Path, root: &Path, endpoint: &str, mode: Mode) -> Command {
    let home = root.join("home");
    let cwd = root.join("work");
    let mut command = Command::new(driver);
    command
        .args([
            "run",
            "claude",
            "--credentials",
            "none",
            "--decisions",
            "ask",
            "--model",
            MODEL,
            "--model-endpoint",
            endpoint,
            "--prompt",
            "Perform only the owned fixture tool call when provided, then return its final answer.",
            "--max-turns",
            if matches!(mode, Mode::RepeatTool) {
                "1"
            } else {
                "3"
            },
            "--cwd",
        ])
        .arg(&cwd)
        .arg("--retain-dir")
        .arg(root.join("retained"))
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("TMPDIR", root.join("tmp"))
        .current_dir(&cwd)
        .stdin(Stdio::piped())
        .stdout(fs::File::create(root.join("events.jsonl")).unwrap())
        .stderr(fs::File::create(root.join("stderr.log")).unwrap())
        .process_group(0);
    command
}

fn collect_evidence(root: &Path, provider: &Provider) -> Vec<Value> {
    retain_provider(root, provider);
    let requests = provider.requests.lock().unwrap().clone();
    let errors = provider.errors.lock().unwrap().clone();
    assert!(errors.is_empty(), "fixture server errors: {errors:?}");
    assert!(
        !requests.is_empty(),
        "native process never reached fixture: {}",
        root.display()
    );
    assert!(requests.iter().all(|r| (r["placeholder"] == true
        || (r["request"] == "HEAD /api/hello HTTP/1.1" && r["key_present"] == false))
        && r["authorization_present"] == false));
    let text = fs::read_to_string(root.join("events.jsonl")).unwrap();
    let events: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "session.started"
                && event["harness_version"] == "2.1.288")
    );
    let closure = events.last().unwrap();
    assert_eq!(closure["event"], "stream.closed");
    assert_eq!(closure["events"], events.len() - 1);
    assert_eq!(
        events
            .iter()
            .filter(|e| e["event"] == "stream.closed")
            .count(),
        1
    );
    events
}

fn retain_provider(root: &Path, provider: &Provider) {
    fs::write(
        root.join("requests.json"),
        serde_json::to_vec_pretty(&*provider.requests.lock().unwrap()).unwrap(),
    )
    .unwrap();
    fs::write(
        root.join("provider-errors.json"),
        serde_json::to_vec_pretty(&*provider.errors.lock().unwrap()).unwrap(),
    )
    .unwrap();
}

#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_success_failure_model_usage_and_extension_inventory() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for failed in [false, true] {
        let (_, events, exit) = run(if failed { Mode::Refusal } else { Mode::Text });
        let started = events
            .iter()
            .find(|e| e["event"] == "session.started")
            .unwrap();
        assert_eq!(started["model"], MODEL);
        // Native built-ins are observed, not silently removed to make H1a look satisfied.
        let names: BTreeSet<&str> = started["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            BTreeSet::from([
                "cc-plugin-agents-md",
                "cc-plugin-telemetry",
                "cc-plugin-plugin-authoring"
            ])
        );
        assert!(
            started["plugins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|plugin| plugin["source"]
                    == format!("{}@builtin", plugin["name"].as_str().unwrap()))
        );
        assert_eq!(started["mcp_servers"], json!([]));
        let terminal = events
            .iter()
            .find(|e| e["event"] == "session.ended")
            .unwrap();
        assert_eq!(terminal["is_error"], failed);
        assert_eq!(exit, if failed { 3 } else { 0 });
        assert_eq!(
            events.last().unwrap()["process"],
            json!({"kind":"exited","code":i32::from(failed)})
        );
        assert_eq!(
            events.last().unwrap()["reason"],
            if failed { "error" } else { "completed" }
        );
        if !failed {
            assert!(
                events
                    .iter()
                    .any(|e| e["event"] == "text" && e["text"] == ANSWER)
            );
            assert!(events.iter().any(|e| e["event"] == "usage"
                && e["model"] == MODEL
                && e["usage"]["input_tokens"] == 3
                && e["usage"]["output_tokens"] == 0));
            assert_eq!(terminal["usage"]["input_tokens"], 3);
            assert_eq!(terminal["usage"]["output_tokens"], 4);
        }
    }
}

#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_hook_allows_and_denies_the_real_marker_effect() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for deny in [false, true] {
        let (root, events, exit) = run(Mode::Tool { deny });
        assert_eq!(exit, 0);
        assert_eq!(root.join("work/native-fixture-marker").exists(), !deny);
        assert!(events.iter().any(|e| e["event"] == "tool.decided"
            && e["decision"]["decision"] == if deny { "deny" } else { "allow" }));
        let result = events
            .iter()
            .find(|e| e["event"] == "tool.result" && e["call_id"] == "fixture-call-1")
            .unwrap();
        assert_eq!(result["is_error"], deny);
    }
}

#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_interrupt_and_halt_close_a_waiting_provider_run() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for command in ["interrupt", "halt"] {
        let (_, events, exit) = run(Mode::Hang { command });
        assert_ne!(exit, 0);
        assert!(events.iter().any(|e| e["event"] == "command.result"
            && e["id"] == "fixture-cancel"
            && e["outcome"]["result"] == "ok"));
        assert_ne!(events.last().unwrap()["reason"], "completed");
    }
}

#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_max_turns_stops_a_fixture_that_keeps_requesting_tools() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_, events, exit) = run(Mode::RepeatTool);
    assert_ne!(exit, 0);
    let terminal = events
        .iter()
        .find(|e| e["event"] == "session.ended")
        .unwrap();
    assert_eq!(terminal["subtype"], "error_max_turns");
    assert_eq!(terminal["is_error"], true);
}
