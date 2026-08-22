//! The `PreToolUse` program the seam installs, and the channel it answers over.
//!
//! M1 constructed the hook *definition* — matcher `""`, `type: command`, neither `async` nor
//! `asyncRewake` — and named a path for the executable without writing one, because
//! [`crate::plan_launch`] is a pure function and *"the executable it names is the caller's to
//! place"*. This module is that executable, as a value.
//!
//! # Why the program parses no JSON
//!
//! A hook is a separate process with a shell's tools, and a guard that has to parse JSON with
//! `sed` is a guard that stops guarding the first time a field moves. So the program **publishes
//! its whole stdin verbatim** under a name only it holds, and metaharness does the parsing in
//! Rust. The correlation is therefore metaharness's to make and not the shell's, and the
//! rendezvous name is the hook process's own — which is the shape design § 12 **Q16** predicted:
//! *"the real hook correlates by which hook process is answering."*
//!
//! # What correlates a response to a call
//!
//! **`tool_use_id`, and it is the same string the transcript's `tool_use` block calls `id`.**
//! Verified twice on 2.1.239 — in the binary, where the payload is built as
//! `{…, hook_event_name:"PreToolUse", tool_name:e, tool_input:r, tool_use_id:t}`, and in a live
//! run, whose hook received `"tool_use_id":"toolu_…"` for the same id the stream-json assistant
//! record carried. That is row **V22** and it is what closes Q16: the envelope M1 guessed —
//! `{"call_id":…, "response":…}` — turns out to be exactly right, so nothing about it changes.
//!
//! # The ordering this depends on, and the evidence for it
//!
//! **The `tool_use` record reaches stdout before the hook runs** (row **V23**). Measured: a hook
//! that recorded how many bytes of stdout had already been flushed when it fired reported 5504,
//! and byte 5504 is the end of the assistant record carrying that call. So metaharness has
//! already seen the call — and in `frame` mode has already decided it — by the time the hook
//! asks. The channel does not *depend* on that ordering (a decision may be parked before its
//! request arrives, and a request may arrive before its decision), but it is why the common path
//! costs no waiting at all.
//!
//! # Fail closed, three ways
//!
//! With no channel, no writable request directory, or no answer inside its own budget, the
//! program **denies with a reason**. That is § 2.2 item 3 — *"a guard that silently stops
//! guarding is the defect this repository writes registers about"* — and it is why the program
//! never exits non-zero to signal trouble: a non-zero exit is a vendor-interpreted signal, while
//! a `deny` with a reason is a decision the model is told about.

use std::path::{Path, PathBuf};

use metaharness_protocol::Decision;

use crate::HOOK_TIMEOUT_SECONDS;
use crate::seam::render_hook_response;

/// How long the installed program waits for metaharness before it denies on its own.
///
/// Between metaharness's own deadline and the vendor's timeout, and that ordering is the whole
/// point: metaharness's deadline (§ 7.7 rule 2, `HOOK_TIMEOUT_SECONDS` less the margin) expires
/// **first**, so a slow decision becomes a refusal metaharness owns and explains. This budget is
/// the backstop for the case that rule cannot cover — metaharness itself being gone — and it
/// still lands before the vendor's own timeout turns the outcome into **Q10**, which is
/// undriven.
pub const HOOK_WAIT_SECONDS: u64 = HOOK_TIMEOUT_SECONDS - 3;

/// The directory under the scratch root that holds the decision channel.
const CHANNEL_DIR: &str = "hook-channel";

/// Where hook processes publish what the vendor handed them.
const REQUESTS_DIR: &str = "requests";

/// Where metaharness writes the answer.
const RESPONSES_DIR: &str = "responses";

/// The decision channel's three paths, derived from one scratch root.
///
/// A value rather than three arguments, because the program and the reader have to agree about
/// all three and a caller that passed them separately could pass two of them from one run and
/// one from another.
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
    /// The two halves are derived from the root in **one** place, here, because a caller that
    /// joined `requests` itself would be a second party deciding the layout — and a channel
    /// whose two ends disagree by one directory does not fail loudly, it simply never carries a
    /// decision.
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
#[must_use]
fn unanswered_reason() -> String {
    format!(
        "metaharness did not answer this call within {HOOK_WAIT_SECONDS}s and the seam fails \
         closed, so the call did not run; this is the backstop inside the hook program itself, \
         not a decision anybody made about the call"
    )
}

/// The reason the installed program gives when it cannot reach its channel at all.
#[must_use]
fn no_channel_reason() -> String {
    "metaharness could not be reached over its decision channel, so the call did not run; a \
     guard that cannot be consulted refuses rather than passing the call through"
        .to_string()
}

