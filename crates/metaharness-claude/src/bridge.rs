//! The seam: this adapter's transcript reader in, its hook response out.
//!
//! It lives here and not in the library face because every line of it is Claude-specific — the
//! hook-response body, the control-request subtypes, which commands reach the child by no line
//! at all. A neutral crate holding one vendor's wire is a neutral crate that will hold two.

use metaharness_protocol::{
    Command, Decision, DecisionCensus, Emission, HarnessSeam, HermeticAttestation, Seam,
    SeamFactory, TranscriptRef,
};

/// The Claude Code seam: the adapter's transcript reader in, its hook response out.
///
/// **The response envelope is provisional and is owed to `metaharness-claude`.** The adapter
/// publishes `render_hook_response`, which is the body the vendor's hook wire wants; it
/// publishes no correlation envelope, because the real hook correlates by *which hook process is
/// answering* and there is no hook process until the real spawner exists. So the envelope below
/// is metaharness's own, it is the shape the scripted process is driven with, and moving it into
/// the adapter is a v0.2 change, not a v0.2 discovery.
pub struct ClaudeSeam {
    reader: crate::TranscriptReader,
}

impl std::fmt::Debug for ClaudeSeam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClaudeSeam").finish_non_exhaustive()
    }
}

impl ClaudeSeam {
    /// A seam over the retained transcript and the attestation that goes into `session.started`.
    #[must_use]
    pub fn new(transcript: TranscriptRef, attestation: HermeticAttestation, seam: Seam) -> Self {
        Self {
            reader: crate::TranscriptReader::new(transcript, attestation).with_seam(seam),
        }
    }
}

impl HarnessSeam for ClaudeSeam {
    fn push_line(&mut self, line: &str) -> Vec<Emission> {
        self.reader.push_line(line)
    }

    fn finish(&mut self) -> Vec<Emission> {
        self.reader.finish()
    }

    fn set_census(&mut self, census: DecisionCensus) {
        self.reader.set_census(census);
    }

    fn decision_line(&self, call_id: &str, decision: &Decision) -> String {
        serde_json::json!({
            "call_id": call_id,
            "response": crate::render_hook_response(decision),
        })
        .to_string()
    }

    fn control_line(&self, command: &Command) -> Option<String> {
        match command {
            // `tool.decide` reaches the child as a decision line, not a control line.
            // `frame.set` reaches the *model* as injected text at the next boundary and reaches
            // the child as nothing, so a line here would be a second copy of the frame.
            // `steer` does not exist headless on this adapter and is refused by name before it
            // reaches here (design § 7.3); the other two reach the child by no line at all.
            Command::ToolDecide { .. } | Command::FrameSet { .. } | Command::Steer { .. } => None,
            Command::MessageInject { text } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "inject", "text": text },
                })
                .to_string(),
            ),
            Command::PermissionSet { posture } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "set_permission_mode", "mode": posture },
                })
                .to_string(),
            ),
            Command::Interrupt { .. } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "interrupt" },
                })
                .to_string(),
            ),
            Command::Halt { .. } => Some(
                serde_json::json!({
                    "type": "control_request",
                    "request": { "subtype": "interrupt", "halt": true },
                })
                .to_string(),
            ),
        }
    }
}

/// The factory for [`ClaudeSeam`].
#[derive(Debug, Clone, Copy, Default)]
pub struct ClaudeSeams;

impl SeamFactory for ClaudeSeams {
    fn build(
        &mut self,
        transcript: TranscriptRef,
        attestation: HermeticAttestation,
        seam: Seam,
    ) -> Box<dyn HarnessSeam> {
        Box::new(ClaudeSeam::new(transcript, attestation, seam))
    }
}
