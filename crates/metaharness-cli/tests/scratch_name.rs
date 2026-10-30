//! `--scratch-name` on the run verb (`story:scratch-root-stable-by-declared-name`, design
//! amendment a72): the help names it, and the value reaches `RunSpec::scratch_name` unchanged.
use std::process::Command;

use clap::Parser as _;
use metaharness_cli::{Cli, Verb};

#[test]
fn the_run_verbs_help_names_scratch_name() {
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["run", "--help"])
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(help.contains("--scratch-name <NAME>"), "{help}");
}

#[test]
fn the_scratch_name_flag_reaches_the_run_spec() {
    let cli = Cli::try_parse_from(["metaharness", "run", "b10x", "--scratch-name", "case-7"])
        .expect("parses");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    assert_eq!(args.spec.scratch_name.as_deref(), Some("case-7"));

    let cli = Cli::try_parse_from(["metaharness", "run", "claude"]).expect("parses");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    assert_eq!(args.spec.scratch_name, None);

    let cli = Cli::try_parse_from(["metaharness", "run", "codex", "--scratch-name", "a/b"])
        .expect("the shape is the library's to refuse, as every face refuses it");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    let refusal = metaharness::check_spec(&args.spec).expect_err("not one plain component");
    assert!(
        refusal
            .to_string()
            .starts_with(metaharness::SCRATCH_NAME_INVALID),
        "{refusal}"
    );
}

/// A scratch `HOME` holding a `b10x-harness` stub and a scratch temp dir. The stub marks that it
/// was started in `started` (its `--version` probe aside). While `linger` exists,
/// the stub starts a background `sleep` (a hook or tool command a vendor starts), writes its pid,
/// then its own, and waits; a background job of a non-interactive shell ignores `SIGINT`.
#[cfg(unix)]
struct Vendor {
    _base: tempfile::TempDir,
    home: std::path::PathBuf,
    temp: std::path::PathBuf,
    started: std::path::PathBuf,
    linger: std::path::PathBuf,
    background: std::path::PathBuf,
    leader: std::path::PathBuf,
}

#[cfg(unix)]
impl Vendor {
    fn new() -> Self {
        use std::os::unix::fs::PermissionsExt as _;

        let base = tempfile::tempdir().expect("a scratch directory");
        let home = base.path().join("home");
        let bin = home.join(".local/bin");
        std::fs::create_dir_all(&bin).expect("the stub's directory");
        let temp = base.path().join("temp");
        std::fs::create_dir(&temp).expect("a temp dir");
        let started = base.path().join("started");
        let linger = base.path().join("linger");
        let background = base.path().join("background.pid");
        let leader = base.path().join("leader.pid");
        let stub = bin.join("b10x-harness");
        std::fs::write(
            &stub,
            format!(
                "#!/bin/sh\n\
                 if [ \"$1\" = --version ]; then echo 'b10x-harness 0.22.0'; exit 0; fi\n\
                 : > '{started}'\n\
                 if [ -e '{linger}' ]; then\n\
                 sleep 60 &\n\
                 echo $! > '{background}'\n\
                 echo $$ > '{leader}'\n\
                 wait\n\
                 fi\n\
                 exit 0\n",
                started = started.display(),
                linger = linger.display(),
                background = background.display(),
                leader = leader.display(),
            ),
        )
        .expect("the stub");
        std::fs::set_permissions(&stub, std::fs::Permissions::from_mode(0o700))
            .expect("the stub is executable");
        Self {
            _base: base,
            home,
            temp,
            started,
            linger,
            background,
            leader,
        }
    }

    fn command(&self, name: &str) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_metaharness"));
        command
            .args([
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
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        command
    }
}

/// The pid written in `path`, once it is there.
#[cfg(unix)]
fn pid_in(path: &std::path::Path) -> Option<i32> {
    for _ in 0..1_000 {
        if let Some(pid) = std::fs::read_to_string(path)
            .ok()
            .and_then(|words| words.trim().parse().ok())
        {
            return Some(pid);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    None
}

/// Whether `pid` is gone: no such process, or one that has exited and waits to be reaped.
#[cfg(unix)]
fn gone(pid: i32) -> bool {
    std::fs::read_to_string(format!("/proc/{pid}/stat")).map_or_else(
        |_| {
            rustix::process::Pid::from_raw(pid)
                .is_none_or(|pid| rustix::process::test_kill_process(pid).is_err())
        },
        |stat| {
            stat.rsplit_once(") ")
                .is_some_and(|(_, rest)| rest.starts_with('Z'))
        },
    )
}

/// Wait up to `seconds` for `child` to exit.
#[cfg(unix)]
fn exited(child: &mut std::process::Child, seconds: u64) -> Option<std::process::ExitStatus> {
    for _ in 0..seconds * 100 {
        if let Ok(Some(status)) = child.try_wait() {
            return Some(status);
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    drop(child.kill());
    drop(child.wait());
    None
}

/// Design amendment a72 § 5: a named run's vendor child leads a process group of its own, so the
/// terminal's `SIGINT` reaches metaharness and not the vendor. metaharness forwards it to the
/// group, kills what ignores it, ends by the signal, and leaves the name free for the next run.
#[cfg(unix)]
#[test]
fn sigint_to_a_named_run_ends_the_vendor_and_what_it_started_and_frees_the_name() {
    use std::os::unix::process::ExitStatusExt as _;

    let vendor = Vendor::new();
    std::fs::write(&vendor.linger, "").expect("the linger marker");
    let name = format!("cli-signal-{}", std::process::id());
    let mut first = vendor.command(&name).spawn().expect("the binary runs");
    let leader = pid_in(&vendor.leader).expect("the stub wrote its pid");
    let background = pid_in(&vendor.background).expect("the stub's background job wrote its pid");
    assert_ne!(
        rustix::process::getpgid(rustix::process::Pid::from_raw(leader)),
        rustix::process::getpgid(None),
        "the vendor child leads a process group other than the test's"
    );

    rustix::process::kill_process(
        rustix::process::Pid::from_child(&first),
        rustix::process::Signal::INT,
    )
    .expect("SIGINT reaches metaharness");
    let status = exited(&mut first, 20).expect("metaharness ends after SIGINT");
    let mut both_gone = false;
    for _ in 0..500 {
        both_gone = gone(leader) && gone(background);
        if both_gone {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        both_gone,
        "after SIGINT the vendor {leader} is gone ({}) and the process it started {background} is \
         gone ({})",
        gone(leader),
        gone(background)
    );
    assert_eq!(
        status.signal(),
        Some(signal_hook::consts::SIGINT),
        "metaharness ends by the signal it was sent: {status:?}"
    );

    std::fs::remove_file(&vendor.linger).expect("the marker goes");
    std::fs::remove_file(&vendor.started).expect("the first run's start marker goes");
    let second = vendor.command(&name).output().expect("the binary runs");
    let said = format!(
        "{}{}",
        String::from_utf8_lossy(&second.stdout),
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        !said.contains(metaharness::SCRATCH_NAME_HELD) && vendor.started.exists(),
        "the name {name:?} is free again after the signalled run, and a second run under it \
         spawns its vendor: {said}"
    );
}
