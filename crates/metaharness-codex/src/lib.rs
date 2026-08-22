//! The codex adapter.
//!
//! Everything codex-specific lives here — how its session record maps onto
//! [`metaharness_protocol::Event`], what it can honour, and what hermetic would mean for it.
//! Nothing outside this crate may know any of that.
//!
//! Pinned to **0.145.0** ([`PINNED_VERSIONS`]). The evidence base is the research record
//! migrated into `docs/research/2026-08-21-codex-harness-research.md`: every claim there is
//! labelled **V** (verified locally against the binary and 2,437 rollout files), **D** (official
//! docs), **I** (inferred) or **?** (unknown), and this crate inherits those labels at the point
//! of use.
//!
//! # What this milestone is, and is not (CX-M2)
//!
//! * **The adapter's input is built** (CX-M1): [`RolloutReader`] maps the session rollout —
//!   `$CODEX_HOME/sessions/…/rollout-*.jsonl`, the record that carries timestamps, durations and
//!   per-turn usage where `codex exec --json` stdout does not — onto the protocol's events,
//!   version-gated, with every unmapped shape preserved as `opaque` (D4). The format has **no
//!   documented stability guarantee** and drifts within one install, which is why the gate is a
//!   warning and never a refusal mid-read.
//! * **The seam is driven** (CX-M2): [`plan_launch`] constructs a hermetic `codex exec` — a
//!   scratch `CODEX_HOME`, a constructed environment, the operator's `auth.json` copied per spawn
//!   — and declares a blocking `PreToolUse` hook in the one place this binary reads one. A live
//!   run refused a shell call at that hook and **the vendor's own record shows the command
//!   blocked with empty output**, so [`capabilities`] declares the call tier `Delivered` and
//!   `tool.decide` `Honoured`. Design amendment a7.
//! * **What is still not claimed:** the `allow` half of the decision wire (only `deny` has been
//!   driven), the turn tier, the registration tier, and the `apply_patch` rendering — the hook's
//!   word for a patch call is the vendor's documentation and not a driven observation.
//!
//! # The three things about this vendor that cost the most to learn
//!
//! Each one is a silent failure, and each is why a claim here is asserted from a run's record
//! rather than from the file that configured it:
//!
//! 1. **A hook is declared in `config.toml`, not in `hooks.json`.** A `hooks.json` is a *plugin
//!    manifest's* file. An unrecognised key under `[hooks]` is dropped **without failing the
//!    config load**, so a misconfigured seam and a run in which nothing was attempted are the
//!    same observation.
//! 2. **A hook in a fresh `CODEX_HOME` does not fire without `--dangerously-bypass-hook-trust`.**
//!    A scratch home cannot hold persisted trust, and the flag's warning is about running
//!    *somebody else's* hook unvetted — not about the one metaharness just wrote.
//! 3. **The hook speaks Claude Code's tool vocabulary.** `tool_name` is `Bash`, where the rollout
//!    calls the same call `exec` and the binary's model-facing list calls it `shell`.

mod bridge;
mod hook;
mod launch;
mod rollout;
mod seam;
mod vectors;

pub use bridge::{CodexSeam, CodexSeams, hook_request_line};
pub use hook::{HOOK_WAIT_SECONDS, HookChannelPaths, hook_program};
pub use launch::{
    CredentialCopy, HOOK_TIMEOUT_SECONDS, LaunchContext, LaunchPlan, LaunchRefusal, config_path,
    hook_program_path, plan_launch,
};
pub use rollout::RolloutReader;
pub use seam::{HookInput, capabilities, parse_hook_input, render_hook_response, render_operation};
pub use vectors::conformance_vectors;

/// This adapter's id, as it appears in `session.started` and on the command line.
pub const ADAPTER_ID: &str = "codex";

/// The vendor versions this adapter was written against (design § 8.4 O1).
///
/// One entry, because the rollout format is not a stable public schema: a verdict that changed
/// because the reader changed must be visible as such rather than as a change in the agent's
/// behaviour.
pub const PINNED_VERSIONS: [&str; 1] = ["0.145.0"];
