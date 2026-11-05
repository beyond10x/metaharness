//! Adversary pass 1 on `story:aep-version-reads-bounded` and `story:doctor-run-help-probe-bounded`
//! (design amendment a65 point 4): the public `metaharness::probe_command` that every `--version`
//! and `run --help` read in this workspace now goes through.
//!
//! Every child is `/bin/sh -c <body>`, so no freshly written script can fail to start with
//! `ETXTBSY`. No model, no network and no credential is involved; each case bounds its own wall
//! time on a thread of its own, so a read that never returns fails the case instead of hanging it.
#![cfg(unix)]

mod adversary_bounded_probes_pass1 {
    use std::process::Command;
    use std::time::{Duration, Instant};

    use metaharness::{ProbeRead, VERSION_DRAIN_GRACE, probe_command};

    /// `sh -c body`, probed within `deadline` on a thread of its own; `None` when it has not
    /// answered within `bound`.
    fn probed_within(
        body: &str,
        env: &[(&str, &str)],
        deadline: Duration,
        bound: Duration,
    ) -> Option<(ProbeRead, String, Duration)> {
        let body = body.to_owned();
        let env: Vec<(String, String)> = env
            .iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect();
        let (send, receive) = std::sync::mpsc::channel();
        let started = Instant::now();
        std::thread::spawn(move || {
            let mut command = Command::new("/bin/sh");
            command.arg("-c").arg(&body);
            for (key, value) in &env {
                command.env(key, value);
            }
            let read = probe_command(&mut command, deadline);
            let refusal = read.refusal(&command).unwrap_or_default();
            let _ = send.send((read, refusal));
        });
        receive
            .recv_timeout(bound)
            .ok()
            .map(|(read, refusal)| (read, refusal, started.elapsed()))
    }

    /// This process's peak resident set, in KiB, from `/proc/self/status`.
    fn peak_resident_kib() -> u64 {
        let status = std::fs::read_to_string("/proc/self/status").expect("/proc/self/status");
        status
            .lines()
            .find_map(|line| line.strip_prefix("VmHWM:"))
            .and_then(|rest| rest.trim().trim_end_matches("kB").trim().parse().ok())
            .expect("a VmHWM line")
    }

    /// The `VERSION_STDOUT_MAX` contract (a65): "the rest is read and discarded, so a child that
    /// writes without end is drained rather than left blocked, and holds no unbounded buffer". A
    /// child that writes without end and never exits is read until the deadline; whatever reads its
    /// pipe must not grow this process by more than a small multiple of the 64 KiB it keeps.
    #[test]
    fn adv_a_child_writing_without_end_until_the_deadline_holds_no_unbounded_buffer() {
        let before = peak_resident_kib();
        let deadline = Duration::from_secs(4);
        let (read, refusal, elapsed) =
            probed_within("exec cat /dev/zero", &[], deadline, Duration::from_secs(30))
                .expect("a child writing without end held the probe past its bound");
        let after = peak_resident_kib();
        assert_eq!(read, ProbeRead::TimedOut(deadline), "{refusal}");
        let grown = after.saturating_sub(before);
        assert!(
            grown < 256 * 1024,
            "a65: the probe holds no unbounded buffer, and this process's peak resident set grew \
             by {grown} KiB ({before} -> {after}) while a child wrote without end for {elapsed:?}"
        );
    }

    /// Boundary of the 64 KiB cap doctor now refuses on: exactly 65,536 bytes are kept whole and
    /// not marked cut; one byte more is marked cut.
    #[test]
    fn adv_the_cap_keeps_exactly_64_kib_whole_and_marks_one_byte_more_as_cut() {
        for (bytes, cut) in [(65_536_usize, false), (65_537, true)] {
            let body = format!("head -c {bytes} /dev/zero | tr '\\000' 'x'; exit 0");
            let (read, refusal, elapsed) =
                probed_within(&body, &[], Duration::from_secs(20), Duration::from_secs(25))
                    .expect("a help of the cap's size did not return");
            let ProbeRead::Exited {
                code,
                stdout,
                capped,
                ..
            } = read
            else {
                panic!("{bytes} bytes: {read:?} {refusal} (after {elapsed:?})");
            };
            assert_eq!(code, Some(0), "{bytes} bytes");
            assert_eq!(capped, cut, "{bytes} bytes: capped");
            assert_eq!(stdout.len(), 65_536, "{bytes} bytes: kept");
        }
    }

