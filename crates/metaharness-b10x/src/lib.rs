//! The b10x adapter: an agent loop we own, **observed** rather than driven.
//!
//! # Why this adapter decides nothing, and why that is the design
//!
//! Every other adapter in this workspace exists to put metaharness between a vendor's loop and its
//! tools: a hook that blocks a call, a control request that answers one, a registration that bounds
//! the set. That is what makes a *driven* run driven.
//!
//! `b10x-harness` is not a vendor's loop. It holds its own, and the toolset it publishes is
//! computed from what the machine can confine — a tool outside the surface does not exist rather
//! than being refused. An evaluation arm exists to measure whether *that* changes what a model
//! does, and a seam that adjudicated its calls would put the driven arm's treatment back on top and
//! measure that instead. The two arms would then differ in name only.
//!
//! So this adapter runs in [`DecisionMode::Observe`](metaharness_protocol::spec::DecisionMode) and
//! nothing else. What it contributes is the half that is **not** control:
//!
//! * **attestation** — what metaharness imposed on the launch, in metaharness's own words, beside
//!   the loop's record so a reader can notice when the two disagree;
//! * **one wire** — the same `metaharness.event/1` stream every other arm is judged from, so a
//!   matrix that compares four cells is comparing runs and not instruments.
//!
//! Refusing to decide is asserted rather than assumed: [`B10xSeam::decision_line`] answers a line
//! no run will ever send, and every `tool.requested` carries `decision_required: false` and
//! [`Seam::None`] — *nobody adjudicated this call*, which is the fact rather than the omission.
//!
//! # `adapter_class` is `loop`, not `harness`
//!
//! Design § 8.4 O5 requires that a harness adapter never silently becomes a direct API call. This
//! is the other direction of the same rule: this adapter drives no vendor binary, and a reader
//! filtering on `adapter_class` must be able to see that without reading the source.
//!
//! # What the loop does not have, written as `null`
//!
//! No slash commands, no skills, no subagents, no MCP servers, no permission mode. Those are not
//! unobserved — they do not exist, and a field invented for them would be a claim about a run
//! nobody made. `hermetic.installed_plugins` is `[]` for the same reason and is a *fact*: this loop
//! has no plugin mechanism for anything to be installed into.

#![allow(missing_docs)]

mod launch;
mod seam;

pub use launch::{B10xLaunch, argv};
pub use seam::{B10xSeam, B10xSeams};

/// What this adapter calls itself on the wire.
pub const ADAPTER_ID: &str = "b10x";

/// Not `harness`: this adapter observes a loop rather than driving somebody else's.
pub const ADAPTER_CLASS: &str = "loop";

/// The `b10x-harness` versions this adapter's claims were read from.
///
/// Pinned for the reason the other adapters pin: every version-specific claim in here — the field
/// names of the loop record, the shape of its terminal event — was observed against these, and a
/// run against another is unverified rather than wrong.
pub const PINNED_VERSIONS: [&str; 1] = ["0.1.0"];
