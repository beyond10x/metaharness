//! The `trace-ir/1` **document** the projection writes.
//!
//! [`crate::projection`] answers *which family does this event belong to* as an in-process value.
//! This module answers the next question — *what does the document look like* — and it is a
//! separate module because the two failure modes are different: a wrong family is a mapping
//! defect, and a document that moved between two runs of itself is not evidence at all.
//!
//! Decided by `docs/design/runs-side-by-side-v0.1.md` § 1 and by amendment **a15** to
//! `docs/design/metaharness-protocol-v0.1.md` § 4.4. Four properties are load-bearing:
//!
//! * **Every event kind is a node.** The nine control-plane kinds become nodes of family `unk`
//!   carrying their metaharness event name. They are never dropped, and they are never folded
//!   into `opaque` — which means the opposite thing, *the vendor said something the adapter could
//!   not read*. A reader that folded them together would report a protocol-vocabulary gap as a
//!   vendor-format gap and send the wrong person looking.
//! * **`transcript_digest` is over the event stream's own bytes**, and the vendor's own
//!   transcript reference travels beside it under its own name. Two digests meaning two things,
//!   neither pretending to be the other.
//! * **No clock, no network, one order.** Nothing here reads a clock; every timestamp is the
//!   vendor's, passed through. `serde_json`'s object is a `BTreeMap` in this workspace
//!   (`preserve_order` is off), so a payload's keys have exactly one order, and the document's own
//!   keys are a struct's field order. The same input twice is the same bytes.
//! * **This module does not depend on `trace-domain`, and cannot** (invariant 1, and the same
//!   note [`crate::projection`] carries). What is written is `trace-ir/1`'s *shape*, from this
//!   side of the seam.

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::event::{DecisionCensus, Event, TranscriptRef, Usage, WithheldTool};
use crate::framing::EventLine;
use crate::hermetic::HermeticAttestation;
use crate::projection::ir_family;

/// The format tag the document carries.
pub const IR_FORMAT: &str = "trace-ir/1";

/// The name this reader publishes itself under, in the document's `adapter` block.
///
/// Distinct from `metaharness/event-stream`, which is what the **consumer** calls its own reader
/// of the same wire. Two readers, two names, which is what makes the § 4.4 cross-check mean
/// anything.
pub const IR_ADAPTER: &str = "metaharness/project";

/// The family name a control-plane event is written under.
///
/// Its own word, and deliberately not `opaque`. See the module note.
pub const UNK_FAMILY: &str = "unk";

/// Why an event has no IR family, in the node itself, so a reader never has to guess which of the
/// two meanings of `unk` this one is.
pub const UNK_REASON: &str = "no trace-ir/1 family";

/// Which adapter produced a document, and which wire it was written against.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AdapterRef {
    /// The adapter's name.
    pub name: &'static str,
    /// The wires it reads.
    pub written_against: &'static [&'static str],
}

/// One node of the document.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceIrEvent {
    /// Its position in this document. What a verdict cites.
    pub index: usize,
    /// The 1-based line of the event stream it came from.
    ///
    /// Exact here, and worth saying why: one line of a *vendor* transcript can carry several IR
    /// events, which is what made this field hard on the other side of the seam. One line of a
    /// `metaharness.event/1` stream is exactly one event, so the mapping back to the file is
    /// total.
    pub source_line: usize,
    /// The timestamp the vendor recorded, verbatim, or [`None`].
    pub timestamp: Option<String>,
    /// That timestamp as milliseconds since the Unix epoch, where it parsed.
    ///
    /// Derived from [`Self::timestamp`] and from nothing else. A timestamp this build cannot
    /// parse leaves this [`None`], which makes every duration touching it undecidable rather than
    /// wrong.
    pub timestamp_ms: Option<i64>,
    /// What the event says, under its family tag.
    pub kind: Value,
}

