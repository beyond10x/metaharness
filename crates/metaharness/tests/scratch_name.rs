//! `story:scratch-root-stable-by-declared-name` (design amendment a72): a caller names the run's
//! scratch root, so every path metaharness derives from it is the same from one session of a case
//! to the next.
//!
//! Each case drives the production builder over a scripted runner: no vendor process, no
//! credential, no network. A named root lies in the process's temporary directory, which these
//! cases share with every other process on the machine, so each case takes a name nothing else
//! uses (this process's id and a counter), and a guard removes whatever is left under that name,
//! the lock file included, even when the case fails.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};

use metaharness::protocol::{CredentialSource, DecisionMode, Kind, RunSpec};
use metaharness::{
    Input, Metaharness, Refusal, Run, SCRATCH_NAME_HELD, SCRATCH_NAME_INVALID,
    SCRATCH_ROOT_FOREIGN, ScriptedLog, ScriptedRunner, ScriptedSeams,
};

const KINDS: [Kind; 3] = [Kind::Claude, Kind::Codex, Kind::B10x];

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// The most bytes a scratch name may have.
const NAME_BYTES_MAX: usize = 64;

fn builder(kind: Kind) -> Metaharness {
    let builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    match kind {
        Kind::B10x => builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called"),
        Kind::Claude | Kind::Codex => builder,
    }
}

/// Start and drain, so the run has spawned and its record has been read when this returns.
fn start(builder: Metaharness, log: &ScriptedLog) -> Result<Run, Refusal> {
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let mut run = builder.start_with(
        Input::Prompt("write the failing test".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    )?;
    run.drain().expect("the scripted run drains");
    Ok(run)
}

/// A scratch name no other case and no other process uses, and the removal of everything under it.
struct Unique {
    name: String,
    root: PathBuf,
    lock: PathBuf,
}

impl Unique {
    fn new(case: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "{case}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        Self::exactly(name)
    }

    /// Exactly [`NAME_BYTES_MAX`] bytes, still unique.
    fn longest(case: &str) -> Self {
        let unique = Self::new(case);
        let name = format!("{:x<NAME_BYTES_MAX$}", format!("{}-", unique.name));
        assert_eq!(name.len(), NAME_BYTES_MAX);
        Self::exactly(name)
    }

    fn exactly(name: String) -> Self {
        let temp = std::env::temp_dir();
        Self {
            root: temp.join(format!("metaharness-{name}")),
            lock: temp.join(format!("metaharness.{name}.lock")),
            name,
        }
    }
}

impl Drop for Unique {
    fn drop(&mut self) {
        // A link or a file is removed itself and never followed; a directory with what it holds.
        match std::fs::symlink_metadata(&self.root) {
            Ok(found) if found.is_dir() => drop(std::fs::remove_dir_all(&self.root)),
            Ok(_) => drop(std::fs::remove_file(&self.root)),
            Err(_) => (),
        }
        drop(std::fs::remove_file(&self.lock));
    }
}

/// Run git with no operator configuration, so the fixture is the same on every machine.
fn git(directory: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} in {}", directory.display());
}

/// A git work tree tracking `todo.py`, for a run that declares it as context.
fn tree() -> tempfile::TempDir {
    let work = tempfile::tempdir().expect("a directory");
    git(work.path(), &["init", "--quiet"]);
    std::fs::write(
        work.path().join("todo.py"),
        "def add(items, item):\n    items.append(item)\n",
    )
    .expect("the file");
    git(work.path(), &["add", "todo.py"]);
    work
}

