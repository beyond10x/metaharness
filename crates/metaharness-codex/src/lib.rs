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
//! # What this milestone is, and is not (CX-M1)
//!
//! * **The adapter's input is built**: [`RolloutReader`] maps the session rollout —
//!   `$CODEX_HOME/sessions/…/rollout-*.jsonl`, the record that carries timestamps, durations and
//!   per-turn usage where `codex exec --json` stdout does not — onto the protocol's events,
//!   version-gated, with every unmapped shape preserved as `opaque` (D4). The format has **no
//!   documented stability guarantee** and drifts within one install, which is why the gate is a
//!   warning and never a refusal mid-read.
//! * **Capabilities are declared, honestly**: the 0.145.0 binary ships a stable `PreToolUse`
//!   hook with the same decision contract as Claude Code's (documented, and read in the binary's
//!   own wire types) — and metaharness has never driven it, so `tool.decide` is **refused**, not
//!   sold as honoured. Every tier is `Unverified` until a driven run proves it, the same road
//!   the Claude adapter took.
//! * **Nothing spawns.** Launch planning and the live seam are CX-M2: `metaharness run codex` is
//!   refused by name at start, with this milestone named in the refusal.

mod rollout;
mod seam;
mod vectors;

pub use rollout::RolloutReader;
pub use seam::capabilities;
pub use vectors::conformance_vectors;

/// This adapter's id, as it appears in `session.started` and on the command line.
pub const ADAPTER_ID: &str = "codex";

/// The vendor versions this adapter was written against (design § 8.4 O1).
///
/// One entry, because the rollout format is not a stable public schema: a verdict that changed
/// because the reader changed must be visible as such rather than as a change in the agent's
/// behaviour.
pub const PINNED_VERSIONS: [&str; 1] = ["0.145.0"];
