//! `project`, as the library sees it: read an event stream, write the document, render the page.
//!
//! Decided by `docs/design/runs-side-by-side-v0.1.md` — P1–P4 for the document, V1–V3 for the
//! viewer. It lives here rather than in `metaharness-cli` on the same rule the rest of the
//! workspace follows: the binary parses and the library decides (design D11).
//!
//! # The alignment is computed here, once
//!
//! **Not in the browser.** A page that computed its own alignment would be a second
//! implementation of V1, and the two would drift the first time one of them was fixed. What the
//! inline script does is expand a row and jump to the next divergence, and that is the whole of
//! it.
//!
//! # Nothing here reads a clock or a network
//!
//! Every duration is a subtraction of two timestamps the vendor recorded, absent where either end
//! has none; every cost is a figure the vendor reported. The page carries no "generated at"
//! footer and no random id, so the same two runs render to the same bytes — which is what lets a
//! rendered page be a committed fixture rather than a screenshot somebody looked at once.

use std::fmt::Write as _;
use std::path::Path;

use metaharness_protocol::{
    EventLine, TraceIrDocument, TraceIrEvent, UNK_FAMILY, parse_event_line, project_document,
};
use serde_json::Value;

use crate::refusal::Refusal;

/// The target form `project` writes. One form, named, so an unknown one is refused rather than
/// silently producing this.
pub const TRACE_IR_FORM: &str = "trace-ir";

/// How two runs were put beside each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlignmentRule {
    /// Both runs are driven, so the key is the workflow state each row belongs to.
    StateEntry,
    /// At least one run is not driven, so the key is the tool call's ordinal within its run.
    ToolCallIndex,
}

impl AlignmentRule {
    /// How the page names the rule it used, so a reader never has to infer it from the shape of
    /// the table.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            AlignmentRule::StateEntry => "aligned by workflow state entry",
            AlignmentRule::ToolCallIndex => "aligned by tool-call index",
        }
    }
}

/// Read one `metaharness.event/1` stream and project it.
///
/// # Errors
///
/// [`Refusal::Io`] when the file cannot be read, and [`Refusal::ProjectionUnreadable`] when a line
/// is not a event this build knows — **by name**, never by producing a shorter document. A stream
/// that lost a line silently is the failure design D4 exists to prevent.
pub fn project_file(path: &Path) -> Result<TraceIrDocument, Refusal> {
    let bytes = std::fs::read(path).map_err(|error| Refusal::Io {
        detail: format!("{} could not be read: {error}", path.display()),
    })?;
    let text = std::str::from_utf8(&bytes).map_err(|error| Refusal::ProjectionUnreadable {
        path: path.to_path_buf(),
        line: 0,
        detail: format!("the stream is not UTF-8: {error}"),
    })?;

    let mut lines: Vec<EventLine> = Vec::new();
    for (position, line) in text.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let parsed = parse_event_line(line).map_err(|error| Refusal::ProjectionUnreadable {
            path: path.to_path_buf(),
            line: position + 1,
            detail: error.to_string(),
        })?;
        lines.push(parsed);
    }
    Ok(project_document(&lines, &bytes))
}

// --- the alignment ------------------------------------------------------------------------------

/// One thing a column shows on one row.
///
/// `Eq` is deliberately not derived: `cost_usd` is the vendor's own float, on the same rule
/// [`metaharness_protocol::Usage`] carries — two runs are compared by what they recorded, never by
/// an equality this workspace invented for a fraction.
#[derive(Debug, Clone, PartialEq)]
struct Cell {
    /// What the row is: a state entry, or a tool call.
    heading: String,
    /// The detail under it, one line per fact.
    detail: Vec<String>,
    /// The decision taken on it, where one was.
    decision: Option<String>,
    /// Whether that decision was a refusal.
    refused: bool,
    /// The duration derived from recorded timestamps, in milliseconds.
    duration_ms: Option<i64>,
    /// The run's cost up to and including this row, in US dollars, as the vendor priced it.
    cost_usd: Option<f64>,
}

/// One column's ordered anchors, and the rule they were built under.
#[derive(Debug, Clone)]
struct Column {
    run: String,
    driven: bool,
    keys: Vec<String>,
    cells: Vec<Cell>,
    total_cost_usd: Option<f64>,
    terminal: String,
}

