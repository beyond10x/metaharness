//! Adversary pass 1 on `story:doctor-run-help-probe-bounded`, acceptance 2, through the binary:
//! `metaharness doctor b10x` over a stand-in `b10x-harness` whose `--version` answers at once and
//! whose `run --help` never exits returns within the production probe deadline of a minute
//! (design amendment a65 point 4) with a refusal naming the binary, its `run --help` and the
//! deadline. The implementor drove only `flag_surface_at` at a 300 ms seam; nothing drove the
//! verb an operator types.
//!
//! The stand-in sits in a scratch `$HOME/.local/bin`, first on the `PATH` doctor constructs. Its
//! `run --help` prints a partial help and then runs `sleep` in the foreground rather than `exec`,
//! so the killed shell leaves the `sleep` behind holding both pipes and only the deadline ends the
//! read. No model, no network and no credential is involved.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// The production deadline, `metaharness::VERSION_PROBE_DEADLINE` (a65).
const DEADLINE: Duration = Duration::from_secs(60);

/// How long the verb may take: the deadline, the drain grace (5 s) and room for a loaded machine.
const BOUND: Duration = Duration::from_secs(90);

/// How long the stand-in's `run --help` sleeps: past [`BOUND`], so only the deadline ends it.
const HANG_SECONDS: u32 = 100;

#[test]
fn adv_doctor_b10x_refuses_a_run_help_that_never_exits_by_name_at_the_production_deadline() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the stand-in's directory");
    let asked = base.path().join("asked-for-help");
    let program = bin.join("b10x-harness");
    std::fs::write(
        &program,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo 'b10x-harness 0.0.0-stand-in'; exit 0; fi\n\
             if [ \"$1\" = run ] && [ \"$2\" = --help ]; then echo asked >> '{}'; \
             echo 'Usage: b10x-harness run [OPTIONS]'; sleep {HANG_SECONDS}; exit 0; fi\n\
             exit 2\n",
            asked.display()
        ),
    )
    .expect("the stand-in");
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
        .expect("the stand-in is executable");

    let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["doctor", "b10x"])
        .env_clear()
        .env("HOME", &home)
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
                "story:doctor-run-help-probe-bounded: `metaharness doctor b10x` still running \
                 after {BOUND:?} over a run --help that never exits"
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
    assert!(
        asked.exists(),
        "the stand-in's run --help never started:\n{said}"
    );
    assert_eq!(
        output.status.code(),
        Some(2),
        "a help that never came is a refusal (`RunExit::Broken`), after {elapsed:?}:\n{said}"
    );
    assert!(
        said.contains(&format!(
            "{} run --help did not exit within {} ms and was killed",
            program.display(),
            DEADLINE.as_millis()
        )),
        "story:doctor-run-help-probe-bounded: the refusal does not name the binary, its run \
         --help and the deadline, after {elapsed:?}:\n{said}"
    );
    assert!(
        elapsed >= DEADLINE,
        "answered after {elapsed:?}, before the deadline: the stand-in did not hang"
    );
}
