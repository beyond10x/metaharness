//! Adversary pass 1 on `story:doctor-run-help-probe-bounded`: the help the bounded `run --help`
//! probe reads is the help the binary printed.
//!
//! The unit's own case (`doctor_reads_the_help_a_run_help_prints_through_the_bounded_probe`) feeds
//! a help that declares none of the adapter's flags and expects every flag to fault as unknown.
//! That is also what an empty read produces, so the case stays green if the probe's stdout is
//! dropped. This file drives `metaharness doctor b10x` over a stand-in whose `run --help` declares
//! every flag the adapter sends with the arity it sends, and expects no fault at all: only a read
//! of the printed help can produce that.
//!
//! The flags and their arity are read off doctor itself, so this file names none: an empty help
//! makes doctor list every flag as unknown, and a help declaring each with a value makes doctor
//! name the ones the adapter sends bare. No model, no network and no credential is involved.
#![cfg(unix)]

use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt as _;
use std::path::Path;
use std::process::{Command, Stdio};

/// `metaharness doctor b10x` over a stand-in whose `run --help` prints `help`: exit code and
/// stdout.
fn doctor_over(help: &str) -> (Option<i32>, String) {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the stand-in's directory");
    let help_file = base.path().join("help.txt");
    std::fs::write(&help_file, help).expect("the help");
    let program = bin.join("b10x-harness");
    write_stand_in(&program, &help_file);
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["doctor", "b10x"])
        .env_clear()
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .stdin(Stdio::null())
        .output()
        .expect("the binary runs");
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        !stdout.is_empty(),
        "doctor printed nothing: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    (output.status.code(), stdout)
}

fn write_stand_in(program: &Path, help: &Path) {
    std::fs::write(
        program,
        format!(
            "#!/bin/sh\n\
             if [ \"$1\" = --version ]; then echo 'b10x-harness 0.0.0-stand-in'; exit 0; fi\n\
             if [ \"$1\" = run ] && [ \"$2\" = --help ]; then cat '{}'; exit 0; fi\n\
             exit 2\n",
            help.display()
        ),
    )
    .expect("the stand-in");
    std::fs::set_permissions(program, std::fs::Permissions::from_mode(0o700))
        .expect("the stand-in is executable");
}

/// The flags doctor names in lines `b10x-harness: --flag: <reason>` whose reason contains `needle`.
fn faulted(stdout: &str, needle: &str) -> Vec<String> {
    stdout
        .lines()
        .filter_map(|line| line.strip_prefix("b10x-harness: --"))
        .filter(|rest| rest.contains(needle))
        .filter_map(|rest| rest.split(':').next())
        .map(|flag| format!("--{flag}"))
        .collect()
}

#[test]
fn adv_doctor_b10x_judges_the_flags_against_the_help_the_probe_read() {
    let (_, empty) = doctor_over("Usage: b10x-harness run [OPTIONS]\n");
    let flags = faulted(&empty, "declares no such flag");
    assert!(
        flags.len() > 5,
        "an empty help faults every flag the adapter sends:\n{empty}"
    );

    let mut all_with_value = String::new();
    for flag in &flags {
        let _ = writeln!(all_with_value, "      {flag} <VALUE>");
    }
    let (_, valued) = doctor_over(&format!(
        "Usage: b10x-harness run [OPTIONS]\n\nOptions:\n{all_with_value}"
    ));
    let bare = faulted(&valued, "the adapter sends none");

    let mut declared = String::new();
    for flag in &flags {
        let _ = if bare.contains(flag) {
            writeln!(declared, "      {flag}\n          A switch")
        } else {
            writeln!(declared, "      {flag} <VALUE>\n          A value")
        };
    }
    let (code, judged) = doctor_over(&format!(
        "Usage: b10x-harness run [OPTIONS]\n\nOptions:\n{declared}"
    ));
    assert!(
        judged.contains("b10x-harness: every flag the adapter sends is one the binary declares"),
        "story:doctor-run-help-probe-bounded: a help declaring every flag the adapter sends, with \
         its arity, was judged as if unread:\n{judged}"
    );
    assert_eq!(
        code,
        Some(1),
        "off the pin and no flag fault is a gap (`RunExit::Gap`):\n{judged}"
    );
}
