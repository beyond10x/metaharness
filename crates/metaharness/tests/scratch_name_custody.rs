//! Security review of `story:scratch-root-stable-by-declared-name` (design amendment a72): the
//! custody of a named scratch root, which holds a copied login, the hook channel and the context
//! copies the loop reads.
//!
//! Each case is an integrity check of a72's own contract: the name is held for the run's life, the
//! lock is refused unless it is a regular file of this user's, a leftover of this user's is
//! replaced, nothing is followed through a link, the root is 0700, and the root lies in the
//! directory an unnamed root is made in. The scratch code is the same on all three kinds
//! (`ScratchRoot::for_spec`), so these drive the Claude kind over the scripted runner: no vendor
//! process, no credential, no network.
//!
//! Two cases change process-wide state (the umask, `TMPDIR`), so they run their body in a child
//! process of this test binary that runs only that case.
#![cfg(unix)]

use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

use metaharness::protocol::{CredentialSource, Kind};
use metaharness::{
    Input, Metaharness, Refusal, Run, SCRATCH_NAME_HELD, SCRATCH_ROOT_FOREIGN, ScriptedLog,
    ScriptedRunner, ScriptedSeams,
};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// Set in a child process of this binary to the one case whose body it runs.
const CHILD_CASE: &str = "METAHARNESS_SCRATCH_CUSTODY_CASE";

fn builder() -> Metaharness {
    Metaharness::new(Kind::Claude).with_credentials(CredentialSource::None)
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
        use std::os::unix::fs::DirBuilderExt as _;

        static NEXT: AtomicU32 = AtomicU32::new(0);
        let name = format!(
            "custody-{case}-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        // The layout of a72's review corrections (decision 3): `<temp dir>/metaharness-u<EUID>/
        // <NAME>`, its lock beside it. The parent is made 0700 here, as metaharness makes it, so
        // a case can put an entry at the root or at the lock path before the first run.
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
        Self {
            root: parent.join(&name),
            lock: parent.join(format!("{name}.lock")),
            name,
        }
    }
}

impl Drop for Unique {
    fn drop(&mut self) {
        match std::fs::symlink_metadata(&self.root) {
            Ok(found) if found.is_dir() => drop(std::fs::remove_dir_all(&self.root)),
            Ok(_) => drop(std::fs::remove_file(&self.root)),
            Err(_) => (),
        }
        drop(std::fs::remove_file(&self.lock));
    }
}

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

fn mode(path: &Path) -> u32 {
    std::fs::symlink_metadata(path)
        .expect("metadata")
        .permissions()
        .mode()
        & 0o7777
}

