//! What a call *is*, read off a record, in one vocabulary for every harness.
//!
//! # The blindness this removes
//!
//! A run is judged after it happened, from its event stream. Until now that stream carried the
//! **vendor's** tool name and nothing else, so one act had a different name in every arm:
//!
//! | the act | b10x records | Claude Code, native | Claude Code, owned surface |
//! |---|---|---|---|
//! | write a file | `workspace_write` | `Write` | `mcp__metaharness__tool_invoke` + `{"name": "file_write"}` |
//! | run a suite | `run` | `Bash` | `mcp__metaharness__tool_invoke` + `{"name": "run"}` |
//!
//! Everything downstream therefore had to learn one vendor's vocabulary, and the evaluation corpus
//! that judges four arms was written in Claude Code's — so it was blind to the others, and two
//! attempts to widen it put *more* vendor names into a document that should hold none.
//!
//! The three verbs fixed the model's side of that. This fixes the reader's: every `tool.requested`
//! now carries the neutral operations it resolves to, and a consumer selects on `file.write`
//! whichever harness produced the run.
//!
//! # Why the tool name alone is not enough
//!
//! Under the owned surface every entry travels through **one** verb. A rendering table maps
//! operations to tool names and would collapse all six onto `tool_invoke` — losing exactly the
//! distinction a reader needs. So resolution takes the call: the name *and* its input.

use serde_json::Value;

pub use b10x_harness_tools::operation_of;

use crate::{DESCRIBE_VERB, INVOKE_VERB, SEARCH_VERB};

/// The prefix a vendor puts on an MCP tool: `mcp__<server>__<tool>`.
const MCP_PREFIX: &str = "mcp__";

/// What one call turned out to be.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    /// The call is these neutral operations. More than one when a vendor tool answers to several —
    /// codex writes *and* edits through one `apply_patch`.
    Operations(Vec<String>),
    /// The call is a question about the catalogue: `tool_search` or `tool_describe`.
    ///
    /// Its own category, because it is neither an operation nor an unknown. It touches nothing a
    /// policy could name — no file, no process, nothing that outlives the call — so a frame that
    /// narrowed it would be refusing the model permission to *read the list of things it may do*,
    /// which is a refusal with no subject.
    Catalogue,
    /// Nothing in this vocabulary covers it, and the caller decides what that means.
    Unknown,
}

/// The three verbs, with any vendor prefix taken off.
///
/// A prefix and not a whole name, because the vendor chooses it: Claude Code publishes MCP tools
/// as `mcp__<server>__<tool>`, and the server name is the launch's. Matching the tail is what makes
/// the same function read a b10x record — where the verbs arrive bare — and a Claude one.
#[must_use]
pub fn unprefixed(tool: &str) -> &str {
    tool.strip_prefix(MCP_PREFIX)
        .and_then(|rest| rest.split_once("__"))
        .map_or(tool, |(_server, name)| name)
}

/// What this call is, when the run publishes the three verbs.
///
/// `None` means the call is not one of the verbs at all, and the caller should fall back to its
/// own rendering table — which is what a `native`-surface run does for every call.
///
/// # The check that stops a native run being fooled
///
/// This is only consulted when the run actually published the verbs. A `native` run whose model
/// invented a tool called `tool_invoke` must **not** have it read as whatever entry it names: the
/// resolution would be a claim about a tool that does not exist, and it would launder an unknown
/// call into a recognised operation. The caller enforces that by only asking under the owned
/// surface; this function does not guess.
#[must_use]
pub fn resolve_verb(tool: &str, input: &Value) -> Option<Resolved> {
    match unprefixed(tool) {
        SEARCH_VERB | DESCRIBE_VERB => Some(Resolved::Catalogue),
        INVOKE_VERB => Some(match input.get("name").and_then(Value::as_str) {
            Some(entry) => operation_of(entry).map_or(Resolved::Unknown, |operation| {
                Resolved::Operations(vec![operation.to_owned()])
            }),
            // A `tool_invoke` with no `name` never reached a tool — the verb refuses it. Reading it
            // as an operation would put an effect in the record that never happened.
            None => Resolved::Unknown,
        }),
        _ => None,
    }
}