/// A node's payload, whichever side of the `unk` split it is on.
fn payload(event: &TraceIrEvent) -> &Value {
    match event.kind.get("payload") {
        Some(payload) => payload,
        None => &event.kind,
    }
}

fn family(event: &TraceIrEvent) -> &str {
    event.kind["event"].as_str().unwrap_or_default()
}

fn unk_kind(event: &TraceIrEvent) -> &str {
    if family(event) == UNK_FAMILY {
        event.kind["event_kind"].as_str().unwrap_or_default()
    } else {
        ""
    }
}

fn text_of(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(ToOwned::to_owned)
}

/// The step a `step.entered` node names, as one readable key.
fn step_key(event: &TraceIrEvent) -> String {
    let step = &payload(event)["step"];
    let workflow = text_of(step, "workflow").unwrap_or_default();
    let node = text_of(step, "node").unwrap_or_default();
    let name = text_of(step, "step").or_else(|| text_of(step, "name"));
    match name {
        Some(name) if !name.is_empty() => format!("{workflow}/{node}.{name}"),
        _ => format!("{workflow}/{node}"),
    }
}

/// Build one column: the anchors, in stream order, under whichever rule applies.
fn column(document: &TraceIrDocument, rule: AlignmentRule) -> Column {
    let run = document
        .metaharness
        .run
        .clone()
        .unwrap_or_else(|| "(unnamed run)".to_string());
    let driven = is_driven(document);

    let mut keys = Vec::new();
    let mut cells = Vec::new();
    let mut cost: Option<f64> = None;
    let mut narration: Vec<String> = Vec::new();
    let mut call_ordinal = 0usize;
    let mut terminal = "no terminal record — nobody found out how this run ended".to_string();

    for event in &document.events {
        let node = payload(event);
        match (family(event), unk_kind(event)) {
            ("assistant_text", _) => narration.push(format!("said: {}", clip(node, "text"))),
            ("assistant_thinking", _) => narration.push(format!("thought: {}", clip(node, "text"))),
            ("synthetic_injection", _) => {
                narration.push(format!("injected: {}", clip(node, "text")));
            }
            ("run_outcome", _) => {
                if let Some(total) = node.get("total_cost_usd").and_then(Value::as_f64) {
                    cost = Some(total);
                }
                if node.get("is_error").is_some() {
                    terminal = terminal_line(node);
                }
                if let Some(priced) = node.pointer("/usage/cost_usd").and_then(Value::as_f64) {
                    cost = Some(cost.unwrap_or_default().max(priced));
                }
            }
            ("tool_call", _) if rule == AlignmentRule::ToolCallIndex || !driven => {
                let mut detail = std::mem::take(&mut narration);
                detail.extend(call_detail(node));
                let call_id = text_of(node, "call_id").unwrap_or_default();
                let (decision, refused) = decision_for(document, &call_id);
                cells.push(Cell {
                    heading: format!(
                        "call {call_ordinal} · {}",
                        text_of(node, "name").unwrap_or_default()
                    ),
                    detail,
                    decision,
                    refused,
                    duration_ms: exec_ms(document, event, &call_id),
                    cost_usd: cost,
                });
                keys.push(call_ordinal.to_string());
                call_ordinal += 1;
            }
            (UNK_FAMILY, "step.entered") if rule == AlignmentRule::StateEntry => {
                let key = step_key(event);
                cells.push(Cell {
                    heading: format!("state {key}"),
                    detail: std::mem::take(&mut narration),
                    decision: None,
                    refused: false,
                    duration_ms: None,
                    cost_usd: cost,
                });
                keys.push(key);
            }
            (UNK_FAMILY, "step.left") if rule == AlignmentRule::StateEntry => {
                if let Some(cell) = cells.last_mut() {
                    cell.detail.push(format!(
                        "left: {}",
                        text_of(&payload(event)["outcome"], "outcome").unwrap_or_default()
                    ));
                    cell.duration_ms = step_ms(document, event);
                }
            }
            (UNK_FAMILY, "tool.decided" | "warning") if rule == AlignmentRule::StateEntry => {
                if let Some(cell) = cells.last_mut() {
                    cell.detail.push(unk_line(event));
                }
            }
            ("tool_call", _) if rule == AlignmentRule::StateEntry => {
                if let Some(cell) = cells.last_mut() {
                    cell.detail.extend(call_detail(node));
                }
            }
            _ => {}
        }
    }

    Column {
        run,
        driven,
        keys,
        cells,
        total_cost_usd: cost,
        terminal,
    }
}

