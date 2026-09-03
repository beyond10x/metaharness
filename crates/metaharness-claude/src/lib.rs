//! The claude adapter.
//!
//! Everything claude-specific lives here — how the binary is launched, how its transcript maps
//! onto [`metaharness_protocol::Event`], how a decision reaches it, and what hermetic means for
//! it. Nothing outside this crate may know any of that.
//!
//! Pinned to **2.1.259** ([`PINNED_VERSIONS`]). Every claim this crate makes about that binary is
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
//!   `AEP` asserts its own argv rather than trusting it *"because every one of
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
mod marketplace;
mod seam;
mod transcript;
mod vectors;

pub use bridge::{ClaudeSeam, ClaudeSeams};
pub use hook::{HOOK_WAIT_SECONDS, HookChannelPaths, hook_program};
pub use launch::{
    CredentialCopy, HOOK_TIMEOUT_SECONDS, LaunchContext, LaunchPlan, LaunchRefusal, LoopbackParams,
    ScratchFile, child_path, hook_program_path, mcp_config_path, plan_launch, settings_path,
};
pub use marketplace::{
    INSTALLED_PLUGINS, KNOWN_MARKETPLACES, MARKETPLACES_HOME, MarketplaceMatch, MarketplaceRefusal,
    PLUGIN_CACHE_HOME, PLUGIN_REGISTRY_HOME, ScratchEntry, resolve_marketplace, scratch_registry,
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
/// Moved 2.1.240 → **2.1.241** on 2026-08-24, and this move is the cheap kind. The earlier note
/// here said moving the pin again was "a **capture**, not a search-and-replace", which read the
/// rule backwards: `fixtures/golden/README.md` § *"Capture version and pin are two different
/// facts"* is the governing text, and it says the opposite for a good reason. The golden's
/// `captured`/`binary` rows are a fact about **those bytes** and are never edited; the pin is what
/// the adapter is *run* against; and `golden-version-pair` is the single place the two are
/// compared, where a disagreement is a **named warning that stands until somebody pays for a
/// re-capture**. So a re-pin is free and honest, and only a re-capture costs money. Holding the pin
/// behind the installed binary bought nothing and cost every `--hermetic strict` run an H9 gap.
///
/// What was read on 2.1.241 before moving it, and what was not:
///
/// * **Verified, live** — two runs through this adapter on 2026-08-24 opened, streamed a tool
///   call, and ended; each opening record reported `claude_code_version` 2.1.241 and was read by
///   this crate's own transcript reader without a dialect complaint.
/// * **Verified, from the binary's own help** — every flag the launch builds still exists, and
///   two of them still say what the design quotes them as saying: `--tools` — *"Use \"\" to
///   disable all tools"* (V11) — and `--strict-mcp-config` — *"Only use MCP servers from
///   --mcp-config, ignoring all other MCP configurations"* (H5).
/// * **Carried over unverified** — the `R8u(e)` parser reading behind V12, read from the 2.1.239
///   bundle; V4's bare-`--allowedTools` auto-approval, never driven on any version; and Q11's
///   `""` hook matcher regime. None of these were re-read, and none is claimed to have been.
///
/// Moved 2.1.241 → **2.1.259** on 2026-09-03, the cheap kind again, and with one thing read that the
/// earlier moves had no occasion to: the **wire grew**. A paid run of the installed 2.1.259 through
/// this adapter (the golden-path eval case, 1,515 events) opened, streamed and ended, its opening
/// record reported `claude_code_version` 2.1.259 — and 183 of its lines were shapes this reader had
/// never seen: `system/task_started`, `system/task_progress`, `system/task_notification`,
/// `system/task_updated` and a top-level `tool_progress`. Each became `opaque`, which is what D4
/// requires, and every `tool.absent` row in the checker that read the stream came back `unk` over
/// them. `transcript::control_plane` names those five as the vendor's own bookkeeping and drops
/// them; anything else unnamed still goes `opaque`. `metaharness doctor claude` on the same day
/// confirmed every flag the launch builds is declared by the 2.1.259 binary, including the
/// `--max-budget-usd` this release starts sending.
///
/// **Two more shapes, and the lesson of how they were met (0.6.2 → 0.6.4).** Later runs of the
/// same case carried `system/background_tasks_changed` and `system/api_retry`. Both were first
/// handled from the record as it appeared in a stream rather than from the binary's own schema,
/// and both were wrong for it:
///
/// * `background_tasks_changed` was dropped as bookkeeping. Its schema says the payload is *"every
///   live background task after the change"* with REPLACE semantics, emitted on completion and
///   kill as well as start — and only the start is on the wire elsewhere. It is read now.
/// * `api_retry` was read for `delayMs` and a string `error`. Its schema is `attempt`,
///   `max_retries`, `retry_delay_ms`, `error_status` and an `error` **object**, so every real
///   retry reported no reason and no backoff.
///
/// Both are now read from the schema the installed binary carries, which is the thing this
/// constant exists to make somebody go and look at. A shape met from one observed record is a
/// guess with a sample size of one.
///
/// The golden fixtures stay labelled 2.1.240, because that is the binary whose bytes they are.
/// `golden-version-pair` therefore warns, by design, and the warning is the outstanding invoice.
pub const PINNED_VERSIONS: [&str; 1] = ["2.1.259"];