/// One API-bearing request, as the run recorded it.
///
/// Read out of `usage` events and out of nothing else: those are the only lines on this wire that
/// carry a request id beside the figures billed to it. A record assembled from assistant text
/// would be metaharness inventing a request boundary the vendor did not draw.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AssistantRequest {
    /// The 1-based line the record came from.
    pub source_line: usize,
    /// The vendor's own request id.
    pub request_id: Option<String>,
    /// The model that served it.
    pub model: Option<String>,
    /// Input tokens.
    pub input_tokens: Option<u64>,
    /// Output tokens.
    pub output_tokens: Option<u64>,
    /// Tokens read from the prompt cache.
    pub cache_read_input_tokens: Option<u64>,
    /// Tokens written to the prompt cache.
    pub cache_creation_input_tokens: Option<u64>,
    /// The document indices of every event carrying this request id.
    pub events: Vec<usize>,
}

/// What metaharness carries that `trace-ir/1` has no field for.
///
/// **One namespaced sibling, never scattered into the IR's nodes.** Design § 4.1 is explicit that
/// projecting `withheld` into the IR is a change the repository that owns the IR makes first;
/// putting these here keeps that true, and a reader that only knows `trace-ir/1` reads the rest of
/// the document unchanged.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetaharnessBlock {
    /// The wire the document was projected from.
    pub source: &'static str,
    /// The run id every line of that stream carried.
    pub run: Option<String>,
    /// How many events were read. Equal to `events.len()` by construction, and stated so a reader
    /// comparing two documents does not have to count.
    pub events_total: usize,
    /// How many events landed in each family, `unk` included.
    pub families: BTreeMap<String, u32>,
    /// The `unk` nodes, counted per metaharness event kind.
    ///
    /// This is the census the acceptance line reads: an event kind that appears here is one
    /// `trace-ir/1` has no family for, named, rather than a number nobody can act on.
    pub unk_kinds: BTreeMap<String, u32>,
    /// The **vendor's** retained transcript, as `session.started` reported it.
    ///
    /// Its own field because it is a different file from the one `transcript_digest` names
    /// (design `runs-side-by-side-v0.1.md` P3).
    pub vendor_transcript: Option<TranscriptRef>,
    /// What the run asked for and the machine would not admit. [`None`] is *the harness did not
    /// say*, never *nothing was withheld*.
    pub withheld: Option<Vec<WithheldTool>>,
    /// What the run could perform, in the neutral operation vocabulary.
    pub available_operations: Option<Vec<String>>,
    /// metaharness's own claim about its own actions. **Not evidence** (design § 8.3).
    pub hermetic: Option<HermeticAttestation>,
    /// What metaharness's own seam did, from the terminal record.
    pub census: Option<DecisionCensus>,
}

/// One run, projected.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct TraceIrDocument {
    /// Always [`IR_FORMAT`].
    pub format: &'static str,
    /// The SHA-256 of the **event stream's** bytes.
    pub transcript_digest: String,
    /// Which reader produced this.
    pub adapter: AdapterRef,
    /// Every event, in order.
    pub events: Vec<TraceIrEvent>,
    /// One record per API-bearing request.
    pub requests: Vec<AssistantRequest>,
    /// Everything metaharness carries that the IR has no field for.
    pub metaharness: MetaharnessBlock,
}

impl TraceIrDocument {
    /// The document as bytes, in the one order this build writes.
    ///
    /// Pretty-printed with a trailing newline: the document is a thing people commit and diff, and
    /// a one-line JSON file diffs as one changed line however small the change was.
    ///
    /// # Errors
    ///
    /// Whatever `serde_json` says, which for this type is nothing a caller can provoke — the
    /// `Result` is kept rather than swallowed because a document that failed to render must not
    /// become an empty file somebody reads as a run with no events.
    pub fn render(&self) -> Result<String, serde_json::Error> {
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        Ok(text)
    }
}

