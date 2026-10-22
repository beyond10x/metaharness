//! Adversary cases for amendment a46: a resumed `--budget-usd` below the remembered assumed charge
//! is refused before anything is written, its message names the three amounts by role, the zero,
//! raise and invalid-terms refusals keep their own wording, and every published sentence that says
//! a resumed cap may narrow also says how far.
#![cfg(unix)]
mod support;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn path_holding_the_binary_under_test() -> std::ffi::OsString {
    let built = Path::new(env!("CARGO_BIN_EXE_metaharness"))
        .parent()
        .expect("the binary under test lives in a directory")
        .to_path_buf();
    let ambient = std::env::var_os("PATH").unwrap_or_default();
    let directories = std::iter::once(built).chain(std::env::split_paths(&ambient));
    std::env::join_paths(directories).expect("a PATH this process can already hold")
}

fn stub_the_binaries_on_the_session_path(project: &Path, root: &Path) -> PathBuf {
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

type Snapshot = BTreeMap<PathBuf, Option<Vec<u8>>>;

fn tree(directory: &Path) -> Snapshot {
    let mut found = BTreeMap::new();
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

fn changed(before: &Snapshot, after: &Snapshot) -> BTreeSet<PathBuf> {
    before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .cloned()
        .collect()
}

fn said(output: &Output) -> String {
    format!(
        "exit {:?}\n{}{}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    )
}

/// A paused finite run started with a cap of $1 and an assumed charge of $0.10, with enough
/// operator steps that every admitted resume pauses again before the `llm` step.
struct Paused {
    _scratch: tempfile::TempDir,
    project: PathBuf,
    root: PathBuf,
    path: std::ffi::OsString,
    run: PathBuf,
}

impl Paused {
    fn start() -> Self {
        let root = support::aep_root();
        let scratch = tempfile::tempdir().expect("scratch project");
        let project = scratch.path().to_path_buf();
        std::fs::create_dir_all(project.join(".engineering/planning")).unwrap();
        std::fs::write(
            project.join("task.yaml"),
            "id: RESUME-1\nkind: feature\nobjective: pause before a model\nprotocol: adp/1\n\
             profile: development.standard\n",
        )
        .unwrap();
        let mut map = String::from(
            "format: aep.driver-steps/1\nid: fixture/adversary\nworkflow: adp/default/2\n\
             states:\n  receive:\n    steps:\n",
        );
        for ordinal in 0..10 {
            writeln!(
                map,
                "      - kind: operator\n        prompt: stop {ordinal}"
            )
            .expect("writing to a String cannot fail");
        }
        map.push_str(
            "      - kind: llm\n        harness: b10x\n        prompt: must never execute\n",
        );
        std::fs::write(project.join("steps.yaml"), map).unwrap();
        let aep = stub_the_binaries_on_the_session_path(&project, &root);
        let paused = Self {
            run: project.join(".engineering/runs/RESUME-1/1"),
            path: path_holding_the_binary_under_test(),
            _scratch: scratch,
            project,
            root,
        };
        let started = paused.invoke(&[
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
        paused
    }

    fn invoke(&self, arguments: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(["aep", "drive"])
            .args(arguments)
            .args([
                "--project",
                self.project.to_str().unwrap(),
                "--root",
                self.root.to_str().unwrap(),
                "--task",
                self.project.join("task.yaml").to_str().unwrap(),
                "--map",
                self.project.join("steps.yaml").to_str().unwrap(),
                "--pause-on-approval",
            ])
            .env("HOME", &self.project)
            .env("PATH", &self.path)
            .env("METAHARNESS_LIVE", "1")
            .output()
            .unwrap()
    }

    fn resume(&self, budget: Option<&str>) -> Output {
        let mut arguments = vec!["resume", "RESUME-1/1"];
        if let Some(budget) = budget {
            arguments.extend(["--budget-usd", budget]);
        }
        self.invoke(&arguments)
    }

    fn launch(&self) -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(self.run.join("launch.json")).unwrap()).unwrap()
    }

    fn policy(&self) -> serde_json::Value {
        self.launch()["spend_policy"].clone()
    }

    fn set_policy(&self, policy: serde_json::Value) {
        let mut launch = self.launch();
        launch["spend_policy"] = policy;
        std::fs::write(
            self.run.join("launch.json"),
            serde_json::to_vec_pretty(&launch).unwrap(),
        )
        .unwrap();
    }

    /// Runs a resume that must be refused and must leave the project byte-identical.
    fn refused(&self, budget: Option<&str>) -> String {
        let before = tree(&self.project);
        let output = self.resume(budget);
        assert!(
            !output.status.success(),
            "--budget-usd {budget:?} was admitted and persisted {}: {}",
            self.policy(),
            said(&output)
        );
        let touched = changed(&before, &tree(&self.project));
        assert!(
            touched.is_empty(),
            "--budget-usd {budget:?} was refused and still changed {touched:?}"
        );
        String::from_utf8_lossy(&output.stderr).into_owned()
    }

    fn admitted(&self, budget: Option<&str>) {
        let output = self.resume(budget);
        assert!(output.status.success(), "{budget:?}: {}", said(&output));
    }
}

fn finite(cap: u64, charge: u64) -> serde_json::Value {
    serde_json::json!({
        "mode": "finite",
        "cap_micro_usd": cap,
        "assumed_micro_usd_per_run": charge,
    })
}

/// a46 § 2: the refusal names the remembered cap, the assumed charge and the asked-for cap. A
/// message that prints the three amounts in the wrong roles still contains all three, so the
/// unit's `contains` checks cannot tell; this reads each amount beside its role.
#[test]
fn the_refusal_names_each_amount_beside_its_role() {
    let paused = Paused::start();
    for _ in 0..2 {
        let refusal = paused.refused(Some("0.05"));
        for labelled in ["cap $1.000000", "charge $0.100000"] {
            assert!(
                refusal.contains(labelled),
                "missing `{labelled}`: {refusal}"
            );
        }
        assert!(refusal.contains("$0.050000"), "{refusal}");
        for mislabelled in [
            "cap $0.100000",
            "cap $0.050000",
            "charge $1.000000",
            "charge $0.050000",
        ] {
            assert!(
                !refusal.contains(mislabelled),
                "`{mislabelled}` names an amount in the wrong role: {refusal}"
            );
        }
    }
    assert_eq!(paused.policy(), finite(1_000_000, 100_000));
}

/// Boundaries at one millionth, and spellings the parser must refuse rather than round.
#[test]
fn a_cap_is_compared_in_exact_millionths_and_never_rounded_up_to_the_charge() {
    let paused = Paused::start();
    for below in [
        "0.099999",
        " 0.099999",
        "$0.099999",
        "0.0999999",
        "0.0999995",
        "1e-1",
        "0.1e0",
    ] {
        paused.refused(Some(below));
        assert_eq!(paused.policy(), finite(1_000_000, 100_000), "{below:?}");
    }
    paused.admitted(Some("$0.100000"));
    assert_eq!(paused.policy(), finite(100_000, 100_000));
    paused.admitted(Some("0.1"));
    assert_eq!(paused.policy(), finite(100_000, 100_000));
    paused.admitted(None);
    assert_eq!(paused.policy(), finite(100_000, 100_000));
    let refusal = paused.refused(Some("0.099999"));
    assert!(refusal.contains("cap $0.100000"), "{refusal}");
    assert!(refusal.contains("$0.099999"), "{refusal}");
    let raised = paused.refused(Some("0.100001"));
    assert!(raised.contains("not raise"), "{raised}");
}

/// a46 § 4: the zero and raise refusals keep their wording; the new floor is checked last.
#[test]
fn zero_and_raise_keep_their_own_refusals() {
    let paused = Paused::start();
    let zero = paused.refused(Some("0"));
    assert!(zero.contains("greater than zero"), "{zero}");
    assert!(!zero.contains("assumed charge"), "{zero}");
    let raised = paused.refused(Some("1.000001"));
    assert!(raised.contains("not raise"), "{raised}");
    assert!(!raised.contains("assumed charge"), "{raised}");
    assert_eq!(paused.policy(), finite(1_000_000, 100_000));
}

/// a46 § 4: terms an earlier release persisted (the 0.12.0 state: cap $0.05 under a $0.10 charge)
/// are refused as invalid, whatever `--budget-usd` says, and a resume does not repair them.
#[test]
fn terms_an_earlier_release_left_invalid_are_refused_as_invalid_not_as_below_the_charge() {
    let paused = Paused::start();
    paused.set_policy(finite(50_000, 100_000));
    for budget in [None, Some("0.04"), Some("0.05"), Some("0.1")] {
        let refusal = paused.refused(budget);
        assert!(refusal.contains("invalid"), "{budget:?}: {refusal}");
        assert!(
            !refusal.contains("down to its assumed charge"),
            "{budget:?}: {refusal}"
        );
    }
    assert_eq!(paused.policy(), finite(50_000, 100_000));
}

/// a46 § 5, D7: a cap at the charge but below what the ledger already reserved is admitted and
/// persisted; the declared limit, pinned so that a later change claiming otherwise fails here.
#[test]
fn a_cap_at_the_charge_below_reserved_spend_is_the_declared_limit() {
    let paused = Paused::start();
    std::fs::write(
        paused.run.join("spend.json"),
        "{\n  \"format\": \"aep.drive-spend/1\",\n  \"spent_micro_usd\": 300000,\n  \"launches\": 3\n}\n",
    )
    .unwrap();
    paused.admitted(Some("0.1"));
    assert_eq!(paused.policy(), finite(100_000, 100_000));
    paused.admitted(None);
    assert_eq!(paused.policy(), finite(100_000, 100_000));
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// Every published sentence that says a resumed cap may narrow also says it stops at the assumed
/// charge (a46). The operator reads these, not the design document.
#[test]
fn every_published_sentence_that_lets_a_resumed_cap_narrow_names_its_floor() {
    let root = workspace_root();
    let mut pages = vec![root.join("README.md")];
    let mut pending = vec![root.join("website/docs")];
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "md") {
                pages.push(path);
            }
        }
    }
    let mut silent = Vec::new();
    for page in pages {
        let text = std::fs::read_to_string(&page).unwrap();
        for paragraph in text.split("\n\n") {
            let flat = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
            let lowered = flat.to_lowercase();
            let lets_narrow = lowered.contains("resume")
                && (lowered.contains("may only narrow")
                    || lowered.contains("may narrow")
                    || lowered.contains("cap may only narrow"));
            if lets_narrow && !lowered.contains("assumed charge") {
                silent.push(format!(
                    "{}: {flat}",
                    page.strip_prefix(&root).unwrap().display()
                ));
            }
        }
    }
    assert!(
        silent.is_empty(),
        "these say a resumed cap may narrow and not that it stops at the assumed charge:\n{}",
        silent.join("\n")
    );
}

/// The operator's own `--help` for `resume --budget-usd` states the floor (a46).
#[test]
fn resume_help_states_that_the_cap_stops_at_the_assumed_charge() {
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["aep", "drive", "resume", "--help"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", said(&output));
    let help = String::from_utf8_lossy(&output.stdout);
    let budget = help
        .split("\n  -")
        .chain(help.split("\n      --"))
        .find(|block| {
            block.trim_start().starts_with("-budget-usd")
                || block.trim_start().starts_with("budget-usd")
        })
        .unwrap_or_else(|| panic!("no --budget-usd in:\n{help}"));
    let flat = budget
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase();
    assert!(
        flat.contains("assum") && (flat.contains("below") || flat.contains("at least")),
        "--budget-usd on resume says nothing about the assumed-charge floor: {flat}"
    );
}
