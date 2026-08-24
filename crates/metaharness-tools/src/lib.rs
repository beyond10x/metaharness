//! The owned tool surface, served to a vendor harness over MCP.
//!
//! # What this removes
//!
//! An evaluation arm that means to measure *these operations and no others* cannot, while the
//! vendor's own `Bash` is on the tool list: the model is offered a shell whatever the frame says,
//! and a seam can only deny it after the fact, one turn and one denial at a time. Design § 7.5
//! calls the way out **strategy C — own the tool surface**: `--tools ""` removes every built-in
//! tool, and an MCP server supplies the ones the run is meant to have. Under it there is no `Bash`
//! to deny, because there is no `Bash`.
//!
//! # Three verbs, not six tools
//!
//! What this server publishes is [`SEARCH_VERB`], [`DESCRIBE_VERB`] and [`INVOKE_VERB`] — the same
//! three the b10x harness binds in-process, over the same [`Catalogue`]. The vendor prefixes them,
//! so the model sees `mcp__metaharness__tool_search` and two others, and nothing else at all.
//!
//! That the *names are ours on every harness* is the point. The evaluation compares arms across
//! three harnesses that each name their tools differently, so everything downstream — the corpus
//! above all — had to learn one vendor's vocabulary or be blind to the rest.
//!
//! # The protocol, and why there is no MCP crate under this
//!
//! MCP over stdio is JSON-RPC 2.0, one object per line. This server answers four methods:
//!
//! | method | answer |
//! |---|---|
//! | `initialize` | the protocol version the client asked for, and a `tools` capability |
//! | `tools/list` | the three verbs, with their real input schemas |
//! | `tools/call` | the verb's own result, as one JSON text content block |
//! | `ping` | `{}` |
//!
//! A notification — a request with no `id` — is performed and not answered, which is the only part
//! of JSON-RPC that is easy to get wrong here. Everything else is `serde_json` and `BufRead`, and
//! this workspace links no async runtime; taking one, or an MCP framework that needs one, to move
//! four message shapes would be the largest dependency decision in the tree for the smallest reason.
//!
//! # The other half: reading a run back
//!
//! Serving the tools is only one direction. [`resolve_verb`] is the other — it takes a call off a
//! record and says what it *was*, in the same neutral vocabulary, so a consumer selects on
//! `file.write` whichever harness produced the run. See [`resolve`] for what that fixes.

mod resolve;
mod server;

pub use b10x_harness_tools::{
    Catalogue, DESCRIBE_VERB, INVOKE_VERB, LocalOperations, Operations, SEARCH_VERB, Verbs,
};
pub use resolve::{Resolved, operation_of, resolve_verb, unprefixed};
pub use server::{PROTOCOL_VERSION, SERVER_NAME, Server, serve};

#[cfg(test)]
mod tests;