/// Run `case` alone in a child process of this test binary, with `env` set and in `cwd`.
fn in_child(case: &str, env: &[(&str, &str)], cwd: &Path) {
    let output = Command::new(std::env::current_exe().expect("this test binary"))
        .args([case, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_CASE, case)
        .envs(env.iter().copied())
        .current_dir(cwd)
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
    assert!(output.status.success(), "{case} in a child: {words}");
}

fn child_case() -> Option<String> {
    std::env::var(CHILD_CASE).ok()
}

/// a72 § 3: "The name is held for the run's life." A lock file unlinked while a run holds it (an
/// operator's `rm`, or an age-based temp cleaner, since nothing a run does changes the lock file's
/// times after it is made) leaves the first run holding a lock on an inode no path names. The
/// second run makes a new lock file, takes it, reads the live root as this user's leftover and
/// removes it.
#[test]
fn a_held_name_stays_held_after_its_lock_file_is_unlinked() {
    let name = Unique::new("unlinked");
    let first_log = ScriptedLog::new();
    let first = start(builder().with_scratch_name(&name.name), &first_log)
        .unwrap_or_else(|refusal| panic!("the first run is refused: {refusal}"));
    let marker = name.root.join("first-run.txt");
    std::fs::write(&marker, "the first run's").expect("a file in the first run's root");

    std::fs::remove_file(&name.lock).expect("the lock file is unlinked under the holder");

    let second_log = ScriptedLog::new();
    let second = start(builder().with_scratch_name(&name.name), &second_log);
    let outcome = second.as_ref().map(drop).map_err(ToString::to_string);
    let kept = std::fs::read_to_string(&marker).ok();
    let spawned = second_log.spawns();
    drop(second);
    drop(first);

    assert!(
        matches!(&outcome, Err(words) if words.starts_with(SCRATCH_NAME_HELD))
            && kept.as_deref() == Some("the first run's")
            && spawned == 0,
        "while the first run held {:?}, a second run under it was {outcome:?} with {spawned} \
         spawn(s), and the first run's root kept its file: {kept:?}",
        name.name
    );
}

/// `open_scratch_lock` promises to refuse a lock path that is not a regular file. A FIFO there is
/// opened `O_RDONLY` without `O_NONBLOCK`, so the start waits in `open` for a writer instead of
/// refusing.
#[test]
fn a_fifo_at_the_lock_path_is_refused_without_waiting_for_a_writer() {
    use rustix::fs::{Mode, OFlags};

    let name = Unique::new("fifo");
    rustix::fs::mkfifoat(rustix::fs::CWD, &name.lock, Mode::from_raw_mode(0o600))
        .expect("a FIFO at the lock path");

    let (sender, receiver) = std::sync::mpsc::channel();
    let thread_name = name.name.clone();
    let worker = std::thread::spawn(move || {
        let log = ScriptedLog::new();
        let outcome = start(builder().with_scratch_name(&thread_name), &log)
            .map(drop)
            .map_err(|refusal| refusal.to_string());
        drop(sender.send(outcome));
    });

    let waited = receiver.recv_timeout(Duration::from_secs(5));
    let blocked = waited.is_err();
    let outcome = if let Ok(outcome) = waited {
        outcome
    } else {
        // Release the start: a writer lets the reader waiting in `open` return.
        let writer = rustix::fs::open(
            &name.lock,
            OFlags::WRONLY | OFlags::NONBLOCK | OFlags::CLOEXEC,
            Mode::empty(),
        );
        let outcome = receiver
            .recv_timeout(Duration::from_secs(30))
            .expect("the start returns once a writer opened the FIFO");
        drop(writer);
        outcome
    };
    worker.join().expect("the worker");

    assert!(
        !blocked,
        "the start waited 5 s in open() on a FIFO at {} and returned {outcome:?} only after a \
         writer opened it",
        name.lock.display()
    );
    let words = outcome.expect_err("a FIFO at the lock path starts no run");
    assert!(words.starts_with(SCRATCH_ROOT_FOREIGN), "{words}");
}

/// a72 § 4: a directory this user owns at the root is a leftover, "removed ... and created again".
/// A child that left a directory without owner write permission under its scratch (a copied
/// read-only tree) makes `remove_dir_all` fail: the run's own drop leaves the root silently, and
/// every later run under the name is refused, where an unnamed run got a new directory.
#[test]
fn a_leftover_holding_a_read_only_directory_is_replaced() {
    let name = Unique::new("read-only");
    let locked = name.root.join("tmp").join("read-only-tree");
    std::fs::create_dir_all(&locked).expect("a leftover with a nested directory");
    std::fs::write(locked.join("file.txt"), "copied read-only").expect("a file");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o500))
        .expect("the directory made read-only, as a copy of a read-only tree is");

    let log = ScriptedLog::new();
    let outcome = start(builder().with_scratch_name(&name.name), &log)
        .map(|run| {
            let replaced = !exists(&locked) && mode(&name.root) == 0o700;
            drop(run);
            replaced
        })
        .map_err(|refusal| refusal.to_string());

    if exists(&locked) {
        std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o700))
            .expect("writable again, so the guard can remove it");
    }
    assert_eq!(
        outcome,
        Ok(true),
        "a leftover of this user's holding a read-only directory was not replaced"
    );
}

