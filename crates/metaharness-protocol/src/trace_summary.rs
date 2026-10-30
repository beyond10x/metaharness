//! `trace-ir-summary/1`: the one projection of a `trace-ir/1` document (design amendment a42).
//!
//! The surviving trace formats beside `trace-ir/1` (takt's harness-step rows, Loom's jsonl for
//! agent work) read this summary rather than a projection shaped for each, and reach it through
//! `metaharness project --to trace-ir-summary`. It is a function of the document and of nothing
//! else: no clock, no file, no environment, and never the event stream or the vendor record behind
//! the document. A value the document does not state is `null`, never `0` and never `""`
//! (invariant 3: absence of evidence is not a property).

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::projection::IrFamily;
use crate::trace_ir::{TraceIrDocument, TraceIrEvent};

/// The `format` every summary carries.
pub const SUMMARY_FORMAT: &str = "trace-ir-summary/1";

/// One `tool_call` node of the document, in document order (a42 point 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummarizedToolCall {
    /// The node's own `call_id`.
    pub call_id: Option<String>,
    /// The node's own `name`: the harness tool that carried the call (`Bash`, `exec_command`,
    /// `Read`), kept as provenance. It is not what the call ran; [`Self::tools`] is.
    pub name: Option<String>,
    /// What the call ran, normalized (amendment a69): the `tool` of each operation in the node's
    /// `semantic_operations`, in order, read off the node and never off its raw `input`. A shell
    /// call names its programs with wrappers unwrapped and glue dropped, never the shell tool;
    /// an operation that names no program and was not read (`resolution: unsupported`) is `null`.
    /// `None` when the node carries no `semantic_operations`.
    pub tools: Option<Vec<Option<String>>>,
    /// Whether any `tool_result` node carries the same `call_id`. A call without a string id is
    /// answered by nothing, because nothing can name it.
    pub answered: bool,
    /// The first such result's `is_error`: `null` both when nothing answered the call and when the
    /// result did not say, which [`Self::answered`] tells apart.
    pub error: Option<bool>,
}

/// How many operations of the document's tool calls name one tool, or one semantic kind
/// (amendment a69).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SummarizedCount {
    /// The normalized tool or the semantic kind; `null` counts the operations whose program
    /// could not be read.
    pub name: Option<String>,
    /// The operations that name it.
    pub operations: u64,
}

/// One model the document names, with the figures the document states for it alone (a42 point 2).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SummarizedModel {
    /// The model's name, as the document spells it.
    pub name: String,
    /// The terminal node's `model_usage[name].input_tokens`.
    pub input_tokens: Option<u64>,
    /// The terminal node's `model_usage[name].output_tokens`.
    pub output_tokens: Option<u64>,
    /// The terminal node's `model_usage[name].cache_read_input_tokens`.
    pub cache_read_input_tokens: Option<u64>,
    /// The terminal node's `model_usage[name].cache_creation_input_tokens`.
    pub cache_creation_input_tokens: Option<u64>,
    /// The terminal node's `model_usage[name].cost_usd`, the vendor's float passed through.
    pub cost_usd: Option<f64>,
}

/// Which nodes the summary's four token counts were read from (a42 point 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum TokensBasis {
    /// The terminal node's `usage`, its totals taken as written.
    Session,
    /// The request nodes' `usage`, summed per key, because the terminal node carries none.
    SumOfRequests,
}

