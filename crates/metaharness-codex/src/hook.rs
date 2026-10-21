//! The `PreToolUse` program this adapter installs, and the channel it answers over.
//!
//! It is the Claude adapter's program in shape and **not** its program in fact: the two vendors'
//! hook wires are near-identical (§ 2.5, and the research record's *"same decision contract shape
//! as Claude Code"*), and a single shared program would be one file that has to stay true of two
//! binaries nobody synchronises. Everything vendor-specific lives in the vendor's own adapter
//! crate, so the duplication is the rule working rather than the rule leaking.
//!
//! # Why the program parses no JSON
//!
//! A hook is a separate process with a shell's tools, and a guard that has to parse JSON with
//! `sed` is a guard that stops guarding the first time a field moves. So the program **publishes
//! its whole stdin verbatim** under a name only it holds, and metaharness does the parsing in
//! Rust.
//!
//! # What a response is routed by, and what a call is correlated by
//!
//! **A response is routed by the hook process's own rendezvous name.** The vendor's payload does
//! carry a per-call id — `tool_use_id`, required by the schema 0.145.0 embeds, the same spelling
//! Claude Code uses (row V22) — and the adapter presents the call under it so a reader can join
//! the live call to the rollout's own record of it. But the *response* travels on the name this
//! process minted for itself, because that is the one thing that is true even of a payload that
//! arrives with a field missing, and a decision delivered to the wrong blocked process would be
//! worse than one delivered to none.
//!
//! # Fail closed, three ways
//!
//! With no channel, no writable request directory, or no answer inside its own budget, the
//! program **denies with a reason**. It never exits non-zero to signal trouble: a non-zero exit is
//! a vendor-interpreted signal (on this wire, `exit 2` + stderr is itself a block), while a `deny`
//! with a reason is a decision the model is told about.

use std::path::{Path, PathBuf};

use metaharness_protocol::Decision;

use crate::HOOK_TIMEOUT_SECONDS;
use crate::seam::render_hook_response;

/// How long the installed program waits for metaharness before it denies on its own.
///
/// Between metaharness's own deadline and the vendor's timeout, and that ordering is the whole
/// point: metaharness's deadline (design § 7.7 rule 2) expires **first**, so a slow decision
/// becomes a refusal metaharness owns and explains. This budget is the backstop for the case that
/// rule cannot cover — metaharness itself being gone.
pub const HOOK_WAIT_SECONDS: u64 = HOOK_TIMEOUT_SECONDS - 3;

/// The directory under the scratch root that holds the decision channel.
const CHANNEL_DIR: &str = "hook-channel";

/// Where hook processes publish what the vendor handed them.
const REQUESTS_DIR: &str = "requests";

/// Where metaharness writes the answer.
const RESPONSES_DIR: &str = "responses";

/// The decision channel's three paths, derived from one scratch root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookChannelPaths {
    /// The channel root.
    pub root: PathBuf,
    /// Where hook processes publish their stdin.
    pub requests: PathBuf,
    /// Where metaharness writes hook responses.
    pub responses: PathBuf,
}

impl HookChannelPaths {
    /// The channel for this scratch root.
    #[must_use]
    pub fn under(scratch_root: &Path) -> Self {
        Self::at_root(&scratch_root.join(CHANNEL_DIR))
    }

    /// The channel whose root is already known.
    ///
    /// The two halves are derived from the root in **one** place, because a channel whose two
    /// ends disagree by one directory does not fail loudly — it simply never carries a decision.
    #[must_use]
    pub fn at_root(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            requests: root.join(REQUESTS_DIR),
            responses: root.join(RESPONSES_DIR),
        }
    }
}

/// The reason the installed program gives when it never heard back.
fn unanswered_reason() -> String {
    format!(
        "metaharness did not answer this call within {HOOK_WAIT_SECONDS}s and the seam fails \
         closed, so the call did not run; this is the backstop inside the hook program itself, \
         not a decision anybody made about the call"
    )
}

/// The reason the installed program gives when it cannot reach its channel at all.
fn no_channel_reason() -> String {
    "metaharness could not be reached over its decision channel, so the call did not run; a \
     guard that cannot be consulted refuses rather than passing the call through"
        .to_string()
}

