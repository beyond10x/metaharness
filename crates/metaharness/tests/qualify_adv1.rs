//! Adversarial cases for `story:qualify-a-binary-without-a-model-call`. Every vendor binary is a
//! script written into a temporary directory; no model, no network and no credential is involved.

mod qualify_adv1 {
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::{Component, Path, PathBuf};
    use std::time::{Duration, Instant};

    use metaharness::protocol::{HermeticMode, HermeticRow, Kind, Verdict};
    use metaharness::{Qualification, qualify};

    struct Fake {
        _dir: tempfile::TempDir,
        program: PathBuf,
        calls: PathBuf,
    }

    impl Fake {
        /// A script named `name` whose body after the call log is `body`.
        fn new(name: &str, body: &str) -> Self {
            Self::in_dir(tempfile::tempdir().expect("a directory"), name, body)
        }

        /// As `new`, in an already made directory.
        fn in_dir(dir: tempfile::TempDir, name: &str, body: &str) -> Self {
            let calls = dir.path().join("calls.log");
            let program = dir.path().join(name);
            std::fs::write(
                &program,
                format!("#!/bin/sh\necho \"$*\" >> '{}'\n{body}\n", calls.display()),
            )
            .expect("the script");
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
                .expect("executable");
            Self {
                _dir: dir,
                program,
                calls,
            }
        }

        fn program(&self) -> &str {
            self.program.to_str().expect("a utf-8 path")
        }

