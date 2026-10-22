//! The argv `metaharness doctor b10x` spawns the harness binary with, read off a stand-in.
//!
//! Doctor resolves `b10x-harness` on the `PATH` the spawn constructs, which starts at
//! `$HOME/.local/bin`, so a `/bin/sh` stand-in there under a scratch `HOME` is what it runs. The
//! stand-in records every argv it is given and answers `--version` and `run --help`; nothing
//! else is started and no model is contacted.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::PathBuf;
use std::process::Command;

/// Doctor asks the harness two questions, `--version` and `run --help`, and no argv it spawns
/// holds a verb harness 0.18.0 removed (`workflow plan|run`, `events`). The removed verbs are
/// assembled at run time so this file names none.
#[test]
fn doctor_spawns_the_harness_with_its_version_and_run_help_only() {
    let case = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("doctor_harness_argv");
    if case.exists() {
        fs::remove_dir_all(&case).expect("clear the previous case");
    }
    let bin = case.join("home/.local/bin");
    fs::create_dir_all(&bin).expect("create the stand-in bin directory");
    let log = case.join("argv.log");
    let stand_in = bin.join("b10x-harness");
    fs::write(
        &stand_in,
        format!(
            "#!/bin/sh\nfor argument in \"$@\"; do printf '[%s]' \"$argument\" >> '{log}'; done\necho >> '{log}'\nif [ \"$1\" = --version ]; then echo 'b10x-harness 0.0.0-stand-in'; exit 0; fi\necho 'Usage: b10x-harness run [OPTIONS] --base-url <BASE_URL>'\n",
            log = log.display(),
        ),
    )
    .expect("write the stand-in harness");
    fs::set_permissions(&stand_in, fs::Permissions::from_mode(0o755))
        .expect("make the stand-in executable");

    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .env_clear()
        .env("HOME", case.join("home"))
        .env("PATH", "/usr/bin:/bin")
        .args(["doctor", "b10x"])
        .output()
        .expect("start metaharness doctor");
    let spawned = fs::read_to_string(&log).unwrap_or_default();
    let spawned: Vec<&str> = spawned.lines().collect();
    assert_eq!(
        spawned,
        ["[--version]", "[run][--help]"],
        "doctor's harness argv changed; stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let removed = [["work", "flow"].concat(), ["eve", "nts"].concat()];
    for argv in &spawned {
        assert!(
            !argv
                .split(['[', ']', ' '])
                .any(|word| removed.iter().any(|verb| word == verb)),
            "{argv}"
        );
    }
}
