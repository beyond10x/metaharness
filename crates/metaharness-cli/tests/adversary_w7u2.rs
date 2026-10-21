//! `run`'s `--spec`/`--auditor` pairing through the built binary, past the cases amendment a43
//! lists.
//!
//! Two groups. The first asserts the outcome for inputs the check before the spawn does not cover:
//! an auditor prefix whose first word names no program, and auditor arguments after `--` with no
//! `--auditor`. In both the expectation document is checked by nobody, so each expects exit `2`
//! with no harness process started.
//!
//! The second pins where the check sits. The scratch tree, the `--strict-version` probe, the
//! loopback proxy, the frame and the catalogue each refuse by their own name when they run first
//! on the inputs below, so a run that reports the pairing refusal shows that none of them ran
//! before it. A FIFO named as the document must be refused rather than waited on.
//!
//! Nothing real is reached: `HOME`, `TMPDIR` and `PATH` are scratch directories, no credential
//! exists in the scratch home, and the only harness on the child's `PATH` is a fake `claude` that
//! writes a marker when it is launched, `--version` included.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What the binary prints first when the document has nobody to check it.
const SPEC_WITHOUT_AUDITOR: &str = "metaharness: --spec was given without --auditor";

/// How long a refused run may take before the case calls it stuck.
const DEADLINE: Duration = Duration::from_secs(20);

struct World {
    _base: tempfile::TempDir,
    base: PathBuf,
    home: PathBuf,
    path: PathBuf,
    tmp: PathBuf,
    harness_marker: PathBuf,
    document: PathBuf,
}

struct Outcome {
    code: i32,
    stderr: String,
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("executable");
}

fn world() -> World {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let path = base.path().join("path");
    let tmp = base.path().join("tmp");
    let bin = home.join(".local/bin");
    for directory in [&home, &path, &tmp, &bin] {
        std::fs::create_dir_all(directory).expect("a scratch directory");
    }
    let harness_marker = base.path().join("harness-launched");
    let marker = harness_marker.to_str().expect("a UTF-8 scratch path");
    assert!(!marker.contains('\''), "the marker path quotes as is");
    executable(
        &bin.join("claude"),
        &format!("#!/bin/sh\necho \"$@\" >> '{marker}'\nexit 0\n"),
    );
    let document = base.path().join("expectations.yaml");
    std::fs::write(&document, "kinds: []\n").expect("written");
    World {
        base: base.path().to_path_buf(),
        _base: base,
        home,
        path,
        tmp,
        harness_marker,
        document,
    }
}

impl World {
    fn document(&self) -> &str {
        self.document.to_str().expect("a UTF-8 scratch path")
    }