fn context_values(argv: &[String]) -> Vec<String> {
    argv.windows(2)
        .filter(|pair| pair[0] == "--context")
        .map(|pair| pair[1].clone())
        .collect()
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

#[test]
fn two_runs_under_one_scratch_name_use_one_scratch_root() {
    let work = tree();
    for kind in KINDS {
        let name = Unique::new("one-root");
        let mut roots = Vec::new();
        let mut contexts = Vec::new();
        for _ in 0..2 {
            let log = ScriptedLog::new();
            let mut declared = builder(kind).with_scratch_name(&name.name);
            if kind == Kind::B10x {
                declared = declared
                    .with_cwd(work.path())
                    .with_scratch_cwd()
                    .with_context("todo.py");
            }
            let run = start(declared, &log)
                .unwrap_or_else(|refusal| panic!("{kind:?} refused a named run: {refusal}"));
            roots.push(
                run.scratch_root()
                    .expect("a run owns a scratch")
                    .to_path_buf(),
            );
            contexts.push(context_values(&log.launched().pop().expect("one launch")));
            drop(run);
        }
        assert_eq!(roots[0], roots[1], "{kind:?}");
        assert_eq!(
            roots[0], name.root,
            "{kind:?}: <temp dir>/metaharness-<NAME>"
        );
        if kind == Kind::B10x {
            let copy = name.root.join("context").join("todo.py");
            assert_eq!(contexts[0], [copy.display().to_string()], "{kind:?}");
            assert_eq!(contexts[0], contexts[1], "{kind:?}");
        }
    }
}

#[test]
fn without_a_scratch_name_each_run_gets_its_own_root() {
    for kind in KINDS {
        let mut roots = Vec::new();
        for _ in 0..2 {
            let log = ScriptedLog::new();
            let run = start(builder(kind), &log)
                .unwrap_or_else(|refusal| panic!("{kind:?} refused an unnamed run: {refusal}"));
            assert_eq!(run.spec().scratch_name, None, "{kind:?}");
            let root = run
                .scratch_root()
                .expect("a run owns a scratch")
                .to_path_buf();
            assert!(root.is_dir(), "{kind:?}: {}", root.display());
            roots.push(root);
            drop(run);
        }
        assert_ne!(roots[0], roots[1], "{kind:?}");
        for root in &roots {
            assert!(
                !exists(root),
                "{kind:?}: {} outlived its run",
                root.display()
            );
            let leaf = root.file_name().expect("a leaf").to_string_lossy();
            assert!(!leaf.starts_with("metaharness-"), "{kind:?}: {leaf}");
        }
    }
}

#[test]
fn a_scratch_name_that_is_not_one_plain_component_is_refused() {
    let too_long = "x".repeat(NAME_BYTES_MAX + 1);
    // Thirty-three characters, sixty-six bytes: the cap counts bytes.
    let wide = "\u{e9}".repeat(33);
    let names = [
        "a/b",
        "..",
        ".",
        "",
        "line\nbreak",
        "bell\u{7}",
        "nul\0",
        "del\u{7f}",
        too_long.as_str(),
        wide.as_str(),
    ];
    let temp = std::env::temp_dir();
    for kind in KINDS {
        for name in names {
            let mut spec = RunSpec::new(kind);
            spec.scratch_name = Some(name.to_owned());
            let checked = metaharness::check_spec(&spec).expect_err(name);
            assert!(
                matches!(checked, Refusal::ScratchNameInvalid { .. }),
                "{kind:?} {name:?}: {checked:?}"
            );

            let log = ScriptedLog::new();
            let refusal = start(builder(kind).with_scratch_name(name), &log)
                .expect_err("a name that is not one plain component starts no run");
            let words = refusal.to_string();
            assert!(
                words.starts_with(SCRATCH_NAME_INVALID),
                "{kind:?} {name:?}: {words}"
            );
            assert!(
                words.contains(&format!("{name:?}")),
                "{kind:?}: the refusal names {name:?}: {words}"
            );
            assert_eq!(log.spawns(), 0, "{kind:?} {name:?}");
            if !name.contains('\0') {
                assert!(
                    !exists(&temp.join(format!("metaharness-{name}"))),
                    "{name:?}"
                );
                assert!(
                    !exists(&temp.join(format!("metaharness.{name}.lock"))),
                    "{name:?}"
                );
            }
        }
        assert!(!exists(&temp.join("metaharness-a")), "{kind:?}");
    }

    // The cap is sixty-four bytes and not fewer.
    for kind in KINDS {
        let name = Unique::longest("cap");
        let log = ScriptedLog::new();
        let run = start(builder(kind).with_scratch_name(&name.name), &log)
            .unwrap_or_else(|refusal| panic!("{kind:?} refused a 64-byte name: {refusal}"));
        assert_eq!(run.scratch_root(), Some(name.root.as_path()), "{kind:?}");
    }
}

#[test]
fn a_leftover_root_of_the_same_user_is_replaced() {
    for kind in KINDS {
        let name = Unique::new("leftover");
        std::fs::create_dir(&name.root).expect("a leftover root");
        std::fs::write(name.root.join("stale.txt"), "left by an earlier run").expect("a file");
        std::fs::create_dir(name.root.join("home")).expect("a nested directory");
        std::fs::write(name.root.join("home/state.json"), "{}").expect("a nested file");
        std::fs::set_permissions(&name.root, std::fs::Permissions::from_mode(0o755))
            .expect("a mode the new root does not have");

        let log = ScriptedLog::new();
        let run = start(builder(kind).with_scratch_name(&name.name), &log)
            .unwrap_or_else(|refusal| panic!("{kind:?} refused over its own leftover: {refusal}"));
        assert_eq!(log.spawns(), 1, "{kind:?}");
        assert_eq!(run.scratch_root(), Some(name.root.as_path()), "{kind:?}");
        assert!(!exists(&name.root.join("stale.txt")), "{kind:?}");
        assert!(!exists(&name.root.join("home/state.json")), "{kind:?}");
        let mode = std::fs::symlink_metadata(&name.root)
            .expect("the root")
            .permissions()
            .mode();
        assert_eq!(mode & 0o7777, 0o700, "{kind:?}: {mode:o}");
    }
}

#[test]
fn a_link_or_foreign_entry_at_the_root_is_refused() {
    let outside = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a directory");
    assert!(
        !outside.path().starts_with(std::env::temp_dir()),
        "the link's target lies outside the temp dir"
    );
    std::fs::write(outside.path().join("keep.txt"), "not metaharness's").expect("a file");
    let absent = outside.path().join("absent");

    for kind in KINDS {
        for target in [outside.path(), absent.as_path()] {
            let name = Unique::new("link");
            std::os::unix::fs::symlink(target, &name.root).expect("a link at the root");
            let log = ScriptedLog::new();
            let refusal = start(builder(kind).with_scratch_name(&name.name), &log)
                .expect_err("a link at the root starts no run");
            let words = refusal.to_string();
            assert!(words.starts_with(SCRATCH_ROOT_FOREIGN), "{kind:?}: {words}");
            assert!(
                words.contains(&name.root.display().to_string()),
                "{kind:?}: {words}"
            );
            assert_eq!(log.spawns(), 0, "{kind:?}");
            assert_eq!(
                std::fs::read_link(&name.root).expect("still a link"),
                target,
                "{kind:?}"
            );
        }
        let entries: Vec<_> = std::fs::read_dir(outside.path())
            .expect("readable")
            .map(|entry| entry.expect("an entry").file_name())
            .collect();
        assert_eq!(
            entries,
            ["keep.txt"],
            "{kind:?}: nothing was made through the link"
        );
        assert_eq!(
            std::fs::read_to_string(outside.path().join("keep.txt")).expect("still there"),
            "not metaharness's"
        );

        let name = Unique::new("file");
        std::fs::write(&name.root, "a file at the root").expect("a file");
        let log = ScriptedLog::new();
        let refusal = start(builder(kind).with_scratch_name(&name.name), &log)
            .expect_err("a file at the root starts no run");
        let words = refusal.to_string();
        assert!(words.starts_with(SCRATCH_ROOT_FOREIGN), "{kind:?}: {words}");
        assert!(
            words.contains(&name.root.display().to_string()),
            "{kind:?}: {words}"
        );
        assert_eq!(log.spawns(), 0, "{kind:?}");
        assert_eq!(
            std::fs::read_to_string(&name.root).expect("still a file"),
            "a file at the root",
            "{kind:?}"
        );
    }
}

#[test]
fn a_second_run_under_a_held_name_is_refused() {
    for kind in KINDS {
        let name = Unique::new("held");
        let first_log = ScriptedLog::new();
        let first = start(builder(kind).with_scratch_name(&name.name), &first_log)
            .unwrap_or_else(|refusal| panic!("{kind:?} refused the first run: {refusal}"));
        let marker = name.root.join("first-run.txt");
        std::fs::write(&marker, "the first run's").expect("a file in the first run's root");

        let second_log = ScriptedLog::new();
        let refusal = start(builder(kind).with_scratch_name(&name.name), &second_log)
            .expect_err("a held name starts no second run");
        let words = refusal.to_string();
        assert!(words.starts_with(SCRATCH_NAME_HELD), "{kind:?}: {words}");
        assert!(
            words.contains(&format!("{:?}", name.name)),
            "{kind:?}: {words}"
        );
        assert_eq!(second_log.spawns(), 0, "{kind:?}");
        assert_eq!(
            std::fs::read_to_string(&marker).expect("the first run's root is untouched"),
            "the first run's",
            "{kind:?}"
        );
        assert_eq!(first.scratch_root(), Some(name.root.as_path()), "{kind:?}");
        drop(first);
    }
}

#[test]
fn the_root_is_removed_when_the_run_ends() {
    for kind in KINDS {
        let name = Unique::new("removed");
        let log = ScriptedLog::new();
        let run = start(builder(kind).with_scratch_name(&name.name), &log)
            .unwrap_or_else(|refusal| panic!("{kind:?} refused a named run: {refusal}"));
        assert!(name.root.is_dir(), "{kind:?}");
        std::fs::write(name.root.join("written.txt"), "by the child").expect("a file");
        drop(run);
        assert!(!exists(&name.root), "{kind:?}: the root outlived its run");
    }
}