    /// A child that closes its stdout and stays alive gives the probe end of file and no exit: it
    /// is still killed at the deadline, not waited on for good.
    #[test]
    fn adv_a_child_that_closes_stdout_and_stays_alive_is_killed_at_the_deadline() {
        let deadline = Duration::from_millis(500);
        let (read, refusal, elapsed) = probed_within(
            "exec >&- ; exec sleep 30",
            &[],
            deadline,
            Duration::from_secs(10),
        )
        .expect("a child with stdout closed held the probe");
        assert_eq!(read, ProbeRead::TimedOut(deadline), "{refusal}");
        assert!(
            refusal.contains("did not exit within 500 ms and was killed"),
            "{refusal}"
        );
        assert!(elapsed >= deadline, "answered after {elapsed:?}");
    }

    /// A child that fills stderr and never exits is killed at the deadline: a full stderr pipe
    /// does not hold the probe.
    #[test]
    fn adv_a_child_flooding_stderr_and_never_exiting_is_killed_at_the_deadline() {
        let deadline = Duration::from_millis(500);
        let (read, refusal, elapsed) = probed_within(
            "exec cat /dev/zero >&2",
            &[],
            deadline,
            Duration::from_secs(10),
        )
        .expect("a child flooding stderr held the probe");
        assert_eq!(read, ProbeRead::TimedOut(deadline), "{refusal}");
        assert!(elapsed >= deadline, "answered after {elapsed:?}");
    }

    /// The killed child's grandchild keeps both pipes open after the kill: the probe answers at
    /// the deadline and does not wait for the grandchild.
    #[test]
    fn adv_a_grandchild_holding_both_pipes_after_the_kill_does_not_hold_the_probe() {
        let deadline = Duration::from_millis(500);
        let (read, refusal, elapsed) =
            probed_within("sleep 20; exit 0", &[], deadline, Duration::from_secs(10))
                .expect("a grandchild holding the pipes held the probe");
        assert_eq!(read, ProbeRead::TimedOut(deadline), "{refusal}");
        assert!(
            elapsed < deadline + Duration::from_secs(3),
            "answered after {elapsed:?}"
        );
    }

    /// No refusal carries an environment value the caller gave the child: not a failure's, not a
    /// kill's, not an unrunnable program's.
    #[test]
    fn adv_no_refusal_names_an_environment_value() {
        let secret = "adv-secret-value-6d1f";
        for (body, deadline) in [
            ("exit 3", Duration::from_secs(20)),
            ("exec sleep 30", Duration::from_millis(300)),
        ] {
            let (read, refusal, _) = probed_within(
                body,
                &[("ADV_PROBE_SECRET", secret)],
                deadline,
                Duration::from_secs(25),
            )
            .expect("the probe answered");
            assert!(!read.succeeded(), "{body}: {read:?}");
            assert!(!refusal.is_empty(), "{body}: a refusal");
            assert!(!refusal.contains(secret), "{body}: {refusal}");
        }
        let mut command = Command::new("/nonexistent/adv-probe");
        command.arg("--version").env("ADV_PROBE_SECRET", secret);
        let read = probe_command(&mut command, Duration::from_secs(5));
        let refusal = read
            .refusal(&command)
            .expect("an unrunnable program is refused");
        assert!(
            refusal.starts_with("/nonexistent/adv-probe --version could not be run: "),
            "{refusal}"
        );
        assert!(!refusal.contains(secret), "{refusal}");
    }

    /// No regression for a fast binary: it prints its version and exits zero, is read whole, is
    /// not refused, and costs none of the drain grace.
    #[test]
    fn adv_a_fast_binary_is_read_whole_and_not_held_for_the_grace() {
        let (read, refusal, elapsed) = probed_within(
            "echo 'aep 0.79.0'",
            &[],
            Duration::from_secs(20),
            Duration::from_secs(25),
        )
        .expect("a fast binary held the probe");
        assert_eq!(
            read,
            ProbeRead::Exited {
                code: Some(0),
                stdout: b"aep 0.79.0\n".to_vec(),
                capped: false,
                stderr: String::new(),
            }
        );
        assert_eq!(refusal, "");
        assert!(
            elapsed < VERSION_DRAIN_GRACE,
            "a binary that exited at once held the probe for {elapsed:?}"
        );
    }
}
