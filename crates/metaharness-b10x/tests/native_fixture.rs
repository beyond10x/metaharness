//! Opt-in actual b10x observations through the production CLI and an owned Responses fixture.
//! Run the entire test process in a network namespace containing only loopback. Fixture token
//! counts and vendor-computed prices are synthetic inputs, never evidence of paid model usage.
#![cfg(target_os = "linux")]

use serde_json::{Value, json};
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

const MODEL: &str = "b10x-native-fixture";
const ANSWER: &str = "native-b10x-fixture-final";
static NATIVE_RUN: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy)]
enum Mode {
    Text,
    Refusal,
    Tool { missing: bool },
    RepeatTool,
    ProcessWrites { declared: bool },
    Hang { command: &'static str },
}

struct Provider {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    errors: Arc<Mutex<Vec<String>>>,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
    evidence: PathBuf,
}

impl Provider {
    fn start(mode: Mode, evidence: &Path) -> Self {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        listener.set_nonblocking(true).unwrap();
        let endpoint = format!("http://{}", listener.local_addr().unwrap());
        let requests = Arc::new(Mutex::new(Vec::new()));
        let errors = Arc::new(Mutex::new(Vec::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let seen = Arc::clone(&requests);
        let problems = Arc::clone(&errors);
        let stop = Arc::clone(&stopping);
        let root = evidence.to_owned();
        let worker = thread::spawn(move || {
            let mut turn = 0;
            while !stop.load(Ordering::SeqCst) {
                match listener.accept() {
                    Ok((stream, _)) => {
                        if let Err(error) = serve(stream, mode, &mut turn, &seen, &stop, &root) {
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
            evidence: evidence.to_owned(),
        }
    }
}

impl Drop for Provider {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        // Preserve the producer's record even when a native deadline/assertion panics.
        for (name, values) in [
            (
                "requests.json",
                serde_json::to_vec_pretty(&*self.requests.lock().unwrap()),
            ),
            (
                "provider-errors.json",
                serde_json::to_vec_pretty(&*self.errors.lock().unwrap()),
            ),
        ] {
            if let Ok(bytes) = values {
                let _ = fs::write(self.evidence.join(name), bytes);
            }
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
    let mut api_key = false;
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
                    api_key = true;
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
    reader.read_exact(&mut bytes)?;
    let body: Value = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).map_err(io::Error::other)?
    };
    // Retain only credential predicates, never an unexpected header's value.
    Ok(
        json!({"request": request, "api_key_present": api_key, "key_present": key_present, "authorization_present": authorization, "body": body}),
    )
}

fn serve(
    mut stream: TcpStream,
    mode: Mode,
    turn: &mut usize,
    seen: &Mutex<Vec<Value>>,
    stopping: &AtomicBool,
    root: &Path,
) -> io::Result<()> {
    let observed = request(&stream)?;
    seen.lock().unwrap().push(observed.clone());
    if observed["authorization_present"] != false || observed["api_key_present"] != false {
        return Err(invalid("fixture refuses any credential header"));
    }
    if observed["request"] != "POST /responses HTTP/1.1" {
        return Err(invalid("unexpected endpoint"));
    }
    if matches!(mode, Mode::Hang { .. }) {
        let deadline = Instant::now() + Duration::from_secs(40);
        while !stopping.load(Ordering::SeqCst) && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        return Ok(());
    }
    if matches!(mode, Mode::Refusal) {
        return reply(&mut stream, "400 Bad Request", "application/json", &json!({"error":{"message":"fixture model refusal","type":"invalid_request_error","code":"model_not_found"}}).to_string());
    }
    if let Mode::ProcessWrites { declared } = mode {
        let offered = observed["body"]["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|t| t["name"] == "run"));
        *turn += 1;
        let response = if *turn == 1 && offered {
            let probe = PathBuf::from(
                std::env::var_os("METAHARNESS_PROCESS_WRITE_PROBE").expect("explicit Rust probe"),
            );
            let mut argv = vec![
                format!(
                    "/toolchain/driver/{}",
                    probe.file_name().unwrap().to_str().unwrap()
                ),
                "--outside-path".to_owned(),
                root.join("outside/native-fixture-probe")
                    .display()
                    .to_string(),
            ];
            if declared {
                argv.push("--expect-writable".to_owned());
            }
            let item = json!({"id":"fc-process","type":"function_call","status":"completed","name":"run","call_id":"fixture-process","arguments":json!({"argv":argv}).to_string()});
            item_stream(&item, true, *turn)
        } else {
            response_stream(false, false, *turn)
        };
        return reply(&mut stream, "200 OK", "text/event-stream", &response);
    }
    let tool =
        matches!(mode, Mode::RepeatTool) || (matches!(mode, Mode::Tool { .. }) && *turn == 0);
    if tool
        && !observed["body"]["tools"]
            .as_array()
            .is_some_and(|tools| tools.iter().any(|t| t["name"] == "file_read"))
    {
        return Err(invalid("native request did not offer file_read"));
    }
    *turn += 1;
    let missing = matches!(mode, Mode::Tool { missing: true });
    reply(
        &mut stream,
        "200 OK",
        "text/event-stream",
        &response_stream(tool, missing, *turn),
    )
}

fn response_stream(tool: bool, missing: bool, turn: usize) -> String {
    let arguments = json!({"path": if missing {"missing.txt"} else {"fixture.txt"}}).to_string();
    let item = if tool {
        json!({"id":format!("fc-{turn}"),"type":"function_call","status":"completed","name":"file_read","call_id":format!("fixture-call-{turn}"),"arguments":arguments})
    } else {
        json!({"id":format!("msg-{turn}"),"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":ANSWER,"annotations":[]}]})
    };
    item_stream(&item, tool, turn)
}

fn item_stream(item: &Value, tool: bool, turn: usize) -> String {
    let response = json!({"id":format!("fixture-response-{turn}"),"object":"response","status":"completed","model":MODEL,"output":[item],"usage":{"input_tokens":3,"output_tokens":4,"input_tokens_details":{"cached_tokens":1},"total_tokens":7}});
    let mut events = vec![
        json!({"type":"response.created","response":{"id":response["id"],"object":"response","status":"in_progress","model":MODEL,"output":[]}}),
    ];
    if !tool {
        events.push(json!({"type":"response.output_text.delta","item_id":item["id"],"output_index":0,"content_index":0,"delta":ANSWER}));
    }
    events.push(json!({"type":"response.output_item.done","output_index":0,"item":item}));
    events.push(json!({"type":"response.completed","response":response}));
    let mut wire = String::new();
    for event in events {
        writeln!(wire, "data: {event}\n").unwrap();
    }
    wire.push_str("data: [DONE]\n\n");
    wire
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
        Ok("loopback-only")
    );
    let interfaces: Vec<String> = fs::read_to_string("/proc/net/dev")
        .unwrap()
        .lines()
        .skip(2)
        .map(|line| line.split_once(':').unwrap().0.trim().to_owned())
        .collect();
    assert_eq!(interfaces, ["lo"], "fixture refuses external interfaces");
}

fn event_lines(root: &Path) -> Vec<Value> {
    fs::read_to_string(root.join("events.jsonl"))
        .unwrap()
        .split_inclusive('\n')
        .filter(|line| line.ends_with('\n'))
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

#[allow(clippy::too_many_lines)]
fn run(mode: Mode, extra: &[&str]) -> (PathBuf, Vec<Value>, Vec<Value>, i32) {
    require_isolation();
    let vendor =
        PathBuf::from(std::env::var_os("METAHARNESS_NATIVE_B10X").expect("explicit actual vendor"));
    let driver = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_DRIVER").expect("explicit production driver"),
    );
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence root"),
    );
    assert!(vendor.is_absolute() && driver.is_absolute() && evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    let root = tempfile::Builder::new()
        .prefix("b10x-")
        .tempdir_in(&evidence)
        .unwrap()
        .keep();
    let home = root.join("home");
    let cwd = root.join("work");
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(root.join("tmp")).unwrap();
    fs::write(cwd.join("fixture.txt"), "owned-native-fixture-content").unwrap();
    if matches!(mode, Mode::ProcessWrites { .. }) {
        for path in [
            cwd.join("target"),
            cwd.join("generated"),
            cwd.join("src"),
            root.join("outside"),
        ] {
            fs::create_dir(&path).unwrap();
        }
    }
    std::os::unix::fs::symlink(vendor, home.join(".local/bin/b10x-harness")).unwrap();
    let provider = Provider::start(mode, &root);
    let mut command = Command::new(driver);
    command
        .args([
            "run",
            "b10x",
            "--credentials",
            "none",
            "--decisions",
            "observe",
            "--model",
            MODEL,
            "--model-endpoint",
            &provider.endpoint,
            "--model-wire",
            "openai-responses",
            "--prompt",
            "Perform only the fixture's file read, then return its answer.",
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
        .args(extra)
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
    if let Mode::ProcessWrites { declared } = mode {
        let probe = PathBuf::from(
            std::env::var_os("METAHARNESS_PROCESS_WRITE_PROBE")
                .expect("explicit Rust containment probe"),
        );
        let cgroup = PathBuf::from(
            std::env::var_os("METAHARNESS_NATIVE_CGROUP").expect("explicit delegated cgroup root"),
        );
        assert!(probe.is_absolute() && probe.is_file() && cgroup.is_absolute() && cgroup.is_dir());
        command
            .arg("--substrate-embedded")
            .arg("--driver")
            .arg(probe)
            .arg("--cgroup-root")
            .arg(cgroup);
        if declared {
            command.args([
                "--process-write-subtree",
                "target",
                "--process-write-subtree",
                "generated",
            ]);
        }
    }
    let mut child = NativeChild {
        child: command.spawn().unwrap(),
        reaped: false,
    };
    let mut input = child.child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let mut cancelled = false;
    let mut cancel_deadline = None;
    let status = loop {
        if let Mode::Hang { command } = mode
            && !cancelled
            && !provider.requests.lock().unwrap().is_empty()
            && event_lines(&root)
                .iter()
                .any(|event| event["event"] == "session.started")
        {
            writeln!(input, "{}", json!({"format":"metaharness.command/1","id":"fixture-cancel","command":command,"reason":"owned fixture cancellation"})).unwrap();
            cancelled = true;
            cancel_deadline = Some(Instant::now() + Duration::from_secs(2));
        }
        if let Some(status) = child.child.try_wait().unwrap() {
            child.reaped = true;
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "native fixture deadline; private evidence: {}",
            root.display()
        );
        assert!(
            cancel_deadline.is_none_or(|end| Instant::now() < end),
            "quiet cancellation exceeded two seconds; private evidence: {}",
            root.display()
        );
        thread::sleep(Duration::from_millis(20));
    };
    let requests = provider.requests.lock().unwrap().clone();
    fs::write(
        root.join("requests.json"),
        serde_json::to_vec_pretty(&requests).unwrap(),
    )
    .unwrap();
    let errors = provider.errors.lock().unwrap().clone();
    fs::write(
        root.join("provider-errors.json"),
        serde_json::to_vec_pretty(&errors).unwrap(),
    )
    .unwrap();
    assert!(
        errors.is_empty(),
        "fixture provider errors: {errors:?}; {}",
        root.display()
    );
    let events = event_lines(&root);
    eprintln!("private native evidence: {}", root.display());
    (root, events, requests, status.code().unwrap_or(-1))
}

fn one<'a>(events: &'a [Value], name: &str) -> &'a Value {
    let found: Vec<&Value> = events.iter().filter(|e| e["event"] == name).collect();
    assert_eq!(found.len(), 1, "one {name}: {events:?}");
    found[0]
}

fn assert_closed(events: &[Value], code: i32) {
    let closed = one(events, "stream.closed");
    assert_eq!(events.last(), Some(closed));
    assert_eq!(closed["process"], json!({"kind":"exited","code":code}));
    let start = one(events, "session.started");
    assert_eq!(start["harness_version"], "0.13.3");
    assert_eq!(start["model"], MODEL);
    assert_eq!(start["credential_source"], "none");
    assert_eq!(start["hermetic"]["decisions"], "observe");
}

#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_terminal_success_failure_and_fixture_usage() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for mode in [Mode::Text, Mode::Refusal] {
        let (_, events, requests, code) = run(mode, &[]);
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["body"]["model"], MODEL);
        let failure = matches!(mode, Mode::Refusal);
        assert_closed(&events, i32::from(failure));
        assert_eq!(code, if failure { 3 } else { 0 });
        if failure {
            assert!(!events.iter().any(|event| event["event"] == "session.ended"));
            assert!(
                events
                    .iter()
                    .any(|event| event["event"] == "warning"
                        && event["code"] == "NO_TERMINAL_RECORD")
            );
            assert_eq!(one(&events, "stream.closed")["reason"], "error");
        } else {
            let ended = one(&events, "session.ended");
            assert_eq!(ended["is_error"], false);
            assert!(ended["total_cost_usd"].is_null());
            assert!(
                events
                    .iter()
                    .any(|e| e["event"] == "text" && e["text"] == ANSWER)
            );
            let usage = one(&events, "usage");
            assert_eq!(usage["model"], MODEL);
            assert_eq!(usage["usage"]["input_tokens"], 3);
            assert_eq!(usage["usage"]["output_tokens"], 4);
            assert_eq!(usage["usage"]["cache_read_input_tokens"], 1);
            assert!(usage["usage"]["cost_usd"].is_null());
        }
    }
}

#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_tools_are_observed_without_decisions() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for missing in [false, true] {
        let (_, events, requests, code) = run(Mode::Tool { missing }, &[]);
        assert_eq!(requests.len(), 2);
        assert_eq!(code, 0);
        assert_closed(&events, 0);
        let tool = one(&events, "tool.requested");
        assert_eq!(tool["name"], "file_read");
        assert_eq!(tool["decision_required"], false);
        assert_eq!(tool["seam"], "none");
        let result = one(&events, "tool.result");
        assert_eq!(result["call_id"], tool["call_id"]);
        assert_eq!(result["is_error"], missing);
        assert!(result["exit_code"].is_null());
        assert_eq!(one(&events, "session.ended")["census"]["allowed"], 0);
        assert_eq!(one(&events, "session.ended")["census"]["denied"], 0);
        assert!(!events.iter().any(|e| e["event"] == "tool.decided"));
        let output = requests[1]["body"]["input"]
            .as_array()
            .unwrap()
            .iter()
            .find(|i| i["type"] == "function_call_output")
            .expect("actual tool result replayed");
        assert_eq!(output["call_id"], tool["call_id"]);
        if !missing {
            assert!(
                output["output"]
                    .to_string()
                    .contains("owned-native-fixture-content")
            );
        }
    }
}

