//! A paused legacy model run keeps its authority when the new host resumes it.
#![cfg(unix)]
mod support;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

#[test]
fn legacy_launch_resumes_without_spending_or_losing_configuration() {
    let root = support::aep_root();
    let scratch = tempfile::tempdir().expect("scratch project");
    let project = scratch.path();
    std::fs::create_dir_all(project.join(".engineering/planning")).unwrap();
    std::fs::write(project.join("task.yaml"),
        "id: RESUME-1\nkind: feature\nobjective: pause before a model\nprotocol: adp/1\nprofile: development.standard\n").unwrap();
    std::fs::write(project.join("steps.yaml"),
        "format: aep.driver-steps/1\nid: fixture/resume\nworkflow: adp/default/2\nstates:\n  receive:\n    steps:\n      - kind: operator\n        prompt: stop here\n      - kind: operator\n        prompt: stop again after resume\n      - kind: llm\n        harness: b10x\n        prompt: must never execute\n").unwrap();
    // Preflight only reads these stubs. An accidental model launch cannot contact a provider.
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
            .env("METAHARNESS_LIVE", "1")
            .output()
            .unwrap()
    };
    let started = invoke(&[
        "run",
        "--allow-evidence-gap",
        "--aep-binary",
        aep.to_str().unwrap(),
        "--budget-usd",
        "1",
        "--assume-usd-per-run",
        "0.1",
        "--b10x-endpoint",
        "http://127.0.0.1:1",
        "--b10x-model",
        "offline-fixture",
    ]);
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
    assert_eq!(after["spend"], launch["spend"]);
}
