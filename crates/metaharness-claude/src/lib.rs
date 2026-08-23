//! The claude adapter.
//!
//! Everything claude-specific lives here — how the binary is launched, how its transcript maps
//! onto [`metaharness_protocol::Event`], how a decision reaches it, and what hermetic means for
//! it. Nothing outside this crate may know any of that.
//!
//! Pinned to **2.1.240** ([`PINNED_VERSIONS`]). Every claim this crate makes about that binary is
//! either read from `docs/design/metaharness-protocol-v0.1.md` § 2.7 — where each row carries its
//! own method — or labelled unverified at the point of use with the `Q` row that would close it.
//! A string in a binary is weaker than a driven call, and this crate says which it has.
//!
//! **Where a claim below names 2.1.239, that is the binary the observation was read from, not the
//! pin** (design amendment a11). The pin moved on 2026-08-23, when the installed 2.1.240 ran a
//! whole session through this adapter; the § 2.7 rows were read on 2026-08-22 from
//! 2.1.239 and are left naming it, because rewriting the version on a dated observation would
//! invent evidence rather than move a pin. What is recorded from 2.1.240 says so at its own point
//! of use — the golden fixtures under `fixtures/golden/` are 2.1.240's own bytes.
//!
//! # Three things this crate deliberately does not do
//!
//! * **It spawns nothing.** [`plan_launch`] returns the command line, the child environment, the
//!   settings document, the hook definition and the copy list as *values*, because
//!   `engineering-protocols` asserts its own argv rather than trusting it *"because every one of
//!   the failures would be silent"* (design § 8.4 O7). [`hook_program`] is the same rule applied
//!   to the seam's executable: the program is a value this crate renders and a caller places.
//! * **It reads no clock.** Every event's timestamp is the one the vendor recorded, passed
//!   through, or absent — [`metaharness_protocol::Emission::at`] and
//!   [`metaharness_protocol::Emission::untimed`] are the only two ways to make one (design D2).
//! * **It drops nothing.** A vendor record this crate cannot map becomes
//!   [`metaharness_protocol::Event::Opaque`], carrying the vendor's own `type`, `subtype`, a
//!   digest of the raw line and its 1-based source line (design D4). An unrecognised *field* is
//!   ignored in silence; an unrecognised *record* is preserved.

mod bridge;
mod hook;
mod launch;
mod seam;
mod transcript;
mod vectors;

pub use bridge::{ClaudeSeam, ClaudeSeams};
pub use hook::{HOOK_WAIT_SECONDS, HookChannelPaths, hook_program};
pub use launch::{
    CredentialCopy, HOOK_TIMEOUT_SECONDS, LaunchContext, LaunchPlan, LaunchRefusal, LoopbackParams,
    child_path, hook_program_path, plan_launch, settings_path,
};
pub use seam::{HookInput, capabilities, parse_hook_input, render_hook_response, render_operation};
pub use transcript::TranscriptReader;
pub use vectors::{CONTRACT_OBLIGATIONS, conformance_vectors};

/// This adapter's id, as it appears in `session.started` and on the command line.
pub const ADAPTER_ID: &str = "claude";

/// The vendor versions this adapter was written against (design § 8.4 O1).
///
/// One entry, because the vendor's transcript and hook shapes are not stable public schemas: a
/// verdict that changed because the reader changed must be visible as such rather than as a
/// change in the agent's behaviour. A version outside this pin is a `warning`, or a refusal
/// before the run under `--strict-version`.
///
/// Moved 2.1.239 → **2.1.240** on 2026-08-23 (design amendment a11). Two pieces of evidence, and
/// they are separate: a live run of the installed 2.1.240 through this adapter spoke the stream
/// dialect this crate reads — the session opened, streamed and ended, and its own opening record
/// reported `claude_code_version` 2.1.240 — and the golden fixtures in `fixtures/golden/` are that
/// same binary's wire, including a `PreToolUse` stdin it really published into the hook channel,
/// replayed here byte for byte in the free tier. A pin is what the adapter is *tested against*, so it names the binary
/// whose bytes are committed here, not the one the older § 2.7 rows were read from.
///
/// **The vendor moved again the same afternoon** — `~/.local/share/claude/versions/2.1.241`,
/// installed 2026-08-23T14:02, and `doctor claude` says *"OFF the adapter's pin"* about it, which
/// is correct and is the verb doing its job. The pin stays at 2.1.240 because 2.1.240 is the
/// version this crate holds bytes for. Moving it again is a **capture**, not a search-and-replace:
/// a pin ahead of the evidence would make every row here a claim about a binary nobody read.
pub const PINNED_VERSIONS: [&str; 1] = ["2.1.240"];