/// A `trace-ir/1` document, summarized. Rendered in this field order (a42 point 4).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceIrSummary {
    /// Always [`SUMMARY_FORMAT`].
    pub format: &'static str,
    /// The document's `metaharness.source`: what the document was made from.
    pub source: &'static str,
    /// The document's own `transcript_digest`, which names the document's input bytes.
    pub transcript_digest: String,
    /// The opening `session_start` node's `adapter`.
    pub adapter: Option<String>,
    /// The opening `session_start` node's `model`. Never the per-turn selections a terminal
    /// record lists: those are under [`Self::models`].
    pub model: Option<String>,
    /// Every model the document names, first-seen order, no repeats.
    pub models: Vec<SummarizedModel>,
    /// The terminal node's `num_turns`.
    pub turns: Option<u64>,
    /// Where the four token counts come from; `None` when nothing stated a `usage`.
    pub tokens_basis: Option<TokensBasis>,
    /// `usage.input_tokens`, read as [`Self::tokens_basis`] says. Not normalized: whether cached
    /// input is inside it is the vendor's definition (a42 point 3).
    pub input_tokens: Option<u64>,
    /// `usage.output_tokens`, read as [`Self::tokens_basis`] says.
    pub output_tokens: Option<u64>,
    /// `usage.cache_read_input_tokens`, read as [`Self::tokens_basis`] says.
    pub cache_read_input_tokens: Option<u64>,
    /// `usage.cache_creation_input_tokens`, read as [`Self::tokens_basis`] says.
    pub cache_creation_input_tokens: Option<u64>,
    /// The terminal node's `total_cost_usd`, the vendor's float passed through.
    pub cost_usd: Option<f64>,
    /// The terminal node's `duration_ms`.
    pub duration_ms: Option<u64>,
    /// The terminal node's `duration_api_ms`.
    pub duration_api_ms: Option<u64>,
    /// The first node `timestamp_ms` that is not `null`, in document order.
    pub first_timestamp_ms: Option<i64>,
    /// The latest node `timestamp_ms` that is not `null`: where the document's order is not the
    /// clock's (a session's sub-agent blocks follow its own records), the record ends at its latest
    /// stamp, not its last one (ADV-SJ-1).
    pub last_timestamp_ms: Option<i64>,
    /// The terminal node's `is_error`.
    pub is_error: Option<bool>,
    /// The terminal node's `stop_reason`.
    pub stop_reason: Option<String>,
    /// One entry per `tool_call` node, in document order.
    pub tool_calls: Vec<SummarizedToolCall>,
    /// The tools the calls ran, normalized: every entry of every [`SummarizedToolCall::tools`],
    /// counted per name, most operations first, then by name. A shell tool such as `Bash` or
    /// `exec_command` is never counted as a tool; its programs are. `None` when the document
    /// names no `metaharness.semantic_rule`, so its calls were never read (amendment a69).
    pub tools: Option<Vec<SummarizedCount>>,
    /// The same operations counted per semantic `kind` (`file.read`, `vcs.status`, ...), in the
    /// same order; `None` as [`Self::tools`] is.
    pub operation_kinds: Option<Vec<SummarizedCount>>,
    /// The document's unk count: the sum over `metaharness.unk_kinds`.
    pub unk: u64,
}

