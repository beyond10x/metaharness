//! A paused legacy model run keeps its authority when the new host resumes it.
#![cfg(unix)]
mod support;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

/// The third binary the launch pre-flights name, provided the same way as the two this file stubs.
///
/// `metaharness_preflight` asks whether `metaharness` is on **this process's** `PATH` — not the
/// constructed session `PATH` the `aep` and `b10x-harness` stubs answer — and the map this test
/// writes has one `llm` step, so an inherited `PATH` decided it rather than the code: green where
/// the operator has run `cargo install`, red on a clean runner. Gate run `34947429804` failed
/// at the first assertion in this file for exactly that reason, and `cargo test` stops at the first
/// failing target, so every later one went unmeasured.
///
/// The build under test already produces that binary, so the child is given its directory instead
/// of the developer's shell. The pre-flight only asks whether the file is there, and
/// `--pause-on-approval` stops the run at the operator step before the `llm` step, so nothing here
/// spawns it.
fn path_holding_the_binary_under_test() -> std::ffi::OsString {
    let built = std::path::Path::new(env!("CARGO_BIN_EXE_metaharness"))
        .parent()
        .expect("the binary under test lives in a directory")
        .to_path_buf();
    let ambient = std::env::var_os("PATH").unwrap_or_default();
    let directories = std::iter::once(built).chain(std::env::split_paths(&ambient));
    std::env::join_paths(directories).expect("a PATH this process can already hold")
}

/// The two binaries the pre-flights resolve on the **constructed** session `PATH`, which is
/// `$HOME/.local/bin` first and `HOME` is this project.
///
/// Pre-flight only reads these stubs, so an accidental model launch cannot contact a provider.
/// Returns the `aep` stub's path, which the invocation also names as `--aep-binary`.
fn stub_the_binaries_on_the_session_path(
    project: &std::path::Path,
    root: &std::path::Path,
) -> std::path::PathBuf {
    let bin = project.join(".local/bin");
    std::fs::create_dir_all(&bin).unwrap();
    std::fs::write(bin.join("b10x-harness"), "#!/bin/sh\nexit 99\n").unwrap();
    let manifest = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let version = manifest
        .lines()
        .find_map(|line| {
            line.strip_prefix("version = \"")
                .and_then(|value| value.strip_suffix('"'))
        })
        .expect("pinned workspace version");
    let aep = bin.join("aep");
    std::fs::write(
        &aep,
        format!("#!/bin/sh\n[ \"$1\" = --version ] || exit 99\necho 'aep {version}'\n"),
    )
    .unwrap();
    std::fs::set_permissions(&aep, std::fs::Permissions::from_mode(0o700)).unwrap();
    aep
}

#[test]
fn legacy_launch_resumes_without_spending_or_losing_configuration() {
    paused_run_resumes(&["--budget-usd", "1", "--assume-usd-per-run", "0.1"]);
}

#[test]
fn uncapped_launch_resumes_without_spending_or_replacing_authority() {
    paused_run_resumes(&[
        "--uncapped-budget",
        "--spend-authorization",
        "operator:approval-14",
    ]);
}