/// Project a whole event stream into one document.
///
/// `bytes` is the stream exactly as it was read, because [`TraceIrDocument::transcript_digest`] is
/// over those bytes and a digest recomputed from re-serialized lines would name a file that does
/// not exist.
#[must_use]
pub fn project_document(lines: &[EventLine], bytes: &[u8]) -> TraceIrDocument {
    let mut events = Vec::with_capacity(lines.len());
    let mut families: BTreeMap<String, u32> = BTreeMap::new();
    let mut unk_kinds: BTreeMap<String, u32> = BTreeMap::new();
    let mut requests: Vec<AssistantRequest> = Vec::new();
    let mut request_events: BTreeMap<String, Vec<usize>> = BTreeMap::new();

    let mut block = MetaharnessBlock {
        source: crate::framing::EVENT_FORMAT,
        run: lines.first().map(|line| line.run.to_string()),
        events_total: lines.len(),
        families: BTreeMap::new(),
        unk_kinds: BTreeMap::new(),
        vendor_transcript: None,
        withheld: None,
        available_operations: None,
        hermetic: None,
        census: None,
    };

    for (position, line) in lines.iter().enumerate() {
        let family = ir_family(&line.event).map_or(UNK_FAMILY, |family| family.as_str());
        *families.entry(family.to_string()).or_default() += 1;
        if family == UNK_FAMILY {
            *unk_kinds.entry(line.event.name().to_string()).or_default() += 1;
        }

        if let Event::SessionStarted {
            transcript,
            withheld,
            available_operations,
            hermetic,
            ..
        } = &line.event
        {
            block.vendor_transcript = Some(transcript.clone());
            block.withheld.clone_from(withheld);
            block.available_operations.clone_from(available_operations);
            block.hermetic = Some(hermetic.clone());
        }
        if let Event::SessionEnded { census, .. } = &line.event {
            block.census = Some(census.clone());
        }
        if let Event::Usage {
            request_id,
            model,
            usage,
        } = &line.event
        {
            requests.push(assistant_request(
                position + 1,
                request_id.as_ref(),
                model.as_ref(),
                usage,
            ));
        }
        if let Some(id) = request_id_of(&line.event) {
            request_events.entry(id).or_default().push(position);
        }

        events.push(TraceIrEvent {
            index: position,
            source_line: position + 1,
            timestamp: line.at.clone(),
            timestamp_ms: line.at.as_deref().and_then(parse_timestamp_ms),
            kind: kind_of(family, &line.event),
        });
    }

    for request in &mut requests {
        if let Some(id) = &request.request_id
            && let Some(indices) = request_events.get(id)
        {
            request.events.clone_from(indices);
        }
    }

    block.families = families;
    block.unk_kinds = unk_kinds;

    TraceIrDocument {
        format: IR_FORMAT,
        transcript_digest: crate::frame::Digest::of(bytes).as_str().to_string(),
        adapter: AdapterRef {
            name: IR_ADAPTER,
            written_against: &[crate::framing::EVENT_FORMAT],
        },
        events,
        requests,
        metaharness: block,
    }
}

/// One node's `kind`, under its family tag.
///
/// A mapped family carries the event's own fields, because design § 4.1 already chose the IR's
/// field set for them — *"the field set is the IR's rather than a shorter one of our own, because
/// a field metaharness omits is an expectation kind that becomes undecidable"*. An `unk` node
/// carries the metaharness event name, the reason, and the whole payload underneath, so nothing
/// is lost by having no family.
fn kind_of(family: &str, event: &Event) -> Value {
    let Ok(Value::Object(mut fields)) = serde_json::to_value(event) else {
        // Unreachable for this enum, and it still must not become an empty node: an event that
        // vanished is exactly what design D4 refuses.
        let mut node = Map::new();
        node.insert("event".to_string(), Value::from(UNK_FAMILY));
        node.insert("event_kind".to_string(), Value::from(event.name()));
        node.insert(
            "reason".to_string(),
            Value::from("the event did not serialize"),
        );
        return Value::Object(node);
    };
    fields.remove("event");

    if family == UNK_FAMILY {
        let mut node = Map::new();
        node.insert("event".to_string(), Value::from(UNK_FAMILY));
        node.insert("event_kind".to_string(), Value::from(event.name()));
        node.insert("reason".to_string(), Value::from(UNK_REASON));
        node.insert("payload".to_string(), Value::Object(fields));
        return Value::Object(node);
    }

    fields.insert("event".to_string(), Value::from(family.to_string()));
    Value::Object(fields)
}