/// A run is driven when its stream says so, and by nothing else (invariant 3).
fn is_driven(document: &TraceIrDocument) -> bool {
    document.metaharness.unk_kinds.contains_key("step.entered")
}

fn clip(value: &Value, key: &str) -> String {
    let text = text_of(value, key).unwrap_or_default();
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 120 {
        let head: String = flat.chars().take(117).collect();
        format!("{head}…")
    } else {
        flat
    }
}

fn call_detail(node: &Value) -> Vec<String> {
    let mut detail = Vec::new();
    if let Some(operations) = node.get("operations").and_then(Value::as_array)
        && !operations.is_empty()
    {
        detail.push(format!("operations: {}", join(operations)));
    }
    if let Some(subjects) = node.get("subjects").and_then(Value::as_array)
        && !subjects.is_empty()
    {
        detail.push(format!("subjects: {}", join(subjects)));
    }
    detail.push(format!("input: {}", clip_json(&node["input"])));
    detail
}

fn join(values: &[Value]) -> String {
    values
        .iter()
        .map(|value| value.as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

fn clip_json(value: &Value) -> String {
    let text = value.to_string();
    if text.chars().count() > 160 {
        let head: String = text.chars().take(157).collect();
        format!("{head}…")
    } else {
        text
    }
}

fn unk_line(event: &TraceIrEvent) -> String {
    format!("{}: {}", unk_kind(event), clip_json(payload(event)))
}

fn terminal_line(node: &Value) -> String {
    let error = node
        .get("is_error")
        .and_then(Value::as_bool)
        .map_or("unk".to_string(), |flag| flag.to_string());
    let reason = text_of(node, "terminal_reason").unwrap_or_else(|| "unk".to_string());
    let turns = node
        .get("num_turns")
        .and_then(Value::as_u64)
        .map_or("unk".to_string(), |turns| turns.to_string());
    format!("is_error {error} · terminal_reason {reason} · {turns} turn(s)")
}

/// The decision taken on one call, and whether it was a refusal.
///
/// [`None`] means **no `tool.decided` crossed the wire for this call**, which is not the same
/// fact as an allow and is not rendered as one.
fn decision_for(document: &TraceIrDocument, call_id: &str) -> (Option<String>, bool) {
    for event in &document.events {
        if unk_kind(event) != "tool.decided" {
            continue;
        }
        let node = payload(event);
        if text_of(node, "call_id").as_deref() != Some(call_id) {
            continue;
        }
        let decision = node
            .get("decision")
            .and_then(|decision| {
                decision
                    .as_str()
                    .map(ToOwned::to_owned)
                    .or_else(|| text_of(decision, "decision"))
            })
            .unwrap_or_else(|| "unk".to_string());
        let by = text_of(node, "decided_by").unwrap_or_else(|| "unk".to_string());
        let seam = text_of(node, "seam").unwrap_or_else(|| "unk".to_string());
        let refused = decision == "deny";
        return (Some(format!("{decision} · by {by} · seam {seam}")), refused);
    }
    (None, false)
}

/// Call issued to result back, from recorded timestamps only.
fn exec_ms(document: &TraceIrDocument, call: &TraceIrEvent, call_id: &str) -> Option<i64> {
    let started = call.timestamp_ms?;
    document
        .events
        .iter()
        .find(|event| {
            family(event) == "tool_result"
                && text_of(payload(event), "call_id").as_deref() == Some(call_id)
        })
        .and_then(|result| result.timestamp_ms)
        .map(|ended| ended - started)
}

/// A state's own duration: entered to left, from recorded timestamps only.
fn step_ms(document: &TraceIrDocument, left: &TraceIrEvent) -> Option<i64> {
    let ended = left.timestamp_ms?;
    document
        .events
        .iter()
        .take(left.index)
        .rev()
        .find(|event| unk_kind(event) == "step.entered")
        .and_then(|entered| entered.timestamp_ms)
        .map(|started| ended - started)
}

/// One rendered row: what each column shows, and whether the two disagree.
#[derive(Debug, Clone)]
struct Row {
    key: String,
    left: Option<Cell>,
    right: Option<Cell>,
    divergent: bool,
}

/// Pair two columns' anchors by key, longest-common-subsequence style, so a key present in one and
/// absent in the other becomes a gap **in place** rather than shifting everything after it.
fn align(left: &Column, right: &Column) -> Vec<Row> {
    let (a, b) = (&left.keys, &right.keys);
    let mut table = vec![vec![0usize; b.len() + 1]; a.len() + 1];
    for i in (0..a.len()).rev() {
        for j in (0..b.len()).rev() {
            table[i][j] = if a[i] == b[j] {
                table[i + 1][j + 1] + 1
            } else {
                table[i + 1][j].max(table[i][j + 1])
            };
        }
    }

    let mut rows = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < a.len() && j < b.len() {
        if a[i] == b[j] {
            rows.push(paired(&a[i], &left.cells[i], &right.cells[j]));
            i += 1;
            j += 1;
        } else if table[i + 1][j] >= table[i][j + 1] {
            rows.push(gap_row(&a[i], Some(left.cells[i].clone()), None));
            i += 1;
        } else {
            rows.push(gap_row(&b[j], None, Some(right.cells[j].clone())));
            j += 1;
        }
    }
    while i < a.len() {
        rows.push(gap_row(&a[i], Some(left.cells[i].clone()), None));
        i += 1;
    }
    while j < b.len() {
        rows.push(gap_row(&b[j], None, Some(right.cells[j].clone())));
        j += 1;
    }
    rows
}

fn paired(key: &str, left: &Cell, right: &Cell) -> Row {
    let divergent = left.heading != right.heading
        || left.decision != right.decision
        || left.refused != right.refused;
    Row {
        key: key.to_string(),
        left: Some(left.clone()),
        right: Some(right.clone()),
        divergent,
    }
}

fn gap_row(key: &str, left: Option<Cell>, right: Option<Cell>) -> Row {
    Row {
        key: key.to_string(),
        left,
        right,
        divergent: true,
    }
}

// --- the page -----------------------------------------------------------------------------------

/// Render one or two projected runs as one static page.
///
/// # Errors
///
/// [`Refusal::ViewerColumnCount`] for anything other than one or two documents. Two columns is
/// the shape this design decided; a third would have to be aligned against something nobody
/// decided, and dropping it silently would be worse than refusing it.
pub fn render_page(documents: &[TraceIrDocument]) -> Result<String, Refusal> {
    let (first, second) = match documents {
        [first] => (first, None),
        [first, second] => (first, Some(second)),
        _ => {
            return Err(Refusal::ViewerColumnCount {
                given: documents.len(),
            });
        }
    };

    let rule = if is_driven(first) && second.is_some_and(is_driven) {
        AlignmentRule::StateEntry
    } else {
        AlignmentRule::ToolCallIndex
    };

    let left = column(first, rule);
    if let Some(second) = second {
        let right = column(second, rule);
        let rows = align(&left, &right);
        return Ok(render(
            &left,
            Some(&right),
            &rows,
            rule,
            first,
            Some(second),
        ));
    }
    // One column is a page too: a run with nothing beside it is still a reading surface, and every
    // row of it is trivially non-divergent because there is nothing for it to diverge from.
    let rows: Vec<Row> = left
        .keys
        .iter()
        .zip(&left.cells)
        .map(|(key, cell)| Row {
            key: key.clone(),
            left: Some(cell.clone()),
            right: None,
            divergent: false,
        })
        .collect();
    Ok(render(&left, None, &rows, rule, first, None))
}

fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for character in text.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(character),
        }
    }
    out
}

