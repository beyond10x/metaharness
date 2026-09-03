//! One JSON object per line, in both directions, each carrying its own format tag.
//!
//! The tag is on **every line** rather than on a handshake, so a truncated capture is still
//! self-describing (design D2). A consumer that reads a tag it does not know refuses the line
//! and says so; it does not guess (design D3).
//!
//! The asymmetry to remember: unknown **fields** on a known event are ignored in silence, an
//! unknown **event or command name** is a named refusal. This wire is an authored schema, so a
//! misspelling here is a mistake the author wants to be told about.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::command::Command;
use crate::event::{CloseReason, Emission, Event};

/// The format tag every event line carries.
pub const EVENT_FORMAT: &str = "metaharness.event/1";

/// The format tag every command line carries.
pub const COMMAND_FORMAT: &str = "metaharness.command/1";

/// Every event name this version of the wire knows.
///
/// Published as a value so a reader can refuse an unknown name **by name** instead of failing
/// with a serde message, and so a test can assert the vocabulary's size against the design.
pub const EVENT_NAMES: [&str; 20] = [
    "session.started",
    "session.ended",
    "step.entered",
    "step.left",
    "turn.started",
    "turn.ended",
    "text",
    "thinking",
    "thinking.estimate",
    "injection",
    "tool.requested",
    "tool.decided",
    "tool.result",
    "usage",
    "rate_limit",
    "command.result",
    "warning",
    "opaque",
    "auth.expired",
    "stream.closed",
];

/// Every command name this version of the wire knows.
pub const COMMAND_NAMES: [&str; 7] = [
    "tool.decide",
    "frame.set",
    "message.inject",
    "steer",
    "permission.set",
    "interrupt",
    "halt",
];

/// Which run a line belongs to.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RunId(String);

impl RunId {
    /// A run id from a string the embedder chose.
    #[must_use]
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// The id as it appears on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for RunId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// One event, framed for the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EventLine {
    /// Always [`EVENT_FORMAT`]; checked on the way in.
    pub format: String,
    /// A monotone per-run counter, assigned in one place ([`EventStream`]) so a verdict cites
    /// one thing (design D2).
    pub seq: u64,
    /// Which run.
    pub run: RunId,
    /// The timestamp **the vendor recorded**, passed through, or absent. Skipped when absent —
    /// the one field on this wire that is, because a timestamp metaharness does not have is not
    /// an `unk` verdict about the run, it is silence about a clock nobody read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub at: Option<String>,
    /// The event.
    #[serde(flatten)]
    pub event: Event,
}

/// One command, framed for the wire.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommandLine {
    /// Always [`COMMAND_FORMAT`]; checked on the way in.
    pub format: String,
    /// The command's id. Its [`crate::Event::CommandResult`] carries the same one.
    pub id: String,
    /// The command.
    #[serde(flatten)]
    pub command: Command,
}

impl CommandLine {
    /// Frame this command under this id.
    #[must_use]
    pub fn new(id: impl Into<String>, command: Command) -> Self {
        Self {
            format: COMMAND_FORMAT.to_string(),
            id: id.into(),
            command,
        }
    }
}

/// The one place a sequence number is assigned.
///
/// A producer that numbered its own events would be a second place that decides what a verdict
/// cites, which is the failure `trace-ir` assigns indices centrally to avoid (design D2).
#[derive(Debug, Clone)]
pub struct EventStream {
    run: RunId,
    next: u64,
    closed: bool,
}

impl EventStream {
    /// A stream for this run, numbering from 1.
    #[must_use]
    pub fn new(run: RunId) -> Self {
        Self {
            run,
            next: 1,
            closed: false,
        }
    }

    /// Frame one emission, taking the next sequence number.
    ///
    /// The timestamp comes from the emission — that is, from the vendor's record — and is never
    /// read from a clock here.
    pub fn stamp(&mut self, emission: Emission) -> EventLine {
        let seq = self.next;
        self.next += 1;
        EventLine {
            format: EVENT_FORMAT.to_string(),
            seq,
            run: self.run.clone(),
            at: emission.at,
            event: emission.event,
        }
    }

    /// How many events this stream has framed.
    #[must_use]
    pub fn emitted(&self) -> u64 {
        self.next - 1
    }

