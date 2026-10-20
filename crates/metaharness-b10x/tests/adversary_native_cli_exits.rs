//! Adversary pass 1 against amendment a30: the exits a30 and the CHANGELOG state, measured through
//! the production CLI and the actual `b10x-harness` binary, for the flag combinations the unit's
//! own native case does not run (it runs `--hermetic strict --cwd <copy> --scratch-cwd` only).
//!
//! Opt-in and ignored, like `native_fixture.rs`: the whole test process runs in a network
//! namespace containing only loopback, against an owned Responses fixture that answers one text
//! turn. Fixture token counts are synthetic inputs, never evidence of paid model usage.
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

struct Provider {
    endpoint: String,
    requests: Arc<Mutex<Vec<Value>>>,
    errors: Arc<Mutex<Vec<String>>>,
    stopping: Arc<AtomicBool>,
    worker: Option<thread::JoinHandle<()>>,
}

impl Provider {
    fn start() -> Self {
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
                        if let Err(error) = serve(stream, &mut turn, &seen) {
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
        .trim()
        .to_owned();
    let mut length = 0;
    let mut credential = false;
    for line in &headers[1..] {
        if let Some((name, value)) = line.split_once(':') {
            match name.to_ascii_lowercase().as_str() {
                "content-length" => {
                    length = value
                        .trim()
                        .parse::<usize>()
                        .map_err(|_| invalid("content length"))?;
                }
                "x-api-key" | "authorization" => credential = true,
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
    Ok(json!({"request": request, "credential_present": credential, "body": body}))
}

fn serve(mut stream: TcpStream, turn: &mut usize, seen: &Mutex<Vec<Value>>) -> io::Result<()> {
    let observed = request(&stream)?;
    seen.lock().unwrap().push(observed.clone());
    if observed["credential_present"] != false {
        return Err(invalid("fixture refuses any credential header"));
    }
    if observed["request"] != "POST /responses HTTP/1.1" {
        return Err(invalid("unexpected endpoint"));
    }
    *turn += 1;
    let item = json!({"id":format!("msg-{turn}"),"type":"message","status":"completed","role":"assistant","content":[{"type":"output_text","text":ANSWER,"annotations":[]}]});
    let response = json!({"id":format!("fixture-response-{turn}"),"object":"response","status":"completed","model":MODEL,"output":[item],"usage":{"input_tokens":3,"output_tokens":4,"input_tokens_details":{"cached_tokens":1},"total_tokens":7}});
    let events = [
        json!({"type":"response.created","response":{"id":response["id"],"object":"response","status":"in_progress","model":MODEL,"output":[]}}),
        json!({"type":"response.output_text.delta","item_id":item["id"],"output_index":0,"content_index":0,"delta":ANSWER}),
        json!({"type":"response.output_item.done","output_index":0,"item":item}),
        json!({"type":"response.completed","response":response}),
    ];
    let mut wire = String::new();
    for event in events {
        writeln!(wire, "data: {event}\n").unwrap();
    }
    wire.push_str("data: [DONE]\n\n");
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{wire}",
        wire.len()
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

/// One `metaharness run b10x` through the production CLI and the actual binary; `--cwd work` only
/// when `named_cwd`. Returns the evidence root, the events, stderr and the exit code.
fn run(named_cwd: bool, extra: &[&str]) -> (PathBuf, Vec<Value>, String, i32) {
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
        .prefix("adv-b10x-")
        .tempdir_in(&evidence)
        .unwrap()
        .keep();
    let home = root.join("home");
    let cwd = root.join("work");
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(root.join("tmp")).unwrap();
    fs::write(cwd.join("fixture.txt"), "owned-native-fixture-content").unwrap();
    std::os::unix::fs::symlink(vendor, home.join(".local/bin/b10x-harness")).unwrap();
    let provider = Provider::start();
    let mut command = Command::new(driver);
    command.args([
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
        "Return the fixture's answer.",
        "--max-turns",
        "3",
    ]);
    if named_cwd {
        command.arg("--cwd").arg(&cwd);
    }
    command
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
    let mut child = NativeChild {
        child: command.spawn().unwrap(),
        reaped: false,
    };
    let _input = child.child.stdin.take().unwrap();
    let deadline = Instant::now() + Duration::from_secs(30);
    let status = loop {
        if let Some(status) = child.child.try_wait().unwrap() {
            child.reaped = true;
            break status;
        }
        assert!(
            Instant::now() < deadline,
            "native fixture deadline; private evidence: {}",
            root.display()
        );
        thread::sleep(Duration::from_millis(20));
    };
    let errors = provider.errors.lock().unwrap().clone();
    assert!(errors.is_empty(), "fixture provider errors: {errors:?}");
    assert_eq!(provider.requests.lock().unwrap().len(), 1);
    let events = event_lines(&root);
    let stderr = fs::read_to_string(root.join("stderr.log")).unwrap();
    eprintln!("private native evidence: {}", root.display());
    (root, events, stderr, status.code().unwrap_or(-1))
}

/// The gating rows the rendered report says are not `ok`, as `(row, verdict)`.
fn gating_not_ok(stderr: &str) -> Vec<(String, String)> {
    stderr
        .lines()
        .filter_map(|line| {
            let words: Vec<&str> = line.split_whitespace().collect();
            (words.len() > 2
                && words[0].starts_with('H')
                && words[0][1..].starts_with(|c: char| c.is_ascii_digit())
                && words[1] != "ok"
                && words[2] == "gating")
                .then(|| (words[0].to_owned(), words[1].to_owned()))
        })
        .collect()
}

fn opening(events: &[Value]) -> &Value {
    let found: Vec<&Value> = events
        .iter()
        .filter(|event| event["event"] == "session.started")
        .collect();
    assert_eq!(found.len(), 1, "{events:?}");
    let start = found[0];
    assert_eq!(start["adapter_class"], "direct_provider");
    assert_eq!(start["harness_version"], "0.15.0");
    assert!(start.get("cwd").is_some_and(Value::is_null), "{start}");
    start
}

/// CHANGELOG: "A strict b10x run without `--cwd` moves the same way" (0 → 3), H7 alone.
#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_strict_without_cwd_exits_3_on_h7_alone() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_root, events, stderr, code) = run(false, &["--hermetic", "strict"]);
    opening(&events);
    assert_eq!(
        gating_not_ok(&stderr),
        [("H7".to_owned(), "unk".to_owned())],
        "{stderr}"
    );
    assert_eq!(code, 3, "{stderr}");
}

/// CHANGELOG and a30: "A strict a6 `--cwd` run without the declaration still exits 1, on H11".
#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_strict_a6_cwd_still_exits_1_on_h11() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let (_root, events, stderr, code) = run(true, &["--hermetic", "strict"]);
    opening(&events);
    let rows = gating_not_ok(&stderr);
    assert!(
        rows.contains(&("H11".to_owned(), "gap".to_owned()))
            && rows.contains(&("H7".to_owned(), "unk".to_owned())),
        "{stderr}"
    );
    assert_eq!(code, 1, "{stderr}");
}

/// Bare `--hermetic` without `--audit` owes no verdict, so the exit stays 0 on b10x.
#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_hermetic_on_without_audit_still_exits_0() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for named in [false, true] {
        let mut extra = vec!["--hermetic"];
        if named {
            extra.push("--scratch-cwd");
        }
        let (_root, events, stderr, code) = run(named, &extra);
        opening(&events);
        assert_eq!(code, 0, "{stderr}");
    }
}

/// Bare `--hermetic --audit` judges the same floor; on b10x it now exits 3, H7 alone — an exit
/// move the CHANGELOG and a30 decision 3 do not name.
#[test]
#[ignore = "actual b10x and production CLI; explicit paths and isolated loopback network namespace"]
fn actual_b10x_hermetic_on_with_audit_exits_3_on_h7_alone() {
    let _serial = NATIVE_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    for named in [false, true] {
        let mut extra = vec!["--hermetic", "--audit"];
        if named {
            extra.push("--scratch-cwd");
        }
        let (_root, events, stderr, code) = run(named, &extra);
        opening(&events);
        assert_eq!(
            gating_not_ok(&stderr),
            [("H7".to_owned(), "unk".to_owned())],
            "named={named}: {stderr}"
        );
        assert_eq!(code, 3, "named={named}: {stderr}");
    }
}
