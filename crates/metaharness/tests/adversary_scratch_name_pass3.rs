//! Adversary pass 3 on `story:scratch-root-stable-by-declared-name` (design amendment a72), against
//! review corrections pass 2: the group kill of a named run's vendor child.
//!
//! a72 § 5: "when the run is dropped the whole group is sent `SIGKILL`, even when the leader was
//! already reaped". Once the leader is reaped and the group is empty, its id is a free pid. The
//! kernel hands it out again, and a process that then leads a group of its own (any job a shell
//! starts, any `process_group(0)` or `setsid` child) owns that group id. The drop kills it.
#![cfg(target_os = "linux")]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use metaharness::protocol::{CredentialSource, Kind};
use metaharness::{
    HarnessProcess, Input, LaunchPlanView, Metaharness, ProcessRunner, ScriptedSeams, SpawnRunner,
};

/// Set in a child process of this binary to the one case whose body it runs.
const CHILD_CASE: &str = "METAHARNESS_SCRATCH_ADV3_CASE";

fn child_case() -> Option<String> {
    std::env::var(CHILD_CASE).ok()
}

/// The real spawner, starting `/bin/sh -c 'exit 0'` in place of the vendor: a child that ends at
/// once, so the run reaps it while the run is still held.
struct ExitingRunner {
    inner: SpawnRunner,
}

impl ProcessRunner for ExitingRunner {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
        let args = vec!["-c".to_owned(), "exit 0".to_owned()];
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

fn exists(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok()
}

/// The body, in a child process that is pid 1 of a user and pid namespace of its own, so it can
/// name the next pid the kernel hands out (`/proc/sys/kernel/ns_last_pid`) and stand in for the
/// wrap of the pid counter that makes a freed pid come round again on a busy machine.
#[test]
fn reused_group_id_body() {
    use std::os::unix::process::CommandExt as _;

    if child_case().as_deref() != Some("reused_group_id_body") {
        return;
    }
    // The process is pid 1 of a namespace of its own, so its pid names nothing: the parent's
    // test process supplies the name.
    let name = std::env::var("ADV3_NAME").expect("the name");
    let mut runner = ExitingRunner {
        inner: SpawnRunner::new(),
    };
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_scratch_name(&name)
        .start_with(
            Input::Prompt("a session".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        )
        .unwrap_or_else(|refusal| panic!("the named run starts: {refusal}"));
    let led = run
        .process_group()
        .expect("a named run's child leads a group of its own");
    let _ = run.drain();
    for _ in 0..500 {
        if !exists(Path::new(&format!("/proc/{led}"))) {
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    assert!(
        !exists(Path::new(&format!("/proc/{led}"))),
        "precondition: the run reaped its child {led} and nothing is left in its group"
    );

    // An unrelated job: a process that leads a group of its own and is handed the freed pid.
    std::fs::write("/proc/sys/kernel/ns_last_pid", format!("{}", led - 1))
        .expect("the next pid is named");
    let mut job = Command::new("/bin/sleep")
        .arg("30")
        .process_group(0)
        .spawn()
        .expect("the job starts");
    let job_pid = i32::try_from(job.id()).expect("a pid");
    assert_eq!(
        job_pid, led,
        "precondition: the job was handed the freed group id"
    );

    drop(run);
    std::thread::sleep(Duration::from_millis(200));
    let after = job.try_wait().expect("the job can be asked");
    drop(job.kill());
    drop(job.wait());
    assert_eq!(
        after, None,
        "dropping the named run, whose child {led} was already reaped, left alone the unrelated \
         job that now leads process group {led}; it ended with {after:?}"
    );
}

/// Whether this machine lets an unprivileged user make a user and pid namespace and set the next
/// pid in it.
fn namespaces_available() -> bool {
    Command::new("unshare")
        .args([
            "-Urpf",
            "--mount-proc",
            "/bin/sh",
            "-c",
            "echo 100 > /proc/sys/kernel/ns_last_pid",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

/// a72 § 5 and review corrections pass 2, decision 1: the group kill on drop is sent "even when
/// the leader was already reaped". After the reap, nothing holds the group id: the kernel frees a
/// pid once no process uses it as a pid, a group id or a session id. A later process given that
/// pid that leads a group of its own (a shell's next job, a `process_group(0)` child of any
/// program) is the group the drop kills. Killing the group before the leader is reaped (the
/// zombie leader keeps the id) closes this; killing it after cannot.
#[test]
fn dropping_a_named_run_whose_child_was_reaped_leaves_a_group_that_reused_its_id_alone() {
    if child_case().is_some() {
        return;
    }
    if !namespaces_available() {
        eprintln!(
            "skipped: this machine does not let an unprivileged user make a user and pid \
             namespace, which this case needs to hand out a chosen pid"
        );
        return;
    }
    let temp = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a temp dir");
    let case = "reused_group_id_body";
    let output = Command::new("unshare")
        .args(["-Urpf", "--mount-proc"])
        .arg(std::env::current_exe().expect("this test binary"))
        .args([case, "--exact", "--nocapture", "--test-threads=1"])
        .env(CHILD_CASE, case)
        .env("ADV3_NAME", format!("adv3-reuse-{}", std::process::id()))
        .env("TMPDIR", temp.path())
        .current_dir(temp.path())
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
    let root: PathBuf = temp.path().to_path_buf();
    assert!(
        output.status.success(),
        "{:?} under {}: {}",
        output.status,
        root.display(),
        words.lines().rev().take(8).collect::<Vec<_>>().join(" | ")
    );
}