    /// Whether this stream has already written its closing marker.
    #[must_use]
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    /// Write the last line: `stream.closed`, with the count and the reason (amendment a17).
    ///
    /// **Here rather than at each producer**, for the reason `seq` is assigned here: the count of
    /// preceding lines and the run id are this type's own facts, and a producer that filled them
    /// itself would be a second place that decides what the marker claims — which is exactly the
    /// disagreement between the line's `run` and the payload's `run_id` that the duplication is
    /// only safe without.
    ///
    /// [`None`] when the stream is already closed, so a driver whose wind-up is reached twice
    /// writes one marker and not two. A stream that has been closed frames nothing further; the
    /// caller that would have is the defect, and the closed stream is not the place to hide it.
    pub fn close(&mut self, reason: CloseReason) -> Option<EventLine> {
        if self.closed {
            return None;
        }
        self.closed = true;
        let events = self.emitted();
        Some(self.stamp(Emission::untimed(Event::StreamClosed {
            events,
            reason,
            run_id: self.run.to_string(),
        })))
    }
}

/// Why a line was refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FramingError {
    /// The line was not a JSON object.
    NotAnObject {
        /// What the parser said.
        detail: String,
    },
    /// The `format` field was missing or was a tag this build does not know. Refused rather than
    /// guessed (design D3).
    UnknownFormat {
        /// The tag that was read, or `None` when the field was missing.
        tag: Option<String>,
        /// The tag that was expected.
        expected: &'static str,
    },
    /// The `event` or `command` field named something this build does not know.
    UnknownName {
        /// The name that was read.
        name: String,
        /// Which vocabulary it was checked against.
        vocabulary: &'static str,
    },
    /// The name was known and the payload did not fit it.
    Malformed {
        /// Which event or command.
        name: String,
        /// What the parser said.
        detail: String,
    },
}

impl fmt::Display for FramingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FramingError::NotAnObject { detail } => {
                write!(f, "not a JSON object: {detail}")
            }
            FramingError::UnknownFormat { tag, expected } => match tag {
                Some(tag) => write!(f, "unknown format tag {tag:?}, expected {expected:?}"),
                None => write!(f, "no format tag, expected {expected:?}"),
            },
            FramingError::UnknownName { name, vocabulary } => {
                write!(f, "unknown {vocabulary} name {name:?}")
            }
            FramingError::Malformed { name, detail } => {
                write!(f, "malformed {name}: {detail}")
            }
        }
    }
}

impl std::error::Error for FramingError {}

fn tagged_object(
    line: &str,
    expected_format: &'static str,
    name_field: &'static str,
    vocabulary: &'static str,
    known: &[&str],
) -> Result<(serde_json::Value, String), FramingError> {
    let value: serde_json::Value =
        serde_json::from_str(line).map_err(|error| FramingError::NotAnObject {
            detail: error.to_string(),
        })?;
    let object = value.as_object().ok_or_else(|| FramingError::NotAnObject {
        detail: "the line is JSON but not an object".to_string(),
    })?;

    let tag = object.get("format").and_then(serde_json::Value::as_str);
    if tag != Some(expected_format) {
        return Err(FramingError::UnknownFormat {
            tag: tag.map(ToString::to_string),
            expected: expected_format,
        });
    }

    let name = object
        .get(name_field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    if !known.contains(&name.as_str()) {
        return Err(FramingError::UnknownName { name, vocabulary });
    }

    Ok((value, name))
}

/// Read one event line.
///
/// # Errors
///
/// [`FramingError::UnknownFormat`] for a tag this build does not know,
/// [`FramingError::UnknownName`] for an event name it does not know, and
/// [`FramingError::Malformed`] when the name was known and the payload did not fit. An
/// unrecognised **field** is not an error and is dropped in silence.
pub fn parse_event_line(line: &str) -> Result<EventLine, FramingError> {
    let (value, name) = tagged_object(line, EVENT_FORMAT, "event", "event", &EVENT_NAMES)?;
    serde_json::from_value(value).map_err(|error| FramingError::Malformed {
        name,
        detail: error.to_string(),
    })
}

/// Read one command line.
///
/// # Errors
///
/// The same three refusals as [`parse_event_line`], against the command vocabulary.
pub fn parse_command_line(line: &str) -> Result<CommandLine, FramingError> {
    let (value, name) = tagged_object(line, COMMAND_FORMAT, "command", "command", &COMMAND_NAMES)?;
    serde_json::from_value(value).map_err(|error| FramingError::Malformed {
        name,
        detail: error.to_string(),
    })
}
