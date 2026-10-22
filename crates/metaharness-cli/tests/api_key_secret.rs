//! `--api-key-secret` through the built binary (amendment a50, `story:b10x-api-key-from-keychain`).
//!
//! The loop is a shell stub in a scratch `HOME`, on the `PATH` metaharness constructs for its child
//! (`$HOME/.local/bin` first). It records the argv and the environment it was started with and
//! exits, so nothing reaches a model, a network or a secret store. The secret name is the fixture
//! `llm/opencode-go-api-key`; the session-bus variables point into the scratch directory, where no
//! bus listens, and `OPENAI_API_KEY` holds a fixture string that must not travel.
#![cfg(unix)]

use std::collections::BTreeMap;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;

const FIXTURE_SECRET: &str = "llm/opencode-go-api-key";
const FIXTURE_KEY: &str = "fixture-key-value-that-must-not-travel";

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
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then echo 'b10x-harness 0.18.0'; exit 0; fi\n\
                 printf '%s\\n' \"$@\" > '{}'\n\
                 /usr/bin/env > '{}'\n\
                 exit 0\n",
                argv.display(),
                env.display()
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

    /// `metaharness run <kind> …` under a cleared environment holding only `HOME`, `PATH`, the two
    /// session-bus variables, `OPENAI_API_KEY` and one unrelated variable.
    fn run(&self, kind: &str, extra: &[&str]) -> (i32, String) {
        let mut args = vec![
            "run",
            kind,
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
        let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(&args)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env("DBUS_SESSION_BUS_ADDRESS", &self.bus)
            .env("XDG_RUNTIME_DIR", &self.runtime)
            .env("OPENAI_API_KEY", FIXTURE_KEY)
            .env("UNRELATED_VARIABLE", "must-not-travel-either")
            .output()
            .expect("the binary runs");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.code().unwrap_or(-1), said)
    }

    fn spawned(&self) -> bool {
        self.argv.exists()
    }

    fn child_argv(&self, said: &str) -> Vec<String> {
        std::fs::read_to_string(&self.argv)
            .unwrap_or_else(|error| panic!("the loop was not spawned ({error}): {said}"))
            .lines()
            .map(ToOwned::to_owned)
            .collect()
    }

    /// The child's environment, without the three variables `/bin/sh` itself exports into it.
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

fn value_after(argv: &[String], flag: &str) -> Option<String> {
    argv.iter()
        .position(|word| word == flag)
        .and_then(|at| argv.get(at + 1))
        .cloned()
}

/// The acceptance's first case: before a50 clap refused the flag as unknown; after it the loop is
/// pointed at the name, no other key route is on its argv, and no key variable is in its
/// environment.
#[test]
fn a_named_secret_reaches_the_loop_as_its_flag_and_no_key_reaches_its_environment() {
    let scratch = Scratch::new();
    let (code, said) = scratch.run(
        "b10x",
        &[
            "--credentials",
            "api-key",
            "--api-key-secret",
            FIXTURE_SECRET,
        ],
    );
    let argv = scratch.child_argv(&said);
    assert_eq!(
        value_after(&argv, "--api-key-secret").as_deref(),
        Some(FIXTURE_SECRET),
        "exit {code}: {argv:?}"
    );
    assert!(
        !argv
            .iter()
            .any(|word| word.starts_with("--api-key-env") || word.starts_with("--api-key-file")),
        "{argv:?}"
    );
    let env = scratch.child_env(&said);
    assert!(
        !env.contains_key("OPENAI_API_KEY"),
        "the key variable reached the child: {env:?}"
    );
    assert!(
        !env.values().any(|value| value.contains(FIXTURE_KEY)),
        "the key's value reached the child: {env:?}"
    );
    assert!(
        !said.contains(FIXTURE_KEY),
        "the key's value reached metaharness's own output: {said}"
    );
}

/// D29: on the secret route the child gets the two variables the store's client finds the session
/// bus by, as the caller had them, and nothing else beyond the constructed base.
#[test]
fn the_secret_route_forwards_the_session_bus_variables_and_nothing_else() {
    let scratch = Scratch::new();
    let (_, said) = scratch.run(
        "b10x",
        &[
            "--credentials",
            "api-key",
            "--api-key-secret",
            FIXTURE_SECRET,
        ],
    );
    let env = scratch.child_env(&said);
    assert_eq!(
        env.get("DBUS_SESSION_BUS_ADDRESS"),
        Some(&scratch.bus),
        "{env:?}"
    );
    assert_eq!(
        env.get("XDG_RUNTIME_DIR").map(PathBuf::from),
        Some(scratch.runtime.clone()),
        "{env:?}"
    );
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
}