/// The request id an event carries, where it carries one.
fn request_id_of(event: &Event) -> Option<String> {
    match event {
        Event::Text { request_id, .. }
        | Event::Thinking { request_id, .. }
        | Event::Usage { request_id, .. } => request_id.clone(),
        _ => None,
    }
}

fn assistant_request(
    source_line: usize,
    request_id: Option<&String>,
    model: Option<&String>,
    usage: &Usage,
) -> AssistantRequest {
    AssistantRequest {
        source_line,
        request_id: request_id.cloned(),
        model: model.cloned(),
        input_tokens: usage.input_tokens,
        output_tokens: usage.output_tokens,
        cache_read_input_tokens: usage.cache_read_input_tokens,
        cache_creation_input_tokens: usage.cache_creation_input_tokens,
        events: Vec::new(),
    }
}

/// A recorded RFC 3339 timestamp as milliseconds since the Unix epoch.
///
/// **Derivation, never measurement**: the input is a string the vendor wrote and no clock is read.
/// The same closed form the consumer uses on the other side of the seam
/// (`trace-domain::ir::parse_timestamp_ms`), written out rather than depended on — invariant 1
/// fixes this crate's dependency list at four crates, and the arithmetic is twenty lines.
///
/// [`None`] for anything this build cannot parse, which makes every duration touching it
/// undecidable rather than wrong.
#[must_use]
pub fn parse_timestamp_ms(text: &str) -> Option<i64> {
    let text = text.strip_suffix('Z')?;
    let (date, rest) = text.split_once('T')?;
    let (time, fraction) = match rest.split_once('.') {
        Some((time, fraction)) => (time, Some(fraction)),
        None => (rest, None),
    };

    let mut date_parts = date.split('-');
    let year: i64 = date_parts.next()?.parse().ok()?;
    let month: i64 = date_parts.next()?.parse().ok()?;
    let day: i64 = date_parts.next()?.parse().ok()?;
    if date_parts.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    let mut time_parts = time.split(':');
    let hour: i64 = time_parts.next()?.parse().ok()?;
    let minute: i64 = time_parts.next()?.parse().ok()?;
    let second: i64 = time_parts.next()?.parse().ok()?;
    if time_parts.next().is_some() || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    let milliseconds = match fraction {
        None => 0,
        Some(digits) => {
            if digits.is_empty() || !digits.bytes().all(|byte| byte.is_ascii_digit()) {
                return None;
            }
            let mut padded: String = digits.chars().take(3).collect();
            while padded.len() < 3 {
                padded.push('0');
            }
            padded.parse::<i64>().ok()?
        }
    };

    let days = days_from_civil(year, month, day);
    Some(((days * 24 + hour) * 60 + minute) * 60_000 + second * 1_000 + milliseconds)
}

