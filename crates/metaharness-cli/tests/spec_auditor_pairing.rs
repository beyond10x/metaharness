//! `run`'s `--spec`/`--auditor` pairing through the built binary (design amendment a43).
//!
//! The pairing is checked before the spawn, so each case below exits `2` with no harness process
//! started. What shows that no process started is a fake harness: `claude` in the scratch
//! `$HOME/.local/bin`, which is first on the `PATH` metaharness constructs for its child, and
//! which writes a marker file when it is launched. A fake auditor named by absolute path writes a
//! second marker. A case passes only when both markers are absent.
//!
//! Nothing real can be reached. `HOME`, `TMPDIR` and the process's own `PATH` are scratch
//! directories, the run copies no credential (`--credentials none`), and the fake answers the
//! launch by writing its marker and exiting. Only `claude` is faked: it is the kind whose child
//! `PATH` puts `$HOME/.local/bin` ahead of every system directory, so the fake is what a launch
//! finds.
#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};

use metaharness::Refusal;
use std::process::{Command, Stdio};

/// One scratch world: a home with the fake harness in it, the fake auditor, and the two markers.
struct World {
    base: tempfile::TempDir,
    home: PathBuf,
    path: PathBuf,
    tmp: PathBuf,
    auditor: PathBuf,
    harness_marker: PathBuf,
    auditor_marker: PathBuf,
    document: PathBuf,
}

struct Outcome {
    code: i32,
    stderr: String,
}

/// A shell script that writes its marker and then does what `rest` says.
fn script(marker: &Path, rest: &str) -> String {
    let marker = marker.to_str().expect("a UTF-8 scratch path");
    assert!(!marker.contains('\''), "the marker path quotes as is");
    format!("#!/bin/sh\n: > '{marker}'\n{rest}")
}

fn executable(path: &Path, text: &str) {
    std::fs::write(path, text).expect("written");
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700)).expect("executable");
}

fn world() -> World {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    let path = base.path().join("path");
    let tmp = base.path().join("tmp");
    let bin = home.join(".local/bin");
    for directory in [&home, &path, &tmp, &bin] {
        std::fs::create_dir_all(directory).expect("a scratch directory");
    }
    let harness_marker = base.path().join("harness-launched");
    let auditor_marker = base.path().join("auditor-launched");
    executable(&bin.join("claude"), &script(&harness_marker, "exit 0\n"));
    let auditor = base.path().join("auditor");
    executable(
        &auditor,
        &script(&auditor_marker, "echo 'one ok'\nexit 0\n"),
    );
    let document = base.path().join("expectations.yaml");
    std::fs::write(&document, "kinds: []\n").expect("written");
    World {
        base,
        home,
        path,
        tmp,
        auditor,
        harness_marker,
        auditor_marker,
        document,
    }
}

impl World {
    fn auditor(&self) -> &str {
        self.auditor.to_str().expect("a UTF-8 scratch path")
    }

    fn document(&self) -> &str {
        self.document.to_str().expect("a UTF-8 scratch path")
    }

    /// `metaharness run claude --prompt go --credentials none <extra…>`.
    fn run(&self, extra: &[&str]) -> Outcome {
        self.run_os(&os(extra))
    }

    /// The same, with arguments that need not be UTF-8.
    fn run_os(&self, extra: &[&OsStr]) -> Outcome {
        let mut args: Vec<&OsStr> = ["run", "claude", "--prompt", "go", "--credentials", "none"]
            .into_iter()
            .map(OsStr::new)
            .collect();
        args.extend_from_slice(extra);
        let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
            .args(&args)
            .env_clear()
            .env("HOME", &self.home)
            .env("PATH", &self.path)
            .env("TMPDIR", &self.tmp)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .expect("the binary runs");
        Outcome {
            code: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        }
    }

    fn harness_launched(&self) -> bool {
        self.harness_marker.exists()
    }

    fn auditor_launched(&self) -> bool {
        self.auditor_marker.exists()
    }

