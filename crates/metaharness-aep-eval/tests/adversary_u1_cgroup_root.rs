//! Adversary pass 1, unit U1 (`story:eval-cgroup-root-check-by-prefix`).
//!
//! The unit's cases call the pure rule `cgroup_below_root` with a stand-in mount of their own, so
//! nothing checks what `below_cgroup_root` hands it: the real `/proc/self/cgroup` and the real
//! mount. A wrong mount constant that still sits under `/sys/fs/cgroup` keeps every existing case
//! green (`adversary_w8u7` passes the mount itself as the root, and `adversary_u4_scope` a root
//! outside the cgroup tree). Here the built `aep-eval preflight` runs below a real ancestor of its
//! own cgroup, named three ways the acceptance lists (plainly, with `..`, through a symlink), and
//! must not try to enter a delegated scope. It has no session bus, so a wrong judgement is
//! refused by the `systemd` adapter before anything moves; a right one goes on to the harness
//! checkout, which is missing on purpose, and fails there.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, PoisonError};

static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn aep_eval() -> PathBuf {
    std::env::var_os("ADV_U1_AEP_EVAL").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_aep-eval")),
        PathBuf::from,
    )
}

fn work_root() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("adversary_u1_cgroup_root")
}

/// The cgroup directory this test process is in; a child it starts begins in the same one.
fn own_cgroup() -> PathBuf {
    let text = fs::read_to_string("/proc/self/cgroup").expect("read /proc/self/cgroup");
    let line = text
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .expect("a unified cgroup line");
    let own = Path::new("/sys/fs/cgroup").join(line.trim_start_matches('/'));
    assert!(own.is_dir(), "{} is not a directory", own.display());
    own
}

/// The parent of this process's cgroup, which is below the mount and an ancestor of the process.
fn ancestor() -> PathBuf {
    let own = own_cgroup();
    let parent = own.parent().expect("a parent").to_path_buf();
    assert!(
        parent.starts_with("/sys/fs/cgroup/") && parent != Path::new("/sys/fs/cgroup"),
        "this host runs the test at the top of the cgroup tree ({}); the case needs a slice",
        own.display()
    );
    parent
}

struct Run {
    stdout: String,
    stderr: String,
}

fn preflight(label: &str, root: &Path) -> Run {
    let case = work_root().join(label);
    if case.exists() {
        fs::remove_dir_all(&case).expect("clear the previous case");
    }
    let aep = case.join("aep");
    let plugins = case.join("agentplugins");
    let harness_repo = case.join("harness");
    let home = case.join("home");
    for directory in [&aep, &plugins, &harness_repo, &home] {
        fs::create_dir_all(directory).expect("create a case directory");
    }
    let harness = case.join("stand-in-harness");
    fs::write(&harness, "#!/bin/sh\nexit 0\n").expect("write the stand-in harness");
    let output = Command::new(aep_eval())
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .env("HOME", &home)
        .arg("preflight")
        .arg("--aep-repo")
        .arg(&aep)
        .arg("--agentplugins-repo")
        .arg(&plugins)
        .arg("--harness-repo")
        .arg(&harness_repo)
        .arg("--b10x-binary")
        .arg(&harness)
        .arg("--scratch-root")
        .arg(case.join("scratch"))
        .arg("--cgroup-root")
        .arg(root)
        .output()
        .expect("start aep-eval");
    Run {
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The eval judged itself below `root`: it did not reach for the adapter and failed at the next
/// step, the harness checkout this case leaves without `.git`.
fn assert_judged_below(run: &Run, root: &Path) {
    assert!(
        !run.stdout.contains("entering a delegated scope"),
        "the eval is below {} and still tried to enter a scope:\nstdout:\n{}\nstderr:\n{}",
        root.display(),
        run.stdout,
        run.stderr
    );
    let failure = run
        .stderr
        .lines()
        .find(|line| line.starts_with("FAIL: "))
        .unwrap_or_else(|| panic!("no FAIL line:\n{}", run.stderr));
    assert!(
        failure.contains("is not a harness checkout"),
        "below {} the eval should fail at the harness checkout, not: {failure}",
        root.display()
    );
}

#[test]
fn a_real_ancestor_of_the_process_cgroup_is_judged_below() {
    let _serial = serial();
    let root = ancestor();
    let run = preflight("plain", &root);
    assert_judged_below(&run, &root);
}

#[test]
fn a_real_ancestor_given_with_dot_dot_is_judged_below() {
    let _serial = serial();
    let own = own_cgroup();
    let root = own.join("..");
    let run = preflight("dot-dot", &root);
    assert_judged_below(&run, &root);
}

#[test]
fn a_real_ancestor_given_through_a_symlink_is_judged_below() {
    let _serial = serial();
    let target = ancestor();
    let links = work_root().join("links");
    fs::create_dir_all(&links).expect("create the link directory");
    let link = links.join("to-ancestor");
    if fs::symlink_metadata(&link).is_ok() {
        fs::remove_file(&link).expect("remove the previous link");
    }
    std::os::unix::fs::symlink(&target, &link).expect("link to the ancestor");
    let run = preflight("symlink", &link);
    assert_judged_below(&run, &link);
}
