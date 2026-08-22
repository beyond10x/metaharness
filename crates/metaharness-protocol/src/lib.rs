//! The harness-neutral wire.
//!
//! A run emits [`Event`]s and accepts [`Command`]s, and nothing on this wire names a vendor: the
//! same stream describes a Claude Code session and a Codex session, which is the whole point —
//! an embedder written against this crate cannot accidentally depend on which harness is inside.
//!
//! The protocol is being designed in `docs/design/metaharness-protocol-v0.1.md`; the types here
//! are the placeholder that keeps the workspace honest until the design is accepted.

use serde::{Deserialize, Serialize};

/// The format tag every event line carries.
///
/// On every line rather than on a handshake, so a truncated capture is still self-describing
/// (design § 3, D2). The version moves when a field is removed, retyped or given new meaning;
/// an added field is additive and does not move it (D3).
pub const EVENT_FORMAT: &str = "metaharness.event/1";

/// The format tag every command line carries. Versioned on the same rule as [`EVENT_FORMAT`].
pub const COMMAND_FORMAT: &str = "metaharness.command/1";

/// Something a run tells the outside world.
///
/// Placeholder until the protocol design is accepted; the design owns the real vocabulary. The
/// two variants here carry the wire names § 4.1 decides, so the crate cannot contradict the
/// document while it is unbuilt.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event")]
pub enum Event {
    /// The run started, and this is what it knows about itself.
    #[serde(rename = "session.started")]
    Started {
        /// The adapter kind driving this run, e.g. `claude` or `codex`.
        kind: String,
    },
    /// The run ended.
    #[serde(rename = "session.ended")]
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
    ///
    /// Every adapter must deliver this one (design § 6): a control surface with no way out is
    /// not a control surface.
    Halt {
        /// Why, for the run report.
        reason: String,
    },
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

    /// The tag is the wire name the design decided, not the variant's Rust spelling. Asserted
    /// because a rename that only lands in a document is a rename that has not landed.
    #[test]
    fn events_carry_the_wire_names_the_design_decided() {
        let started = serde_json::to_string(&Event::Started {
            kind: "codex".into(),
        })
        .expect("serializes");
        assert!(
            started.contains(r#""event":"session.started""#),
            "{started}"
        );

        let ended =
            serde_json::to_string(&Event::Ended { exit_code: Some(0) }).expect("serializes");
        assert!(ended.contains(r#""event":"session.ended""#), "{ended}");

        let halt = serde_json::to_string(&Command::Halt {
            reason: "the operator asked".into(),
        })
        .expect("serializes");
        assert!(halt.contains(r#""command":"halt""#), "{halt}");
    }
}