/// One deny body, compact, as the program will print it.
fn deny_body(reason: String) -> String {
    render_hook_response(&Decision::Deny { reason }).to_string()
}

/// A path that is not UTF-8, and so has no faithful rendering in a program or in `config.toml`
/// (design amendment a37).
///
/// Refused rather than rendered through its display form, which replaces the offending bytes and
/// so names a directory that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathNotUtf8 {
    /// The path as it was handed over.
    pub path: PathBuf,
}

impl PathNotUtf8 {
    /// The path's bytes through Rust's `escape_ascii`: printable ASCII as written except `'`, `"`
    /// and `\`, which carry a backslash; `\t`, `\r`, `\n` as those escapes; every other byte as
    /// `\xNN` (`\xff`). The one form of it that names the directory unambiguously, where the
    /// display form would replace the bytes that are not UTF-8.
    #[must_use]
    pub fn escaped(&self) -> String {
        self.path
            .as_os_str()
            .as_encoded_bytes()
            .escape_ascii()
            .to_string()
    }
}

impl std::fmt::Display for PathNotUtf8 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the path {} is not UTF-8, so it has no faithful rendering in the hook program or \
             config.toml (design amendment a37)",
            self.escaped()
        )
    }
}

impl std::error::Error for PathNotUtf8 {}

/// One POSIX shell word: the value between single quotes, each `'` written `'\''`.
///
/// Inside single quotes the shell interprets no byte at all, so a `"`, `$`, backtick, backslash,
/// space or newline arrives as written (design amendment a37). The Claude adapter carries its own
/// copy: vendor rendering lives in the vendor's crate.
#[must_use]
pub fn shell_quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', r"'\''"))
}

/// A path as one POSIX shell word, or the refusal for a path that is not UTF-8.
///
/// # Errors
///
/// [`PathNotUtf8`] when the path is not UTF-8.
pub fn shell_quote_path(path: &Path) -> Result<String, PathNotUtf8> {
    path.to_str().map(shell_quote).ok_or_else(|| PathNotUtf8 {
        path: path.to_path_buf(),
    })
}

