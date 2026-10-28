//! The b10x pin as a binary meets it (`story:b10x-harness-pin-0-22`): a `b10x-harness 0.21.0`
//! banner is off the pin, reads H9 `gap` and is refused by `--strict-version` before the spawn,
//! and a `b10x-harness 0.22.0` banner is admitted and spawned.
//!
//! The loop is a shell stub in a scratch `HOME`, on the `PATH` metaharness constructs for its
//! child (`$HOME/.local/bin` first). It answers `--version` with the banner it was written with,
//! records every other argv it is started with, and exits, so nothing reaches a model, a network
//! or a credential. The two banners are literals on purpose: a pin move that forgot
//! `metaharness_b10x::PINNED_VERSIONS` would agree with itself through the constant.
//!
//! One test, run in sequence: a stub written while another thread of this binary forks can be
//! refused `ETXTBSY`, and nothing here needs parallelism.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt;
use std::path::PathBuf;
use std::process::Command;

use metaharness::protocol::{HermeticMode, HermeticRow, Kind, Verdict};

const PREVIOUS: &str = "0.21.0";
const PINNED: &str = "0.22.0";

struct Stub {
    _base: tempfile::TempDir,
    home: PathBuf,
    program: PathBuf,
    argv: PathBuf,
}

impl Stub {
    fn new(version: &str) -> Self {
        let base = tempfile::tempdir().expect("a scratch directory");
        let home = base.path().join("home");
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).expect("the stub's directory");
        let argv = base.path().join("argv");
        let program = bin.join("b10x-harness");
        std::fs::write(
            &program,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then echo 'b10x-harness {version}'; exit 0; fi\n\
                 printf '%s\\n' \"$@\" > '{}'\n\
                 exit 0\n",
                argv.display()
            ),
        )
        .expect("the stub");
        std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
            .expect("the stub is executable");
        Self {
            _base: base,
            home,
            program,
            argv,
        }
    }

    /// `metaharness run b10x --strict-version --decisions observe …` under a cleared environment
    /// holding only `HOME` and `PATH`.
    fn run_strict(&self) -> (i32, String) {
        let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args([
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
            ])
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .output()
            .expect("the binary runs");
        let said = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        (output.status.code().unwrap_or(-1), said)
    }

    /// The argv the loop was spawned with, or `None` when only `--version` ran.
    fn spawned(&self) -> Option<Vec<String>> {
        std::fs::read_to_string(&self.argv)
            .ok()
            .map(|argv| argv.lines().map(ToOwned::to_owned).collect())
    }
}

fn h9(version: &str, stub: &Stub) -> (Verdict, bool, String) {
    let program = stub.program.to_str().expect("a utf-8 path");
    let found =
        metaharness::qualify(Kind::B10x, program, HermeticMode::Strict).expect("the stub resolves");
    assert_eq!(found.version.as_deref(), Some(version), "{found:?}");
    let row = found
        .rows
        .iter()
        .find(|row| row.row.row == HermeticRow::H9)
        .unwrap_or_else(|| panic!("H9 is not in {:?}", found.rows));
    (
        row.row.verdict,
        found.strict_version_refuses,
        row.row.detail.clone(),
    )
}

#[test]
fn harness_0_21_0_is_off_the_pin_and_0_22_0_is_admitted() {
    let previous = Stub::new(PREVIOUS);
    let (verdict, refuses, detail) = h9(PREVIOUS, &previous);
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    assert!(refuses, "--strict-version admits {PREVIOUS}: {detail}");
    assert!(detail.contains(PREVIOUS), "{detail}");

    let (code, said) = previous.run_strict();
    assert_eq!(code, 2, "{said}");
    assert!(
        said.contains("--strict-version requires one of [\"0.22.0\"]") && said.contains(PREVIOUS),
        "{said}"
    );
    assert_eq!(previous.spawned(), None, "refused before the spawn: {said}");

    let pinned = Stub::new(PINNED);
    let (verdict, refuses, detail) = h9(PINNED, &pinned);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(!refuses, "--strict-version refuses {PINNED}: {detail}");

    let (_, said) = pinned.run_strict();
    assert!(!said.contains("--strict-version requires"), "{said}");
    let argv = pinned
        .spawned()
        .unwrap_or_else(|| panic!("the pinned loop was not spawned: {said}"));
    assert_eq!(argv.first().map(String::as_str), Some("run"), "{argv:?}");

    assert_eq!(metaharness_b10x_pins(), vec![PINNED.to_owned()]);
}

/// The pin the adapter reports through the library's own surface, the one `doctor` and
/// `capabilities b10x` read.
fn metaharness_b10x_pins() -> Vec<String> {
    metaharness::capabilities(Kind::B10x)
        .expect("b10x has capabilities")
        .versions_pinned
}
