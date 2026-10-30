//! Adversary pass 3 on `story:scratch-root-stable-by-declared-name` (design amendment a72 § 5),
//! against the CLI's signal forwarding.
//!
//! A named run's vendor child now leads a process group of its own, so the terminal's job-control
//! signals stop at metaharness. The forwarding takes over `SIGINT`, `SIGTERM` and `SIGHUP`. The
//! terminal sends two more to its foreground job: `SIGQUIT` (Ctrl-\) and `SIGTSTP` (Ctrl-Z). An
//! unnamed run's vendor, in metaharness's group, receives both; a named run's receives neither.
//!
//! Each case starts `metaharness run --scratch-name` as a shell starts a job, leading a process
//! group of its own, and sends the signal to that group, as the terminal does.
#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

/// A scratch `HOME` holding a `b10x-harness` stub and a scratch temp dir. The stub starts a
/// background `sleep` (a hook or tool command a vendor starts), writes its pid and its own, and
/// waits.
struct Vendor {
    base: tempfile::TempDir,
    home: PathBuf,
    temp: PathBuf,
    background: PathBuf,
    leader: PathBuf,
}

impl Vendor {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt as _;

        let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory");
        let home = base.path().join("home");
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).expect("the stub's directory");
        let temp = base.path().join("temp");
        std::fs::create_dir(&temp).expect("a temp dir");
        let background = base.path().join("background.pid");
        let leader = base.path().join("leader.pid");
        let stub = bin.join("b10x-harness");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then echo 'b10x-harness 0.22.0'; exit 0; fi\n\
                 sleep 60 &\n\
                 echo $! > '{background}'\n\
                 echo $$ > '{leader}'\n\
                 wait\n\
                 exit 0\n",
                background = background.display(),
                leader = leader.display(),
            ),
        )
        .expect("the stub");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700))
            .expect("the stub is executable");
        Self {
            base,
            home,
            temp,
            background,
            leader,
        }
    }

    /// `metaharness run b10x --scratch-name <name>`, started as a shell starts a job: leading a
    /// process group of its own, with core dumps off and its working directory in scratch.
    fn job(&self, name: &str) -> std::process::Child {
        use std::os::unix::process::CommandExt as _;

        Command::new("/bin/sh")
            .args([
                "-c",
                "ulimit -c 0 && exec \"$@\"",
                "job",
                env!("CARGO_BIN_EXE_metaharness"),
                "run",
                "b10x",
                "--decisions",
                "observe",
                "--credentials",
                "none",
                "--model-endpoint",
                "http://127.0.0.1:1/v1",
                "--model",
                "a-model",
                "--scratch-name",
                name,
                "-p",
                "go",
            ])
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", "/usr/bin:/bin")
            .env("TMPDIR", &self.temp)
            .current_dir(self.base.path())
            .process_group(0)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the binary runs")
    }
}

