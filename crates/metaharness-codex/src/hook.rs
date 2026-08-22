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

/// The `PreToolUse` program, as text.
///
/// POSIX `sh` and nothing the child's reduced `PATH` does not already carry: `mktemp`, `cat`,
/// `mv`, `sleep`, `printf`. No `jq`, no `python3`, no `node` — the way to never need the
/// "interpreter is missing" branch is to need no interpreter.
#[must_use]
pub fn hook_program(channel: &HookChannelPaths) -> String {
    let requests = channel.requests.display();
    let responses = channel.responses.display();
    let ticks = HOOK_WAIT_SECONDS * 10;
    let unanswered = deny_body(unanswered_reason());
    let no_channel = deny_body(no_channel_reason());
    format!(
        r#"#!/bin/sh
# metaharness PreToolUse seam for codex. Generated per run; edited by nobody.
#
# It publishes the vendor's hook input verbatim and waits for metaharness to answer it. It parses
# no JSON: metaharness does that in Rust, and routes by the rendezvous name this process minted.
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
        # vendor's own approval policy decides. That is not the same as allowing.
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

    /// Every exit is `0` with a body on stdout. On this wire a non-zero exit **is** a block with
    /// no reason attached, and a reason is the difference between a wall and an instruction.
    #[test]
    fn every_failure_path_denies_with_a_reason_rather_than_exiting_non_zero() {
        let program = hook_program(&channel());
        assert_eq!(program.matches("exit 0").count(), 2);
        assert!(!program.contains("exit 1"));
        assert!(!program.contains("exit 2"));
        assert!(program.contains(r#""permissionDecision":"deny""#));
        assert_eq!(program.matches("fail_closed '").count(), 4);
    }

    #[test]
    fn the_waiting_budget_is_expressed_in_tenths_of_a_second() {
        assert!(hook_program(&channel()).contains(&format!("TICKS={}", HOOK_WAIT_SECONDS * 10)));
    }
}
