//! What this adapter can honour, declared rather than discovered by refusal.
//!
//! Every status below is labelled the way the research record labels its facts: the hook
//! contract is **documented and read in the vendor's binary** (0.145.0 ships a stable
//! `PreToolUse` hook with the same `permissionDecision` wire as Claude Code), and **none of it
//! has been driven by metaharness yet** — so nothing here claims `Delivered`. A tier upgrades
//! when a driven run proves it, the same way the Claude adapter's did (its M2), never before.

use std::collections::BTreeMap;

use metaharness_protocol::{
    AdapterClass, AdapterId, COMMAND_NAMES, Capabilities, CommandSupport, Operation, RefusalCode,
    Tier, TierStatus,
};

use crate::{ADAPTER_ID, PINNED_VERSIONS};

/// The codex adapter's declared capabilities.
#[must_use]
pub fn capabilities() -> Capabilities {
    let refused = CommandSupport::Refused(RefusalCode::UnsupportedControl);
    let mut commands: BTreeMap<String, CommandSupport> = COMMAND_NAMES
        .iter()
        .map(|name| ((*name).to_string(), refused))
        .collect();
    // Process-level controls the run loop delivers itself, adapter-independent.
    commands.insert("interrupt".to_string(), CommandSupport::Honoured);
    commands.insert("halt".to_string(), CommandSupport::Honoured);
    // `tool.decide` stays refused until a driven codex run answers the seam: the vendor's
    // PreToolUse hook carries the same decision contract (documented, and read in the 0.145.0
    // binary), but a control this adapter has never exercised must not be sold as honoured.
    // The refusal flips with CX-M2, the codex spawn milestone.

    Capabilities {
        adapter: AdapterId {
            id: ADAPTER_ID.to_string(),
            class: AdapterClass::Harness,
        },
        versions_pinned: PINNED_VERSIONS.iter().map(|v| (*v).to_string()).collect(),
        tiers: BTreeMap::from([
            (Tier::Registration, TierStatus::Unverified),
            (Tier::Call, TierStatus::Unverified),
            (Tier::Turn, TierStatus::Unverified),
            (Tier::Kill, TierStatus::Unverified),
        ]),
        commands,
        rendering: rendering(),
    }
}

/// The neutral-operation → vendor-tool table, published as a value (design § 8.4 O6).
///
/// Codex's tool surface is narrower than the vocabulary: a shell call is `exec`, an edit is an
/// `apply_patch`, and most of the rest has no dedicated vendor tool — the model reaches files
/// through the shell. `None` means exactly that, and the adapter never re-decides what an
/// admission implies.
fn rendering() -> BTreeMap<String, Option<String>> {
    let mut table: BTreeMap<String, Option<String>> = Operation::PARAMETERLESS
        .iter()
        .map(|operation| ((operation.name().to_string()), render(operation)))
        .collect();
    table.insert("mcp.call".to_string(), None);
    table
}

fn render(operation: &Operation) -> Option<String> {
    match operation {
        Operation::Shell => Some("exec".to_string()),
        Operation::FileWrite | Operation::FileEdit => Some("apply_patch".to_string()),
        _ => None,
    }
}
