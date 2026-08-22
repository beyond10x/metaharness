//! What the outside world tells a run.
//!
//! Every command carries an `id` and produces exactly one [`crate::Event::CommandResult`].
//! Silence is not a legal outcome: a command that can be silently ignored is a control surface
//! that cannot be tested (design D9). A control this adapter cannot honour is refused **by
//! name**, at run start rather than at the call, so a run that will fail on control fails before
//! it spends money (design § 6.1).

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::frame::Frame;

/// What the embedder decided about one tool call.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "decision", rename_all = "snake_case")]
pub enum Decision {
    /// Let the call run.
    ///
    /// **This grants, and that is a departure worth naming** (design § 6, finding F8): the
    /// harness honours a hook `allow` and bypasses the rest of its permission pipeline, so an
    /// `allow` from metaharness overrides a stricter rule elsewhere in the vendor's settings. A
    /// run that also relies on such a rule must use `deny`-only policy and say so.
    Allow,
    /// Refuse the call, and tell the model why.
    ///
    /// The reason is required and must be non-empty: both vendors' hook wires require it, and
    /// the reason is the only part the model can act on — the difference between a wall and an
    /// instruction (design § 6).
    Deny {
        /// Why, in words the model is told.
        reason: String,
    },
    /// Run the call with this input instead.
    ///
    /// Exists because both vendors' hook wires carry an updated input, and refusing to expose it
    /// would push embedders into deny-and-re-prompt, which costs a turn to express something the
    /// wire already supports. A `replace` an adapter cannot deliver is refused by name; it never
    /// silently becomes an `allow` (design § 6).
    Replace {
        /// The input to run instead.
        input: Value,
    },
    /// Claim nothing, and let the vendor's own permission pipeline decide.
    ///
    /// **Added by amendment a3, and it is the default policy's answer when no frame is in
    /// force.** Without it the only way to let a call through is [`Decision::Allow`], which
    /// *grants* — it bypasses the rest of the vendor's permission pipeline and overrides a
    /// stricter rule in the vendor's own settings (§ 6). So a run with no frame, deciding
    /// `allow` because it had nothing to narrow with, would silently be a run with the vendor's
    /// permission system switched off. Abstaining says the true thing instead: metaharness
    /// adjudicated nothing here.
    ///
    /// It is the convention § 2.2 records as proven on Claude Code — the reference hook passes a
    /// call through by exiting 0 and emitting **no `permissionDecision` at all**, because
    /// "saying `allow` here would claim an authority the layer does not have".
    Abstain,
}

impl Decision {
    /// Whether this decision is well-formed on its own terms.
    ///
    /// The one rule is the empty deny reason, which every vendor wire rejects and which would
    /// reach the model as a wall.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        match self {
            Decision::Deny { reason } => !reason.trim().is_empty(),
            Decision::Allow | Decision::Replace { .. } | Decision::Abstain => true,
        }
    }
}

/// Why a command was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RefusalCode {
    /// This adapter cannot honour this command at all. Emitted at run start for every command
    /// the run's configuration will need.
    UnsupportedControl,
    /// The `call_id` does not correlate to an open request.
    UnknownCall,
    /// The window closed — the decision deadline expired, or the turn ended.
    TooLate,
    /// The command did not parse, or a required field is missing.
    Malformed,
    /// The vendor would accept this and another layer would silently override it. Refused rather
    /// than delivered, because a control that appears to work and does not is worse than one
    /// that is absent (design § 6.1).
    Shadowed,
}

impl RefusalCode {
    /// The code's wire spelling.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            RefusalCode::UnsupportedControl => "UNSUPPORTED_CONTROL",
            RefusalCode::UnknownCall => "UNKNOWN_CALL",
            RefusalCode::TooLate => "TOO_LATE",
            RefusalCode::Malformed => "MALFORMED",
            RefusalCode::Shadowed => "SHADOWED",
        }
    }
}

/// A refusal, by name and with a reason.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Refused {
    /// Which refusal.
    pub code: RefusalCode,
    /// Why, for a person reading the run.
    pub reason: String,
}

impl Refused {
    /// A refusal with this code and reason.
    #[must_use]
    pub fn new(code: RefusalCode, reason: impl Into<String>) -> Self {
        Self {
            code,
            reason: reason.into(),
        }
    }
}

/// What happened to a command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum CommandOutcome {
    /// The command was honoured.
    Ok {
        /// Which boundary it applies at, when that is not immediately. `frame.set` takes effect
        /// at the **next** turn or step boundary and its result says which, because a frame that
        /// took effect mid-turn would mean a call adjudicated against a frame the model was
        /// never shown (design § 5.4).
        applies_at: Option<String>,
    },
    /// The command was refused, by name.
    Refused {
        /// The refusal.
        refused: Refused,
    },
}

/// Something the outside world tells a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "command")]
pub enum Command {
    /// Decide one pending tool call. Needs the call tier.
    #[serde(rename = "tool.decide")]
    ToolDecide {
        /// Which call.
        call_id: String,
        /// What to do with it.
        decision: Decision,
    },
    /// Put a new frame in force at the next boundary.
    ///
    /// Not partially deliverable: an adapter that can inject the text but cannot enforce it
    /// would tell the model "strictly only these operations" and make it false, so a run whose
    /// configuration needs `frame.set` without call-level enforcement is refused at start
    /// (design § 6, finding F9). An embedder that genuinely wants advisory text uses
    /// [`Command::MessageInject`], which claims nothing.
    #[serde(rename = "frame.set")]
    FrameSet {
        /// The frame.
        frame: Box<Frame>,
    },
    /// Add text to the conversation between turns. Needs the turn tier.
    #[serde(rename = "message.inject")]
    MessageInject {
        /// The text.
        text: String,
    },
    /// Steer a running turn. Needs a mid-turn tier, which Claude Code headless does not have —
    /// on that adapter this is always refused by name (design § 7.3).
    #[serde(rename = "steer")]
    Steer {
        /// The steer.
        text: String,
    },
    /// Change the permission posture mid-run. Needs the run tier.
    #[serde(rename = "permission.set")]
    PermissionSet {
        /// The posture, in the vendor's own vocabulary.
        posture: String,
    },
    /// Stop the running turn. Every adapter must deliver this.
    #[serde(rename = "interrupt")]
    Interrupt {
        /// Why, for the run report.
        reason: String,
    },
    /// Stop the run. Every adapter must deliver this: a control surface with no way out is not a
    /// control surface.
    #[serde(rename = "halt")]
    Halt {
        /// Why, for the run report.
        reason: String,
    },
}

impl Command {
    /// The command's wire name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Command::ToolDecide { .. } => "tool.decide",
            Command::FrameSet { .. } => "frame.set",
            Command::MessageInject { .. } => "message.inject",
            Command::Steer { .. } => "steer",
            Command::PermissionSet { .. } => "permission.set",
            Command::Interrupt { .. } => "interrupt",
            Command::Halt { .. } => "halt",
        }
    }
}