fn money(value: Option<f64>) -> String {
    value.map_or_else(|| "—".to_string(), |value| format!("${value:.4}"))
}

fn millis(value: Option<i64>) -> String {
    value.map_or_else(|| "—".to_string(), |value| format!("{value} ms"))
}

fn cell_html(cell: Option<&Cell>) -> String {
    let Some(cell) = cell else {
        return "<td class=\"gap\"><span class=\"gapmark\">— absent in this run —</span></td>"
            .to_string();
    };
    let mut html = String::new();
    let _ = write!(
        html,
        "<td><div class=\"head\">{}</div>",
        escape(&cell.heading)
    );
    let _ = write!(
        html,
        "<div class=\"meta\">{} · {}</div>",
        millis(cell.duration_ms),
        money(cell.cost_usd)
    );
    match &cell.decision {
        Some(decision) if cell.refused => {
            let _ = write!(
                html,
                "<div class=\"decision refused\">refused: {}</div>",
                escape(decision)
            );
        }
        Some(decision) => {
            let _ = write!(html, "<div class=\"decision\">{}</div>", escape(decision));
        }
        None => html.push_str(
            "<div class=\"decision none\">no tool.decided crossed the wire for this call</div>",
        ),
    }
    if !cell.detail.is_empty() {
        html.push_str("<ul class=\"detail\">");
        for line in &cell.detail {
            let _ = write!(html, "<li>{}</li>", escape(line));
        }
        html.push_str("</ul>");
    }
    html.push_str("</td>");
    html
}

