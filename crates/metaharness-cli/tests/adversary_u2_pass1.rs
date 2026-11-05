//! Adversary pass 1 on `story:version-probe-everywhere-bounded`, driven through the binary.
//!
//! The unit's own cases reach the deadline only through `installed_at` and `b10x_version_within`
//! at 300 ms, and reach the binary only with a stand-in that exits and leaves a process holding
//! its stdout, which the drain grace bounds, not the deadline. Nothing runs the acceptance itself:
//! a stand-in whose `--version` never exits, under `metaharness doctor b10x` and a strict
//! `metaharness run b10x`, at the production deadline of a minute. These cases do, so a deadline
//! that `installed` or `b10x_version` stopped passing would fail here.
//!
//! The stand-in sits in a scratch `$HOME/.local/bin`, first on the `PATH` metaharness constructs
//! for its child. Its `--version` runs `sleep` in the foreground rather than `exec`, so the killed
//! shell leaves the `sleep` behind holding the pipe. Nothing else is started and no model is
//! contacted (`--credentials none`, an endpoint on a closed port).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The production deadline, `VERSION_PROBE_DEADLINE` (a65).
const DEADLINE: Duration = Duration::from_secs(60);

/// How long one command may take: the deadline, the drain grace (5 s) and room for a loaded
/// machine.
const BOUND: Duration = Duration::from_secs(90);

/// How long the stand-in's `--version` sleeps: past the deadline, so only the deadline ends it.
const HANG_SECONDS: u32 = 75;

struct Stand {
    _base: tempfile::TempDir,
    home: PathBuf,
    ran: PathBuf,
    argv: PathBuf,
}

/// A `b10x-harness` stand-in whose `--version` runs `version_body` after recording that it started.
fn stand_in(version_body: &str) -> Stand {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the stand-in's directory");
    let ran = base.path().join("ran");
    let argv = base.path().join("argv");
    let program = bin.join("b10x-harness");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo ran >> '{ran}'; {version_body}; fi\n\
             if [ \"$1\" = run ] && [ \"$2\" = --help ]; then echo 'Usage: b10x-harness run [OPTIONS]'; exit 0; fi\n\
             printf '%s\\n' \"$@\" > '{argv}'\n\
             exit 0\n",
            ran = ran.display(),
            argv = argv.display()
        ),
    )
    .expect("the stand-in");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
        .expect("the stand-in is executable");
    Stand {
        _base: base,
        home,
        ran,
        argv,
    }
}

impl Stand {
    /// `metaharness <args…>` under a cleared environment holding only `HOME` and `PATH`, killed and
    /// failed when it has not exited within [`BOUND`]. A freshly written script can fail to start
    /// (`ETXTBSY`) while another test thread forks; such an attempt never reached the stand-in, so
    /// it is repeated until `ran` shows one that did.
    fn metaharness(&self, args: &[&str]) -> (i32, String, Duration) {
        for _ in 0..5 {
            let answer = self.once(args);
            if self.ran.exists() {
                return answer;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        panic!("the stand-in never started");
    }

    fn once(&self, args: &[&str]) -> (i32, String, Duration) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(args)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the binary runs");
        let started = Instant::now();
        while child.try_wait().expect("waitable").is_none() {
            if started.elapsed() > BOUND {
                child.kill().expect("killed");
                child.wait().expect("reaped");
                panic!(
                    "story:version-probe-everywhere-bounded: `metaharness {}` still running after \
                     {BOUND:?}, past the {DEADLINE:?} deadline and its grace",
                    args.join(" ")
                );
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let elapsed = started.elapsed();
        let output = child.wait_with_output().expect("the output");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.code().unwrap_or(-1), said, elapsed)
    }
}

/// The acceptance on `doctor`, through the binary at the production deadline: a `--version` that
/// never exits is refused by name, exit 2, after the deadline and inside the bound.
#[test]
fn doctor_over_a_version_that_never_exits_refuses_by_name_at_the_production_deadline() {
    let stand = stand_in(&format!("sleep {HANG_SECONDS}"));
    let (code, said, elapsed) = stand.metaharness(&["doctor", "b10x"]);
    assert_eq!(code, 2, "after {elapsed:?}:\n{said}");
    assert!(
        said.contains("--version did not exit within 60000 ms and was killed"),
        "after {elapsed:?}:\n{said}"
    );
    assert!(
        elapsed >= DEADLINE,
        "answered after {elapsed:?}, before the deadline: the stand-in did not hang:\n{said}"
    );
}

/// The acceptance on a strict b10x launch, through the binary at the production deadline: a
/// `--version` that never exits is refused before the spawn, exit 2, inside the bound, and the
/// refusal names why, as the Claude and Codex launches name it (a33, a34: "refused by name … when
/// the version is not pinned, not readable or late"). A binary that was killed at the deadline
/// reported nothing; "reported no readable version" names a different cause.
#[test]
fn a_strict_b10x_launch_over_a_version_that_never_exits_refuses_naming_the_deadline() {
    let stand = stand_in(&format!("sleep {HANG_SECONDS}"));
    let (code, said, elapsed) = stand.metaharness(&[
        "run",
        "b10x",
        "--strict-version",
        "--decisions",
        "observe",
        "--credentials",
        "none",
        "--model-endpoint",
        "http://127.0.0.1:1/v1",
        "--model",
        "a-model",
        "-p",
        "go",
    ]);
    assert_eq!(code, 2, "after {elapsed:?}:\n{said}");
    assert!(said.contains("--strict-version requires"), "{said}");
    assert!(
        !stand.argv.exists(),
        "refused before the spawn, and the loop was spawned anyway: {said}"
    );
    assert!(
        elapsed >= DEADLINE,
        "answered after {elapsed:?}, before the deadline: the stand-in did not hang:\n{said}"
    );
    assert!(
        said.contains("did not exit within 60000 ms"),
        "the strict b10x refusal does not say the binary was killed at the deadline (after \
         {elapsed:?}):\n{said}"
    );
}

/// `doctor` is the free pre-flight an operator reads to find out why a binary is unusable. Before
/// this unit, a `--version` that failed was refused with its exit status and its stderr; the
/// bounded probe keeps neither, and the refusal now reads the same for a binary that could not be
/// started, one that failed and one that printed no version.
#[test]
fn doctor_names_the_exit_status_and_stderr_of_a_version_that_fails() {
    let stand = stand_in(
        "echo 'b10x-harness: error while loading shared libraries: libstand.so' >&2; exit 3",
    );
    let (code, said, elapsed) = stand.metaharness(&["doctor", "b10x"]);
    assert_eq!(code, 2, "after {elapsed:?}:\n{said}");
    assert!(
        said.contains("exited 3") && said.contains("libstand.so"),
        "doctor's refusal names neither the exit status nor what the binary said:\n{said}"
    );
}
