//! Adversary pass 2 on `story:scratch-root-stable-by-declared-name` (design amendment a72), against
//! the review corrections.
//!
//! The corrections kill and reap the run's own child, hold the name and the root under two locks,
//! put both under a private parent and remove a leftover descriptor-relative. These cases ask what
//! each of those still leaves: a process the vendor started, a name the charset admits that the
//! layout already uses, and a tree the recursive removal cannot walk.
#![cfg(unix)]

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use metaharness::protocol::{CredentialSource, Kind};
use metaharness::{
    HarnessProcess, Input, LaunchPlanView, Metaharness, ProcessRunner, Refusal, Run, ScriptedLog,
    ScriptedRunner, ScriptedSeams, SpawnRunner,
};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// Set in a child process of this binary to the one case whose body it runs.
const CHILD_CASE: &str = "METAHARNESS_SCRATCH_ADV2_CASE";

fn builder() -> Metaharness {
    Metaharness::new(Kind::Claude).with_credentials(CredentialSource::None)
}

/// Start over the scripted runner and drain, so the run has spawned when this returns.
fn start(name: &str) -> Result<Run, Refusal> {
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], ScriptedLog::new());
    let mut run = builder().with_scratch_name(name).start_with(
        Input::Prompt("a session".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    )?;
    run.drain().expect("the scripted run drains");
    Ok(run)
}

/// `<temp dir>/metaharness-u<EUID>`, made 0700 as metaharness makes it.
fn private_parent() -> PathBuf {
    use std::os::unix::fs::DirBuilderExt as _;

    let parent = std::path::absolute(std::env::temp_dir())
        .expect("an absolute temp dir")
        .join(format!(
            "metaharness-u{}",
            rustix::process::geteuid().as_raw()
        ));
    std::fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&parent)
        .expect("the private parent");
    parent
}

/// A name nothing else uses, and the removal of every entry this file makes under it.
struct Unique {
    name: String,
    parent: PathBuf,
}

impl Unique {
    fn new(case: &str) -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Self {
            name: format!(
                "adv2-{case}-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ),
            parent: private_parent(),
        }
    }

    fn root(&self) -> PathBuf {
        self.parent.join(&self.name)
    }
}

impl Drop for Unique {
    fn drop(&mut self) {
        // The name's root and lock, and the root and lock of `<name>.lock`.
        for leaf in [
            self.name.clone(),
            format!("{}.lock", self.name),
            format!("{}.lock.lock", self.name),
        ] {
            remove_any(&self.parent.join(leaf));
        }
    }
}

/// Remove `path`, whatever it is, without recursion: a deep chain is lifted one level at a time
/// by renaming its grandchild over its child, so neither the stack nor the descriptor table of
/// this process grows with the depth.
fn remove_any(path: &Path) {
    use std::os::unix::fs::PermissionsExt as _;

    let Ok(found) = std::fs::symlink_metadata(path) else {
        return;
    };
    if !found.is_dir() {
        drop(std::fs::remove_file(path));
        return;
    }
    let lifted = path.with_extension("lifted");
    // Bounded, so an entry that cannot be removed ends the loop rather than the suite.
    for _ in 0..(8 * DEEP + 1_000) {
        drop(std::fs::set_permissions(
            path,
            std::fs::Permissions::from_mode(0o700),
        ));
        let Some(entry) = std::fs::read_dir(path)
            .ok()
            .and_then(|mut entries| entries.next())
            .and_then(Result::ok)
        else {
            break;
        };
        let child = entry.path();
        let is_dir = std::fs::symlink_metadata(&child).is_ok_and(|found| found.is_dir());
        if !is_dir {
            drop(std::fs::remove_file(&child));
            continue;
        }
        drop(std::fs::set_permissions(
            &child,
            std::fs::Permissions::from_mode(0o700),
        ));
        // Move the child's entries up into `path`, then remove the child.
        let inner: Vec<_> = std::fs::read_dir(&child)
            .map(|entries| entries.flatten().map(|entry| entry.file_name()).collect())
            .unwrap_or_default();
        if inner.is_empty() {
            drop(std::fs::remove_dir(&child));
            continue;
        }
        for name in inner {
            // A name in the child may equal one in `path`; go through a side name.
            drop(std::fs::rename(child.join(&name), &lifted));
            drop(std::fs::rename(
                &lifted,
                path.join(format!(
                    "{}-{}",
                    name.to_string_lossy(),
                    NEXT_LIFT.fetch_add(1, Ordering::Relaxed)
                )),
            ));
        }
        drop(std::fs::remove_dir(&child));
    }
    drop(std::fs::remove_dir(path));
}

static NEXT_LIFT: AtomicU32 = AtomicU32::new(0);

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// Run `case` alone in a child process of this test binary, with `env` set, and return whether
/// it passed and what it printed.
fn in_child(case: &str, env: &[(&str, &str)]) -> (bool, String) {
    let output = Command::new(std::env::current_exe().expect("this test binary"))
        .args([case, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_CASE, case)
        .envs(env.iter().copied())
        // A child that aborts dumps core into its working directory: not the crate's.
        .current_dir(env!("CARGO_TARGET_TMPDIR"))
        .output()
        .expect("the child runs");
    let words = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        words.contains("running 1 test"),
        "the child selected {case}: {words}"
    );
    (
        output.status.success(),
        format!("{:?}: {words}", output.status),
    )
}