/// The `PreToolUse` program, as text.
///
/// POSIX `sh` and nothing the child's reduced `PATH` does not already carry: `mktemp`, `cat`,
/// `mv`, `sleep`, `printf`. No `jq`, no `python3`, no `node` — the way to never need the
/// "interpreter is missing" branch is to need no interpreter.
///
/// The two channel paths are written as single-quoted shell words (amendment a37), so a scratch
/// root carrying a quote, a parameter or a newline still yields a program that parses.
///
/// # Errors
///
/// [`PathNotUtf8`] when either channel path is not UTF-8.
pub fn hook_program(channel: &HookChannelPaths) -> Result<String, PathNotUtf8> {
    let requests = shell_quote_path(&channel.requests)?;
    let responses = shell_quote_path(&channel.responses)?;
    let ticks = HOOK_WAIT_SECONDS * 10;
    let unanswered = deny_body(unanswered_reason());
    let no_channel = deny_body(no_channel_reason());
    Ok(format!(
        r#"#!/bin/sh
# metaharness PreToolUse seam for codex. Generated per run; edited by nobody.
#
# It publishes the vendor's hook input verbatim and waits for metaharness to answer it. It parses
# no JSON: metaharness does that in Rust, and routes by the rendezvous name this process minted.
set -u

REQUESTS={requests}
RESPONSES={responses}
TICKS={ticks}

fail_closed() {{
    printf '%s\n' "$1"
    exit 0
}}

umask 077

tmp=$(mktemp "$REQUESTS/.pending.XXXXXXXX" 2>/dev/null) || fail_closed '{no_channel}'
key=${{tmp##*/.pending.}}

cat > "$tmp" || fail_closed '{no_channel}'
# Published under one rename, so metaharness never reads a half-written request.
mv -f "$tmp" "$REQUESTS/$key.json" || fail_closed '{no_channel}'

tick=0
while [ "$tick" -lt "$TICKS" ]; do
    if [ -e "$RESPONSES/$key.json" ]; then
        # An empty file is metaharness abstaining: no bytes, no permissionDecision, and the
        # vendor's own approval policy decides. That is not the same as allowing.
        cat "$RESPONSES/$key.json"
        exit 0
    fi
    sleep 0.1 2>/dev/null || sleep 1
    tick=$((tick + 1))
done

fail_closed '{unanswered}'
"#
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> HookChannelPaths {
        HookChannelPaths::under(Path::new("/scratch/run-1"))
    }

    #[test]
    fn the_channel_hangs_off_the_scratch_root_and_not_off_the_codex_home() {
        let paths = channel();
        assert_eq!(paths.root, Path::new("/scratch/run-1/hook-channel"));
        assert_eq!(
            paths.requests,
            Path::new("/scratch/run-1/hook-channel/requests")
        );
        assert_eq!(
            paths.responses,
            Path::new("/scratch/run-1/hook-channel/responses")
        );
    }

    /// The program is embedded in single quotes inside itself, so a reason carrying one would end
    /// the quoting and produce a script that does something else entirely.
    #[test]
    fn no_embedded_deny_body_carries_a_single_quote() {
        for body in [
            deny_body(unanswered_reason()),
            deny_body(no_channel_reason()),
        ] {
            assert!(!body.contains('\''), "{body}");
        }
    }

    /// Design § 7.7 rule 2's ordering, as a value: metaharness denies first, the hook's own
    /// backstop lands second, and the vendor's timeout is never reached.
    #[test]
    fn the_hooks_backstop_sits_between_metaharnesss_deadline_and_the_vendors_timeout() {
        // What the run loop derives from `HOOK_TIMEOUT_SECONDS`: the vendor's timeout less the
        // 5s margin. Written here rather than imported because the library depends on this crate
        // and not the other way round.
        const METAHARNESS_DEADLINE_SECONDS: u64 = 55;
        const { assert!(HOOK_WAIT_SECONDS < HOOK_TIMEOUT_SECONDS) };
        const { assert!(METAHARNESS_DEADLINE_SECONDS < HOOK_WAIT_SECONDS) };
    }

    #[test]
    fn the_program_needs_no_interpreter_the_reduced_path_does_not_carry() {
        let program = hook_program(&channel()).expect("a UTF-8 channel");
        assert!(program.starts_with("#!/bin/sh\n"));
        for forbidden in ["jq", "python3", "node", "bash"] {
            assert!(!program.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn the_program_publishes_under_one_rename_and_waits_on_its_own_key() {
        let program = hook_program(&channel()).expect("a UTF-8 channel");
        assert!(program.contains(r#"mv -f "$tmp" "$REQUESTS/$key.json""#));
        assert!(program.contains(r#"if [ -e "$RESPONSES/$key.json" ]; then"#));
        assert!(program.contains("/scratch/run-1/hook-channel/requests"));
        assert!(program.contains("/scratch/run-1/hook-channel/responses"));
    }

    /// Every exit is `0` with a body on stdout. On this wire a non-zero exit **is** a block with
    /// no reason attached, and a reason is the difference between a wall and an instruction.
    #[test]
    fn every_failure_path_denies_with_a_reason_rather_than_exiting_non_zero() {
        let program = hook_program(&channel()).expect("a UTF-8 channel");
        assert_eq!(program.matches("exit 0").count(), 2);
        assert!(!program.contains("exit 1"));
        assert!(!program.contains("exit 2"));
        assert!(program.contains(r#""permissionDecision":"deny""#));
        assert_eq!(program.matches("fail_closed '").count(), 4);
    }

    #[test]
    fn the_waiting_budget_is_expressed_in_tenths_of_a_second() {
        assert!(
            hook_program(&channel())
                .expect("a UTF-8 channel")
                .contains(&format!("TICKS={}", HOOK_WAIT_SECONDS * 10))
        );
    }

    /// A scratch root name carrying every byte the shell reads specially: a single quote, a
    /// double quote, a parameter, a backtick, a space and a newline (amendment a37).
    const AWKWARD: &str = "it's \"q\" $HOME `true` a b\nc";

    /// What the vendor would hand the hook, and what metaharness answers.
    const INPUT: &[u8] = br#"{"hook_event_name":"PreToolUse","tool_use_id":"call_1"}"#;
    const ANSWER: &[u8] = br#"{"hookSpecificOutput":{"permissionDecision":"allow"}}"#;

    /// The first published request in `requests`, while the program is still running.
    fn published_request(
        requests: &Path,
        child: &mut std::process::Child,
    ) -> Option<std::path::PathBuf> {
        for _ in 0..200 {
            let found = std::fs::read_dir(requests).ok().and_then(|entries| {
                entries.filter_map(Result::ok).map(|e| e.path()).find(|p| {
                    p.extension().is_some_and(|x| x == "json")
                        && !p
                            .file_name()
                            .is_some_and(|n| n.to_string_lossy().starts_with('.'))
                })
            });
            if found.is_some() {
                return found;
            }
            if child.try_wait().ok().flatten().is_some() {
                return None;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        None
    }

    /// Acceptance of `story:hook-program-quotes-channel-paths`: the program for a root the shell
    /// would otherwise read as code parses under `/bin/sh -n`, publishes the request at the
    /// channel's own path, and prints the answer written for it. No vendor binary.
    #[test]
    fn a_root_the_shell_would_read_as_code_still_parses_and_reaches_its_channel() {
        use std::io::Write as _;
        use std::process::{Command, Stdio};

        let base = tempfile::tempdir().expect("a directory");
        let channel = HookChannelPaths::under(&base.path().join(AWKWARD));
        std::fs::create_dir_all(&channel.requests).expect("requests");
        std::fs::create_dir_all(&channel.responses).expect("responses");
        let program = base.path().join("pretooluse");
        std::fs::write(&program, hook_program(&channel).expect("a UTF-8 channel"))
            .expect("written");

        let syntax = Command::new("/bin/sh")
            .arg("-n")
            .arg(&program)
            .output()
            .expect("sh runs");
        assert!(
            syntax.status.success(),
            "sh -n: {}",
            String::from_utf8_lossy(&syntax.stderr)
        );

        let mut child = Command::new("/bin/sh")
            .arg(&program)
            .env_clear()
            .env("PATH", "/usr/bin:/bin")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the hook runs");
        child
            .stdin
            .take()
            .expect("stdin")
            .write_all(INPUT)
            .expect("input written");
        let request = published_request(&channel.requests, &mut child);
        let Some(request) = request else {
            let output = child.wait_with_output().expect("the hook ends");
            panic!(
                "no request arrived under {:?}; stdout {:?}, stderr {:?}",
                channel.requests,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
        };
        assert_eq!(request.parent(), Some(channel.requests.as_path()));
        assert_eq!(std::fs::read(&request).expect("request"), INPUT);
        std::fs::write(
            channel
                .responses
                .join(request.file_name().expect("a file name")),
            ANSWER,
        )
        .expect("answered");
        let output = child.wait_with_output().expect("the hook ends");
        assert!(output.status.success(), "{output:?}");
        assert_eq!(output.stdout, ANSWER);
    }

    /// Amendment a37: a channel that is not UTF-8 is refused by name, carrying the path it was
    /// handed, and never rendered through the lossy display form.
    #[test]
    fn a_channel_that_is_not_utf8_is_refused_rather_than_rendered_lossily() {
        use std::os::unix::ffi::OsStrExt as _;
        let root = Path::new(std::ffi::OsStr::from_bytes(b"/scratch/run-\xff"));
        let channel = HookChannelPaths::under(root);
        let refusal = hook_program(&channel).expect_err("a non-UTF-8 channel is refused");
        assert_eq!(refusal.path, channel.requests);
        let words = refusal.to_string();
        assert!(words.contains("not UTF-8"), "{words}");
        assert!(words.contains(r"/scratch/run-\xff/"), "{words}");
        assert!(!words.contains(char::REPLACEMENT_CHARACTER), "{words}");
    }
}