    /// `metaharness run <args…>` with this world's `HOME`, `PATH` and the given `TMPDIR`, waited
    /// on for at most [`DEADLINE`].
    fn run_in(&self, args: &[&str], tmpdir: &Path) -> Outcome {
        let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .arg("run")
            .args(args)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("TMPDIR", tmpdir)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let started = Instant::now();
        loop {
            if child.try_wait().expect("waitable").is_some() {
                break;
            }
            if started.elapsed() > DEADLINE {
                child.kill().expect("killed");
                child.wait().expect("reaped");
                panic!("{args:?}: still running after {DEADLINE:?}");
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let output = child.wait_with_output().expect("the output");
        Outcome {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn run(&self, args: &[&str]) -> Outcome {
        self.run_in(args, &self.tmp)
    }

    /// What the fake harness was launched with, one line per launch, or `None` when it never was.
    fn launches(&self) -> Option<String> {
        std::fs::read_to_string(&self.harness_marker).ok()
    }

    /// Exit `2`, no launch, and stderr opening with `words`.
    fn refused_before_the_spawn(&self, args: &[&str], tmpdir: &Path, words: &str) {
        let outcome = self.run_in(args, tmpdir);
        assert_eq!(
            self.launches(),
            None,
            "{args:?}: the harness was launched (exit {}); stderr: {}",
            outcome.code,
            outcome.stderr
        );
        assert_eq!(outcome.code, 2, "{args:?}: stderr: {}", outcome.stderr);
        assert!(
            outcome.stderr.starts_with(words),
            "{args:?}: expected stderr to open with {words:?}; stderr: {}",
            outcome.stderr
        );
    }
}

// ------------------------------------------------- documents the check before the spawn admits

/// An `--auditor` whose first word resolves to no program: an absolute path to nothing, a name on
/// no `PATH` directory, and a prefix whose first word is a flag. Each is refused by the audit after
/// the session, so the harness runs and the document is checked by nobody.
#[test]
fn an_auditor_whose_program_cannot_be_started_exits_2_before_the_harness_is_launched() {
    let mut failures = Vec::new();
    for prefix in ["<base>/no-such-auditor", "no-such-auditor check", "--spec"] {
        let world = world();
        let prefix = prefix.replace("<base>", world.base.to_str().expect("a UTF-8 scratch path"));
        let auditor = format!("--auditor={prefix}");
        let args = [
            "claude",
            "--prompt",
            "go",
            "--credentials",
            "none",
            "--audit",
            "--spec",
            world.document(),
            &auditor,
        ];
        let outcome = world.run(&args);
        if let Some(launches) = world.launches() {
            failures.push(format!(
                "--auditor {prefix:?}: the harness was launched ({}), exit {}, stderr: {}",
                launches.trim_end(),
                outcome.code,
                outcome.stderr.trim_end()
            ));
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

/// Arguments after `--` are the auditor's. With no `--auditor` nothing receives them, and here
/// they name an expectation document that nothing checks.
#[test]
fn auditor_arguments_without_an_auditor_exit_2_before_the_harness_is_launched() {
    let mut failures = Vec::new();
    for audit in [&["--audit"][..], &[][..]] {
        let world = world();
        let mut args = vec!["claude", "--prompt", "go", "--credentials", "none"];
        args.extend_from_slice(audit);
        args.extend_from_slice(&["--", "--spec", world.document()]);
        let outcome = world.run(&args);
        if let Some(launches) = world.launches() {
            failures.push(format!(
                "{args:?}: the harness was launched ({}), exit {}, stderr: {}",
                launches.trim_end(),
                outcome.code,
                outcome.stderr.trim_end()
            ));
        } else {
            assert_eq!(outcome.code, 2, "{args:?}: stderr: {}", outcome.stderr);
        }
    }
    assert!(failures.is_empty(), "\n{}", failures.join("\n"));
}

// ------------------------------------------------------------- where the check sits in `start`

/// A `TMPDIR` that does not exist: the scratch tree, made first, would refuse with an I/O error.
#[test]
fn a_refused_pairing_makes_no_scratch_tree_on_any_kind() {
    for kind in ["claude", "codex", "b10x"] {
        let world = world();
        let absent = world.base.join("no-such-tmpdir");
        world.refused_before_the_spawn(
            &[
                kind,
                "--prompt",
                "go",
                "--credentials",
                "none",
                "--spec",
                world.document(),
            ],
            &absent,
            SPEC_WITHOUT_AUDITOR,
        );
        assert!(!absent.exists(), "{kind}: nothing made the temporary root");
    }
}

/// `--strict-version` runs `claude --version` before the spawn; the fake records that call too.
#[test]
fn a_refused_pairing_runs_no_version_probe() {
    let world = world();
    world.refused_before_the_spawn(
        &[
            "claude",
            "--prompt",
            "go",
            "--credentials",
            "none",
            "--strict-version",
            "--spec",
            world.document(),
        ],
        &world.tmp,
        SPEC_WITHOUT_AUDITOR,
    );
}

/// `--credentials loopback` with no credential file in the scratch home: the proxy, started
/// first, would refuse for the missing file.
#[test]
fn a_refused_pairing_starts_no_loopback_proxy() {
    let world = world();
    world.refused_before_the_spawn(
        &[
            "claude",
            "--prompt",
            "go",
            "--credentials",
            "loopback",
            "--spec",
            world.document(),
        ],
        &world.tmp,
        SPEC_WITHOUT_AUDITOR,
    );
}

/// A frame document and a catalogue that do not exist: either, read first, refuses by its name.
#[test]
fn a_refused_pairing_reads_no_frame_and_no_catalogue() {
    let world = world();
    let missing = world.base.join("no-such-document.json");
    let missing = missing.to_str().expect("a UTF-8 scratch path");
    for flag in ["--frame", "--catalogue"] {
        world.refused_before_the_spawn(
            &[
                "claude",
                "--prompt",
                "go",
                "--credentials",
                "none",
                flag,
                missing,
                "--spec",
                world.document(),
            ],
            &world.tmp,
            SPEC_WITHOUT_AUDITOR,
        );
    }
}

/// A FIFO named as the document is refused, and the refusal does not wait for a writer.
#[test]
fn a_fifo_named_as_the_expectation_document_is_refused_without_waiting() {
    let world = world();
    let fifo_path = world.base.join("expectations.fifo");
    let made = Command::new("mkfifo")
        .arg(&fifo_path)
        .status()
        .expect("mkfifo runs");
    assert!(made.success(), "a FIFO was made");
    let fifo = fifo_path.to_str().expect("a UTF-8 scratch path");
    let auditor = world.base.join("auditor");
    executable(&auditor, "#!/bin/sh\necho 'one ok'\n");
    world.refused_before_the_spawn(
        &[
            "claude",
            "--prompt",
            "go",
            "--credentials",
            "none",
            "--audit",
            "--spec",
            fifo,
            "--auditor",
            auditor.to_str().expect("a UTF-8 scratch path"),
        ],
        &world.tmp,
        &format!(
            "metaharness: the expectation document {fifo} could not be read: it is not a regular \
             file"
        ),
    );
}
