//! `story:aep-version-reads-bounded`, acceptance 1, through the public face: `metaharness aep drive
//! run --aep-binary <stub>`, where the stub's `--version` never exits, returns within the probe
//! deadline of a minute (amendment a65) with a refusal that names the binary, its `--version` and
//! the deadline.
//!
//! The verb is the one the binary dispatches to (`drive::run`, which `metaharness_aep::execute`
//! wraps), parsed the way the binary parses it, over a map of `operator` steps only: no model, no
//! spend and nothing on the network. `--aep-binary` alone takes the run past the command-only host
//! to the check of the selected executable, before any run directory is allocated. The stub runs
//! `sleep` in the foreground rather than `exec`, so the killed shell leaves the `sleep` behind
//! holding the pipe, and only the deadline ends the read.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::Parser;
use metaharness_aep::drive::DriveCommand;

/// The production deadline, `metaharness::VERSION_PROBE_DEADLINE` (a65).
const DEADLINE: Duration = Duration::from_secs(60);

/// How long the command may take: the deadline, the drain grace (5 s) and room for a loaded
/// machine.
const BOUND: Duration = Duration::from_secs(90);

/// How long the stub's `--version` sleeps: past the deadline, so only the deadline ends it.
const HANG_SECONDS: u32 = 75;

/// `metaharness aep drive`, parsed the way the binary parses it.
#[derive(Debug, Parser)]
#[command(name = "drive")]
struct Drive {
    #[command(subcommand)]
    command: DriveCommand,
}

/// The pinned AEP source tree, resolved from the exact dependency and never an ambient sibling.
fn aep_root() -> PathBuf {
    let output = std::process::Command::new(env!("CARGO"))
        .args(["metadata", "--locked", "--format-version", "1"])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: serde_json::Value =
        serde_json::from_slice(&output.stdout).expect("metadata is JSON");
    let manifest = metadata["packages"]
        .as_array()
        .expect("packages")
        .iter()
        .find(|package| package["name"] == "aep-cli")
        .expect("the linked aep-cli")["manifest_path"]
        .as_str()
        .expect("a manifest path")
        .to_owned();
    Path::new(&manifest)
        .parent()
        .expect("the crate directory")
        .join("../../..")
        .canonicalize()
        .expect("the AEP source tree")
}

/// One `operator` step: a map that needs no model, so only `--aep-binary` reaches the check.
const MAP: &str = concat!(
    "format: aep.driver-steps/1\n",
    "id: fixture/aep-binary-version-bounded\n",
    "workflow: adp/default/2\n",
    "states:\n",
    "  receive:\n",
    "    steps:\n",
    "      - kind: operator\n",
    "        prompt: judge this change\n",
);

#[test]
fn drive_run_refuses_an_aep_binary_whose_version_never_exits_by_name_at_the_deadline() {
    let root = aep_root();
    let scratch = tempfile::tempdir().expect("scratch directory");
    let project = scratch.path().join("project");
    std::fs::create_dir_all(project.join(".engineering/planning")).expect("the project");
    std::fs::write(
        project.join("task.yaml"),
        "id: VERSION-1\nkind: feature\nobjective: refuse a selected executable that never answers\n\
         protocol: adp/1\nprofile: development.standard\n",
    )
    .expect("the task");
    std::fs::write(project.join("steps.yaml"), MAP).expect("the map");
    let ran = scratch.path().join("ran");
    let stub = scratch.path().join("aep");
    std::fs::write(
        &stub,
        format!(
            "#!/bin/sh\nif [ \"$1\" = --version ]; then echo ran >> '{}'; sleep {HANG_SECONDS}; fi\nexit 0\n",
            ran.display()
        ),
    )
    .expect("the stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700)).expect("executable");

    let path = |path: &Path| path.to_str().expect("a printable path").to_owned();
    let argv = vec![
        "drive".to_owned(),
        "run".to_owned(),
        "--project".to_owned(),
        path(&project),
        "--root".to_owned(),
        path(&root),
        "--task".to_owned(),
        path(&project.join("task.yaml")),
        "--map".to_owned(),
        path(&project.join("steps.yaml")),
        "--allow-evidence-gap".to_owned(),
        "--aep-binary".to_owned(),
        path(&stub),
    ];
    let parsed = Drive::try_parse_from(&argv).unwrap_or_else(|error| {
        panic!(
            "`metaharness aep {}` does not parse: {error}",
            argv.join(" ")
        )
    });

    let (send, receive) = std::sync::mpsc::channel();
    let started = Instant::now();
    std::thread::spawn(move || {
        let _ =
            send.send(metaharness_aep::drive::run(parsed.command).map_err(|e| format!("{e:#}")));
    });
    let answer = receive.recv_timeout(BOUND).unwrap_or_else(|_| {
        panic!(
            "story:aep-version-reads-bounded: `metaharness aep drive run --aep-binary` did not \
             return within {BOUND:?} of a stub whose --version never exits"
        )
    });
    let elapsed = started.elapsed();
    assert!(
        ran.exists(),
        "the stub's --version never started: {answer:?}"
    );
    let refusal = answer.expect_err("a selected executable that never answered is refused");
    let canonical = std::fs::canonicalize(&stub).expect("the stub resolves");
    assert!(
        refusal.contains(&format!(
            "{} --version did not exit within {} ms and was killed",
            canonical.display(),
            DEADLINE.as_millis()
        )),
        "story:aep-version-reads-bounded: the refusal does not name the binary, its --version and \
         the deadline: {refusal} (after {elapsed:?})"
    );
    assert!(
        elapsed >= DEADLINE,
        "answered after {elapsed:?}, before the deadline: the stub did not hang"
    );
    assert!(
        !project.join(".engineering/runs/VERSION-1").exists(),
        "a refused executable allocated a run"
    );
}
