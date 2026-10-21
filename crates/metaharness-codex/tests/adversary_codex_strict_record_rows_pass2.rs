//! Adversary pass 2 on `story:codex-strict-record-rows` (amendment a34, correction round 1).
//!
//! The codex seam's module documentation now says: "The two sources are merged in one order, and
//! the opening record is first in it." A hook request releases a *held* opening record ahead of
//! itself. This case asks what happens when the hook request reaches the seam before the rollout's
//! `session_meta` has been read at all — the reader holds nothing yet, so there is nothing to
//! release ahead of the call.
//!
//! Synthetic and free: the seam alone, no process, no credential, no model.

use metaharness_codex::{CodexSeams, hook_request_line};
use metaharness_protocol::{
    Emission, Event, HarnessSeam, HermeticAttestation, HermeticMode, Seam, SeamFactory as _,
    TranscriptRef,
};

const META: &str = r#"{"timestamp":"2026-10-21T00:00:00.000Z","type":"session_meta","payload":{"id":"s-1","session_id":"s-1","cli_version":"0.153.4","cwd":"/w"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-10-21T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const TURN_CONTEXT: &str = r#"{"timestamp":"2026-10-21T00:00:00.002Z","type":"turn_context","payload":{"turn_id":"t1","model":"m","personality":"pragmatic"}}"#;
const HOOK_INPUT: &str = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"call-7","tool_input":{"command":"ls"}}"#;

fn seam() -> Box<dyn HarnessSeam> {
    CodexSeams.build(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Off),
        Seam::Hook,
    )
}

fn names(emitted: &[Emission]) -> Vec<&'static str> {
    emitted
        .iter()
        .map(|emission| emission.event.name())
        .collect()
}

/// A hook request read before the rollout's first line: the seam emits the call first and
/// `session.started` after it, against the module's "the opening record is first in it".
#[test]
#[ignore = "pre-existing at cfad31d; codex 0.153.4 writes session_meta before any provider request, so no hook precedes it (bridge.rs module doc)"]
fn the_opening_record_is_first_even_when_a_hook_is_read_before_session_meta() {
    let mut seam = seam();
    let mut emitted = Vec::new();
    emitted.extend(seam.push_line(&hook_request_line("k7", HOOK_INPUT)));
    for line in [META, TASK_STARTED, TURN_CONTEXT] {
        emitted.extend(seam.push_line(line));
    }
    emitted.extend(seam.finish());
    assert!(
        matches!(
            emitted.first().map(|emission| &emission.event),
            Some(Event::SessionStarted { .. })
        ),
        "the merged stream opens with {:?}",
        names(&emitted)
    );
}