#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_declared_turn_ceiling_stops_repeated_tools() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_, events, requests, code) = run(Mode::RepeatTool, &[]);
    assert_eq!(requests.len(), 1);
    assert_eq!(code, 3);
    assert_closed(&events, 2);
    assert_eq!(
        events
            .iter()
            .filter(|e| e["event"] == "turn.started")
            .count(),
        1
    );
    assert_eq!(
        one(&events, "session.ended")["terminal_reason"],
        "max-turns"
    );
}

#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_strict_version_refuses_the_unqualified_binary_before_a_request() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (root, events, requests, code) = run(Mode::Text, &["--strict-version"]);
    assert_eq!(code, 2);
    assert_eq!(requests, [] as [Value; 0]);
    assert_eq!(events, [] as [Value; 0]);
    let refusal = fs::read_to_string(root.join("stderr.log")).unwrap();
    assert!(
        refusal.contains("0.13.3") && refusal.contains("0.12.1"),
        "{refusal}"
    );
}

#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_cancellation_closes_a_quiet_native_request() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for command in ["halt", "interrupt"] {
        let (_, events, requests, code) = run(Mode::Hang { command }, &[]);
        assert_eq!(requests.len(), 1);
        assert_eq!(code, 3);
        let result = one(&events, "command.result");
        assert_eq!(result["id"], "fixture-cancel");
        assert_eq!(result["outcome"]["result"], "ok");
        let closed = one(&events, "stream.closed");
        assert_eq!(events.last(), Some(closed));
        assert_eq!(closed["process"], json!({"kind":"signaled","signal":9}));
        assert!(
            !events
                .iter()
                .any(|event| event["event"] == "session.ended" && event["is_error"] == false)
        );
        assert_eq!(
            closed["reason"],
            if command == "halt" {
                "steer-halt"
            } else {
                "error"
            }
        );
    }
}

