//! Independent invariant review of wave 8 unit 6 (`story:b10x-api-key-from-keychain`, amendment a50), through
//! the built binary.
//!
//! The loop is a shell stub in a scratch `HOME`, on the `PATH` metaharness builds for its child. It
//! writes its argv NUL-separated and its environment, then exits: no model, no network, no secret
//! store. The secret name is the fixture `llm/opencode-go-api-key` or a hand-built name that
//! passes metaharness's shape check; the bus address names a path that does not exist.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

const FIXTURE_SECRET: &str = "llm/opencode-go-api-key";
const FIXTURE_KEY: &str = "fixture-key-value-that-must-not-travel";
const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

struct Scratch {
    _base: tempfile::TempDir,
    home: PathBuf,
    argv: PathBuf,
    env: PathBuf,
    bus: String,
    runtime: PathBuf,
}

impl Scratch {
    fn new() -> Self {
        let base = tempfile::tempdir().expect("a scratch directory");
        let home = base.path().join("home");
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).expect("the stub's directory");
        let runtime = base.path().join("runtime");
        std::fs::create_dir(&runtime).expect("a runtime directory");
        let argv = base.path().join("argv");
        let env = base.path().join("env");
        let stub = bin.join("b10x-harness");
        let record = base.path().join("loop.jsonl");
        std::fs::write(&record, B10X_RECORDED_LOOP).expect("the recorded loop");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then echo 'b10x-harness 0.21.0'; exit 0; fi\n\
                 printf '%s\\0' \"$@\" > '{}'\n\
                 /usr/bin/env > '{}'\n\
                 cat '{}'\n\
                 exit 0\n",
                argv.display(),
                env.display(),
                record.display()
            ),
        )
        .expect("the stub");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700))
            .expect("the stub is executable");
        let bus = format!("unix:path={}", base.path().join("no-bus-here").display());
        Self {
            _base: base,
            home,
            argv,
            env,
            bus,
            runtime,
        }
    }

    /// `metaharness run b10x …` under a cleared environment. `bus` decides whether the caller has
    /// the two session-bus variables.
    fn run(&self, extra: &[&str], bus: bool) -> (i32, String, String) {
        let mut args = vec![
            "run",
            "b10x",
            "--hermetic=on",
            "--decisions",
            "observe",
            "--model-endpoint",
            "http://127.0.0.1:1/v1",
            "--model",
            "a-model",
            "-p",
            "go",
        ];
        args.extend_from_slice(extra);
        let mut command = Command::new(env!("CARGO_BIN_EXE_metaharness"));
        command
            .args(&args)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env("OPENAI_API_KEY", FIXTURE_KEY);
        if bus {
            command
                .env("DBUS_SESSION_BUS_ADDRESS", &self.bus)
                .env("XDG_RUNTIME_DIR", &self.runtime);
        }
        let output = command.output().expect("the binary runs");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    fn child_argv(&self, said: &str) -> Vec<String> {
        std::fs::read(&self.argv)
            .unwrap_or_else(|error| panic!("the loop was not spawned ({error}): {said}"))
            .split(|byte| *byte == 0)
            .filter(|word| !word.is_empty())
            .map(|word| String::from_utf8_lossy(word).into_owned())
            .collect()
    }

    fn child_env(&self, said: &str) -> BTreeMap<String, String> {
        std::fs::read_to_string(&self.env)
            .unwrap_or_else(|error| panic!("the loop was not spawned ({error}): {said}"))
            .lines()
            .filter_map(|line| line.split_once('='))
            .filter(|(name, _)| !matches!(*name, "PWD" | "SHLVL" | "_" | "OLDPWD"))
            .map(|(name, value)| (name.to_owned(), value.to_owned()))
            .collect()
    }
}