fn child_case() -> Option<String> {
    std::env::var(CHILD_CASE).ok()
}

/// The real spawner, starting `/bin/sh` in place of the vendor. The shell starts a second shell
/// in the background and waits for it, as a vendor waits for a hook or a tool command it runs.
/// That grandchild writes its pid, sleeps past the run's drop and then writes into the decision
/// channel by path.
struct HookRunner {
    inner: SpawnRunner,
    channel: Rc<RefCell<Option<PathBuf>>>,
    pid: PathBuf,
}

const HOOK_FILE: &str = "written-by-the-first-runs-hook";

impl ProcessRunner for HookRunner {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        *self.channel.borrow_mut() = Some(plan.decision_channel.to_path_buf());
        let hook = format!("echo $$ > \"$2\"; sleep 3; printf late > \"$1/{HOOK_FILE}\"");
        let args = vec![
            "-c".to_owned(),
            "sh -c \"$3\" hook \"$1\" \"$2\" & wait".to_owned(),
            "vendor".to_owned(),
            plan.decision_channel.display().to_string(),
            self.pid.display().to_string(),
            hook,
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

/// a72 § 5: "the child is gone first ... and no child of the previous run can write into it".
/// The correction kills and reaps the one process metaharness spawned. Pass 1's late writer was
/// that process itself; a vendor's late write comes from a process the vendor started (a hook,
/// a tool command, an MCP server), and `Child::kill` signals one pid. The grandchild is
/// reparented, keeps running, and writes into the next run's root under the same name.
#[test]
fn a_process_the_vendor_started_does_not_outlive_its_dropped_run() {
    let name = Unique::new("hook");
    let records = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a directory");
    let channel = Rc::new(RefCell::new(None));
    let mut runner = HookRunner {
        inner: SpawnRunner::new(),
        channel: Rc::clone(&channel),
        pid: records.path().join("hook.pid"),
    };
    let first = builder()
        .with_scratch_name(&name.name)
        .start_with(
            Input::Prompt("first".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        )
        .unwrap_or_else(|refusal| panic!("the first run starts: {refusal}"));
    let mut hook = None;
    for _ in 0..500 {
        hook = std::fs::read_to_string(&runner.pid)
            .ok()
            .and_then(|words| words.trim().parse::<i32>().ok());
        if hook.is_some() {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let hook = hook.expect("the vendor's hook wrote its pid");
    let first_channel = channel.borrow().clone().expect("the first run spawned");

    drop(first);
    std::thread::sleep(Duration::from_millis(200));
    let alive = rustix::process::test_kill_process(
        rustix::process::Pid::from_raw(hook).expect("a positive pid"),
    )
    .is_ok();

    let mut second = Err(Refusal::Io {
        detail: "not started".to_owned(),
    });
    for _ in 0..100 {
        second = start(&name.name);
        if !matches!(second, Err(Refusal::ScratchNameHeld { .. })) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    let second = second.unwrap_or_else(|refusal| panic!("the second run starts: {refusal}"));
    std::thread::sleep(Duration::from_millis(3500));
    let leaked = first_channel.join(HOOK_FILE);
    let landed = exists(&leaked);
    drop(second);

    assert!(
        !alive && !landed,
        "the first run's vendor-started process {hook} was alive after the run was dropped \
         ({alive}), and its write landed in the second run's root under the same name at {} \
         ({landed})",
        leaked.display()
    );
}

/// a72 § 3: "Because `.` is in the charset, the root of the name `x.lock` is the lock path of the
/// name `x`. ... Only one of the two names can be in use at a time." The lock file of `x` is left
/// in place when its run ends (§ 3, "The lock file is left in place"), so once `x` has run, the
/// name `x.lock` is refused for good, with no run of `x` in use.
#[test]
fn a_name_ending_in_lock_runs_once_the_name_it_shadows_has_ended() {
    let shadowed = Unique::new("shadow");
    let first = start(&shadowed.name)
        .unwrap_or_else(|refusal| panic!("the run under {:?}: {refusal}", shadowed.name));
    drop(first);

    let shadow = format!("{}.lock", shadowed.name);
    let second = start(&shadow);
    let outcome = second.as_ref().map(drop).map_err(ToString::to_string);
    drop(second);
    assert!(
        outcome.is_ok(),
        "no run under {:?} is in use, yet a run under {shadow:?}, a name the charset admits, is \
         refused: {outcome:?}",
        shadowed.name
    );
}

/// How deep the tree a child leaves is: a chain of one-letter directories. `d/` 1 500 times is
/// 3 000 bytes, under `PATH_MAX`, so a plain `mkdir -p` makes it. On a 2 MiB stack in this test
/// profile, `std::fs::remove_dir_all` (which removed the root before the review corrections, and
/// still removes an unnamed root through `TempDir`) removes a chain of 10 000; the corrections'
/// recursive removal overflows at 1 500 (measured 2026-10-30: 1 000 removed, 1 500 aborted).
const DEEP: usize = 1_500;

/// Make a chain of `depth` directories named `d` under `root`, with a file at the bottom, keeping
/// one descriptor open at a time.
fn make_chain(root: &Path, depth: usize) {
    use rustix::fs::{Mode, OFlags};

    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC;
    let mut at = rustix::fs::open(root, flags, Mode::empty()).expect("the root opens");
    for _ in 0..depth {
        rustix::fs::mkdirat(&at, "d", Mode::from_raw_mode(0o700)).expect("a level");
        at = rustix::fs::openat(&at, "d", flags, Mode::empty()).expect("the level opens");
    }
    let file = rustix::fs::openat(
        &at,
        "bottom",
        OFlags::WRONLY | OFlags::CREATE | OFlags::CLOEXEC,
        Mode::from_raw_mode(0o600),
    )
    .expect("a file at the bottom");
    drop(file);
}

/// The body of the deep-tree case, in a child process: a named run whose child left a deep tree
/// in its root is dropped on a thread with the 2 MiB stack Rust gives a spawned thread.
#[test]
fn deep_tree_drop_body() {
    if child_case().as_deref() != Some("deep_tree_drop_body") {
        return;
    }
    let name = std::env::var("ADV2_NAME").expect("the name");
    // An abort here is the finding; its core file is not wanted.
    let _ = rustix::process::setrlimit(
        rustix::process::Resource::Core,
        rustix::process::Rlimit {
            current: Some(0),
            ..rustix::process::getrlimit(rustix::process::Resource::Core)
        },
    );
    // A `Run` is not `Send`, so the thread that drops it starts it.
    let root = std::thread::Builder::new()
        .stack_size(2 << 20)
        .spawn(move || {
            let run = start(&name).unwrap_or_else(|refusal| panic!("the run starts: {refusal}"));
            let root = run.scratch_root().expect("a root").to_path_buf();
            std::fs::create_dir(root.join("deep")).expect("a directory");
            make_chain(&root.join("deep"), DEEP);
            drop(run);
            root
        })
        .expect("a thread")
        .join()
        .expect("the drop returns");
    assert!(!exists(&root), "the root outlived its run");
}

/// a72 § 5: "then the root is removed the same way a leftover is". The removal recurses once per
/// level, holding a descriptor and a stack frame each, so a tree the child left that the earlier
/// `remove_dir_all` removed now overflows the stack of the thread that drops the run and aborts
/// the embedding process, leaving the root behind for the next start under the name, whose
/// leftover takeover recurses the same way.
#[test]
fn a_deep_tree_left_in_the_root_is_removed_when_the_run_is_dropped() {
    if child_case().is_some() {
        return;
    }
    let name = Unique::new("deep");
    let (passed, words) = in_child("deep_tree_drop_body", &[("ADV2_NAME", name.name.as_str())]);
    let left = exists(&name.root());
    assert!(
        passed && !left,
        "dropping a named run whose root holds a {DEEP}-deep tree: the dropping process {}, and \
         the root was {}: {}",
        if passed {
            "survived"
        } else {
            "did not survive"
        },
        if left { "left behind" } else { "removed" },
        words.lines().rev().take(6).collect::<Vec<_>>().join(" | ")
    );
}

/// a72 § 3 and review correction 1: a `flock` belongs to the open file description, so a child
/// another thread forked holds it until its `exec`, and the run "releases both locks explicitly
/// ... and only then closes them". Only `Drop` does. A start refused after it took the name's lock
/// (here: a file at the root, `scratch-root-foreign`) closes the lock without releasing it, so
/// while another thread spawns processes, the next start under the name is told
/// `scratch-name-held`, "a run ... is still going", when no run exists and the real refusal is
/// the file at the root.
#[test]
fn a_refused_start_releases_the_name_while_another_thread_spawns_processes() {
    let name = Unique::new("refused");
    std::fs::write(name.root(), "a file at the root").expect("a file");
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let spawner = {
        let stop = std::sync::Arc::clone(&stop);
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                drop(Command::new("/bin/true").status());
            }
        })
    };
    let mut held = Vec::new();
    for attempt in 0..300 {
        match start(&name.name) {
            Err(Refusal::ScratchRootForeign { .. }) => {}
            Err(refusal) => held.push(format!("attempt {attempt}: {refusal}")),
            Ok(_) => held.push(format!(
                "attempt {attempt}: started over a file at the root"
            )),
        }
    }
    stop.store(true, Ordering::Relaxed);
    spawner.join().expect("the spawner thread");
    assert!(
        held.is_empty(),
        "{} of 300 starts, each made after the previous one was refused for the file at the root, \
         were refused for something else; first: {}",
        held.len(),
        held.first().map_or("", String::as_str)
    );
}
