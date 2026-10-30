//! Adversary pass 1 on `story:scratch-root-stable-by-declared-name` (design amendment a72).
//!
//! The unit's own cases drive the scripted runner, whose child is a replay that ends when the
//! script does. These cases ask what a stable root changes about the two things a fresh
//! temporary directory made harmless: a child that outlives its run, and a leftover the run
//! itself cannot remove.
#![cfg(unix)]

use std::cell::RefCell;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
use metaharness::{
    HarnessProcess, Input, LaunchPlanView, Metaharness, ProcessRunner, Refusal, ScriptedLog,
    ScriptedRunner, ScriptedSeams, SpawnRunner,
};

const KINDS: [Kind; 3] = [Kind::Claude, Kind::Codex, Kind::B10x];
const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

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

/// A name nothing else uses, and the removal of its root and lock file, read-only parts included.
struct Unique {
    name: String,
    root: PathBuf,
    lock: PathBuf,
}

impl Unique {
    fn new(case: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "adv-{case}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let temp = std::env::temp_dir();
        Self {
            root: temp.join(format!("metaharness-{name}")),
            lock: temp.join(format!("metaharness.{name}.lock")),
            name,
        }
    }
}

fn make_writable(path: &Path) {
    let Ok(found) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !found.is_dir() {
        return;
    }
    drop(std::fs::set_permissions(
        path,
        std::fs::Permissions::from_mode(0o700),
    ));
    if let Ok(entries) = std::fs::read_dir(path) {
        for entry in entries.flatten() {
            make_writable(&entry.path());
        }
    }
}

impl Drop for Unique {
    fn drop(&mut self) {
        make_writable(&self.root);
        drop(std::fs::remove_dir_all(&self.root));
        drop(std::fs::remove_file(&self.lock));
    }
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// The real spawner, starting `/bin/sh` in place of the vendor: the child sleeps past its run's
/// drop and then writes into the decision channel, as a hook the vendor ran late would.
struct LateWriter {
    inner: SpawnRunner,
    channel: Rc<RefCell<Option<PathBuf>>>,
}

const LATE_FILE: &str = "written-by-the-first-run";

impl ProcessRunner for LateWriter {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        *self.channel.borrow_mut() = Some(plan.decision_channel.to_path_buf());
        let args = vec![
            "-c".to_owned(),
            format!("sleep 1; printf late > \"$1/{LATE_FILE}\""),
            "sh".to_owned(),
            plan.decision_channel.display().to_string(),
        ];
        let mut env = plan.env.clone();
        env.insert("PATH".to_owned(), "/usr/bin:/bin".to_owned());
        let view = LaunchPlanView {
            program: "/bin/sh",
            args: &args,
            env: &env,
            cwd: plan.cwd,
            credential_copies: plan.credential_copies,
            decision_channel: plan.decision_channel,
            transcript: plan.transcript,
        };
        self.inner.start(&view)
    }
}

