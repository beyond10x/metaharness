//! The harness-neutral wire.
//!
//! A run emits [`Event`]s and accepts [`Command`]s, and nothing on this wire names a vendor: the
//! same stream describes a Claude Code session and a Codex session, which is the whole point —
//! an embedder written against this crate cannot accidentally depend on which harness is inside.
//!
//! The protocol is being designed in `docs/design/metaharness-protocol-v0.1.md`; the types here
//! are the placeholder that keeps the workspace honest until the design is accepted.

use serde::{Deserialize, Serialize};

/// Something a run tells the outside world.
///
/// Placeholder until the protocol design is accepted; the design owns the real vocabulary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum Event {
    /// The run started, and this is what it knows about itself.
    Started {
        /// The adapter kind driving this run, e.g. `claude` or `codex`.
        kind: String,
    },
    /// The run ended.
    Ended {
        /// Process exit code of the underlying harness, when there was one.
        exit_code: Option<i32>,
    },
}

/// Something the outside world tells a run.
///
/// Placeholder until the protocol design is accepted.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum Command {
    /// Stop the run as soon as the harness allows.
    Halt,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_wire_round_trips_as_json_lines() {
        let event = Event::Started {
            kind: "claude".into(),
        };
        let line = serde_json::to_string(&event).expect("serializes");
        let back: Event = serde_json::from_str(&line).expect("parses");
        assert!(matches!(back, Event::Started { kind } if kind == "claude"));
    }
}
