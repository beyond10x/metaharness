//! What an adapter says it can do, published as a value.
//!
//! An adapter declares which tiers it delivers, which commands it can honour and which it
//! refuses, and a tier it has not driven is declared `unverified` — an embedder that *requires*
//! an unverified tier gets a refusal rather than a silent no-op (design § 8.4 O4). The operation
//! rendering is published here too, because a rendering that only exists inside a run cannot be
//! asserted on before one (O6).

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::command::RefusalCode;
use crate::frame::Operation;
use crate::spec::{DecisionMode, RunSpec, ToolSurface};

/// Which class of adapter this is.
///
/// Named on every session, because neither class silently falls back to the other: a harness
/// adapter never becomes a direct API call (design § 8.4 O5, § 11).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterClass {
    /// The vendor keeps its loop, its sessions, its tools, its authentication and its credential
    /// custody; metaharness drives its documented outside surface.
    Harness,
    /// The embedder holds the conversation and calls a model API. Not in v0.1.
    DirectProvider,
}

/// Which adapter, and of which class.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdapterId {
    /// The adapter's id, as it appears in `session.started`.
    pub id: String,
    /// Its class.
    pub class: AdapterClass,
}

/// One control tier (design § 7.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Tier {
    /// The set of tools the model is offered is decided before the session starts. Cannot see an
    /// argument; cannot change within a session.
    Registration,
    /// Every call is presented for a decision **before it executes**, and the harness waits.
    /// Costs a round trip; only as universal as the seam's coverage.
    Call,
    /// Text can be added to the conversation between turns. Cannot stop a call; only advises.
    Turn,
    /// A running turn can be stopped. Loses the turn; cannot be selective.
    Kill,
}

impl Tier {
    /// Every tier, in the design's order.
    pub const ALL: [Tier; 4] = [Tier::Registration, Tier::Call, Tier::Turn, Tier::Kill];

    /// The tier's name, as a report prints it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            Tier::Registration => "registration",
            Tier::Call => "call",
            Tier::Turn => "turn",
            Tier::Kill => "kill",
        }
    }
}

/// How well an adapter delivers a tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TierStatus {
    /// Driven, and asserted by a conformance vector.
    Delivered,
    /// The mechanism is present on the vendor's surface and metaharness has not driven it. An
    /// embedder that requires it is refused, not quietly served.
    Unverified,
    /// The vendor has no such mechanism at this tier.
    Absent,
}

/// Whether a command can be honoured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandSupport {
    /// The adapter delivers it.
    Honoured,
    /// The adapter refuses it, with this code, at run start.
    Refused(RefusalCode),
}

/// What an adapter says it can do.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Capabilities {
    /// Which adapter.
    pub adapter: AdapterId,
    /// The vendor versions this adapter was written against (design § 8.4 O1).
    pub versions_pinned: Vec<String>,
    /// What it delivers, per tier.
    pub tiers: BTreeMap<Tier, TierStatus>,
    /// What it will do with each command, keyed by the command's wire name.
    pub commands: BTreeMap<String, CommandSupport>,
    /// The neutral operation → vendor tool rendering. `None` means the vendor has no tool for
    /// that operation, which is a fact worth publishing rather than an omission.
    pub rendering: BTreeMap<String, Option<String>>,
}

impl Capabilities {
    /// What this adapter will do with a command, by wire name.
    ///
    /// A command absent from the table is refused as [`RefusalCode::UnsupportedControl`]: an
    /// adapter that never mentioned a command cannot be assumed to deliver it.
    #[must_use]
    pub fn support(&self, command_name: &str) -> CommandSupport {
        self.commands
            .get(command_name)
            .copied()
            .unwrap_or(CommandSupport::Refused(RefusalCode::UnsupportedControl))
    }

    /// The vendor tool this operation renders to, when there is one.
    #[must_use]
    pub fn renders(&self, operation: &Operation) -> Option<&str> {
        self.rendering
            .get(operation.name())
            .and_then(Option::as_deref)
    }

    /// Every command this run's configuration will need that this adapter refuses.
    ///
    /// Run at start rather than at the call, so a run that will fail on control fails before it
    /// spends money (design § 6.1).
    #[must_use]
    pub fn refusals_for(&self, spec: &RunSpec) -> Vec<(&'static str, RefusalCode)> {
        required_commands(spec)
            .into_iter()
            .filter_map(|name| match self.support(name) {
                CommandSupport::Refused(code) => Some((name, code)),
                CommandSupport::Honoured => None,
            })
            .collect()
    }
}

/// Which commands a run of this shape will need.
///
/// `interrupt` and `halt` are in every list because every adapter must deliver them and a run
/// that cannot be stopped is not a run anyone should start.
#[must_use]
pub fn required_commands(spec: &RunSpec) -> Vec<&'static str> {
    let mut needed = vec!["interrupt", "halt"];
    if spec.decisions == DecisionMode::Ask {
        needed.push("tool.decide");
    }
    if spec.frame.is_some() {
        // Both halves or neither: a frame whose text reaches the model while nothing enforces it
        // tells the model "strictly only these operations" and makes it false (finding F9).
        needed.push("frame.set");
    }
    if spec.tool_surface == ToolSurface::Owned {
        needed.push("tool.decide");
    }
    needed.sort_unstable();
    needed.dedup();
    needed
}