/// a72 § 4: the leftover is removed with `remove_dir_all`, "which removes a link inside it and
/// never what the link names". Links at the top and one level down, to a directory and to a file.
#[test]
fn a_link_inside_a_leftover_is_removed_and_never_followed() {
    let outside = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a directory");
    std::fs::write(outside.path().join("keep.txt"), "not metaharness's").expect("a file");
    std::fs::create_dir(outside.path().join("nested")).expect("a directory");
    std::fs::write(outside.path().join("nested/deep.txt"), "nor this").expect("a file");

    let name = Unique::new("inner-link");
    std::fs::create_dir_all(name.root.join("home")).expect("a leftover root");
    std::os::unix::fs::symlink(outside.path(), name.root.join("dir-link")).expect("a link");
    std::os::unix::fs::symlink(outside.path().join("keep.txt"), name.root.join("file-link"))
        .expect("a link");
    std::os::unix::fs::symlink(outside.path(), name.root.join("home/nested-link")).expect("a link");

    let log = ScriptedLog::new();
    let run = start(builder().with_scratch_name(&name.name), &log)
        .unwrap_or_else(|refusal| panic!("refused over its own leftover: {refusal}"));
    assert!(!exists(&name.root.join("dir-link")));
    assert!(!exists(&name.root.join("home")));
    drop(run);

    assert_eq!(
        std::fs::read_to_string(outside.path().join("keep.txt")).expect("still there"),
        "not metaharness's"
    );
    assert_eq!(
        std::fs::read_to_string(outside.path().join("nested/deep.txt")).expect("still there"),
        "nor this"
    );
}

/// a72 § 4: "the root is created with mode 0700", and § 3 the lock file 0600, whatever the umask.
/// Run with an empty umask, so nothing but the mode the code asks for decides.
#[test]
fn the_root_and_the_lock_are_private_under_an_empty_umask() {
    const CASE: &str = "the_root_and_the_lock_are_private_under_an_empty_umask";
    if child_case().as_deref() == Some(CASE) {
        rustix::process::umask(rustix::fs::Mode::empty());
        let name = Unique::new("umask");
        let log = ScriptedLog::new();
        let run = start(builder().with_scratch_name(&name.name), &log)
            .unwrap_or_else(|refusal| panic!("a named run is refused: {refusal}"));
        assert_eq!(mode(&name.root), 0o700, "the root");
        assert_eq!(mode(&name.lock), 0o600, "the lock file");
        drop(run);
        return;
    }
    in_child(CASE, &[], Path::new(env!("CARGO_TARGET_TMPDIR")));
}

/// a72 § 1: the named root lies in "the temp dir ... an unnamed root is made in". `tempfile` makes
/// a relative `TMPDIR` absolute against the working directory before it makes an unnamed root; the
/// named root keeps the relative path, so `Run::scratch_root`, and every path the child is given
/// from it (`TMPDIR`, `CLAUDE_CONFIG_DIR`, `CODEX_HOME`), is read against whatever directory the
/// reader is in.
#[test]
fn a_named_root_is_absolute_as_an_unnamed_one_is() {
    const CASE: &str = "a_named_root_is_absolute_as_an_unnamed_one_is";
    if child_case().as_deref() == Some(CASE) {
        let log = ScriptedLog::new();
        let unnamed = start(builder(), &log)
            .unwrap_or_else(|refusal| panic!("an unnamed run is refused: {refusal}"));
        let unnamed_root = unnamed.scratch_root().expect("a scratch").to_path_buf();
        drop(unnamed);
        assert!(unnamed_root.is_absolute(), "{}", unnamed_root.display());

        let name = Unique::new("relative");
        let log = ScriptedLog::new();
        let named = start(builder().with_scratch_name(&name.name), &log)
            .unwrap_or_else(|refusal| panic!("a named run is refused: {refusal}"));
        let named_root = named.scratch_root().expect("a scratch").to_path_buf();
        drop(named);
        assert!(
            named_root.is_absolute(),
            "under TMPDIR=relative-temp the unnamed root is {} and the named root is {}",
            unnamed_root.display(),
            named_root.display()
        );
        return;
    }
    let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a directory");
    std::fs::create_dir(base.path().join("relative-temp")).expect("the relative temp dir");
    in_child(CASE, &[("TMPDIR", "relative-temp")], base.path());
}
