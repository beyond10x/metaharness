//! Adversary pass 1, unit U4 (`story:aep-eval-scope-through-substrate`).
//!
//! Two surfaces.
//!
//! - Acceptance 1, the source scan. The unit's scan looks for the program name between two
//!   double quotes, so an invocation by absolute path, or inside a shell string, is not seen. The
//!   unit's matcher is two lines inside its test function, so it is replicated here verbatim
//!   (`format!("\"{program}\"")` and `str::contains`) and fed a hand-built source holding such an
//!   invocation. A scan that reads every code line of `src/` is the replacement this file offers.
//! - Acceptance 2, at the eval's entry point. The unit pins the refusal on `enter_delegated_scope`
//!   with a hand-built request; nothing drives `ensure_cgroup_scope`, the function `preflight`
//!   and `driven --arm b10x` call. Here the built `aep-eval preflight` runs with no session bus
//!   in its environment and a cgroup root it is not below, and the refusal it prints is read.
//!   Substrate refuses an unset bus before it calls `systemctl` or `busctl`, so nothing moves.
//!
//! The program name never appears here as a quoted literal: it is assembled at run time, as the
//! unit's own scan does, so this file is not itself an offence.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

/// Writing an executable while another thread forks leaves the write descriptor open in the
/// child until it execs, and the exec then fails with `ETXTBSY`. The cases run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn program() -> String {
    ["systemd", "run"].join("-")
}

/// Every line of `text` that names the program in code: not a comment, not an attribute.
fn code_lines_naming_the_program(text: &str) -> Vec<String> {
    let program = program();
    text.lines()
        .map(str::trim_start)
        .filter(|line| {
            !line.starts_with("//") && !line.starts_with("#[") && !line.starts_with("#![")
        })
        .filter(|line| line.contains(&program))
        .map(ToOwned::to_owned)
        .collect()
}

/// The same invocation the unit removed, with the program named by its absolute path, as
/// `std::process::Command` takes it.
fn absolute_path_invocation() -> String {
    format!(
        "fn enter_scope() {{\n    let mut command = Command::new(\"/usr/bin/{}\");\n    command.arg(\"--user\").arg(\"--scope\");\n}}\n",
        program()
    )
}

/// The same invocation, through a shell.
fn shell_invocation() -> String {
    format!(
        "fn enter_scope() {{\n    Command::new(\"/bin/sh\").arg(\"-c\").arg(\"{} --user --scope -- aep-eval preflight\");\n}}\n",
        program()
    )
}

/// The replacement scan: no code line of `src/` names the program, and it sees both forms the
/// unit's matcher misses.
#[test]
fn no_code_line_of_the_eval_source_names_the_program() {
    for source in [absolute_path_invocation(), shell_invocation()] {
        assert!(
            !code_lines_naming_the_program(&source).is_empty(),
            "the replacement scan misses:\n{source}"
        );
    }
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut offending = Vec::new();
    let mut read = 0;
    for entry in fs::read_dir(&src).expect("read src") {
        let path = entry.expect("src entry").path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).expect("read a source");
        read += 1;
        for line in code_lines_naming_the_program(&text) {
            offending.push(format!("{}: {line}", path.display()));
        }
    }
    assert!(read >= 2, "the scan read {read} sources");
    assert!(
        offending.is_empty(),
        "the program is still named in code: {offending:#?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2 at the entry point.
// ---------------------------------------------------------------------------------------------

fn work_root() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("adversary_u4_scope")
}

fn write_script(path: &Path, body: &str) {
    fs::write(path, body).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("make it executable");
}

/// A host with no session bus is refused by `aep-eval preflight` itself, before it reads the
/// harness: the refusal names the `systemd` adapter and carries the contract's code, and the
/// harness is never started. A preflight that swallowed the adapter's error, built its request
/// from anything but the environment, or skipped the open, fails here and not in the unit's
/// suite.
#[test]
fn the_preflight_refuses_a_host_without_a_session_bus_naming_the_systemd_adapter() {
    let _serial = serial();
    let case = work_root().join("no-session-bus");
    if case.exists() {
        fs::remove_dir_all(&case).expect("clear the previous case");
    }
    let aep = case.join("aep");
    let plugins = case.join("agentplugins");
    let harness_repo = case.join("harness");
    let home = case.join("home");
    let root = case.join("cgroup-root");
    for directory in [&aep, &plugins, &harness_repo, &home, &root] {
        fs::create_dir_all(directory).expect("create a case directory");
    }
    let started = case.join("harness-started");
    let harness = case.join("stand-in-harness");
    write_script(
        &harness,
        &format!("#!/bin/sh\n: > \"{}\"\nexit 0\n", started.display()),
    );

    let output = Command::new(env!("CARGO_BIN_EXE_aep-eval"))
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .arg("preflight")
        .arg("--aep-repo")
        .arg(&aep)
        .arg("--agentplugins-repo")
        .arg(&plugins)
        .arg("--harness-repo")
        .arg(&harness_repo)
        .arg("--b10x-binary")
        .arg(&harness)
        .arg("--scratch-root")
        .arg(case.join("scratch"))
        .arg("--cgroup-root")
        .arg(&root)
        .output()
        .expect("start aep-eval");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout:\n{stdout}\nstderr:\n{stderr}"
    );
    assert!(
        stdout.contains("entering a delegated scope from substrate's systemd adapter"),
        "the eval did not take the adapter path:\n{stdout}"
    );
    let failure = stderr
        .lines()
        .find(|line| line.starts_with("FAIL: "))
        .unwrap_or_else(|| panic!("no FAIL line:\n{stderr}"));
    assert!(
        failure.contains("the `systemd` confinement adapter serves no delegated scope"),
        "the refusal does not name the adapter: {failure}"
    );
    assert!(
        failure.contains("confinement.adapter-unserved"),
        "the refusal does not carry the contract's code: {failure}"
    );
    assert!(
        !started.exists(),
        "the harness was started after the refusal"
    );
}
