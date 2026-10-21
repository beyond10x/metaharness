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
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

const MODEL: &str = "claude-sonnet-4-5-20250929";
const ANSWER: &str = "native-claude-fixture-final";
/// The token the `loopback` fixture puts in custody: written by this test into its own scratch
/// HOME, issued by no account, and worth nothing anywhere.
const CUSTODY_TOKEN: &str = "native-fixture-custody-token-issued-by-no-account";
/// The placeholder `credentials: none` hands the child for a declared endpoint.
const ENDPOINT_PLACEHOLDER: &str = "metaharness-model-endpoint";
/// The built-ins a 2.1.304 session opens with under either credential-free mode, observed: the
/// launch sets `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC` in both, which keeps
/// `cc-plugin-telemetry` out (design amendment a33).
const BUILTINS_SEEN: [&str; 2] = ["cc-plugin-agents-md", "cc-plugin-plugin-authoring"];
static NATIVE_RUN: Mutex<()> = Mutex::new(());

/// The two credential-free modes the fixture drives. Neither puts an operator credential anywhere:
/// `none` hands the child a placeholder for the stub, and `loopback` puts this test's own fake
/// token in custody behind metaharness's proxy, whose upstream is the stub.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Credentials {
    None,
    Loopback,
}

impl Credentials {
    const ALL: [Self; 2] = [Self::None, Self::Loopback];

    fn flag(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Loopback => "loopback",
        }
    }
}

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
    fn start(mode: Mode, credentials: Credentials) -> Self {
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
                        if let Err(error) =
                            serve(stream, mode, credentials, &mut turn, &seen, &stop)
                        {
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
    let mut custody = false;
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
                    placeholder = value.trim() == ENDPOINT_PLACEHOLDER;
                }
                "authorization" => {
                    authorization = true;
                    custody = value.trim() == format!("Bearer {CUSTODY_TOKEN}");
                }
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
        json!({"request": request, "placeholder": placeholder, "key_present": key_present, "authorization_present": authorization, "custody_bearer": custody, "body": body}),
    )
}

fn serve(
    mut stream: TcpStream,
    mode: Mode,
    credentials: Credentials,
    turn: &mut usize,
    seen: &Mutex<Vec<Value>>,
    stopping: &AtomicBool,
) -> io::Result<()> {
    let observed = request(&stream)?;
    seen.lock().unwrap().push(observed.clone());
    let request = observed["request"].as_str().unwrap();
    let placeholder = observed["placeholder"] == true;
    let authorization = observed["authorization_present"] == true;
    let key_present = observed["key_present"] == true;
    let custody = observed["custody_bearer"] == true;
    let body = &observed["body"];
    match credentials {
        Credentials::None => {
            if request == "HEAD /api/hello HTTP/1.1" && !key_present && !authorization {
                return reply(&mut stream, "200 OK", "application/json", "");
            }
            if !placeholder || authorization {
                return Err(io::Error::other(format!(
                    "unexpected credential source on {request}"
                )));
            }
        }
        // Everything arrives through metaharness's proxy, which strips the child's placeholder and
        // attaches the custody bearer: so the stub admits exactly that bearer and no key at all.
        Credentials::Loopback => {
            if !custody || key_present {
                return Err(io::Error::other(format!(
                    "unexpected credential source on {request}"
                )));
            }
            if request == "HEAD /api/hello HTTP/1.1" {
                return reply(&mut stream, "200 OK", "application/json", "");
            }
        }
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

fn run(mode: Mode, credentials: Credentials) -> (PathBuf, Vec<Value>, i32) {
    run_in(mode, credentials, &[], |_| {})
}

/// What a Claude binary prints for `--version`, reduced to its version token, read in a scratch
/// home of its own with nothing inherited. No model call.
fn version_of(binary: &Path, evidence: &Path) -> String {
    let home = evidence.join("version-probe-home");
    fs::create_dir_all(home.join("config")).unwrap();
    let output = Command::new(binary)
        .arg("--version")
        .env_clear()
        .env("HOME", &home)
        .env("CLAUDE_CONFIG_DIR", home.join("config"))
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .output()
        .unwrap();
    assert!(output.status.success(), "{} --version", binary.display());
    String::from_utf8_lossy(&output.stdout)
        .split_whitespace()
        .next()
        .expect("a version token")
        .to_owned()
}

/// The version of the binary under test, read once. It must be one the adapter pins: the lane is
/// run once per pinned binary, and every opening record it produces must report this version.
fn vendor_version() -> &'static str {
    static VERSION: OnceLock<String> = OnceLock::new();
    VERSION.get_or_init(|| {
        let (vendor, evidence) = vendor_and_evidence();
        let version = version_of(&vendor, &evidence);
        assert!(
            metaharness_claude::PINNED_VERSIONS.contains(&version.as_str()),
            "the binary under test reports {version}, which the adapter does not pin"
        );
        version
    })
}

/// The explicit vendor binary and the private evidence root, both absolute.
fn vendor_and_evidence() -> (PathBuf, PathBuf) {
    let vendor = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CLAUDE").expect("explicit installed vendor"),
    );
    let evidence = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_EVIDENCE").expect("private evidence root"),
    );
    assert!(vendor.is_absolute() && evidence.is_absolute());
    fs::create_dir_all(&evidence).unwrap();
    (vendor, evidence)
}