    /// Exit `2`, the refusal's words first on stderr, and neither fake launched.
    fn refused_before_the_spawn(&self, extra: &[&str], words: &str) {
        let outcome = self.run(extra);
        assert!(
            !self.harness_launched(),
            "{extra:?}: the harness was launched; stderr: {}",
            outcome.stderr
        );
        assert!(
            !self.auditor_launched(),
            "{extra:?}: the auditor was launched"
        );
        assert_eq!(outcome.code, 2, "{extra:?}: stderr: {}", outcome.stderr);
        assert!(
            outcome.stderr.starts_with(&format!("metaharness: {words}")),
            "{extra:?}: stderr: {}",
            outcome.stderr
        );
    }

    /// Exit `2`, stderr exactly the library's own refusal, and neither fake launched.
    fn refused_by_name(&self, extra: &[&OsStr], refusal: &Refusal) {
        let outcome = self.run_os(extra);
        assert!(
            !self.harness_launched(),
            "{extra:?}: the harness was launched; stderr: {}",
            outcome.stderr
        );
        assert!(
            !self.auditor_launched(),
            "{extra:?}: the auditor was launched"
        );
        assert_eq!(outcome.code, 2, "{extra:?}: stderr: {}", outcome.stderr);
        assert_eq!(
            outcome.stderr,
            format!("metaharness: {refusal}\n"),
            "{extra:?}"
        );
    }
}

/// What `start` says about an auditor program it cannot start (design amendment a43, row 7).
fn not_invokable(detail: String) -> Refusal {
    Refusal::AuditorNotInvokable {
        argv: Vec::new(),
        detail,
    }
}

/// These words as arguments.
fn os<'a>(words: &[&'a str]) -> Vec<&'a OsStr> {
    words.iter().map(|word| OsStr::new(*word)).collect()
}

#[test]
fn a_spec_without_an_auditor_exits_2_before_the_harness_is_launched() {
    for audit in [&[][..], &["--audit"][..], &["--hermetic", "strict"][..]] {
        let world = world();
        let mut extra = vec!["--spec", world.document()];
        extra.extend_from_slice(audit);
        world.refused_before_the_spawn(&extra, "--spec was given without --auditor");
    }
}

#[test]
fn an_auditor_without_a_spec_exits_2_before_the_harness_is_launched() {
    for audit in [&[][..], &["--audit"][..], &["--hermetic", "strict"][..]] {
        let world = world();
        let mut extra = vec!["--auditor", world.auditor()];
        extra.extend_from_slice(audit);
        world.refused_before_the_spawn(&extra, "--auditor was given without --spec");
    }
}

#[test]
fn a_spec_and_an_auditor_without_an_audit_exit_2_before_the_harness_is_launched() {
    for hermetic in [&[][..], &["--hermetic", "on"][..]] {
        let world = world();
        let mut extra = vec!["--spec", world.document(), "--auditor", world.auditor()];
        extra.extend_from_slice(hermetic);
        world.refused_before_the_spawn(
            &extra,
            "the adapter refused to plan the launch: --spec and --auditor were given on a run \
             that wants no audit",
        );
    }
}

#[test]
fn an_expectation_document_that_is_not_a_readable_regular_file_exits_2_before_the_harness_is_launched()
 {
    let world = world();
    let missing = world.home.join("no-such-expectations.yaml");
    let missing = missing.to_str().expect("a UTF-8 scratch path");
    world.refused_before_the_spawn(
        &["--audit", "--spec", missing, "--auditor", world.auditor()],
        &format!("the expectation document {missing} could not be read: "),
    );

    let world = self::world();
    let directory = world.home.to_str().expect("a UTF-8 scratch path");
    world.refused_before_the_spawn(
        &["--audit", "--spec", directory, "--auditor", world.auditor()],
        &format!(
            "the expectation document {directory} could not be read: it is not a regular file"
        ),
    );
}

#[test]
fn an_empty_auditor_prefix_exits_2_before_the_harness_is_launched() {
    for prefix in ["", "   "] {
        let world = world();
        world.refused_before_the_spawn(
            &["--audit", "--spec", world.document(), "--auditor", prefix],
            "the auditor could not be invoked ([]): the auditor prefix is empty",
        );
    }
}

#[test]
fn auditor_arguments_without_an_auditor_exit_2_before_the_harness_is_launched() {
    for audit in [&[][..], &["--audit"][..], &["--hermetic", "strict"][..]] {
        let world = world();
        let mut extra = audit.to_vec();
        extra.extend_from_slice(&["--", "--spec", world.document()]);
        world.refused_by_name(&os(&extra), &Refusal::AuditorArgsWithoutAuditor);
    }
}