/// The pid written in `path`, once it is there.
fn pid_in(path: &Path) -> i32 {
    for _ in 0..1_000 {
        if let Some(pid) = std::fs::read_to_string(path)
            .ok()
            .and_then(|words| words.trim().parse().ok())
        {
            return pid;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("no pid was written in {}", path.display());
}

/// The state letter of `pid` in `/proc/<pid>/stat`, `None` once it is gone.
fn state(pid: i32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    stat.rsplit_once(") ")
        .and_then(|(_, rest)| rest.chars().next())
}

/// Whether `pid` is gone or a zombie.
fn gone(pid: i32) -> bool {
    matches!(state(pid), None | Some('Z'))
}

/// Wait up to `seconds` for `test` to hold.
fn within(seconds: u64, mut test: impl FnMut() -> bool) -> bool {
    for _ in 0..seconds * 100 {
        if test() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    test()
}

fn group(pid: i32) -> rustix::process::Pid {
    rustix::process::Pid::from_raw(pid).expect("a positive pid")
}

/// Kill the vendor's group and metaharness's, whatever is left of them.
fn end_all(job: &mut std::process::Child, leader: i32) {
    let _ = rustix::process::kill_process_group(group(leader), rustix::process::Signal::KILL);
    let job_pid = i32::try_from(job.id()).expect("a pid");
    let _ = rustix::process::kill_process_group(group(job_pid), rustix::process::Signal::KILL);
    let _ = job.wait();
}

/// a72 § 5: "no process of the previous run's group can write into" the next run's root, and the
/// forwarding exists because "a group of its own no longer receives the terminal's `SIGINT`". It
/// no longer receives the terminal's `SIGQUIT` either, which is not forwarded. Ctrl-\ ends
/// metaharness by its default action, so the run is never dropped and its group never killed:
/// the vendor and what it started keep running, with the name's locks released, and the next
/// run under the name takes their root over as a leftover while they still write into it. In
/// metaharness's own group, as an unnamed run's vendor is, the vendor died with it.
#[test]
fn sigquit_from_the_terminal_ends_a_named_runs_vendor_with_metaharness() {
    let vendor = Vendor::new();
    let name = format!("adv3-quit-{}", std::process::id());
    let mut job = vendor.job(&name);
    let leader = pid_in(&vendor.leader);
    let background = pid_in(&vendor.background);
    let job_pid = i32::try_from(job.id()).expect("a pid");

    rustix::process::kill_process_group(group(job_pid), rustix::process::Signal::QUIT)
        .expect("the terminal's SIGQUIT reaches the job");
    let ended = within(20, || matches!(job.try_wait(), Ok(Some(_))));
    let leader_gone = within(5, || gone(leader));
    let background_gone = gone(background);
    end_all(&mut job, leader);

    assert!(ended, "metaharness ends on SIGQUIT");
    assert!(
        leader_gone && background_gone,
        "after the terminal's SIGQUIT ended metaharness, the named run's vendor {leader} is gone \
         ({leader_gone}) and the process it started {background} is gone ({background_gone})"
    );
}

/// The terminal's Ctrl-Z stops its foreground job. An unnamed run's vendor, in metaharness's
/// group, stops with metaharness. A named run's vendor leads a group the terminal does not
/// signal and `SIGTSTP` is not forwarded, so metaharness stops while the vendor keeps running
/// the session (its tool commands included) in the background, with nobody reading its record.
#[test]
fn sigtstp_from_the_terminal_stops_a_named_runs_vendor_with_metaharness() {
    let vendor = Vendor::new();
    let name = format!("adv3-tstp-{}", std::process::id());
    let mut job = vendor.job(&name);
    let leader = pid_in(&vendor.leader);
    let job_pid = i32::try_from(job.id()).expect("a pid");

    rustix::process::kill_process_group(group(job_pid), rustix::process::Signal::TSTP)
        .expect("the terminal's SIGTSTP reaches the job");
    let job_stopped = within(5, || state(job_pid) == Some('T'));
    let leader_stopped = within(2, || state(leader) == Some('T'));
    let leader_state = state(leader);
    let _ = rustix::process::kill_process_group(group(job_pid), rustix::process::Signal::CONT);
    end_all(&mut job, leader);

    assert!(job_stopped, "metaharness stops on SIGTSTP");
    assert!(
        leader_stopped,
        "with metaharness stopped by the terminal's SIGTSTP, the named run's vendor {leader} is \
         stopped too; its state is {leader_state:?}"
    );
}

/// Set in a child process of this binary to the one case whose body it runs.
const CHILD_CASE: &str = "METAHARNESS_CLI_SCRATCH_ADV3_CASE";

/// The body of the in-process case: `execute` a named run whose vendor ends at once, then send
/// this process `SIGINT` after `execute` has returned.
#[test]
fn handler_after_execute_body() {
    use clap::Parser as _;

    if std::env::var(CHILD_CASE).as_deref() != Ok("handler_after_execute_body") {
        return;
    }
    let name = std::env::var("ADV3_NAME").expect("the name");
    let cli = metaharness_cli::Cli::try_parse_from([
        "metaharness",
        "run",
        "b10x",
        "--decisions",
        "observe",
        "--credentials",
        "none",
        "--model-endpoint",
        "http://127.0.0.1:1/v1",
        "--model",
        "a-model",
        "--scratch-name",
        name.as_str(),
        "-p",
        "go",
    ])
    .expect("parses");
    let code = metaharness_cli::execute(cli);
    eprintln!("execute returned {code}");
    std::thread::sleep(Duration::from_millis(300));
    rustix::process::kill_process(rustix::process::getpid(), rustix::process::Signal::INT)
        .expect("SIGINT is sent");
    std::thread::sleep(Duration::from_millis(500));
    panic!("this process survived a SIGINT sent after the named run's execute returned");
}

/// a72 § 5: "`metaharness run` with a `--scratch-name` takes over `SIGINT`, `SIGTERM` and
/// `SIGHUP` for the run's life", and "a run without a name keeps the default handlers". The
/// forwarding closes its `Signals` when the run ends, which unregisters its action but leaves
/// signal-hook's handler installed with nothing behind it, so after the run the three signals
/// are ignored for the rest of the process, not given back their default action. The binary
/// exits at once; a caller of the public `metaharness_cli::execute` that goes on (a later
/// unnamed run, a test harness) can no longer be interrupted.
#[test]
fn the_default_sigint_action_is_back_after_a_named_runs_execute_returns() {
    use std::os::unix::fs::PermissionsExt as _;
    use std::os::unix::process::ExitStatusExt as _;

    if std::env::var(CHILD_CASE).is_ok() {
        return;
    }
    let base = tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a scratch directory");
    let home = base.path().join("home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the stub's directory");
    let temp = base.path().join("temp");
    std::fs::create_dir(&temp).expect("a temp dir");
    let stub = bin.join("b10x-harness");
    std::fs::write(
        &stub,
        "#!/bin/sh\n\
         if [ \"$1\" = --version ]; then echo 'b10x-harness 0.22.0'; exit 0; fi\n\
         exit 0\n",
    )
    .expect("the stub");
    std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700))
        .expect("the stub is executable");
    let case = "handler_after_execute_body";
    let output = Command::new(std::env::current_exe().expect("this test binary"))
        .args([case, "--exact", "--nocapture", "--test-threads=1"])
        .env_clear()
        .env(CHILD_CASE, case)
        .env("ADV3_NAME", format!("adv3-handler-{}", std::process::id()))
        .env("HOME", &home)
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", &temp)
        .current_dir(base.path())
        .output()
        .expect("the child runs");
    let words = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        words.contains("running 1 test") && words.contains("execute returned"),
        "the child ran the named run in process: {words}"
    );
    assert_eq!(
        output.status.signal(),
        Some(signal_hook::consts::SIGINT),
        "a SIGINT after the named run ended takes its default action: {:?}: {}",
        output.status,
        words.lines().rev().take(4).collect::<Vec<_>>().join(" | ")
    );
}
