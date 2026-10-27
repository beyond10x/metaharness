//! Adversary cases, pass 1, against `story:off-pin-vendor-version-is-advisory` (design amendment
//! a62), driven through the built binary.
//!
//! Each case runs `metaharness run` against fake `claude` and `codex` executables in a scratch
//! `$HOME/.local/bin`, the first entry of the `PATH` metaharness builds for its child. Unlike
//! `version_outside_pin.rs`, the fakes write their opening record in the directory metaharness
//! started them in, so H7 holds and only the version decides the comparison, and the `--version`
//! banner and the record's version are chosen apart. Nothing real is reached: `HOME`, `TMPDIR` and
//! the process's own `PATH` are scratch, and the run copies no credential (`--credentials none`).
#![cfg(unix)]

use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use metaharness::protocol::Kind;

const DEADLINE: Duration = Duration::from_secs(30);
const CODE: &str = "version_outside_pin";

fn pinned(kind: Kind) -> String {
    metaharness::capabilities(kind)
        .expect("the adapter describes itself")
        .versions_pinned
        .last()
        .expect("a pin")
        .clone()
}

/// What a fake answers to `--version`, and what its opening record says.
#[derive(Clone, Copy)]
enum Banner<'a> {
    /// Prints this version, as the vendor spells its banner, and exits 0.
    Version(&'a str),
    /// Prints a line with no version token in it, and exits 0.
    NoVersion,
    /// Prints nothing and exits 1.
    Fails,
}

struct World {
    _base: tempfile::TempDir,
    home: PathBuf,
    path: PathBuf,
    tmp: PathBuf,
    launched: PathBuf,
}

struct Outcome {
    code: i32,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn context(&self, what: &str) -> String {
        format!(
            "{what}: exit {}\nstdout:\n{}\nstderr:\n{}",
            self.code, self.stdout, self.stderr
        )
    }
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("executable");
}

fn banner_lines(banner: Banner<'_>, spelled: impl Fn(&str) -> String) -> String {
    match banner {
        Banner::Version(version) => format!(
            "if [ \"$1\" = --version ]; then echo '{}'; exit 0; fi\n",
            spelled(version)
        ),
        Banner::NoVersion => {
            "if [ \"$1\" = --version ]; then echo 'a vendor binary'; exit 0; fi\n".to_owned()
        }
        Banner::Fails => "if [ \"$1\" = --version ]; then exit 1; fi\n".to_owned(),
    }
}

impl World {
    fn new() -> Self {
        let base = tempfile::tempdir().expect("a scratch directory");
        let home = base.path().join("home");
        let path = base.path().join("path");
        let tmp = base.path().join("tmp");
        for directory in [&home, &path, &tmp, &home.join(".local/bin")] {
            std::fs::create_dir_all(directory).expect("a scratch directory");
        }
        let launched = base.path().join("launched");
        assert!(
            !launched.to_string_lossy().contains('\''),
            "the marker path quotes as is"
        );
        World {
            _base: base,
            home,
            path,
            tmp,
            launched,
        }
    }

    fn bin(&self) -> PathBuf {
        self.home.join(".local/bin")
    }

    /// A fake `claude`: its banner, then `inits` opening records naming `record` in the
    /// directory it was started in, then a successful terminal record.
    fn claude(&self, banner: Banner<'_>, record: &str, inits: usize) {
        let marker = self.launched.display();
        let init = format!(
            r#"{{"type":"system","subtype":"init","claude_code_version":"{record}","cwd":"%s","tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[]}}"#
        );
        let mut script = String::from("#!/bin/sh\n");
        script.push_str(&banner_lines(banner, |version| {
            format!("{version} (Claude Code)")
        }));
        let _ = write!(script, "echo claude >> '{marker}'\ncwd=$(pwd)\n");
        for _ in 0..inits {
            let _ = writeln!(script, "printf '{init}\\n' \"$cwd\"");
        }
        script.push_str(
            "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\",\"is_error\":false}'\nexit 0\n",
        );
        executable(&self.bin().join("claude"), &script);
    }

    /// A fake `codex`: its banner, then a rollout naming `record` over the directory it was
    /// started in, with the personality the launch writes.
    fn codex(&self, banner: Banner<'_>, record: &str) {
        let marker = self.launched.display();
        let meta = format!(
            r#"{{"timestamp":"2026-10-27T00:00:00.000Z","type":"session_meta","payload":{{"id":"s-1","session_id":"s-1","cli_version":"{record}","cwd":"%s"}}}}"#
        );
        let rest = [
            r#"{"timestamp":"2026-10-27T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#,
            r#"{"timestamp":"2026-10-27T00:00:00.002Z","type":"turn_context","payload":{"turn_id":"t1","model":"m","personality":"pragmatic"}}"#,
            r#"{"timestamp":"2026-10-27T00:00:01.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","error":null,"last_agent_message":"done"}}"#,
        ];
        let quoted: Vec<String> = rest.iter().map(|line| format!("'{line}'")).collect();
        let mut script = String::from("#!/bin/sh\n");
        script.push_str(&banner_lines(banner, |version| {
            format!("codex-cli {version}")
        }));
        let _ = write!(
            script,
            "echo codex >> '{marker}'\nhome=\"${{CODEX_HOME:-$HOME/.codex}}\"\n\
             dir=\"$home/sessions/2026/10/27\"\nmkdir -p \"$dir\"\ncwd=$(pwd)\n\
             file=\"$dir/rollout-2026-10-27T00-00-00-0000.jsonl\"\n\
             printf '{meta}\\n' \"$cwd\" > \"$file\"\n\
             printf '%s\\n' {} >> \"$file\"\nexit 0\n",
            quoted.join(" ")
        );
        executable(&self.bin().join("codex"), &script);
    }

