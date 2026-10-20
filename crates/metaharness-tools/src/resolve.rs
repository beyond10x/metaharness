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
//!
//! # A contract name is the entry harness maps it to — where metaharness serves the verbs
//!
//! On metaharness's own server the entry a verb names may be spelled as its `tool-contract/1`
//! command (`fs.read`, `code.write`) as well as by its harness name (`file_read`, `file_write`).
//! Which contract name stands for which entry is harness's to say —
//! [`harness_catalogue::harness_operation`] — and this crate keeps no table of its own. The b10x
//! loop metaharness launches publishes its entries flat by default (`--surface flat`, harness
//! `crates/harness-cli/src/lib.rs:744`), and at the pinned release answers harness names only under
//! either surface, so on its record a contract name reached no tool. [`VerbSurface`] says which of the two a call went
//! through, and resolution and subjects take the same one, so a contract name cannot resolve to an
//! operation while naming no subject: a call that names none is never judged on where it acts, and
//! a frame's subject scope would let it through with any path. What is *served* keeps harness's
//! names; the b10x run publishes the same catalogue, and two vocabularies would reach the model.

use serde_json::Value;

pub use harness_catalogue::operation_of;

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

/// The catalogue entry a verb's `name` stands for.
///
/// A `tool-contract/1` command name harness maps is the entry it maps to; any other name is itself,
/// so a harness entry name resolves as it always did and an unmapped name stays whatever the
/// catalogue makes of it — unknown. Contract names are dotted and entry names are not, so the two
/// cannot collide.
pub(crate) fn entry_of(name: &str) -> &str {
    harness_catalogue::harness_operation(name).unwrap_or(name)
}

/// Whose three verbs a call went through, which decides what a contract name in it was.
///
/// The verbs are the same three on every run that publishes them; what answers them is not.
/// metaharness's own [`Server`](crate::Server) performs a `tool-contract/1` name harness maps as
/// that entry. The b10x loop's verbs at the pinned release look an entry up by harness name only,
/// so there a contract name was refused and reached no tool — and reading it as its entry would
/// put an effect in the record that never happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerbSurface {
    /// Served by metaharness over MCP (`--tool-surface owned`): a contract name harness maps is
    /// its entry.
    Served,
    /// A b10x record: the harness loop answers harness names only (flat by default, and its own
    /// verbs under `--surface verbs` look an entry up the same way), so a contract name is outside
    /// the vocabulary.
    HarnessLoop,
}

impl VerbSurface {
    /// The catalogue entry a verb's `name` stands for on this surface.
    fn entry(self, name: &str) -> &str {
        match self {
            Self::Served => entry_of(name),
            Self::HarnessLoop => name,
        }
    }
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
///
/// `surface` is whose verbs answered the call, and the same rule keeps it honest: a contract name
/// is an operation only where it was performed as one ([`VerbSurface`]).
#[must_use]
pub fn resolve_verb(tool: &str, input: &Value, surface: VerbSurface) -> Option<Resolved> {
    match unprefixed(tool) {
        SEARCH_VERB | DESCRIBE_VERB => Some(Resolved::Catalogue),
        INVOKE_VERB => Some(match input.get("name").and_then(Value::as_str) {
            Some(entry) => operation_of(surface.entry(entry))
                .map_or(Resolved::Unknown, |operation| {
                    Resolved::Operations(vec![operation.to_owned()])
                }),
            // A `tool_invoke` with no `name` never reached a tool — the verb refuses it. Reading it
            // as an operation would put an effect in the record that never happened.
            None => Resolved::Unknown,
        }),
        _ => None,
    }
}

/// What this call would touch, when the run publishes the three verbs.
///
/// The other half of [`resolve_verb`]. `operations` says *a write happened*; this says *which
/// file*, and neither implies the other: without both, an expectation can assert that a run wrote
/// something and never that it wrote the test before the source.
///
/// Read from the catalogue's own rule rather than from the arguments directly, so a reader of a
/// finished run and a live gate answer the same question the same way.
///
/// Empty for a catalogue question, for an entry outside the vocabulary, and for a tool that is not
/// one of the verbs — in each case there is no entry whose arguments could name a subject. Pass the
/// `surface` [`resolve_verb`] was given: a name that resolved to an operation names its subject.
#[must_use]
pub fn subjects_of_verb(tool: &str, input: &Value, surface: VerbSurface) -> Vec<String> {
    if unprefixed(tool) != INVOKE_VERB {
        return Vec::new();
    }
    let Some(entry) = input.get("name").and_then(Value::as_str) else {
        return Vec::new();
    };
    let arguments = input.get("arguments").cloned().unwrap_or(Value::Null);
    harness_catalogue::subjects_of(surface.entry(entry), &arguments)
        .into_iter()
        .map(|subject| subject.as_str().to_owned())
        .collect()
}

/// What a **vendor's own** tool call would touch, from the argument a path is recorded under.
///
/// Claude Code writes `file_path`, its notebook editor writes `notebook_path`, and several tools
/// write a bare `path`. Three names and no vendor table: this is *which key holds a path*, not
/// *which tool is a write* — the second is the rendering's job and has one owner already.
///
/// **`command` is deliberately not read.** A shell string is not a program, and pulling an argv[0]
/// out of one is parsing a language this crate does not speak; a `proc:` subject guessed that way
/// would be a claim about what ran. An expectation about a command still reads `args.command`,
/// which is exactly as decidable as it was.
#[must_use]
pub fn subjects_of_vendor_call(input: &Value) -> Vec<String> {
    const PATH_ARGUMENTS: [&str; 3] = ["file_path", "notebook_path", "path"];
    PATH_ARGUMENTS
        .iter()
        .filter_map(|key| input.get(*key).and_then(Value::as_str))
        .map(|path| harness_catalogue::Subject::file(path).as_str().to_owned())
        .collect()
}