#[test]
fn uncapped_cli_requires_authority_and_conflicts_with_finite_budget() {
    for arguments in [
        vec!["--uncapped-budget"],
        vec!["--spend-authorization", "ref"],
        vec![
            "--uncapped-budget",
            "--spend-authorization",
            "ref",
            "--budget-usd",
            "1",
        ],
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(["aep", "drive", "run"])
            .args(arguments)
            .output()
            .unwrap();
        assert_eq!(
            output.status.code(),
            Some(2),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

fn paused_run_resumes(spend_args: &[&str]) {
    let root = support::aep_root();
    let scratch = tempfile::tempdir().expect("scratch project");
    let project = scratch.path();
    std::fs::create_dir_all(project.join(".engineering/planning")).unwrap();
    std::fs::write(project.join("task.yaml"),
        "id: RESUME-1\nkind: feature\nobjective: pause before a model\nprotocol: adp/1\nprofile: development.standard\n").unwrap();
    std::fs::write(project.join("steps.yaml"),
        "format: aep.driver-steps/1\nid: fixture/resume\nworkflow: adp/default/2\nstates:\n  receive:\n    steps:\n      - kind: operator\n        prompt: stop here\n      - kind: operator\n        prompt: stop again after resume\n      - kind: llm\n        harness: b10x\n        prompt: must never execute\n").unwrap();
    let aep = stub_the_binaries_on_the_session_path(project, &root);
    let path = path_holding_the_binary_under_test();
    let invoke = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(["aep", "drive"])
            .args(arguments)
            .args([
                "--project",
                project.to_str().unwrap(),
                "--root",
                root.to_str().unwrap(),
                "--task",
                project.join("task.yaml").to_str().unwrap(),
                "--map",
                project.join("steps.yaml").to_str().unwrap(),
                "--pause-on-approval",
            ])
            .env("HOME", project)
            .env("PATH", &path)
            .env("METAHARNESS_LIVE", "1")
            .output()
            .unwrap()
    };
    let mut start_args = vec![
        "run",
        "--allow-evidence-gap",
        "--aep-binary",
        aep.to_str().unwrap(),
        "--b10x-endpoint",
        "http://127.0.0.1:1",
        "--b10x-model",
        "offline-fixture",
    ];
    start_args.extend_from_slice(spend_args);
    let started = invoke(&start_args);
    assert!(
        started.status.success(),
        "{}{}",
        String::from_utf8_lossy(&started.stdout),
        String::from_utf8_lossy(&started.stderr)
    );
    let run = project.join(".engineering/runs/RESUME-1/1");
    assert!(
        String::from_utf8_lossy(&started.stdout)
            .contains("resume with: metaharness aep drive resume RESUME-1/1")
    );
    let launch_path = run.join("launch.json");
    let mut launch: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&launch_path).unwrap()).unwrap();
    launch["b10x"].as_object_mut().unwrap().remove("aep_binary");
    if launch["spend_policy"]["mode"] == "finite" {
        let mut finite = launch
            .as_object_mut()
            .unwrap()
            .remove("spend_policy")
            .unwrap();
        finite.as_object_mut().unwrap().remove("mode");
        launch["spend"] = finite;
    }
    std::fs::write(&launch_path, serde_json::to_vec_pretty(&launch).unwrap()).unwrap();
    let ledger = std::fs::read(run.join("spend.json")).unwrap();
    let resumed = invoke(&[
        "resume",
        "RESUME-1/1",
        "--aep-binary",
        aep.to_str().unwrap(),
    ]);
    assert!(
        resumed.status.success(),
        "{}{}",
        String::from_utf8_lossy(&resumed.stdout),
        String::from_utf8_lossy(&resumed.stderr)
    );
    assert_eq!(std::fs::read(run.join("spend.json")).unwrap(), ledger);
    assert!(
        String::from_utf8_lossy(&resumed.stdout)
            .contains("resume with: metaharness aep drive resume RESUME-1/1")
    );
    assert!(!run.join("transcripts").exists());
    let after: serde_json::Value =
        serde_json::from_slice(&std::fs::read(launch_path).unwrap()).unwrap();
    assert_eq!(after["b10x"]["model"], "offline-fixture");
    if spend_args.contains(&"--uncapped-budget") {
        assert_eq!(
            after["spend_policy"]["authorization_ref"],
            "operator:approval-14"
        );
        assert_eq!(after["spend_policy"], launch["spend_policy"]);
    } else {
        assert_eq!(after["spend_policy"]["mode"], "finite");
        assert_eq!(after["spend_policy"]["cap_micro_usd"], 1_000_000);
    }
}

/// Every file, directory and link under `directory`, by relative path, with a file's bytes.
fn tree(
    directory: &std::path::Path,
) -> std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>> {
    let mut found = std::collections::BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(at) = pending.pop() {
        for entry in std::fs::read_dir(&at).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            let relative = path.strip_prefix(directory).unwrap().to_path_buf();
            let kind = entry.file_type().unwrap();
            if kind.is_dir() {
                found.insert(relative, None);
                pending.push(path);
            } else if kind.is_symlink() {
                let target = std::fs::read_link(&path).unwrap();
                found.insert(relative, Some(target.into_os_string().into_encoded_bytes()));
            } else {
                found.insert(relative, Some(std::fs::read(&path).unwrap()));
            }
        }
    }
    found
}

/// Every path whose bytes, link target or presence differ between two [`tree`] snapshots.
fn changed_paths(
    before: &std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
    after: &std::collections::BTreeMap<std::path::PathBuf, Option<Vec<u8>>>,
) -> std::collections::BTreeSet<std::path::PathBuf> {
    before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}