#[test]
fn an_expectation_document_whose_path_is_not_utf8_exits_2_before_the_harness_is_launched() {
    let world = world();
    let document = world
        .base
        .path()
        .join(<OsStr as std::os::unix::ffi::OsStrExt>::from_bytes(
            b"expectations-\xff.yaml",
        ));
    std::fs::write(&document, "kinds: []\n").expect("written");
    let refusal = metaharness::run_auditor(
        Some(&document),
        Some(world.auditor()),
        &[],
        Path::new("t.jsonl"),
        &mut metaharness::FakeAuditor::default(),
    )
    .expect_err("the library refuses the path");
    assert!(
        matches!(&refusal, Refusal::SpecUnreadable { detail, .. }
            if detail.contains(r"expectations-\xff.yaml is not UTF-8")),
        "{refusal:?}"
    );
    let mut extra = os(&["--audit", "--spec"]);
    extra.push(document.as_os_str());
    extra.extend(os(&["--auditor", world.auditor()]));
    world.refused_by_name(&extra, &refusal);
}

#[test]
fn an_auditor_whose_program_cannot_be_started_exits_2_before_the_harness_is_launched() {
    let world = world();
    let base = world.base.path().to_str().expect("a UTF-8 scratch path");
    let nothing = format!("{base}/no-such-auditor");
    let not_executable = format!("{base}/not-executable");
    std::fs::write(&not_executable, "#!/bin/sh\n").expect("written");
    std::fs::set_permissions(&not_executable, std::fs::Permissions::from_mode(0o644))
        .expect("not executable");
    let missing = std::fs::metadata(&nothing).expect_err("nothing is there");
    let cases = [
        (
            format!("{nothing} check"),
            format!("the auditor program {nothing:?} cannot be found: {missing}"),
        ),
        (
            base.to_string(),
            format!("the auditor program {base:?} is not a regular file"),
        ),
        (
            not_executable.clone(),
            format!("the auditor program {not_executable:?} has no execute bit"),
        ),
        (
            "no-such-auditor check".to_string(),
            "the auditor program \"no-such-auditor\" is not an executable regular file in any \
             directory of PATH"
                .to_string(),
        ),
        (
            "--spec".to_string(),
            "the auditor prefix starts with \"--spec\", which reads as an option and names no \
             program"
                .to_string(),
        ),
    ];
    for (prefix, detail) in cases {
        let world = self::world();
        let auditor = format!("--auditor={prefix}");
        world.refused_by_name(
            &os(&["--audit", "--spec", world.document(), &auditor]),
            &not_invokable(detail),
        );
    }
}

/// The control: the audited pairing launches the harness and then the auditor, and exits with
/// the code it had before amendment a43. The fake harness writes no record, so the floor finds
/// nothing out and the run is `3`, the code § 9.4 gives a run nobody could judge.
#[test]
fn an_audited_spec_and_auditor_launch_the_harness_and_the_auditor_as_before() {
    let world = world();
    let outcome = world.run(&[
        "--audit",
        "--spec",
        world.document(),
        "--auditor",
        world.auditor(),
    ]);
    assert!(world.harness_launched(), "stderr: {}", outcome.stderr);
    assert!(world.auditor_launched(), "stderr: {}", outcome.stderr);
    assert_eq!(outcome.code, 3, "stderr: {}", outcome.stderr);
}

/// The second control: an auditor named by a bare word is found on the `PATH` the binary runs
/// with, as the process that later starts it finds it, and both are launched as before.
#[test]
fn an_auditor_named_by_a_bare_word_on_the_path_is_launched() {
    let world = world();
    executable(
        &world.path.join("pairing-auditor"),
        &script(&world.auditor_marker, "echo 'one ok'\nexit 0\n"),
    );
    let outcome = world.run(&[
        "--audit",
        "--spec",
        world.document(),
        "--auditor",
        "pairing-auditor check",
    ]);
    assert!(world.harness_launched(), "stderr: {}", outcome.stderr);
    assert!(world.auditor_launched(), "stderr: {}", outcome.stderr);
    assert_eq!(outcome.code, 3, "stderr: {}", outcome.stderr);
}
