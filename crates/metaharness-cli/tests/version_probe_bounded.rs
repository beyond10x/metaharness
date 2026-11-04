//! `doctor` and the b10x launch read `--version` under the probe's deadline and drain grace
//! (design amendment a65, `story:version-probe-everywhere-bounded`).
//!
//! The stand-in `b10x-harness` sits in a scratch `$HOME/.local/bin`, first on the `PATH` metaharness
//! constructs for its child. It answers `--version` with a release outside the pin and exits, and
//! leaves a background process behind that holds its stdout open for longer than each case waits.
//! A reader that waits for end of file blocks on that process; the bounded probe takes what the
//! binary printed once the drain grace has passed. Nothing else is started and no model is
//! contacted (`--credentials none`, an endpoint on a closed port).
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// What the stand-in reports: off the adapter's pin, so a strict launch refuses it by name.
const REPORTED: &str = "0.0.0-stand-in";

/// How long the background process holds the pipe: longer than [`BOUND`], so a reader that waits
/// for end of file is still waiting when the case gives up on it.
const HELD_SECONDS: u32 = 40;

/// How long one command may take: the drain grace (5 s) plus room for a loaded machine, and well
/// under [`HELD_SECONDS`].
const BOUND: Duration = Duration::from_secs(20);

struct Stand {
    _base: tempfile::TempDir,
    home: PathBuf,
    argv: PathBuf,
}

fn stand_in() -> Stand {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the stand-in's directory");
    let argv = base.path().join("argv");
    let program = bin.join("b10x-harness");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo 'b10x-harness {REPORTED}'; sleep {HELD_SECONDS} & exit 0; fi\n\
             if [ \"$1\" = run ] && [ \"$2\" = --help ]; then echo 'Usage: b10x-harness run [OPTIONS]'; exit 0; fi\n\
             printf '%s\\n' \"$@\" > '{}'\n\
             exit 0\n",
            argv.display()
        ),
    )
    .expect("the stand-in");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
        .expect("the stand-in is executable");
    Stand {
        _base: base,
        home,
        argv,
    }
}

impl Stand {
    /// `metaharness <args…>` under a cleared environment holding only `HOME` and `PATH`, killed and
    /// failed when it has not exited within [`BOUND`].
    fn metaharness(&self, args: &[&str]) -> (i32, String, Duration) {
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
                     {BOUND:?}; its --version read waits on a pipe the stand-in's leftover process \
                     holds",
                    args.join(" ")
                );
            }
            std::thread::sleep(Duration::from_millis(50));
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

/// `doctor b10x` reports the version the stand-in printed, inside the bound, although a process it
/// left behind still holds the pipe.
#[test]
fn doctor_reads_the_version_of_a_binary_whose_leftover_process_holds_stdout_within_the_grace() {
    let stand = stand_in();
    let (code, said, elapsed) = stand.metaharness(&["doctor", "b10x"]);
    assert!(
        said.contains(REPORTED) && said.contains("OFF the adapter's pin"),
        "doctor's report after {elapsed:?}, exit {code}:\n{said}"
    );
    assert_eq!(
        code, 1,
        "an off-pin binary is a gap (`RunExit::Gap`): {said}"
    );
}

/// A strict b10x launch refuses the off-pin stand-in by name, before the spawn, inside the bound.
#[test]
fn a_strict_b10x_launch_refuses_a_binary_whose_leftover_process_holds_stdout_within_the_grace() {
    let stand = stand_in();
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
    assert!(
        said.contains("--strict-version requires") && said.contains(REPORTED),
        "{said}"
    );
    assert!(
        !stand.argv.exists(),
        "refused before the spawn, and the loop was spawned anyway: {said}"
    );
}