    fn run(&self, kind: &str, extra: &[&str]) -> Outcome {
        let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args([
                "run",
                kind,
                "--prompt",
                "go",
                "--credentials",
                "none",
                "--decisions",
                "observe",
            ])
            .args(extra)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("TMPDIR", &self.tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let started = Instant::now();
        while child.try_wait().expect("waitable").is_none() {
            if started.elapsed() > DEADLINE {
                child.kill().expect("killed");
                child.wait().expect("reaped");
                panic!("{kind} {extra:?}: still running after {DEADLINE:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let output = child.wait_with_output().expect("the output");
        Outcome {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn sessions_launched(&self) -> usize {
        std::fs::read_to_string(&self.launched).map_or(0, |text| text.lines().count())
    }
}

fn warnings(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|event| event["event"] == "warning" && event["code"] == CODE)
        .map(|event| event["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// The `H9` line of the report printed on stderr.
fn h9_line(stderr: &str) -> Option<&str> {
    stderr
        .lines()
        .map(str::trim_start)
        .find(|line| line.split_whitespace().next() == Some("H9"))
}

/// Acceptance 1, through the binary: a strict Codex run on 0.164.0 exits as the same run on the
/// pin, and the report renders H9 `gap advisory` naming both versions.
#[test]
fn adversary_off_pin_pass1_codex_strict_through_the_binary_exits_as_the_pinned_run() {
    let pin = pinned(Kind::Codex);
    let on = World::new();
    on.codex(Banner::Version(&pin), &pin);
    let on = on.run("codex", &["--hermetic", "strict"]);
    assert_ne!(
        on.code,
        1,
        "precondition: the pinned run has no gating gap, so a gap off the pin would show; {}",
        on.context("pinned")
    );
    let off = World::new();
    off.codex(Banner::Version("0.164.0"), "0.164.0");
    let off = off.run("codex", &["--hermetic", "strict"]);
    assert_eq!(
        off.code,
        on.code,
        "story:off-pin-vendor-version-is-advisory acceptance 1; {}",
        off.context("off the pin")
    );
    let line = h9_line(&off.stderr).unwrap_or_default();
    assert!(
        line.starts_with("H9   gap advisory") && line.contains("0.164.0") && line.contains(&pin),
        "{}",
        off.context("off the pin")
    );
}

/// A Claude child that prints its opening record on every turn, as stream-json does, earns one
/// warning on the stream and one line on stderr, not one per record.
#[test]
fn adversary_off_pin_pass1_claude_repeated_opening_records_warn_once_through_the_binary() {
    let world = World::new();
    world.claude(Banner::Version("2.1.400"), "2.1.400", 3);
    let outcome = world.run("claude", &["--hermetic", "on"]);
    let context = outcome.context("claude, three opening records");
    let warned = warnings(&outcome.stdout);
    assert_eq!(warned.len(), 1, "{context}");
    let printed = format!("metaharness: {}", warned[0]);
    assert_eq!(
        outcome
            .stderr
            .lines()
            .filter(|line| *line == printed)
            .count(),
        1,
        "{context}"
    );
}

/// `--strict-version` is unchanged (acceptance 4) for a binary whose banner names no version and
/// one whose `--version` fails: each is refused before the spawn, exit 2, and no session is
/// launched. A binary absent from the child's `PATH` is not constructed here: that `PATH` always
/// ends in `/usr/local/bin:/usr/bin:/bin`, so a machine-wide install would answer instead.
#[test]
fn adversary_off_pin_pass1_strict_version_refuses_an_unreadable_binary() {
    for kind in ["claude", "codex"] {
        for banner in [Banner::NoVersion, Banner::Fails] {
            let world = World::new();
            match kind {
                "claude" => world.claude(banner, &pinned(Kind::Claude), 1),
                _ => world.codex(banner, &pinned(Kind::Codex)),
            }
            let outcome = world.run(kind, &["--strict-version"]);
            let context = outcome.context(&format!("{kind}, unreadable banner"));
            assert_eq!(outcome.code, 2, "{context}");
            assert_eq!(world.sessions_launched(), 0, "{context}");
            assert!(outcome.stderr.contains("no readable version"), "{context}");
        }
    }
}

/// A banner on the pin over a record off it: `--strict-version` admits the binary from its
/// banner, and since a62 nothing holds the record to the pin. The run must still say so: one
/// warning, one stderr line, H9 `gap advisory` in the report.
#[test]
fn adversary_off_pin_pass1_a_pinned_banner_over_an_off_pin_record_is_still_reported() {
    let pin = pinned(Kind::Codex);
    let world = World::new();
    world.codex(Banner::Version(&pin), "0.164.0");
    let outcome = world.run("codex", &["--strict-version", "--hermetic", "strict"]);
    let context = outcome.context("codex, pinned banner, record 0.164.0");
    assert_eq!(world.sessions_launched(), 1, "{context}");
    assert_eq!(warnings(&outcome.stdout).len(), 1, "{context}");
    let line = h9_line(&outcome.stderr).unwrap_or_default();
    assert!(line.starts_with("H9   gap advisory"), "{context}");
    assert_ne!(outcome.code, 1, "{context}");
}
