//! What a recorded tool call **does**, as semantic operations on its `trace-ir/1` node.
//!
//! A `tool_call` node already says which tool was called and with what input. This crate adds,
//! under [`FIELD`], the operations that input declares: for a shell call, one node per command the
//! shell runs, with its executable, its semantic kind (`file.read`, `stream.select`, `vcs.status`,
//! ...), the sequence, pipeline, loop and conditional structure around it, and the `bash -c`
//! unwrap. A Codex `exec` script is read the same way through its `tools.*` calls.
//!
//! The extractor is context-lens's (MIT; [`extract`]), parsed with tree-sitter and never executed.
//! It lives in its own crate because `metaharness-protocol` depends on four crates and no parser
//! (AGENTS.md invariant 1): the protocol projects the document and the production writers
//! (`metaharness::project_file`, `metaharness::project_vendor_record`) call [`annotate`] on it.
//! Decided by amendment a68 of `docs/design/metaharness-protocol-v0.1.md`; additive under decision
//! D3, so the document stays `trace-ir/1`.

pub mod extract;

pub use extract::{Node, VERSION, extract};

use metaharness_protocol::TraceIrDocument;
use serde_json::Value;

/// The key a `tool_call` node's operations are written under.
pub const FIELD: &str = "semantic_operations";

/// Write every `tool_call` node's semantic operations into the node, and name the rule.
///
/// The provider is the opening `session_start` node's `adapter`, and the working directory its
/// `cwd`, as the source reads them; a Codex call's own `workdir` overrides it. A tool input that is
/// a string (a Codex `exec` script) is read as that string, any other input as its JSON text,
/// which is what the source reads ("serialized native arguments"). Calling this twice writes the
/// same bytes: each call replaces the field it wrote.
pub fn annotate(document: &mut TraceIrDocument) {
    let mut provider = String::from("unknown");
    let mut cwd: Option<String> = None;
    for event in &mut document.events {
        let Some(kind) = event.kind.as_object_mut() else {
            continue;
        };
        match kind.get("event").and_then(Value::as_str) {
            Some("session_start") => {
                if let Some(adapter) = kind.get("adapter").and_then(Value::as_str) {
                    adapter.clone_into(&mut provider);
                }
                cwd = kind.get("cwd").and_then(Value::as_str).map(str::to_owned);
            }
            Some("tool_call") => {
                let name = kind
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                let input = match kind.get("input") {
                    Some(Value::String(script)) => script.clone(),
                    Some(other) => other.to_string(),
                    None => Value::Null.to_string(),
                };
                let nodes = extract(&provider, name, &input, cwd.as_deref());
                let nodes = serde_json::to_value(nodes).unwrap_or(Value::Null);
                kind.insert(FIELD.to_owned(), nodes);
            }
            _ => {}
        }
    }
    document.metaharness.semantic_rule = Some(VERSION);
}