/// One deny body, compact, as the program will print it.
///
/// Rendered through [`render_hook_response`] rather than written out by hand, so the program's
/// fail-closed answer and the run loop's answers are the same wire shape and cannot drift.
fn deny_body(reason: String) -> String {
    render_hook_response(&Decision::Deny { reason }).to_string()
}

/// The `PreToolUse` program, as text.
///
/// POSIX `sh` and nothing the child's reduced `PATH` does not already carry: `mktemp`, `cat`,
/// `mv`, `sleep`, `printf`. There is deliberately no `jq`, no `python3` and no `node` — the
/// reference hooks fail closed when their interpreter is absent, and the way to never need that
/// branch is to need no interpreter.
#[must_use]
pub fn hook_program(channel: &HookChannelPaths) -> String {
    let requests = channel.requests.display();
    let responses = channel.responses.display();
    let ticks = HOOK_WAIT_SECONDS * 10;
    let unanswered = deny_body(unanswered_reason());
    let no_channel = deny_body(no_channel_reason());
    format!(
        r#"#!/bin/sh
# metaharness PreToolUse seam. Generated per run; edited by nobody.
#
# It publishes the vendor's hook input verbatim and waits for metaharness to answer it. It parses
# no JSON: metaharness does that in Rust, and correlates by the tool_use_id the input carries.
set -u

REQUESTS='{requests}'
RESPONSES='{responses}'
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
        # vendor's own permission pipeline decides. That is not the same as allowing.
        cat "$RESPONSES/$key.json"
        exit 0
    fi
    sleep 0.1 2>/dev/null || sleep 1
    tick=$((tick + 1))
done

fail_closed '{unanswered}'
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn channel() -> HookChannelPaths {
        HookChannelPaths::under(Path::new("/scratch/run-1"))
    }

    #[test]
    fn the_channel_hangs_off_the_scratch_root_and_not_off_the_config_home() {
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

    /// The program is embedded in single quotes inside itself, so a reason carrying one would
    /// end the quoting and produce a script that does something else entirely.
    #[test]
    fn no_embedded_deny_body_carries_a_single_quote() {
        for body in [
            deny_body(unanswered_reason()),
            deny_body(no_channel_reason()),
        ] {
            assert!(!body.contains('\''), "{body}");
        }
    }

    /// Rule 2's ordering, as a value: metaharness denies first, the hook's own backstop lands
    /// second, and the vendor's timeout — whose behaviour is **Q10** and undriven — is never
    /// reached.
    #[test]
    fn the_hooks_backstop_sits_between_metaharnesss_deadline_and_the_vendors_timeout() {
        const METAHARNESS_DEADLINE_SECONDS: u64 = 55;
        const { assert!(HOOK_WAIT_SECONDS < HOOK_TIMEOUT_SECONDS) };
        const { assert!(METAHARNESS_DEADLINE_SECONDS < HOOK_WAIT_SECONDS) };
    }

    #[test]
    fn the_program_needs_no_interpreter_the_reduced_path_does_not_carry() {
        let program = hook_program(&channel());
        assert!(program.starts_with("#!/bin/sh\n"));
        for forbidden in ["jq", "python3", "node", "bash"] {
            assert!(!program.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn the_program_publishes_under_one_rename_and_waits_on_its_own_key() {
        let program = hook_program(&channel());
        assert!(program.contains(r#"mv -f "$tmp" "$REQUESTS/$key.json""#));
        assert!(program.contains(r#"if [ -e "$RESPONSES/$key.json" ]; then"#));
        assert!(program.contains("/scratch/run-1/hook-channel/requests"));
        assert!(program.contains("/scratch/run-1/hook-channel/responses"));
    }

    /// Every exit is `0` with a body on stdout. A non-zero exit is a vendor-interpreted signal;
    /// a `deny` with a reason is a decision the model is told about (design § 2.2 item 1).
    #[test]
    fn every_failure_path_denies_with_a_reason_rather_than_exiting_non_zero() {
        let program = hook_program(&channel());
        assert_eq!(program.matches("exit 0").count(), 2);
        assert!(!program.contains("exit 1"));
        assert!(program.contains(r#""permissionDecision":"deny""#));
        assert_eq!(program.matches("fail_closed '").count(), 4);
    }

    #[test]
    fn the_waiting_budget_is_expressed_in_tenths_of_a_second() {
        assert!(hook_program(&channel()).contains(&format!("TICKS={}", HOOK_WAIT_SECONDS * 10)));
    }
}