fn said(output: &std::process::Output) -> String {
    format!(
        "exit {:?}\n{}{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// Amendment a46: a resumed `--budget-usd` below the run's remembered assumed charge is refused
/// while the host prepares the resume, before the run is locked or its launch record rewritten.
/// The project is byte-identical afterwards and no model session starts; a cap equal to the charge
/// is admitted and the next unchanged resume reads it back.
#[test]
fn a_cap_narrowed_below_the_assumed_charge_is_refused_before_anything_is_written() {
    let root = support::aep_root();
    let scratch = tempfile::tempdir().expect("scratch project");
    let project = scratch.path();
    std::fs::create_dir_all(project.join(".engineering/planning")).unwrap();
    std::fs::write(project.join("task.yaml"),
        "id: RESUME-1\nkind: feature\nobjective: pause before a model\nprotocol: adp/1\nprofile: development.standard\n").unwrap();
    std::fs::write(project.join("steps.yaml"),
        "format: aep.driver-steps/1\nid: fixture/narrow\nworkflow: adp/default/2\nstates:\n  receive:\n    steps:\n      - kind: operator\n        prompt: stop at launch\n      - kind: operator\n        prompt: stop after the admitted narrowing\n      - kind: operator\n        prompt: stop after the unchanged resume\n      - kind: llm\n        harness: b10x\n        prompt: must never execute\n").unwrap();
    let aep = stub_the_binaries_on_the_session_path(project, &root);
    let path = path_holding_the_binary_under_test();
    let invoke = |arguments: &[&str]| {
        Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(["aep", "drive"])
            .args(arguments)
            .args([
                "--project",
                project.to_str().unwrap(),
                "--root",
                root.to_str().unwrap(),
                "--task",
                project.join("task.yaml").to_str().unwrap(),
                "--map",
                project.join("steps.yaml").to_str().unwrap(),
                "--pause-on-approval",
            ])
            .env("HOME", project)
            .env("PATH", &path)
            .env("METAHARNESS_LIVE", "1")
            .output()
            .unwrap()
    };
    let started = invoke(&[
        "run",
        "--allow-evidence-gap",
        "--aep-binary",
        aep.to_str().unwrap(),
        "--b10x-endpoint",
        "http://127.0.0.1:1",
        "--b10x-model",
        "offline-fixture",
        "--budget-usd",
        "1",
        "--assume-usd-per-run",
        "0.1",
    ]);
    assert!(started.status.success(), "{}", said(&started));
    let run = project.join(".engineering/runs/RESUME-1/1");
    let policy = || {
        serde_json::from_slice::<serde_json::Value>(
            &std::fs::read(run.join("launch.json")).unwrap(),
        )
        .unwrap()["spend_policy"]
            .clone()
    };
    let resume = |budget: Option<&str>| {
        let mut arguments = vec!["resume", "RESUME-1/1"];
        if let Some(budget) = budget {
            arguments.extend(["--budget-usd", budget]);
        }
        invoke(&arguments)
    };

    let before = tree(project);
    let below = resume(Some("0.05"));
    if below.status.success() {
        let persisted = policy();
        let next = resume(None);
        panic!(
            "narrowing $1 to $0.05 under a $0.10 assumed charge was admitted and persisted \
             {persisted}; the next unchanged resume answered {}",
            said(&next)
        );
    }
    assert_eq!(below.status.code(), Some(1), "{}", said(&below));
    let refusal = String::from_utf8_lossy(&below.stderr);
    for amount in ["$1.000000", "$0.100000", "$0.050000"] {
        assert!(refusal.contains(amount), "{refusal}");
    }
    let changed = changed_paths(&before, &tree(project));
    assert!(
        changed.is_empty(),
        "a refused narrowing changed {changed:?}"
    );

    let equal = serde_json::json!({
        "mode": "finite",
        "cap_micro_usd": 100_000,
        "assumed_micro_usd_per_run": 100_000,
    });
    let at_charge = resume(Some("0.1"));
    assert!(at_charge.status.success(), "{}", said(&at_charge));
    assert_eq!(policy(), equal);
    let unchanged = resume(None);
    assert!(unchanged.status.success(), "{}", said(&unchanged));
    assert_eq!(policy(), equal);
}