/// A run dropped while its child is still going releases the name while the child can still
/// write by path into the root, so the next run under the name receives the first child's
/// writes in its own hook channel. A new temporary directory per run made such a write land
/// nowhere; a stable root makes it land in the next session.
#[test]
fn a_child_that_outlives_its_dropped_run_cannot_write_into_the_next_run_under_the_name() {
    let name = Unique::new("late-child");
    let channel = Rc::new(RefCell::new(None));
    let mut late = LateWriter {
        inner: SpawnRunner::new(),
        channel: Rc::clone(&channel),
    };
    let first = builder(Kind::Claude)
        .with_scratch_name(&name.name)
        .start_with(
            Input::Prompt("first".to_owned()),
            &mut late,
            &mut ScriptedSeams,
        )
        .unwrap_or_else(|refusal| panic!("the first run starts: {refusal}"));
    // Abandoned before its stream ended, as run.rs's `retain_wire` documents a run may be.
    drop(first);
    let first_channel = channel.borrow().clone().expect("the first run spawned");

    // Retried for a fifth of the child's sleep, so a name a forked child of another test thread
    // still holds (the case below) does not decide this one. A fix that keeps the name held
    // until the first child is gone passes: the second run is then refused and nothing lands.
    let mut second = Err(Refusal::Io {
        detail: "not started".to_owned(),
    });
    for _ in 0..100 {
        let mut runner = ScriptedRunner::of_lines([OPENING], ScriptedLog::new());
        second = builder(Kind::Claude)
            .with_scratch_name(&name.name)
            .start_with(
                Input::Prompt("second".to_owned()),
                &mut runner,
                &mut ScriptedSeams,
            );
        if !matches!(second, Err(Refusal::ScratchNameHeld { .. })) {
            break;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    std::thread::sleep(Duration::from_millis(2500));
    let leaked = first_channel.join(LATE_FILE);
    let saw = exists(&leaked);
    drop(second);
    assert!(
        !saw,
        "the first run's child wrote {} into the second run's root under the same scratch name: \
         dropping a named run removes the root and releases the name while its child still runs",
        leaked.display()
    );
}

/// One embedder thread runs sessions of a case in turn under one name, each dropped before the
/// next starts, while another thread of the same process starts unrelated child processes, as a
/// host running several cases at once does. The lock descriptor is `O_CLOEXEC`, yet a child
/// forked by the other thread holds a copy of it until its `exec`, and a `flock` belongs to the
/// open file description, so the name stays held after the run that held it has been dropped:
/// the next session is refused as `scratch-name-held`, "a run ... is still going", when none is.
#[test]
fn a_dropped_runs_name_is_free_for_the_next_run_while_another_thread_spawns_processes() {
    let name = Unique::new("fork");
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let spawner = {
        let stop = std::sync::Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                drop(std::process::Command::new("/bin/true").status());
            }
        })
    };
    let mut held = Vec::new();
    for session in 0..300 {
        let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], ScriptedLog::new());
        match builder(Kind::Claude)
            .with_scratch_name(&name.name)
            .start_with(
                Input::Prompt("session".to_owned()),
                &mut runner,
                &mut ScriptedSeams,
            ) {
            Ok(mut run) => {
                run.drain().expect("drains");
                drop(run);
            }
            Err(refusal) => held.push(format!("session {session}: {refusal}")),
        }
    }
    stop.store(true, Ordering::Relaxed);
    spawner.join().expect("the spawner thread");
    assert!(
        held.is_empty(),
        "{} of 300 sessions, each started after the previous one was dropped, were refused; first: {}",
        held.len(),
        held.first().map_or("", String::as_str)
    );
}

/// A child that leaves a read-only directory in its scratch (`chmod a-w`, a module cache) makes
/// the root impossible to remove by `remove_dir_all`. With a new temporary directory per run
/// that was a leak; under a name it blocks every later run of the case, although the leftover is
/// "a directory this user owns", which the design says is replaced.
#[test]
fn a_leftover_holding_a_read_only_directory_of_this_user_is_replaced() {
    for kind in KINDS {
        let name = Unique::new("read-only");
        let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], ScriptedLog::new());
        let mut run = builder(kind)
            .with_scratch_name(&name.name)
            .start_with(
                Input::Prompt("first".to_owned()),
                &mut runner,
                &mut ScriptedSeams,
            )
            .unwrap_or_else(|refusal| panic!("{kind:?}: {refusal}"));
        run.drain().expect("drains");
        let locked = name.root.join("tmp-made-by-the-child");
        std::fs::create_dir_all(&locked).expect("a directory");
        std::fs::write(locked.join("file"), "x").expect("a file");
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500))
            .expect("read-only");
        drop(run);

        let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], ScriptedLog::new());
        let next = builder(kind).with_scratch_name(&name.name).start_with(
            Input::Prompt("next".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        );
        assert!(
            next.is_ok(),
            "{kind:?}: the next run under the name is refused over this user's own leftover: {}",
            next.err()
                .map(|refusal| refusal.to_string())
                .unwrap_or_default()
        );
    }
}