#[test]
#[ignore = "actual confined b10x; explicit binaries, Rust probe, delegated cgroup and loopback-only namespace"]
fn actual_b10x_process_writes_are_contained_or_explicitly_withheld() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for declared in [false, true] {
        let (root, events, requests, code) = run(Mode::ProcessWrites { declared }, &[]);
        assert_eq!(code, 0);
        assert_closed(&events, 0);
        let start = one(&events, "session.started");
        let run_offered = start["offered_tools"]
            .as_array()
            .unwrap()
            .iter()
            .any(|tool| tool == "run");
        assert!(!root.join("work/src/native-fixture-probe").exists());
        assert!(!root.join("outside/native-fixture-probe").exists());
        let outcome = if run_offered {
            assert_eq!(requests.len(), 2);
            let call = one(&events, "tool.requested");
            assert_eq!(call["name"], "run");
            assert_eq!(call["decision_required"], false);
            let result = one(&events, "tool.result");
            assert_eq!(result["call_id"], "fixture-process");
            assert_eq!(
                result["is_error"], false,
                "the Rust probe must finish all write assertions"
            );
            for directory in ["target", "generated"] {
                assert_eq!(
                    root.join("work")
                        .join(directory)
                        .join("native-fixture-probe")
                        .exists(),
                    declared
                );
            }
            "contained"
        } else {
            assert_eq!(requests.len(), 1);
            assert!(!events.iter().any(|e| e["event"] == "tool.requested"));
            assert!(
                start["withheld"]
                    .as_array()
                    .is_some_and(|items| !items.is_empty()),
                "native withholding evidence is required: {start}"
            );
            for directory in ["target", "generated"] {
                assert!(
                    !root
                        .join("work")
                        .join(directory)
                        .join("native-fixture-probe")
                        .exists()
                );
            }
            "withheld-unverified"
        };
        fs::write(root.join("containment-result.json"), serde_json::to_vec_pretty(&json!({"outcome":outcome,"declared":declared,"native_version":start["harness_version"],"withheld":start["withheld"]})).unwrap()).unwrap();
        eprintln!("process containment declaration={declared}: {outcome}");
    }
}