/// The route that names no secret is the route it was: the key is read from `OPENAI_API_KEY` by
/// name, and the session-bus variables are not forwarded to it.
#[test]
fn without_a_name_the_api_key_route_still_reads_the_variable_and_forwards_no_bus() {
    let scratch = Scratch::new();
    let (_, said) = scratch.run("b10x", &["--credentials", "api-key"]);
    let argv = scratch.child_argv(&said);
    assert_eq!(
        value_after(&argv, "--api-key-env").as_deref(),
        Some("OPENAI_API_KEY"),
        "{argv:?}"
    );
    assert!(
        !argv.iter().any(|word| word.starts_with("--api-key-secret")),
        "{argv:?}"
    );
    let env = scratch.child_env(&said);
    assert_eq!(
        env.get("OPENAI_API_KEY").map(String::as_str),
        Some(FIXTURE_KEY)
    );
    assert!(!env.contains_key("DBUS_SESSION_BUS_ADDRESS"), "{env:?}");
    assert!(!env.contains_key("XDG_RUNTIME_DIR"), "{env:?}");
}

fn assert_refused_before_the_spawn(scratch: &Scratch, code: i32, said: &str) {
    assert_eq!(code, 2, "{said}");
    assert!(!scratch.spawned(), "the loop was spawned: {said}");
}

/// A name that is not `<namespace>/<name>` with both halves present is refused before the spawn,
/// naming the value as given.
#[test]
fn a_name_that_is_not_namespace_slash_name_is_refused_naming_the_value() {
    for bad in ["opencode-go-api-key", "llm/", "/opencode-go-api-key", ""] {
        let scratch = Scratch::new();
        let (code, said) = scratch.run(
            "b10x",
            &["--credentials", "api-key", "--api-key-secret", bad],
        );
        assert_refused_before_the_spawn(&scratch, code, &said);
        assert!(
            said.contains(&format!("`{bad}`")),
            "{bad:?} is not named: {said}"
        );
        assert!(
            said.contains("<namespace>/<name>"),
            "{bad:?}: the shape is not named: {said}"
        );
    }
}

/// A name beside any credential source but `api-key`, or beside a subscription source, is two
/// credentials for one request, in the words of the refusal that already said so.
#[test]
fn a_name_beside_another_credential_is_refused_as_two_credentials() {
    for source in ["none", "operator-login", "loopback"] {
        let scratch = Scratch::new();
        let (code, said) = scratch.run(
            "b10x",
            &["--credentials", source, "--api-key-secret", FIXTURE_SECRET],
        );
        assert_refused_before_the_spawn(&scratch, code, &said);
        assert!(
            said.contains(&format!(
                "--api-key-secret and --credentials {source} are two credentials for one request"
            )),
            "{source}: {said}"
        );
    }
    for subscription in [
        ["--subscription-token-file", "/a/store.json"],
        ["--subscription-token-env", "A_TOKEN"],
    ] {
        let scratch = Scratch::new();
        let mut args = vec![
            "--credentials",
            "api-key",
            "--api-key-secret",
            FIXTURE_SECRET,
        ];
        args.extend_from_slice(&subscription);
        let (code, said) = scratch.run("b10x", &args);
        assert_refused_before_the_spawn(&scratch, code, &said);
        assert!(
            said.contains(
                "a subscription token and --credentials api-key are two credentials for one request"
            ),
            "{subscription:?}: {said}"
        );
    }
}

/// The vendor kinds have no secret route: the flag is refused by name before anything is spawned.
/// `PATH` holds neither `claude` nor `codex`, so a refusal that failed to fire would end at "not
/// found", never at a spawn.
#[test]
fn claude_and_codex_refuse_the_name_by_name() {
    for kind in ["claude", "codex"] {
        let scratch = Scratch::new();
        let (code, said) = scratch.run(
            kind,
            &[
                "--credentials",
                "api-key",
                "--api-key-secret",
                FIXTURE_SECRET,
            ],
        );
        assert_eq!(code, 2, "{kind}: {said}");
        assert!(
            said.contains("--api-key-secret is the b10x loop's"),
            "{kind}: {said}"
        );
        assert!(said.contains(kind), "{kind}: {said}");
        assert!(!said.contains("unexpected argument"), "{kind}: {said}");
    }
}

#[test]
fn the_fixture_is_a_name_and_the_scratch_holds_no_store() {
    // The tests above never hand the stub anything a store could answer: the name is the fixture,
    // and the bus address names a path in the scratch directory that does not exist.
    let scratch = Scratch::new();
    assert!(!Path::new(scratch.bus.trim_start_matches("unix:path=")).exists());
    assert_eq!(
        FIXTURE_SECRET.split_once('/'),
        Some(("llm", "opencode-go-api-key"))
    );
}