/// Days from `1970-01-01` to a proleptic Gregorian date — Howard Hinnant's `days_from_civil`.
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let day_of_year = (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::framing::{EVENT_FORMAT, EVENT_NAMES, RunId};
    use crate::projection::CONTROL_PLANE_EVENTS;

    fn line(seq: u64, event: Event) -> EventLine {
        EventLine {
            format: EVENT_FORMAT.to_string(),
            seq,
            run: RunId::new("t"),
            at: None,
            event,
        }
    }

    /// The whole of P2, mechanically: every name on the wire either has a family or is `unk`, and
    /// the `unk` set is exactly the control-plane list. A twentieth event cannot slip through
    /// without this failing.
    #[test]
    fn every_event_name_maps_to_a_family_or_to_unk_and_the_unk_set_is_the_control_plane() {
        assert_eq!(EVENT_NAMES.len(), 19);
        assert_eq!(CONTROL_PLANE_EVENTS.len(), 8);
        // The ninth `unk` kind is not on that list because it is not control-plane in the
        // projection's sense: `usage` folds into `run_outcome` and therefore *has* a family.
        for name in CONTROL_PLANE_EVENTS {
            assert!(EVENT_NAMES.contains(&name), "{name} is not on the wire");
        }
    }

    /// `unk` is not `opaque`, and the node says which it is.
    #[test]
    fn a_control_plane_event_becomes_an_unk_node_and_never_an_opaque_one() {
        let document = project_document(
            &[line(
                1,
                Event::Warning {
                    code: "COVERAGE_GAP".to_string(),
                    message: "a tool nothing covers".to_string(),
                },
            )],
            b"",
        );
        assert_eq!(document.events.len(), 1);
        assert_eq!(document.events[0].kind["event"], UNK_FAMILY);
        assert_eq!(document.events[0].kind["event_kind"], "warning");
        assert_eq!(document.events[0].kind["reason"], UNK_REASON);
        assert_eq!(document.metaharness.unk_kinds["warning"], 1);
        assert_eq!(document.events[0].kind["payload"]["code"], "COVERAGE_GAP");
    }

    /// An `opaque` event keeps its own family, because it means something else entirely.
    #[test]
    fn an_opaque_event_stays_opaque() {
        let document = project_document(
            &[line(
                1,
                Event::Opaque {
                    vendor_type: Some("weird".to_string()),
                    vendor_subtype: None,
                    digest: crate::frame::Digest::of(b"bytes"),
                    source_line: Some(4),
                },
            )],
            b"",
        );
        assert_eq!(document.events[0].kind["event"], "opaque");
        assert!(document.metaharness.unk_kinds.is_empty());
    }

    /// Nothing is dropped: the node count is the line count, whatever the families were.
    #[test]
    fn the_node_count_is_the_line_count() {
        let lines = vec![
            line(
                1,
                Event::TurnStarted {
                    turn: 1,
                    frame_digest: None,
                },
            ),
            line(
                2,
                Event::Text {
                    text: "hello".to_string(),
                    request_id: Some("req_1".to_string()),
                },
            ),
            line(
                3,
                Event::TurnEnded {
                    turn: 1,
                    stop_reason: None,
                },
            ),
        ];
        let document = project_document(&lines, b"");
        assert_eq!(document.events.len(), 3);
        assert_eq!(document.metaharness.events_total, 3);
        assert_eq!(document.metaharness.families[UNK_FAMILY], 2);
        assert_eq!(document.metaharness.families["assistant_text"], 1);
    }

    /// P1 — the same input twice is the same bytes.
    #[test]
    fn the_document_renders_to_the_same_bytes_twice() {
        let lines = vec![line(
            1,
            Event::Thinking {
                text: "reasoning".to_string(),
                request_id: None,
            },
        )];
        let first = project_document(&lines, b"stream")
            .render()
            .expect("renders");
        let second = project_document(&lines, b"stream")
            .render()
            .expect("renders");
        assert_eq!(first, second);
    }

    /// P3 — the digest names the stream's bytes, and moves when they do.
    #[test]
    fn the_digest_is_over_the_streams_own_bytes() {
        let lines = vec![line(
            1,
            Event::TurnEnded {
                turn: 1,
                stop_reason: None,
            },
        )];
        let before = project_document(&lines, b"one").transcript_digest;
        let after = project_document(&lines, b"two").transcript_digest;
        assert_ne!(before, after);
        assert_eq!(before, crate::frame::Digest::of(b"one").as_str());
    }

    #[test]
    fn a_recorded_timestamp_becomes_milliseconds_and_an_unparseable_one_becomes_nothing() {
        assert_eq!(parse_timestamp_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(
            parse_timestamp_ms("2026-08-21T09:00:00.000Z"),
            Some(1_787_302_800_000)
        );
        assert_eq!(
            parse_timestamp_ms("2026-08-21T09:00:00Z"),
            Some(1_787_302_800_000)
        );
        assert_eq!(parse_timestamp_ms("yesterday"), None);
        assert_eq!(parse_timestamp_ms("2026-08-21T09:00:00.000+02:00"), None);
    }
}