        fn calls(&self) -> Vec<String> {
            std::fs::read_to_string(&self.calls)
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    /// Qualify, retrying the ETXTBSY transient of a freshly written script (it never ran then).
    fn qualified(kind: Kind, fake: &Fake, program: &str, hermetic: HermeticMode) -> Qualification {
        let mut last = None;
        for _ in 0..20 {
            let found = qualify(kind, program, hermetic).expect("the fake resolves");
            if !fake.calls().is_empty() {
                return found;
            }
            last = Some(found);
            std::thread::sleep(Duration::from_millis(25));
        }
        panic!("the fake never ran: {last:?}");
    }

    fn verdict(found: &Qualification, which: HermeticRow) -> Verdict {
        found
            .rows
            .iter()
            .find(|row| row.row.row == which)
            .unwrap_or_else(|| panic!("{} is not in {:?}", which.id(), found.rows))
            .row
            .verdict
    }

    /// `qualify`'s doc: "It is run once, as `program --version`, ... within a deadline". The
    /// deadline is five seconds. A `--version` that exits at once but leaves a background child
    /// holding its stdout (an updater, a daemon a wrapper starts) must not hold `qualify` past it.
    #[test]
    fn a_version_whose_child_keeps_stdout_open_still_meets_the_deadline() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        let fake = Fake::new(
            "claude",
            &format!("echo '{pinned} (Claude Code)'\nsleep 25 &\nexit 0"),
        );
        let program = fake.program().to_owned();
        let (send, receive) = std::sync::mpsc::channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            let _ = send.send(qualify(Kind::Claude, &program, HermeticMode::Strict));
        });
        let answer = receive.recv_timeout(Duration::from_secs(12));
        assert!(
            answer.is_ok(),
            "qualify did not return within 12 s (deadline 5 s); still blocked after {:?}",
            started.elapsed()
        );
    }

    /// A trailing suffix on a pinned version is not the pinned version.
    #[test]
    fn a_pinned_version_with_trailing_garbage_is_off_the_pin() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        let fake = Fake::new("claude", &format!("echo '{pinned}-patched (Claude Code)'"));
        let found = qualified(Kind::Claude, &fake, fake.program(), HermeticMode::Strict);
        assert_eq!(found.version, Some(format!("{pinned}-patched")));
        assert_eq!(verdict(&found, HermeticRow::H9), Verdict::Gap);
        assert_ne!(verdict(&found, HermeticRow::H11), Verdict::Ok);
        assert!(found.strict_version_refuses);
        assert!(!found.qualified());
    }

    /// A pinned version printed only on stderr is no version: the probe reads stdout alone.
    #[test]
    fn a_version_on_stderr_only_is_unread() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        let fake = Fake::new("claude", &format!("echo '{pinned} (Claude Code)' >&2"));
        let found = qualified(Kind::Claude, &fake, fake.program(), HermeticMode::Strict);
        assert_eq!(found.version, None);
        assert!(!found.qualified());
        assert!(found.strict_version_refuses);
    }

    /// A pinned version followed by a non-zero exit is not a qualified binary.
    #[test]
    fn a_failing_version_call_is_unread_even_when_it_printed_a_pin() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        let fake = Fake::new("claude", &format!("echo '{pinned} (Claude Code)'\nexit 3"));
        let found = qualified(Kind::Claude, &fake, fake.program(), HermeticMode::Strict);
        assert_eq!(found.version, None);
        assert!(!found.qualified());
    }

    /// `qualify` must not hand the binary the caller's stdin: a `--version` that reads stdin sees
    /// end of file at once, not a hang on the test harness's stdin.
    #[test]
    fn the_probe_reads_no_stdin() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        let fake = Fake::new(
            "claude",
            &format!(
                "if read line; then echo \"stdin:$line\" >> \"$0.stdin\"; fi\necho '{pinned} (Claude Code)'"
            ),
        );
        let found = qualified(Kind::Claude, &fake, fake.program(), HermeticMode::Strict);
        assert!(found.qualified(), "{:?}", found.rows);
        assert!(!Path::new(&format!("{}.stdin", fake.program())).exists());
    }

    /// Under `Off` and `On` nothing gates, under `Strict` the unmeasured H11 does, on the same
    /// binary; Codex never gates on its version under any mode.
    #[test]
    fn modes_and_kinds() {
        let claude = Fake::new("claude", "echo '2.1.311 (Claude Code)'");
        for (mode, gates) in [
            (HermeticMode::Off, false),
            (HermeticMode::On, false),
            (HermeticMode::Strict, true),
        ] {
            let found = qualified(Kind::Claude, &claude, claude.program(), mode);
            assert_eq!(found.qualified(), !gates, "{mode:?}: {:?}", found.rows);
        }
        let codex = Fake::new("codex", "echo 'codex-cli 0.0.1'");
        for mode in [HermeticMode::Off, HermeticMode::On, HermeticMode::Strict] {
            let found = qualified(Kind::Codex, &codex, codex.program(), mode);
            assert!(found.qualified(), "{mode:?}: {:?}", found.rows);
            assert!(found.strict_version_refuses);
            assert_eq!(found.rows.len(), 1, "{:?}", found.rows);
        }
    }

    /// The path from the current directory to `target`, through `..` up to their common ancestor.
    fn relative_from_cwd(target: &Path) -> PathBuf {
        let cwd = std::env::current_dir().expect("a cwd");
        let from: Vec<Component> = cwd.components().collect();
        let to: Vec<Component> = target.components().collect();
        let common = from
            .iter()
            .zip(&to)
            .take_while(|(left, right)| left == right)
            .count();
        let mut relative = PathBuf::new();
        for _ in common..from.len() {
            relative.push("..");
        }
        for component in &to[common..] {
            relative.push(component.as_os_str());
        }
        relative
    }

    /// `qualify`'s doc: an absolute path is that path; a relative one, with or without a `/`, is
    /// looked up under each child `PATH` entry, because the child does not run in this directory.
    /// A relative path that names an executable only from the current directory is refused.
    #[test]
    fn a_relative_path_with_a_slash_is_looked_up_on_the_child_path() {
        let pinned = metaharness_claude::PINNED_VERSIONS[0];
        // Under the target directory, shallow from the cwd, so no child PATH entry reaches it by a
        // chain of `..` whatever its depth.
        let fake = Fake::in_dir(
            tempfile::tempdir_in(env!("CARGO_TARGET_TMPDIR")).expect("a directory"),
            "claude",
            &format!("echo '{pinned} (Claude Code)'"),
        );
        let relative = relative_from_cwd(&fake.program);
        let relative = relative.to_str().expect("utf-8").to_owned();
        assert!(
            relative.contains('/') && !relative.starts_with('/'),
            "{relative}"
        );
        assert!(
            Path::new(&relative).is_file(),
            "{relative} names the fake from the cwd"
        );
        let found = qualify(Kind::Claude, &relative, HermeticMode::Strict);
        assert!(
            matches!(found, Err(metaharness::Refusal::Io { .. })),
            "story:qualify-a-binary-without-a-model-call: a relative path is resolved on the child PATH, not from the cwd: {found:?}"
        );
    }
}