fn header(column: &Column, document: &TraceIrDocument) -> String {
    let families = document
        .metaharness
        .families
        .iter()
        .map(|(family, count)| format!("{family} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "<th><div class=\"run\">{}</div><div class=\"meta\">{} · {} event(s) · {}</div>\
         <div class=\"meta\">{}</div><div class=\"meta\">total {}</div>\
         <div class=\"meta digest\">stream sha256:{}</div></th>",
        escape(&column.run),
        if column.driven {
            "driven"
        } else {
            "not driven"
        },
        document.metaharness.events_total,
        escape(&families),
        escape(&column.terminal),
        money(column.total_cost_usd),
        escape(&document.transcript_digest),
    )
}

#[allow(clippy::too_many_lines, reason = "one template, read top to bottom")]
fn render(
    left: &Column,
    right: Option<&Column>,
    rows: &[Row],
    rule: AlignmentRule,
    first: &TraceIrDocument,
    second: Option<&TraceIrDocument>,
) -> String {
    let divergences = rows.iter().filter(|row| row.divergent).count();
    let mut html = String::new();
    html.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    html.push_str("<title>metaharness — two runs, side by side</title>\n<style>\n");
    html.push_str(STYLE);
    html.push_str("</style>\n</head>\n<body>\n");

    let _ = write!(
        html,
        "<h1>Two runs, side by side</h1>\n<p class=\"rule\">{} · {} divergence(s) · \
         {} row(s)</p>\n",
        rule.as_str(),
        divergences,
        rows.len()
    );
    html.push_str(
        "<p class=\"note\">A step present in one run and absent in the other is a row with a gap, \
         never a skipped one. Durations are derived from timestamps the harness recorded and are \
         absent where either end has none; costs are what the vendor reported and were never \
         multiplied out here. Nothing on this page scores either run.</p>\n",
    );
    html.push_str("<p><button id=\"next\">jump to next divergence</button></p>\n");

    html.push_str("<table>\n<thead>\n<tr><th class=\"key\">key</th>");
    html.push_str(&header(left, first));
    if let (Some(right), Some(second)) = (right, second) {
        html.push_str(&header(right, second));
    }
    html.push_str("</tr>\n</thead>\n<tbody>\n");

    for row in rows {
        let _ = write!(
            html,
            "<tr class=\"{}\"><td class=\"key\">{}</td>",
            if row.divergent { "divergence" } else { "same" },
            escape(&row.key)
        );
        html.push_str(&cell_html(row.left.as_ref()));
        if right.is_some() {
            html.push_str(&cell_html(row.right.as_ref()));
        }
        html.push_str("</tr>\n");
    }

    html.push_str("</tbody>\n</table>\n<script>\n");
    html.push_str(SCRIPT);
    html.push_str("</script>\n</body>\n</html>\n");
    html
}

/// The whole stylesheet, inline. No external fetch: the page has to work from a `file://` URL.
const STYLE: &str = "\
:root{color-scheme:light dark}
body{font:14px/1.5 ui-sans-serif,system-ui,sans-serif;margin:2rem;max-width:100rem}
h1{font-size:1.4rem;margin:0 0 .25rem}
p.rule{font-weight:600;margin:0 0 .5rem}
p.note{max-width:60rem;opacity:.75;margin:0 0 1rem}
table{border-collapse:collapse;width:100%;table-layout:fixed}
th,td{border:1px solid rgba(128,128,128,.45);padding:.5rem .6rem;vertical-align:top}
th{text-align:left}
td.key,th.key{width:9rem;font-variant-numeric:tabular-nums;opacity:.8}
tr.divergence>td.key{font-weight:700}
tr.divergence{outline:2px solid rgba(220,120,0,.55);outline-offset:-2px}
td.gap{background:repeating-linear-gradient(45deg,transparent,transparent 6px,rgba(128,128,128,.12) 6px,rgba(128,128,128,.12) 12px)}
.gapmark{opacity:.7;font-style:italic}
.run{font-weight:700}
.head{font-weight:600}
.meta{opacity:.75;font-size:.9em}
.digest{word-break:break-all}
.decision{font-size:.9em;margin-top:.25rem}
.decision.none{opacity:.6;font-style:italic}
.decision.refused{font-weight:700}
ul.detail{margin:.35rem 0 0;padding-left:1.1rem}
ul.detail li{overflow-wrap:anywhere}
tbody tr.collapsed ul.detail{display:none}
button{font:inherit;padding:.3rem .7rem}
";

/// The whole script, inline, and it does exactly two things.
const SCRIPT: &str = "\
document.querySelectorAll('tbody tr').forEach(function(row){
  row.classList.add('collapsed');
  row.addEventListener('click', function(){ row.classList.toggle('collapsed'); });
});
var at = -1;
document.getElementById('next').addEventListener('click', function(){
  var marks = document.querySelectorAll('tr.divergence');
  if (!marks.length) { return; }
  at = (at + 1) % marks.length;
  marks[at].scrollIntoView({block: 'center'});
});
";

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::{EVENT_FORMAT, Event, RunId, project_document};

    fn line(seq: u64, at: Option<&str>, event: Event) -> EventLine {
        EventLine {
            format: EVENT_FORMAT.to_string(),
            seq,
            run: RunId::new("t"),
            at: at.map(ToOwned::to_owned),
            event,
        }
    }

    fn call(seq: u64, at: &str, id: &str, name: &str) -> EventLine {
        line(
            seq,
            Some(at),
            Event::ToolRequested {
                call_id: id.to_string(),
                name: name.to_string(),
                input: serde_json::json!({"command": "echo hi"}),
                operations: Vec::new(),
                subjects: Vec::new(),
                decision_required: false,
                deadline_ms: None,
                seam: metaharness_protocol::Seam::None,
            },
        )
    }

    fn document(names: &[&str]) -> TraceIrDocument {
        let lines: Vec<EventLine> = names
            .iter()
            .enumerate()
            .map(|(index, name)| {
                call(
                    index as u64 + 1,
                    "2026-08-21T09:00:00.000Z",
                    &format!("c{index}"),
                    name,
                )
            })
            .collect();
        project_document(&lines, b"stream")
    }

    /// V1's fallback: neither run is driven, so the key is the ordinal.
    #[test]
    fn two_undriven_runs_align_by_tool_call_index() {
        let page = render_page(&[document(&["Bash", "Read"]), document(&["Bash", "Read"])])
            .expect("renders");
        assert!(page.contains("aligned by tool-call index"));
        assert!(page.contains("0 divergence(s)"), "{page}");
    }

    /// V2 — the sixth call of a five-call run is a row with a gap in it.
    #[test]
    fn a_call_only_one_run_made_is_a_gap_row_and_a_divergence() {
        let page = render_page(&[
            document(&["Bash", "Read", "Glob"]),
            document(&["Bash", "Read"]),
        ])
        .expect("renders");
        assert!(page.contains("class=\"gap\""));
        assert!(page.contains("1 divergence(s)"), "{page}");
    }

    /// Two runs that made the same number of calls with different tools diverge at the first one
    /// that differs, and not before it.
    #[test]
    fn a_different_tool_at_the_same_index_is_a_divergence() {
        let page = render_page(&[document(&["Bash", "Read"]), document(&["Bash", "Glob"])])
            .expect("renders");
        assert!(page.contains("1 divergence(s)"), "{page}");
    }

    /// V3 — nothing in the page varies between two renderings of the same input.
    #[test]
    fn the_page_is_the_same_bytes_twice() {
        let documents = [document(&["Bash"]), document(&["Read"])];
        assert_eq!(
            render_page(&documents).expect("renders"),
            render_page(&documents).expect("renders")
        );
    }

    #[test]
    fn a_third_column_is_refused_rather_than_dropped() {
        let documents = [
            document(&["Bash"]),
            document(&["Read"]),
            document(&["Glob"]),
        ];
        assert!(matches!(
            render_page(&documents),
            Err(Refusal::ViewerColumnCount { given: 3 })
        ));
    }

    /// A call nobody adjudicated says so, rather than rendering as an allow.
    #[test]
    fn a_call_with_no_decision_says_so_rather_than_reading_as_an_allow() {
        let page = render_page(&[document(&["Bash"])]).expect("renders");
        assert!(page.contains("no tool.decided crossed the wire for this call"));
    }
}
