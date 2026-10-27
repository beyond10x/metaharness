//! An off-pin vendor binary is reported once and never refused for its version
//! (`story:off-pin-vendor-version-is-advisory`, design amendment a62).
//!
//! Each case runs the built binary against a fake `claude` or `codex` in a scratch
//! `$HOME/.local/bin`, which is first on the `PATH` metaharness constructs for its child. The fake
//! answers `--version` with a release outside the adapter's pin and otherwise writes an opening
//! record naming that release: Claude's `system/init` on stdout, Codex's `session_meta` in a
//! rollout under the scratch `CODEX_HOME`. The run's event stream must carry exactly one
//! `version_outside_pin` warning, and stderr must print that warning's message exactly once, in
//! every hermetic mode. `--strict-version` still refuses the same binary before the spawn.
//!
//! Nothing real is reached: `HOME`, `TMPDIR` and the process's own `PATH` are scratch
//! directories and the run copies no credential (`--credentials none`).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The release the fakes report: off both pins, as 0.164.0 was off Codex's on 2026-10-27.
const CLAUDE_OFF_PIN: &str = "2.1.400";
const CODEX_OFF_PIN: &str = "0.164.0";

/// How long one run may take before the case calls it stuck.
const DEADLINE: Duration = Duration::from_secs(30);

/// The warning code an off-pin opening record earns.
const CODE: &str = "version_outside_pin";

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

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("executable");
}

/// A scratch world with a fake `claude` and a fake `codex`, each off its pin.
fn world() -> World {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let path = base.path().join("path");
    let tmp = base.path().join("tmp");
    let bin = home.join(".local/bin");
    for directory in [&home, &path, &tmp, &bin] {
        std::fs::create_dir_all(directory).expect("a scratch directory");
    }
    let launched = base.path().join("launched");
    let marker = launched.to_str().expect("a UTF-8 scratch path");
    assert!(!marker.contains('\''), "the marker path quotes as is");
    let init = format!(
        r#"{{"type":"system","subtype":"init","claude_code_version":"{CLAUDE_OFF_PIN}","tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[]}}"#
    );
    let result = r#"{"type":"result","subtype":"success","is_error":false}"#;
    executable(
        &bin.join("claude"),
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo '{CLAUDE_OFF_PIN} (Claude Code)'; exit 0; fi\n\
             echo claude >> '{marker}'\nprintf '%s\\n' '{init}' '{result}'\nexit 0\n"
        ),
    );
    let rollout = [
        format!(
            r#"{{"timestamp":"2026-10-27T00:00:00.000Z","type":"session_meta","payload":{{"id":"s-1","session_id":"s-1","cli_version":"{CODEX_OFF_PIN}","cwd":"/scratch"}}}}"#
        ),
        r#"{"timestamp":"2026-10-27T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#.to_owned(),
        r#"{"timestamp":"2026-10-27T00:00:00.002Z","type":"turn_context","payload":{"turn_id":"t1","model":"m"}}"#.to_owned(),
        r#"{"timestamp":"2026-10-27T00:00:01.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","error":null,"last_agent_message":"done"}}"#.to_owned(),
    ];
    let quoted: Vec<String> = rollout.iter().map(|line| format!("'{line}'")).collect();
    executable(
        &bin.join("codex"),
        &format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo 'codex-cli {CODEX_OFF_PIN}'; exit 0; fi\n\
             echo codex >> '{marker}'\nhome=\"${{CODEX_HOME:-$HOME/.codex}}\"\n\
             dir=\"$home/sessions/2026/10/27\"\nmkdir -p \"$dir\"\n\
             printf '%s\\n' {} > \"$dir/rollout-2026-10-27T00-00-00-0000.jsonl\"\nexit 0\n",
            quoted.join(" ")
        ),
    );
    World {
        _base: base,
        home,
        path,
        tmp,
        launched,
    }
}

impl World {
    /// `metaharness run <kind> --prompt go --credentials none --decisions observe <extra…>`,
    /// waited on for at most [`DEADLINE`].
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

    /// How many times a fake vendor was launched for a session (its `--version` excluded).
    fn sessions_launched(&self) -> usize {
        std::fs::read_to_string(&self.launched).map_or(0, |text| text.lines().count())
    }
}

/// The messages of every `version_outside_pin` warning on the event stream.
fn warnings(stdout: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
        .filter(|event| event["event"] == "warning" && event["code"] == CODE)
        .map(|event| event["message"].as_str().unwrap_or_default().to_owned())
        .collect()
}

/// One off-pin run per kind and hermetic mode: one warning in the stream, its message once on
/// stderr, and the version named in it.
#[test]
fn an_off_pin_binary_is_warned_once_on_the_stream_and_once_on_stderr_in_every_hermetic_mode() {
    for (kind, version) in [("claude", CLAUDE_OFF_PIN), ("codex", CODEX_OFF_PIN)] {
        for mode in ["off", "on", "strict"] {
            let world = world();
            let outcome = world.run(kind, &["--hermetic", mode]);
            let context = format!(
                "{kind} --hermetic {mode}: exit {}\nstdout:\n{}\nstderr:\n{}",
                outcome.code, outcome.stdout, outcome.stderr
            );
            assert_eq!(world.sessions_launched(), 1, "{context}");
            let warned = warnings(&outcome.stdout);
            assert_eq!(
                warned.len(),
                1,
                "story:off-pin-vendor-version-is-advisory: one {CODE} warning in the stream; \
                 {context}"
            );
            assert!(warned[0].contains(version), "{context}");
            let printed = format!("metaharness: {}", warned[0]);
            let times = outcome
                .stderr
                .lines()
                .filter(|line| *line == printed)
                .count();
            assert_eq!(
                times, 1,
                "story:off-pin-vendor-version-is-advisory: the warning's message is printed on \
                 stderr once per run; {context}"
            );
            assert_ne!(
                outcome.code, 2,
                "an off-pin version is no refusal; {context}"
            );
        }
    }
}

/// `--strict-version` is the operator's refusal and is unchanged: the same off-pin binary is
/// refused before the spawn, exit 2, by name.
#[test]
fn strict_version_still_refuses_the_off_pin_binary_before_the_spawn() {
    for (kind, version) in [("claude", CLAUDE_OFF_PIN), ("codex", CODEX_OFF_PIN)] {
        let world = world();
        let outcome = world.run(kind, &["--strict-version"]);
        let context = format!(
            "{kind}: exit {}\nstdout:\n{}\nstderr:\n{}",
            outcome.code, outcome.stdout, outcome.stderr
        );
        assert_eq!(outcome.code, 2, "{context}");
        assert_eq!(world.sessions_launched(), 0, "{context}");
        assert!(
            outcome.stderr.starts_with("metaharness: ")
                && outcome.stderr.contains("--strict-version requires one of")
                && outcome.stderr.contains(version),
            "{context}"
        );
    }
}