/// A fresh run root under the evidence directory: `home/` with the vendor on its `PATH`, `work/`,
/// `tmp/`, and — under `loopback` — this test's own fake login in `home/.claude/`.
fn fresh_root(vendor: &Path, evidence: &Path, credentials: Credentials) -> PathBuf {
    let root = tempfile::Builder::new()
        .prefix(&format!("claude-{}-", credentials.flag()))
        .tempdir_in(evidence)
        .unwrap()
        .keep();
    let home = root.join("home");
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    fs::create_dir_all(root.join("work")).unwrap();
    fs::create_dir_all(root.join("tmp")).unwrap();
    std::os::unix::fs::symlink(vendor, home.join(".local/bin/claude")).unwrap();
    if credentials == Credentials::Loopback {
        fs::create_dir_all(home.join(".claude")).unwrap();
        fs::write(
            home.join(".claude/.credentials.json"),
            json!({"claudeAiOauth": {"accessToken": CUSTODY_TOKEN}}).to_string(),
        )
        .unwrap();
    }
    root
}

/// One production-driver run: `plant` writes into the fresh root before the spawn, and `extra`
/// is appended to the driver's argv.
fn run_in(
    mode: Mode,
    credentials: Credentials,
    extra: &[&str],
    plant: impl FnOnce(&Path),
) -> (PathBuf, Vec<Value>, i32) {
    require_isolation();
    let driver = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_DRIVER").expect("explicit production driver"),
    );
    assert!(driver.is_absolute());
    let (vendor, evidence) = vendor_and_evidence();
    let root = fresh_root(&vendor, &evidence, credentials);
    plant(&root);
    let provider = Provider::start(mode, credentials);
    let mut command = driver_command(&driver, &root, &provider.endpoint, mode, credentials);
    command.args(extra);
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
    let events = collect_evidence(&root, &provider, credentials);
    (root, events, status.code().unwrap_or(-1))
}