impl TraceIrSummary {
    /// The summary as bytes: pretty-printed in field order with a trailing newline, the form the
    /// document itself is written in, so the same document renders the same bytes every time.
    ///
    /// # Errors
    ///
    /// A cost that is not finite, at the top level or in a `models` entry. `serde_json` would
    /// write it as `null`, which reads as a document that stated no cost (invariant 3). A
    /// document carries none, because its own render refuses it first; an embedder holding a
    /// summary it changed can.
    pub fn render(&self) -> Result<String, serde_json::Error> {
        let costs = std::iter::once(self.cost_usd).chain(self.models.iter().map(|m| m.cost_usd));
        if let Some(cost) = costs.flatten().find(|cost| !cost.is_finite()) {
            return Err(<serde_json::Error as serde::ser::Error>::custom(format!(
                "a summary cost of {cost} has no JSON spelling, and writing it as null would \
                 read as a document that stated no cost"
            )));
        }
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// Summarize one `trace-ir/1` document (a42).
///
/// The *terminal node* is the last `run_outcome` node carrying the key `num_turns`, which
/// `session.ended` always writes and the `usage` fold into `run_outcome` never does. The *opening
/// node* is node 0 when its family is `session_start`.
#[must_use]
pub fn summarize(document: &TraceIrDocument) -> TraceIrSummary {
    let opening = document
        .events
        .first()
        .map(|node| &node.kind)
        .filter(|node| family_is(node, IrFamily::SessionStart));
    let terminal = document
        .events
        .iter()
        .rev()
        .map(|node| &node.kind)
        .find(|node| family_is(node, IrFamily::RunOutcome) && node.get("num_turns").is_some());
    let text = |node: Option<&Value>, key: &str| {
        node.and_then(|node| node[key].as_str())
            .map(ToOwned::to_owned)
    };
    let count = |node: Option<&Value>, key: &str| node.and_then(|node| node[key].as_u64());
    let (tokens_basis, [input, output, cache_read, cache_creation]) =
        token_counts(&document.events, terminal);
    let stamps = || document.events.iter().filter_map(|node| node.timestamp_ms);

    // The first answer per call id, in document order: a call is answered by the first result
    // naming it, and that result's verdict is the one the summary keeps.
    let mut answers: BTreeMap<&str, Option<bool>> = BTreeMap::new();
    for node in &document.events {
        if family_is(&node.kind, IrFamily::ToolResult)
            && let Some(call_id) = node.kind["call_id"].as_str()
        {
            answers
                .entry(call_id)
                .or_insert_with(|| node.kind["is_error"].as_bool());
        }
    }
    let tool_calls = document
        .events
        .iter()
        .map(|node| &node.kind)
        .filter(|node| family_is(node, IrFamily::ToolCall))
        .map(|node| {
            let answer = node["call_id"]
                .as_str()
                .and_then(|call_id| answers.get(call_id));
            SummarizedToolCall {
                call_id: text(Some(node), "call_id"),
                name: text(Some(node), "name"),
                tools: ran(node).map(|ran| ran.into_iter().map(|(tool, _)| tool).collect()),
                answered: answer.is_some(),
                error: answer.copied().flatten(),
            }
        })
        .collect();
    let (tools, operation_kinds) = if document.metaharness.semantic_rule.is_some() {
        let ran: Vec<(Option<String>, Option<String>)> = document
            .events
            .iter()
            .map(|node| &node.kind)
            .filter(|node| family_is(node, IrFamily::ToolCall))
            .filter_map(ran)
            .flatten()
            .collect();
        (
            Some(counted(ran.iter().map(|(tool, _)| tool.clone()))),
            Some(counted(ran.into_iter().map(|(_, kind)| kind))),
        )
    } else {
        (None, None)
    };

    TraceIrSummary {
        format: SUMMARY_FORMAT,
        source: document.metaharness.source,
        transcript_digest: document.transcript_digest.clone(),
        adapter: text(opening, "adapter"),
        model: text(opening, "model"),
        models: models(&document.events, terminal),
        turns: count(terminal, "num_turns"),
        tokens_basis,
        input_tokens: input,
        output_tokens: output,
        cache_read_input_tokens: cache_read,
        cache_creation_input_tokens: cache_creation,
        cost_usd: terminal.and_then(|node| node["total_cost_usd"].as_f64()),
        duration_ms: count(terminal, "duration_ms"),
        duration_api_ms: count(terminal, "duration_api_ms"),
        first_timestamp_ms: stamps().next(),
        last_timestamp_ms: stamps().max(),
        is_error: terminal.and_then(|node| node["is_error"].as_bool()),
        stop_reason: text(terminal, "stop_reason"),
        tool_calls,
        tools,
        operation_kinds,
        unk: document
            .metaharness
            .unk_kinds
            .values()
            .map(|count| u64::from(*count))
            .sum(),
    }
}

/// What one `tool_call` node ran (amendment a69): per operation of its `semantic_operations`
/// that is not a structure node, its normalized `tool` and its `kind`. An operation without a
/// `tool` is glue (a builtin such as `cd`, a redirection) and is left out, unless it names no
/// program and was not read (`resolution: unsupported`), which stays as an unknown `null`. Read off the
/// operations alone: the node's raw `input` is provenance, and no summary reads it. `None` when
/// the node carries no operations.
fn ran(node: &Value) -> Option<Vec<(Option<String>, Option<String>)>> {
    let operations = node["semantic_operations"].as_array()?;
    Some(
        operations
            .iter()
            .filter(|operation| operation["container"] != true)
            .filter_map(|operation| {
                let tool = operation["tool"].as_str().map(ToOwned::to_owned);
                let unread =
                    operation["executable"].is_null() && operation["resolution"] == "unsupported";
                if tool.is_none() && !unread {
                    return None;
                }
                Some((tool, operation["kind"].as_str().map(ToOwned::to_owned)))
            })
            .collect(),
    )
}

/// Count names, most first, then by name (`null` before any name).
fn counted(names: impl Iterator<Item = Option<String>>) -> Vec<SummarizedCount> {
    let mut counts: BTreeMap<Option<String>, u64> = BTreeMap::new();
    for name in names {
        *counts.entry(name).or_default() += 1;
    }
    let mut counted: Vec<SummarizedCount> = counts
        .into_iter()
        .map(|(name, operations)| SummarizedCount { name, operations })
        .collect();
    counted.sort_by(|a, b| b.operations.cmp(&a.operations).then(a.name.cmp(&b.name)));
    counted
}

/// The four `usage` keys the summary carries, in its field order.
const TOKEN_KEYS: [&str; 4] = [
    "input_tokens",
    "output_tokens",
    "cache_read_input_tokens",
    "cache_creation_input_tokens",
];

fn family_is(node: &Value, family: IrFamily) -> bool {
    node["event"] == family.as_str()
}

/// The token counts and the nodes they were read from (a42 point 3). The terminal node's `usage`
/// when it is an object; otherwise, and only when there is a terminal node, the sum over the
/// request nodes (`run_outcome` without `num_turns`) whose `usage` is an object, per key, `None`
/// where no request node states the key. A document without a terminal node did not close, and a
/// Codex rollout that did not close carries running totals a sum would multiply.
fn token_counts(
    nodes: &[TraceIrEvent],
    terminal: Option<&Value>,
) -> (Option<TokensBasis>, [Option<u64>; 4]) {
    let Some(terminal) = terminal else {
        return (None, [None; 4]);
    };
    let usage = &terminal["usage"];
    if usage.is_object() {
        return (
            Some(TokensBasis::Session),
            TOKEN_KEYS.map(|key| usage[key].as_u64()),
        );
    }
    let requests: Vec<&Value> = nodes
        .iter()
        .map(|node| &node.kind)
        .filter(|node| family_is(node, IrFamily::RunOutcome) && node.get("num_turns").is_none())
        .map(|node| &node["usage"])
        .filter(|usage| usage.is_object())
        .collect();
    if requests.is_empty() {
        return (None, [None; 4]);
    }
    let sum = |key: &str| {
        let mut stated = requests
            .iter()
            .filter_map(|usage| usage[key].as_u64())
            .peekable();
        // A key no request states is not a zero; a sum past `u64` is no figure either.
        stated.peek()?;
        stated.try_fold(0_u64, u64::checked_add)
    };
    (Some(TokensBasis::SumOfRequests), TOKEN_KEYS.map(sum))
}

/// Every model the document names, in first-seen order without repeats (a42 point 2): per
/// `session_start` or `run_outcome` node, its `model`, then its `model_usage` keys, then its
/// `observed_models` entries' `model`. The figures are the terminal node's `model_usage[name]`
/// and nothing else: a request node's `usage` is one call, never a model's share.
fn models(nodes: &[TraceIrEvent], terminal: Option<&Value>) -> Vec<SummarizedModel> {
    let mut names: Vec<&str> = Vec::new();
    let naming = nodes.iter().map(|node| &node.kind).filter(|node| {
        family_is(node, IrFamily::SessionStart) || family_is(node, IrFamily::RunOutcome)
    });
    for node in naming {
        let usage_keys = node["model_usage"]
            .as_object()
            .into_iter()
            .flat_map(|by_model| by_model.keys().map(String::as_str));
        let observed = node["observed_models"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|entry| entry["model"].as_str());
        for name in node["model"]
            .as_str()
            .into_iter()
            .chain(usage_keys)
            .chain(observed)
        {
            if !names.contains(&name) {
                names.push(name);
            }
        }
    }
    names
        .into_iter()
        .map(|name| {
            let entry = terminal.map(|node| &node["model_usage"][name]);
            let figure = |key: &str| entry.and_then(|entry| entry[key].as_u64());
            SummarizedModel {
                name: name.to_owned(),
                input_tokens: figure("input_tokens"),
                output_tokens: figure("output_tokens"),
                cache_read_input_tokens: figure("cache_read_input_tokens"),
                cache_creation_input_tokens: figure("cache_creation_input_tokens"),
                cost_usd: entry.and_then(|entry| entry["cost_usd"].as_f64()),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::event::Event;
    use crate::framing::{EVENT_FORMAT, EventLine, RunId};
    use crate::trace_ir::project_document;

    /// A real document from the production writer whose nodes are then replaced, so each test
    /// states exactly the nodes it reads.
    fn document_of(nodes: Vec<(Option<i64>, Value)>) -> TraceIrDocument {
        let mut document = project_document(
            &[EventLine {
                format: EVENT_FORMAT.to_string(),
                seq: 1,
                run: RunId::new("t"),
                at: None,
                event: Event::Text {
                    text: "x".to_string(),
                    request_id: None,
                },
            }],
            b"stream",
        );
        document.events = nodes
            .into_iter()
            .enumerate()
            .map(|(index, (timestamp_ms, kind))| TraceIrEvent {
                index,
                source_line: index + 1,
                timestamp: timestamp_ms.map(|ms| ms.to_string()),
                timestamp_ms,
                kind,
            })
            .collect();
        document
    }

    fn terminal(fields: &Value) -> Value {
        let mut node = json!({
            "event": "run_outcome",
            "num_turns": null,
            "usage": null,
            "total_cost_usd": null,
            "duration_ms": null,
            "duration_api_ms": null,
            "is_error": null,
            "stop_reason": null,
        });
        for (key, value) in fields.as_object().expect("an object") {
            node[key] = value.clone();
        }
        node
    }

    /// Invariant 3: a document that states nothing summarizes to `null` everywhere it could, never
    /// to `0` or `""`; and a stated `0` stays `0`, because a zero the vendor wrote is evidence.
    #[test]
    fn absent_stays_null_and_a_stated_zero_stays_zero() {
        let silent = document_of(vec![
            (
                None,
                json!({"event": "session_start", "adapter": "codex", "model": null}),
            ),
            (None, terminal(&json!({}))),
        ]);
        let summary = summarize(&silent);
        let rendered: Value = serde_json::from_str(&summary.render().unwrap()).unwrap();
        for key in [
            "model",
            "turns",
            "tokens_basis",
            "input_tokens",
            "output_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "cost_usd",
            "duration_ms",
            "duration_api_ms",
            "first_timestamp_ms",
            "last_timestamp_ms",
            "is_error",
            "stop_reason",
        ] {
            assert!(rendered[key].is_null(), "{key} is {}", rendered[key]);
        }
        assert_eq!(rendered["adapter"], "codex");
        assert_eq!(rendered["tool_calls"], json!([]));
        assert_eq!(rendered["models"], json!([]), "a null model names nothing");
        assert_eq!(rendered["unk"], 0);

        let empty = summarize(&document_of(vec![]));
        assert_eq!(empty.adapter, None, "no opening node, no adapter");
        assert_eq!(empty.turns, None, "no terminal node, no turns");

        let not_opening = summarize(&document_of(vec![(
            None,
            json!({"event": "assistant_text", "adapter": "claude", "model": "m"}),
        )]));
        assert_eq!(not_opening.adapter, None, "node 0 is not session_start");
        assert_eq!(not_opening.model, None);

        let zero = summarize(&document_of(vec![(
            Some(5),
            terminal(&json!({
                "num_turns": 0,
                "usage": {"input_tokens": 0, "output_tokens": 0},
                "total_cost_usd": 0.0,
                "is_error": false,
            })),
        )]));
        assert_eq!(zero.turns, Some(0));
        assert_eq!(zero.input_tokens, Some(0));
        assert_eq!(zero.cache_read_input_tokens, None, "the usage did not say");
        assert_eq!(zero.cost_usd, Some(0.0));
        assert_eq!(zero.is_error, Some(false));
        assert_eq!(zero.first_timestamp_ms, Some(5));
        assert_eq!(zero.last_timestamp_ms, Some(5));
    }

    /// The terminal node is the session close, not a later usage fold, and its totals are taken as
    /// written: per-request usage is never added up.
    #[test]
    fn the_terminal_node_is_the_last_run_outcome_carrying_num_turns() {
        let document = document_of(vec![
            (
                Some(10),
                json!({"event": "session_start", "adapter": "claude", "model": "m"}),
            ),
            (
                None,
                json!({"event": "run_outcome", "request_id": "r1", "model": "m", "usage": {"output_tokens": 20}}),
            ),
            (
                Some(30),
                terminal(
                    &json!({"num_turns": 2, "usage": {"output_tokens": 96}, "total_cost_usd": 0.25, "stop_reason": "end_turn"}),
                ),
            ),
            (
                Some(20),
                json!({"event": "run_outcome", "request_id": "r2", "model": "m", "usage": {"output_tokens": 1}}),
            ),
            (None, json!({"event": "unk", "kind": "turn.started"})),
        ]);
        let summary = summarize(&document);
        assert_eq!(summary.turns, Some(2));
        assert_eq!(summary.tokens_basis, Some(TokensBasis::Session));
        assert_eq!(
            summary.output_tokens,
            Some(96),
            "the record's total, not 20 + 96 + 1"
        );
        assert_eq!(summary.cost_usd, Some(0.25));
        assert_eq!(summary.stop_reason.as_deref(), Some("end_turn"));
        assert_eq!(summary.model.as_deref(), Some("m"));
        assert_eq!(summary.first_timestamp_ms, Some(10));
        assert_eq!(
            summary.last_timestamp_ms,
            Some(30),
            "the latest stated timestamp, not the last in document order: a document whose \
             order is not the clock's (a session's sub-agent blocks follow its own records) \
             ends at its latest record (ADV-SJ-1)"
        );
    }

    /// A call nothing answered reads `answered: false, error: null`; an answer that did not say
    /// reads `answered: true, error: null`; the first answer's verdict is the one kept.
    #[test]
    fn a_tool_call_with_no_result_has_error_null() {
        let document = document_of(vec![
            (
                None,
                json!({"event": "tool_call", "call_id": "c1", "name": "Read"}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c2", "name": "Bash"}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c3", "name": "Edit"}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": null, "name": "Write"}),
            ),
            (
                None,
                json!({"event": "tool_result", "call_id": "c2", "is_error": true}),
            ),
            (
                None,
                json!({"event": "tool_result", "call_id": "c2", "is_error": false}),
            ),
            (
                None,
                json!({"event": "tool_result", "call_id": "c3", "is_error": null}),
            ),
            (
                None,
                json!({"event": "tool_result", "call_id": "c9", "is_error": true}),
            ),
            (
                None,
                json!({"event": "tool_result", "call_id": null, "is_error": true}),
            ),
        ]);
        let calls = summarize(&document).tool_calls;
        let call = |call_id: Option<&str>, name: &str, answered: bool, error: Option<bool>| {
            SummarizedToolCall {
                call_id: call_id.map(ToOwned::to_owned),
                name: Some(name.to_owned()),
                tools: None,
                answered,
                error,
            }
        };
        assert_eq!(
            calls,
            vec![
                call(Some("c1"), "Read", false, None),
                call(Some("c2"), "Bash", true, Some(true)),
                call(Some("c3"), "Edit", true, None),
                call(None, "Write", false, None),
            ]
        );
    }

    /// a42 point 4: the same document renders the same bytes, keys in the struct's order, one
    /// trailing newline; and the summary names the document by its digest and its source.
    #[test]
    fn the_render_is_byte_stable_and_names_the_document() {
        let document = document_of(vec![
            (
                Some(1),
                json!({"event": "session_start", "adapter": "claude", "model": "m"}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c1", "name": "Read"}),
            ),
            (
                Some(2),
                terminal(&json!({"num_turns": 1, "total_cost_usd": 0.238_876})),
            ),
        ]);
        let first = summarize(&document).render().unwrap();
        assert_eq!(first, summarize(&document).render().unwrap());
        assert_eq!(first, summarize(&document.clone()).render().unwrap());
        assert!(first.ends_with("}\n") && !first.ends_with("\n\n"));
        let keys = [
            "format",
            "source",
            "transcript_digest",
            "adapter",
            "model",
            "models",
            "turns",
            "tokens_basis",
            "input_tokens",
            "output_tokens",
            "cache_read_input_tokens",
            "cache_creation_input_tokens",
            "cost_usd",
            "duration_ms",
            "duration_api_ms",
            "first_timestamp_ms",
            "last_timestamp_ms",
            "is_error",
            "stop_reason",
            "tool_calls",
            "unk",
        ];
        let positions: Vec<usize> = keys
            .iter()
            .map(|key| first.find(&format!("\n  \"{key}\":")).expect(key))
            .collect();
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(first.contains("\"cost_usd\": 0.238876,"), "{first}");
        let summary = summarize(&document);
        assert_eq!(summary.format, SUMMARY_FORMAT);
        assert_eq!(summary.source, document.metaharness.source);
        assert_eq!(summary.transcript_digest, document.transcript_digest);
    }

    /// a42 point 3: a terminal node without `usage` takes the request nodes' `usage`, summed per
    /// key, wherever in the document they stand; a key no request states stays `null`; a
    /// document without a terminal node, or without any request usage, has no basis at all.
    #[test]
    fn a_terminal_node_without_usage_sums_the_requests() {
        let request = |usage: Value| json!({"event": "run_outcome", "request_id": "r", "model": "m", "usage": usage});
        let document = document_of(vec![
            (
                None,
                request(json!({"input_tokens": 40, "output_tokens": 5})),
            ),
            (
                None,
                request(
                    json!({"input_tokens": 44, "output_tokens": 7, "cache_read_input_tokens": 14}),
                ),
            ),
            (None, request(Value::Null)),
            (None, terminal(&json!({"num_turns": 2}))),
            (None, request(json!({"input_tokens": 0}))),
        ]);
        let summary = summarize(&document);
        assert_eq!(summary.tokens_basis, Some(TokensBasis::SumOfRequests));
        assert_eq!(summary.input_tokens, Some(84));
        assert_eq!(summary.output_tokens, Some(12));
        assert_eq!(summary.cache_read_input_tokens, Some(14));
        assert_eq!(summary.cache_creation_input_tokens, None, "no request said");
        let rendered: Value = serde_json::from_str(&summary.render().unwrap()).unwrap();
        assert_eq!(rendered["tokens_basis"], "sum-of-requests");

        let no_terminal = summarize(&document_of(vec![(
            None,
            request(json!({"input_tokens": 40})),
        )]));
        assert_eq!(no_terminal.tokens_basis, None, "a run that did not close");
        assert_eq!(no_terminal.input_tokens, None);

        let nothing = summarize(&document_of(vec![
            (None, request(Value::Null)),
            (None, terminal(&json!({"num_turns": 1}))),
        ]));
        assert_eq!(nothing.tokens_basis, None);
        assert_eq!(nothing.input_tokens, None);

        let overflow = summarize(&document_of(vec![
            (None, request(json!({"input_tokens": u64::MAX}))),
            (None, request(json!({"input_tokens": 1}))),
            (None, terminal(&json!({"num_turns": 1}))),
        ]));
        assert_eq!(overflow.tokens_basis, Some(TokensBasis::SumOfRequests));
        assert_eq!(overflow.input_tokens, None, "a sum past u64 is no figure");
    }

    /// a42 point 2: every model the document names, first-seen order, no repeats, with figures
    /// only from the terminal node's `model_usage`; a request's `usage` is never a model's share.
    #[test]
    fn models_names_every_model_once_with_the_terminal_figures() {
        let document = document_of(vec![
            (
                None,
                json!({"event": "session_start", "adapter": "claude", "model": "big"}),
            ),
            (
                None,
                json!({"event": "run_outcome", "request_id": "r1", "model": "small", "usage": {"input_tokens": 9}}),
            ),
            (
                None,
                terminal(&json!({
                    "num_turns": 1,
                    "model_usage": {
                        "big[1m]": {"input_tokens": 4, "output_tokens": 96, "cost_usd": 0.237_784},
                        "small": {"input_tokens": 1032, "cache_read_input_tokens": 0, "cost_usd": 0.001_092},
                    },
                    "observed_models": [{"model": "big"}, {"model": "seen"}],
                })),
            ),
        ]);
        let models = summarize(&document).models;
        let names: Vec<&str> = models.iter().map(|model| model.name.as_str()).collect();
        assert_eq!(names, ["big", "small", "big[1m]", "seen"]);
        assert_eq!(models[0].input_tokens, None, "big states nothing per model");
        assert_eq!(models[1].input_tokens, Some(1032), "not the request's 9");
        assert_eq!(models[1].cache_read_input_tokens, Some(0));
        assert_eq!(models[1].output_tokens, None);
        assert_eq!(models[1].cost_usd, Some(0.001_092));
        assert_eq!(models[2].output_tokens, Some(96));
        assert_eq!(models[2].cost_usd, Some(0.237_784));
        assert_eq!(models[3].cost_usd, None);
    }

    /// The render refuses a non-finite cost, at the top level and per model, rather than write
    /// `null`, which reads as a document that stated no cost.
    #[test]
    fn the_render_refuses_a_non_finite_cost() {
        let document = document_of(vec![
            (
                None,
                json!({"event": "session_start", "adapter": "claude", "model": "m"}),
            ),
            (None, terminal(&json!({"num_turns": 1}))),
        ]);
        for cost in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let mut top = summarize(&document);
            top.cost_usd = Some(cost);
            let error = top.render().expect_err("a non-finite cost renders nothing");
            assert!(error.to_string().contains("no JSON spelling"), "{error}");

            let mut per_model = summarize(&document);
            per_model.models[0].cost_usd = Some(cost);
            assert!(per_model.render().is_err(), "{cost} in models");
        }
    }

    /// The unk count is the document's own, summed over the kinds it names.
    #[test]
    fn unk_is_the_documents_unk_count() {
        let mut document = document_of(vec![]);
        document
            .metaharness
            .unk_kinds
            .extend([("turn.started".to_string(), 2), ("warning".to_string(), 1)]);
        assert_eq!(summarize(&document).unk, 3);
    }

    fn operation(tool: Option<&str>, kind: &str, resolution: &str, container: bool) -> Value {
        json!({"tool": tool, "kind": kind, "resolution": resolution, "container": container})
    }

    fn annotated_calls() -> TraceIrDocument {
        let mut document = document_of(vec![
            (
                None,
                json!({"event": "tool_call", "call_id": "c1", "name": "Bash",
                "input": {"command": "cd src && timeout 5 rg -n x . | head -n 3"},
                "semantic_operations": [
                    operation(None, "sequence", "classified", true),
                    json!({"tool": null, "executable": "cd", "kind": "process.run", "resolution": "unsupported", "container": false}),
                    operation(None, "pipeline", "classified", true),
                    operation(Some("rg"), "file.search", "classified", false),
                    operation(Some("head"), "stream.select", "classified", false),
                ]}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c2", "name": "exec_command",
                "input": {"cmd": "cat 'oops"},
                "semantic_operations": [
                    operation(None, "wrapper", "classified", true),
                    operation(None, "other", "unsupported", false),
                ]}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c3", "name": "Read",
                    "input": {"file_path": "/repo/a"},
                    "semantic_operations": [operation(Some("Read"), "file.read", "classified", false)]}),
            ),
            (
                None,
                json!({"event": "tool_call", "call_id": "c4", "name": "Bash",
                    "input": {"command": "rg y"},
                    "semantic_operations": [operation(Some("rg"), "file.search", "classified", false)]}),
            ),
        ]);
        document.metaharness.semantic_rule = Some("metaharness-semantic/1");
        document
    }

    /// a69: a shell call is summarized by the programs it ran, never by the shell tool that
    /// carried it; glue is dropped and an unreadable program is an unknown `null`.
    #[test]
    fn a_shell_call_counts_its_programs_never_the_shell_tool() {
        let summary = summarize(&annotated_calls());
        let per_call: Vec<_> = summary
            .tool_calls
            .iter()
            .map(|call| call.tools.clone())
            .collect();
        let named = |names: &[Option<&str>]| {
            Some(
                names
                    .iter()
                    .map(|name| name.map(ToOwned::to_owned))
                    .collect::<Vec<_>>(),
            )
        };
        assert_eq!(
            per_call,
            [
                named(&[Some("rg"), Some("head")]),
                named(&[None]),
                named(&[Some("Read")]),
                named(&[Some("rg")]),
            ]
        );
        let count = |name: Option<&str>, operations: u64| SummarizedCount {
            name: name.map(ToOwned::to_owned),
            operations,
        };
        assert_eq!(
            summary.tools.unwrap(),
            [
                count(Some("rg"), 2),
                count(None, 1),
                count(Some("Read"), 1),
                count(Some("head"), 1)
            ]
        );
        assert_eq!(
            summary.operation_kinds.unwrap(),
            [
                count(Some("file.search"), 2),
                count(Some("file.read"), 1),
                count(Some("other"), 1),
                count(Some("stream.select"), 1),
            ]
        );
    }

    /// a69: the raw input is provenance and no summary reads it.
    #[test]
    fn the_summary_reads_no_raw_input() {
        let original = summarize(&annotated_calls()).render().unwrap();
        let mut document = annotated_calls();
        for node in &mut document.events {
            node.kind["input"] = json!({"command": "Bash exec_command sentinel"});
        }
        assert_eq!(summarize(&document).render().unwrap(), original);
    }

    /// a69: a document no rule annotated has no tool counts, which is not zero tools.
    #[test]
    fn an_unannotated_document_has_no_tool_counts() {
        let mut document = annotated_calls();
        document.metaharness.semantic_rule = None;
        for node in &mut document.events {
            node.kind
                .as_object_mut()
                .unwrap()
                .remove("semantic_operations");
        }
        let summary = summarize(&document);
        assert_eq!(summary.tools, None);
        assert_eq!(summary.operation_kinds, None);
        assert!(summary.tool_calls.iter().all(|call| call.tools.is_none()));
    }
}