/// Every `how` of an imposed H3 control in the run's own `session.started` event.
fn imposed_h3(stdout: &str) -> Vec<String> {
    fn walk(value: &Value, found: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                if map
                    .get("row")
                    .and_then(Value::as_str)
                    .is_some_and(|row| row.eq_ignore_ascii_case("h3"))
                    && let Some(how) = map.get("how").and_then(Value::as_str)
                {
                    found.push(how.to_owned());
                }
                for inner in map.values() {
                    walk(inner, found);
                }
            }
            Value::Array(items) => {
                for inner in items {
                    walk(inner, found);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    for line in stdout.lines() {
        if let Ok(event) = serde_json::from_str::<Value>(line)
            && event["event"] == "session.started"
            && let Some(hermetic) = event.get("hermetic")
            && let Some(imposed) = hermetic.get("imposed")
        {
            walk(imposed, &mut found);
        }
    }
    found
}

/// Brief item 2: H3 is the one record of what the child's environment was built from, and on the
/// secret route it widens by two variables. The run's own `session.started` should say whether it
/// did. Measured: the sentence is the same whether the caller had both variables or neither, so
/// the record cannot tell a run whose loop was handed the bus from one whose loop was not.
#[test]
fn h3_says_whether_the_session_bus_was_forwarded() {
    let secret = [
        "--credentials",
        "api-key",
        "--api-key-secret",
        FIXTURE_SECRET,
    ];

    let with_bus = Scratch::new();
    let (_, stdout_with, stderr_with) = with_bus.run(&secret, true);
    let env_with = with_bus.child_env(&stderr_with);
    assert!(
        env_with.contains_key("DBUS_SESSION_BUS_ADDRESS"),
        "precondition: forwarded when present: {env_with:?}"
    );

    let without_bus = Scratch::new();
    let (_, stdout_without, stderr_without) = without_bus.run(&secret, false);
    let env_without = without_bus.child_env(&stderr_without);
    assert!(
        !env_without.contains_key("DBUS_SESSION_BUS_ADDRESS")
            && !env_without.contains_key("XDG_RUNTIME_DIR"),
        "precondition: nothing forwarded when absent: {env_without:?}"
    );

    let how_with = imposed_h3(&stdout_with);
    let how_without = imposed_h3(&stdout_without);
    assert_eq!(how_with.len(), 1, "one H3 control: {stdout_with}");
    assert_eq!(how_without.len(), 1, "one H3 control: {stdout_without}");
    assert_ne!(
        how_with, how_without,
        "the H3 attestation reads the same whether the child was handed the session bus or not: \
         {how_with:?}"
    );
}

/// Brief item 3: a name that passes metaharness's shape check and looks like another of the
/// loop's flags still reaches the loop as the value of `--api-key-secret`, the one credential flag
/// on the argv, and never as a word of its own.
#[test]
fn a_name_that_looks_like_a_flag_never_becomes_one() {
    const CREDENTIAL_FLAGS: [&str; 4] = [
        "--api-key-env",
        "--api-key-file",
        "--oauth-token-file",
        "--oauth-token-env",
    ];
    for name in [
        "--api-key-file=/etc/x",
        "--api-key-env/OPENAI_API_KEY",
        "-x/--api-key-env",
        "llm/x --api-key-env OPENAI_API_KEY",
        "llm/--api-key-file",
        "llm/x=y",
    ] {
        let scratch = Scratch::new();
        let attached = format!("--api-key-secret={name}");
        let (code, stdout, stderr) =
            scratch.run(&["--credentials", "api-key", attached.as_str()], true);
        let said = format!("exit {code}: {stdout}{stderr}");
        let argv = scratch.child_argv(&said);
        let secret_words: Vec<&String> = argv
            .iter()
            .filter(|word| word.starts_with("--api-key-secret"))
            .collect();
        assert_eq!(secret_words.len(), 1, "{name}: {argv:?}");
        let carried = if secret_words[0] == "--api-key-secret" {
            let at = argv
                .iter()
                .position(|word| word == "--api-key-secret")
                .expect("present");
            argv.get(at + 1).cloned()
        } else {
            secret_words[0]
                .strip_prefix("--api-key-secret=")
                .map(ToOwned::to_owned)
        };
        assert_eq!(carried.as_deref(), Some(name), "{argv:?}");
        if name.starts_with('-') {
            assert!(
                !argv.contains(&"--api-key-secret".to_owned()),
                "{name}: a dash-led value travels attached: {argv:?}"
            );
        }
        for flag in CREDENTIAL_FLAGS {
            assert!(
                !argv
                    .iter()
                    .any(|word| word == flag || word.starts_with(&format!("{flag}="))),
                "{name}: {flag} became a word of its own: {argv:?}"
            );
        }
        let env = scratch.child_env(&said);
        assert!(!env.contains_key("OPENAI_API_KEY"), "{name}: {env:?}");
        assert!(
            !env.values().any(|value| value.contains(FIXTURE_KEY)),
            "{name}: {env:?}"
        );
        assert!(!stdout.contains(FIXTURE_KEY) && !stderr.contains(FIXTURE_KEY));
    }
}

/// Brief item 1: on the secret route no event metaharness writes, and nothing it prints, carries
/// the caller's key value, and the child's environment holds only the constructed base and the two
/// bus variables, whatever else the caller exported.
#[test]
fn no_output_and_no_child_variable_carries_the_key_on_the_secret_route() {
    let scratch = Scratch::new();
    let (code, stdout, stderr) = scratch.run(
        &[
            "--credentials",
            "api-key",
            "--api-key-secret",
            FIXTURE_SECRET,
        ],
        true,
    );
    let said = format!("exit {code}: {stdout}{stderr}");
    assert!(!stdout.contains(FIXTURE_KEY), "{said}");
    assert!(!stderr.contains(FIXTURE_KEY), "{said}");
    let env = scratch.child_env(&said);
    let mut names: Vec<&str> = env.keys().map(String::as_str).collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            "DBUS_SESSION_BUS_ADDRESS",
            "PATH",
            "XDG_CONFIG_HOME",
            "XDG_RUNTIME_DIR"
        ],
        "{env:?}"
    );
    assert!(
        !std::fs::read(&scratch.argv)
            .expect("argv")
            .windows(FIXTURE_KEY.len())
            .any(|window| window == FIXTURE_KEY.as_bytes()),
        "the key value reached the child's argv"
    );
}