fn driver_command(
    driver: &Path,
    root: &Path,
    endpoint: &str,
    mode: Mode,
    credentials: Credentials,
) -> Command {
    let home = root.join("home");
    let cwd = root.join("work");
    let mut command = Command::new(driver);
    command
        .args([
            "run",
            "claude",
            "--credentials",
            credentials.flag(),
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

fn collect_evidence(root: &Path, provider: &Provider, credentials: Credentials) -> Vec<Value> {
    retain_provider(root, provider);
    let requests = provider.requests.lock().unwrap().clone();
    let errors = provider.errors.lock().unwrap().clone();
    assert!(errors.is_empty(), "fixture server errors: {errors:?}");
    assert!(
        !requests.is_empty(),
        "native process never reached fixture: {}",
        root.display()
    );
    match credentials {
        Credentials::None => assert!(requests.iter().all(|r| (r["placeholder"] == true
            || (r["request"] == "HEAD /api/hello HTTP/1.1" && r["key_present"] == false))
            && r["authorization_present"] == false)),
        Credentials::Loopback => assert!(
            requests
                .iter()
                .all(|r| r["custody_bearer"] == true && r["key_present"] == false),
            "every request reached the stub through the proxy, carrying custody and no key"
        ),
    }
    let text = fs::read_to_string(root.join("events.jsonl")).unwrap();
    let events: Vec<Value> = text
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect();
    assert!(
        events
            .iter()
            .any(|event| event["event"] == "session.started"
                && event["harness_version"] == vendor_version()),
        "the opening record reports the version of the binary under test: {}",
        root.display()
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
    for (credentials, failed) in Credentials::ALL
        .into_iter()
        .flat_map(|credentials| [(credentials, false), (credentials, true)])
    {
        let (_, events, exit) = run(if failed { Mode::Refusal } else { Mode::Text }, credentials);
        let started = events
            .iter()
            .find(|e| e["event"] == "session.started")
            .unwrap();
        assert_eq!(started["model"], MODEL);
        // Native built-ins are observed, not silently removed to make H1a look satisfied: the
        // traffic switch keeps `cc-plugin-telemetry` out, and the two that remain are compiled in.
        let names: BTreeSet<&str> = started["plugins"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["name"].as_str().unwrap())
            .collect();
        assert_eq!(names, BTreeSet::from(BUILTINS_SEEN), "{credentials:?}");
        assert!(
            names
                .iter()
                .all(|name| metaharness_claude::builtin_plugins(vendor_version())
                    .is_some_and(|listed| listed.contains(name))),
            "every plugin seen is one the adapter declares for its pin"
        );
        assert!(
            started["plugins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|plugin| plugin["source"]
                    == metaharness_claude::builtin_plugin_source(plugin["name"].as_str().unwrap()))
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
    for (credentials, deny) in Credentials::ALL
        .into_iter()
        .flat_map(|credentials| [(credentials, false), (credentials, true)])
    {
        let (root, events, exit) = run(Mode::Tool { deny }, credentials);
        assert_eq!(exit, 0, "{credentials:?}");
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
    for (credentials, command) in Credentials::ALL
        .into_iter()
        .flat_map(|credentials| [(credentials, "interrupt"), (credentials, "halt")])
    {
        let (_, events, exit) = run(Mode::Hang { command }, credentials);
        assert_ne!(exit, 0, "{credentials:?}");
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
    for credentials in Credentials::ALL {
        let (_, events, exit) = run(Mode::RepeatTool, credentials);
        assert_ne!(exit, 0, "{credentials:?}");
        let terminal = events
            .iter()
            .find(|e| e["event"] == "session.ended")
            .unwrap();
        assert_eq!(terminal["subtype"], "error_max_turns");
        assert_eq!(terminal["is_error"], true);
    }
}

/// The floor reads the pin, the credential and the built-ins from what the record of the binary
/// under test carried (design amendment a33): `--strict-version` lets the pinned binary through,
/// H9 is `ok` on its version, H4 is `ok` under both credential-free modes — under `none` the record
/// names the API-key variable, which holds the launch's own attested placeholder — and H1a is `ok`
/// naming each built-in it admitted.
///
/// Only those rows are asserted. The fixture names its own `--cwd`, so H7 and H11 are attested
/// unavailable by design (amendment a6) and the run's exit is not the question here.
#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_audit_reads_the_pin_and_names_each_builtin_it_admitted() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for credentials in Credentials::ALL {
        let (root, _, _) = run_in(
            Mode::Text,
            credentials,
            &["--audit", "--strict-version"],
            |_| {},
        );
        let report = fs::read_to_string(root.join("stderr.log")).unwrap();
        let row = |id: &str| {
            report
                .lines()
                .find(|line| line.trim_start().starts_with(&format!("{id} ")))
                .unwrap_or_else(|| panic!("no {id} row in {}", root.display()))
                .to_owned()
        };
        let h9 = row("H9");
        assert!(
            h9.split_whitespace().nth(1) == Some("ok") && h9.contains(vendor_version()),
            "{credentials:?}: {h9}"
        );
        let h4 = row("H4");
        assert_eq!(
            h4.split_whitespace().nth(1),
            Some("ok"),
            "{credentials:?}: {h4}"
        );
        let h1a = row("H1a");
        assert_eq!(
            h1a.split_whitespace().nth(1),
            Some("ok"),
            "{credentials:?}: {h1a}"
        );
        for name in BUILTINS_SEEN {
            assert!(
                h1a.contains(&metaharness_claude::builtin_plugin_source(name)),
                "{credentials:?}: H1a names {name}: {h1a}"
            );
        }
    }
}

/// `--strict-version` refuses a binary off the pin before anything is spawned (design amendment
/// a33): the driver asks the binary on the child's `PATH` for its `--version`, no model request
/// reaches the stub, no session opens, and the refusal names the version it read. Run against
/// `METAHARNESS_NATIVE_CLAUDE_OFF_PIN` — 2.1.303 on this machine, kept off the pin.
#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_strict_version_refuses_an_off_pin_binary_before_the_spawn() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    require_isolation();
    let off_pin = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_CLAUDE_OFF_PIN").expect("an explicit off-pin vendor"),
    );
    let driver = PathBuf::from(
        std::env::var_os("METAHARNESS_NATIVE_DRIVER").expect("explicit production driver"),
    );
    assert!(off_pin.is_absolute() && driver.is_absolute());
    let (_, evidence) = vendor_and_evidence();
    let reported = version_of(&off_pin, &evidence);
    assert!(
        !metaharness_claude::PINNED_VERSIONS.contains(&reported.as_str()),
        "the off-pin binary reports {reported}, which is pinned"
    );
    for credentials in Credentials::ALL {
        let root = fresh_root(&off_pin, &evidence, credentials);
        let provider = Provider::start(Mode::Text, credentials);
        let mut command =
            driver_command(&driver, &root, &provider.endpoint, Mode::Text, credentials);
        command.arg("--strict-version");
        let mut child = NativeChild {
            child: command.spawn().unwrap(),
            reaped: false,
        };
        let deadline = Instant::now() + Duration::from_secs(45);
        let status = loop {
            if let Some(status) = child.child.try_wait().unwrap() {
                child.reaped = true;
                break status;
            }
            assert!(Instant::now() < deadline, "{}", root.display());
            thread::sleep(Duration::from_millis(20));
        };
        retain_provider(&root, &provider);
        let stderr = fs::read_to_string(root.join("stderr.log")).unwrap();
        assert_eq!(status.code(), Some(2), "{credentials:?}: {stderr}");
        assert!(
            stderr.contains("--strict-version") && stderr.contains(&reported),
            "{credentials:?}: {stderr}"
        );
        assert!(
            provider.requests.lock().unwrap().is_empty(),
            "{credentials:?}: nothing reached the stub"
        );
        let events = fs::read_to_string(root.join("events.jsonl")).unwrap();
        assert!(
            !events.contains("session.started"),
            "{credentials:?}: no session opened: {events}"
        );
    }
}

/// The memory shapes the pinned loader reads from a directory once a project or local source is
/// on, read from its own list of memory files (design amendment a33), less the two `AGENTS.md`
/// shapes, which the next constant holds. The H11 walk counts only `CLAUDE.md` and `AGENTS.md`;
/// the rest are `story:claude-h11-memory-walk-shapes`.
const LOADED_SHAPES: [&str; 4] = [
    "CLAUDE.md",
    ".claude/CLAUDE.md",
    "CLAUDE.local.md",
    ".claude/rules/canary.md",
];

/// The two `AGENTS.md` shapes, read by the `cc-plugin-agents-md` built-in where a project has no
/// `CLAUDE.md` of its own.
const AGENTS_SHAPES: [&str; 2] = ["AGENTS.md", ".claude/AGENTS.md"];

/// The text a canary memory file carries, unique per place and shape so a hit names the file that
/// leaked.
fn canary(place: &str, shape: &str) -> String {
    format!(
        "native-canary-{place}-{}-7d1f3a",
        shape.replace(['/', '.'], "-")
    )
}

/// Plant one canary of each shape in the run's working directory and one in its parent, which is
/// an ancestor the vendor's walk from the cwd passes through.
fn plant_canaries(root: &Path, shapes: &[&str]) {
    for shape in shapes {
        for (place, directory) in [("cwd", root.join("work")), ("ancestor", root.to_path_buf())] {
            let path = directory.join(shape);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, canary(place, shape)).unwrap();
        }
    }
}

/// Every canary of these shapes found in any request body the stub recorded, in plant order.
fn canaries_in_requests(root: &Path, shapes: &[&str]) -> Vec<String> {
    let requests = fs::read_to_string(root.join("requests.json")).unwrap();
    shapes
        .iter()
        .flat_map(|shape| ["cwd", "ancestor"].map(|place| canary(place, shape)))
        .filter(|text| requests.contains(text.as_str()))
        .collect()
}

/// The positive control: the same vendor, the same world, run directly **with** the project and
/// local setting sources, which the pinned loader gates memory discovery on. A probe that cannot
/// see a canary the vendor loads would make the negative result below worth nothing (invariant 3).
fn vendor_with_project_and_local_sources(shapes: &[&str]) -> Vec<String> {
    let (vendor, evidence) = vendor_and_evidence();
    let root = fresh_root(&vendor, &evidence, Credentials::None);
    plant_canaries(&root, shapes);
    let provider = Provider::start(Mode::Text, Credentials::None);
    let config = root.join("home/.claude-config");
    fs::create_dir_all(&config).unwrap();
    let mut command = Command::new(&vendor);
    command
        .args([
            "-p",
            "Return the fixture answer.",
            "--output-format",
            "stream-json",
            "--verbose",
            "--model",
            MODEL,
            "--max-turns",
            "1",
            "--strict-mcp-config",
            "--setting-sources",
            "project,local",
        ])
        .env_clear()
        .env("HOME", root.join("home"))
        .env("CLAUDE_CONFIG_DIR", &config)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .env("LANG", "C.UTF-8")
        .env("TMPDIR", root.join("tmp"))
        .env("ANTHROPIC_BASE_URL", &provider.endpoint)
        .env("ANTHROPIC_API_KEY", ENDPOINT_PLACEHOLDER)
        .env("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", "1")
        .current_dir(root.join("work"))
        .stdin(Stdio::null())
        .stdout(fs::File::create(root.join("vendor-stdout.jsonl")).unwrap())
        .stderr(fs::File::create(root.join("stderr.log")).unwrap())
        .process_group(0);
    let mut child = NativeChild {
        child: command.spawn().unwrap(),
        reaped: false,
    };
    let deadline = Instant::now() + Duration::from_secs(45);
    while child.child.try_wait().unwrap().is_none() {
        assert!(
            Instant::now() < deadline,
            "the control exceeded 45 seconds: {}",
            root.display()
        );
        thread::sleep(Duration::from_millis(20));
    }
    child.reaped = true;
    retain_provider(&root, &provider);
    canaries_in_requests(&root, shapes)
}

/// H11's re-probe (design amendment a33): a canary of each memory shape in the working directory
/// and in an ancestor, then the request bodies the stub recorded.
///
/// * **`CLAUDE.md`, `.claude/CLAUDE.md`, `CLAUDE.local.md`, a `.claude/rules` file** — run directly
///   with the project and local sources, the vendor sends every canary, from the cwd and from the
///   ancestor; through the production launch, which passes `--setting-sources ""`, none reaches any
///   request, under `none` or `loopback`. The control is what makes the negative an observation.
/// * **`AGENTS.md`, `.claude/AGENTS.md`** — the same, planted where no `CLAUDE.md`-family file sits
///   in the cwd or above it: the `cc-plugin-agents-md` built-in loads them only for a project with
///   no `CLAUDE.md` of its own, which is why a world with any such file above it (the operator's own
///   `~/.claude/CLAUDE.md`, for one) sends none even with the sources on. The lane therefore runs
///   with an empty directory bound over `$HOME/.claude`.
#[test]
#[ignore = "actual Claude and production driver; explicit binaries/evidence and isolated loopback network namespace required"]
fn native_memory_canaries_in_the_cwd_and_an_ancestor_never_reach_the_model() {
    let _lease = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    require_isolation();
    let (_, evidence) = vendor_and_evidence();
    let above: Vec<PathBuf> = evidence
        .ancestors()
        .flat_map(|dir| {
            LOADED_SHAPES
                .iter()
                .chain(&AGENTS_SHAPES)
                .map(move |file| dir.join(file))
        })
        .filter(|path| path.is_file())
        .collect();
    assert!(
        above.is_empty(),
        "memory files above the evidence root change what the control loads; run the lane with \
         them hidden: {above:?}"
    );
    for shapes in [&LOADED_SHAPES[..], &AGENTS_SHAPES[..]] {
        let control = vendor_with_project_and_local_sources(shapes);
        let expected: Vec<String> = shapes
            .iter()
            .flat_map(|shape| ["cwd", "ancestor"].map(|place| canary(place, shape)))
            .collect();
        assert_eq!(
            control, expected,
            "the control with the project and local sources: {shapes:?}"
        );
        for credentials in Credentials::ALL {
            let (root, events, _) = run_in(Mode::Text, credentials, &[], |root| {
                plant_canaries(root, shapes);
            });
            assert!(
                events
                    .iter()
                    .any(|e| e["event"] == "text" && e["text"] == ANSWER),
                "the run reached the model: {}",
                root.display()
            );
            assert_eq!(
                canaries_in_requests(&root, shapes),
                Vec::<String>::new(),
                "{credentials:?} {shapes:?}: {}",
                root.display()
            );
        }
    }
}
