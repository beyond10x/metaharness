//! Adversary pass 1 on `story:version-probe-under-load` (design amendment a65). Every vendor binary
//! is a script written into a temporary directory and read through the public `qualify`, which
//! probes with the launch's own `VERSION_PROBE_DEADLINE`; no model, no network and no credential is
//! involved. Each case bounds its own wall time well under 15 s.
#![cfg(unix)]

mod adversary_version_probe_pass1 {
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use metaharness::protocol::{HermeticMode, Kind};
    use metaharness::{Qualification, qualify};

    struct Fake {
        _dir: tempfile::TempDir,
        program: PathBuf,
        calls: PathBuf,
    }

    impl Fake {
        /// A `claude` script whose body after the call log is `body`.
        fn new(body: &str) -> Self {
            let dir = tempfile::tempdir().expect("a directory");
            let calls = dir.path().join("calls.log");
            let program = dir.path().join("claude");
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

        fn ran(&self) -> bool {
            std::fs::read_to_string(&self.calls).is_ok_and(|text| !text.is_empty())
        }
    }

    /// `qualify` on the fake, on a thread of its own, retrying the ETXTBSY transient of a freshly
    /// written script (it never ran then). `None` when it has not answered within `bound`.
    fn qualified_within(fake: Fake, bound: Duration) -> Option<(Qualification, Duration)> {
        let (send, receive) = std::sync::mpsc::channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            let program = fake.program.to_str().expect("a utf-8 path").to_owned();
            for _ in 0..20 {
                let found = qualify(Kind::Claude, &program, HermeticMode::Strict)
                    .expect("the fake resolves");
                if fake.ran() {
                    let _ = send.send(found);
                    return;
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        });
        receive
            .recv_timeout(bound)
            .ok()
            .map(|found| (found, started.elapsed()))
    }

    fn pinned() -> &'static str {
        metaharness_claude::PINNED_VERSIONS[0]
    }

    /// Acceptance 1 on the `qualify` path, not only on `vendor_version`: a pinned binary that
    /// answers after seven seconds is read and `--strict-version` would not refuse it.
    #[test]
    fn adv_qualify_admits_a_pinned_binary_that_answers_after_seven_seconds() {
        let fake = Fake::new(&format!(
            "/usr/bin/sleep 7\necho '{} (Claude Code)'",
            pinned()
        ));
        let (found, took) = qualified_within(fake, Duration::from_secs(14))
            .expect("qualify answered a 7 s binary within 14 s");
        assert_eq!(found.version.as_deref(), Some(pinned()), "after {took:?}");
        assert!(!found.strict_version_refuses, "{found:?}");
    }

    /// a65 point 2: "what it printed before exiting is already in the pipe, and output a leftover
    /// process writes later is not its answer." The binary prints a banner with no version and
    /// exits; a process it left behind prints a pinned version one second later, inside the drain
    /// grace. Pass 1 asserted that later output is not the binary's answer; the decision after it
    /// (story:version-probe-under-load, a65 point 2 as corrected) is that the pipe cannot tell the
    /// two apart and output inside the grace is read, so this case asserts the read version.
    #[test]
    fn adv_output_a_leftover_writes_after_the_binary_exited_is_not_its_answer() {
        let fake = Fake::new(&format!(
            "(/usr/bin/sleep 1; echo '{} (Claude Code)') &\necho 'Claude Code'\nexit 0",
            pinned()
        ));
        let (found, took) =
            qualified_within(fake, Duration::from_secs(12)).expect("qualify answered within 12 s");
        assert_eq!(
            found.version.as_deref(),
            Some(pinned()),
            "story:version-probe-under-load: decided after adversary pass 1 (a65 point 2 as \
             corrected): the pipe cannot tell a leftover's bytes from the binary's, so output that \
             arrives within the drain grace is read, as the base read to end of file; after \
             {took:?}"
        );
    }

    /// a65 point 3 keeps at most 64 KiB of stdout. A version token that straddles that cap must not
    /// be reported cut short: it is either the whole token or no version. 65532 spaces, then
    /// `2.1.304 (Claude Code)`: the cap falls after `2.1.`.
    #[test]
    fn adv_a_version_token_cut_by_the_64_kib_cap_is_not_reported_as_a_version() {
        let fake = Fake::new(&format!(
            "printf '%65532s' ''\necho '{} (Claude Code)'\nexit 0",
            pinned()
        ));
        let (found, _) =
            qualified_within(fake, Duration::from_secs(12)).expect("qualify answered within 12 s");
        assert!(
            found.version.is_none() || found.version.as_deref() == Some(pinned()),
            "a65 point 3: the 64 KiB cap reported a truncated token {:?} as the binary's version",
            found.version
        );
    }

    /// a65 points 2 and 3: a leftover that writes without end after the binary exited holds the
    /// probe for the drain grace (5 s), not for as long as it writes, and the binary's own answer
    /// is read. The leftover is bounded at 20 s by `timeout`, so a probe that never stops draining
    /// shows as an answer later than 12 s.
    #[test]
    fn adv_a_leftover_writing_without_end_holds_qualify_for_the_grace_only() {
        let fake = Fake::new(&format!(
            "echo '{} (Claude Code)'\n/usr/bin/timeout 20 /usr/bin/yes &\nexit 0",
            pinned()
        ));
        let (found, took) = qualified_within(fake, Duration::from_secs(12)).expect(
            "a65: qualify still draining a leftover's endless output 12 s after the binary exited",
        );
        assert_eq!(found.version.as_deref(), Some(pinned()), "after {took:?}");
    }

    /// a65 point 3: a binary that writes far past the cap before it exits is drained, not left
    /// blocked on a full pipe; it exits and is read promptly.
    #[test]
    fn adv_a_binary_writing_a_mebibyte_before_it_exits_is_not_left_blocked() {
        let fake = Fake::new(&format!(
            "/usr/bin/yes x | /usr/bin/head -c 1048576\necho '{} (Claude Code)'\nexit 0",
            pinned()
        ));
        let (found, took) = qualified_within(fake, Duration::from_secs(12))
            .expect("a binary writing 1 MiB was left blocked for 12 s");
        assert!(took < Duration::from_secs(5), "took {took:?}");
        assert_eq!(
            found.version, None,
            "the version follows 1 MiB of output, past the 64 KiB kept"
        );
    }

    /// End of file before exit: the binary prints its version, closes stdout and lives two seconds
    /// more. Its answer is what it printed.
    #[test]
    fn adv_a_binary_that_closes_stdout_before_it_exits_is_read() {
        let fake = Fake::new(&format!(
            "echo '{} (Claude Code)'\nexec 1>&-\n/usr/bin/sleep 2\nexit 0",
            pinned()
        ));
        let (found, took) =
            qualified_within(fake, Duration::from_secs(12)).expect("qualify answered within 12 s");
        assert_eq!(found.version.as_deref(), Some(pinned()), "after {took:?}");
        assert!(took < Duration::from_secs(5), "took {took:?}");
    }
}
