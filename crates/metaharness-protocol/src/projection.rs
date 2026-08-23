//! The projection into `trace-ir/1`.
//!
//! metaharness invents no second IR (design D1). The event stream is the transport, `trace-ir/1`
//! is the judged form, and the projection between them is a **total** function: every event maps
//! to exactly one family, or to none — and the events mapping to none are listed exhaustively in
//! [`CONTROL_PLANE_EVENTS`], so "none" is a decision rather than an omission (design D6).
//!
//! **This module does not depend on `trace-domain`, and cannot.** `trace-ir/1` is today a
//! `Serialize`-only Rust type in another repository with no published schema (finding F1, Q9),
//! so what is asserted here is *structural*: the family each event belongs to, and that the wire
//! line carries the keys that family's fields are filled from. Three IR fields are exempt
//! because they are properties of a **file** rather than of an event stream —
//! `transcript_digest`, `source_line` and `adapter` (design D6a) — and the first two are
//! reachable only because § 8.4 O8 makes the adapter retain the bytes.

use std::collections::BTreeMap;

use crate::event::{Event, TranscriptRef};

/// The `trace-ir/1` families this protocol projects into.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IrFamily {
    /// The opening record.
    SessionStart,
    /// The terminal record.
    RunOutcome,
    /// What the model said.
    AssistantText,
    /// What the model reasoned.
    AssistantThinking,
    /// The harness's live thinking estimate.
    ThinkingEstimate,
    /// Text the harness injected.
    SyntheticInjection,
    /// A tool call.
    ToolCall,
    /// A tool result.
    ToolResult,
    /// A rate-limit window.
    RateLimit,
    /// A record the reader did not understand, preserved.
    Opaque,
}

impl IrFamily {
    /// The family's name in `trace-ir/1`.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            IrFamily::SessionStart => "session_start",
            IrFamily::RunOutcome => "run_outcome",
            IrFamily::AssistantText => "assistant_text",
            IrFamily::AssistantThinking => "assistant_thinking",
            IrFamily::ThinkingEstimate => "thinking_estimate",
            IrFamily::SyntheticInjection => "synthetic_injection",
            IrFamily::ToolCall => "tool_call",
            IrFamily::ToolResult => "tool_result",
            IrFamily::RateLimit => "rate_limit",
            IrFamily::Opaque => "opaque",
        }
    }
}

/// The events that map to no IR family.
///
/// Exhaustive on purpose: this list is what makes "maps to none" a decision. The design lists
/// seven; the eighth, `auth.expired`, is a **deviation from design v0.1** recorded as Q13 — it
/// is a control-plane fact about the credential, and the IR has no family for it.
pub const CONTROL_PLANE_EVENTS: [&str; 8] = [
    "step.entered",
    "step.left",
    "turn.started",
    "turn.ended",
    "tool.decided",
    "command.result",
    "warning",
    "auth.expired",
];

/// Which family an event projects into, if any.
///
/// The match is exhaustive with no wildcard arm, so a new event cannot be added without this
/// question being answered for it — which is the mechanical form of "the projection is total".
#[must_use]
pub fn ir_family(event: &Event) -> Option<IrFamily> {
    match event {
        Event::SessionStarted { .. } => Some(IrFamily::SessionStart),
        // `usage` folds into `run_outcome.usage` / `requests` rather than standing alone: a
        // computed summary on the wire would be a second copy of the numbers (design § 4.3).
        Event::SessionEnded { .. } | Event::Usage { .. } => Some(IrFamily::RunOutcome),
        Event::Text { .. } => Some(IrFamily::AssistantText),
        Event::Thinking { .. } => Some(IrFamily::AssistantThinking),
        Event::ThinkingEstimate { .. } => Some(IrFamily::ThinkingEstimate),
        Event::Injection { .. } => Some(IrFamily::SyntheticInjection),
        Event::ToolRequested { .. } => Some(IrFamily::ToolCall),
        Event::ToolResult { .. } => Some(IrFamily::ToolResult),
        Event::RateLimit { .. } => Some(IrFamily::RateLimit),
        Event::Opaque { .. } => Some(IrFamily::Opaque),
        Event::StepEntered { .. }
        | Event::StepLeft { .. }
        | Event::TurnStarted { .. }
        | Event::TurnEnded { .. }
        | Event::ToolDecided { .. }
        | Event::CommandResult { .. }
        | Event::Warning { .. }
        | Event::AuthExpired { .. } => None,
    }
}

/// The `trace-ir/1` fields a family carries, as this protocol fills them.
///
/// Named here so the claim "the event payload can fill this family" is a value a test reads,
/// rather than a sentence in a design document. The three exempt fields (`transcript_digest`,
/// `source_line`, `adapter`) are not in these lists: two come from the retained transcript and
/// the third is expected to differ, because the whole point of the cross-check is that two
/// different readers agreed (design D6a).
#[must_use]
pub fn required_ir_fields(family: IrFamily) -> &'static [&'static str] {
    match family {
        IrFamily::SessionStart => &[
            "model",
            "permission_mode",
            "credential_source",
            "harness_version",
            "output_style",
            "cwd",
            "offered_tools",
            "slash_commands",
            "skills",
            "agents",
            "plugins",
            "mcp_servers",
        ],
        IrFamily::RunOutcome => &[
            "is_error",
            "subtype",
            "stop_reason",
            "terminal_reason",
            "api_error_status",
            "num_turns",
            "duration_ms",
            "duration_api_ms",
            "ttft_ms",
            "time_to_request_ms",
            "total_cost_usd",
            "permission_denials",
            "subagents_spawned",
            "usage",
            "model_usage",
        ],
        IrFamily::AssistantText | IrFamily::AssistantThinking => &["text"],
        IrFamily::ThinkingEstimate => &["estimate", "delta"],
        IrFamily::SyntheticInjection => &["text", "origin"],
        IrFamily::ToolCall => &["call_id", "name", "input"],
        // `tool_use_result` since amendment a9: the IR's per-tool result fields are read out of
        // it, and while it was missing the strongest single claim a checker can make about a
        // step — that the skill it invoked also *completed* — was undecidable for a driven run.
        IrFamily::ToolResult => &["call_id", "is_error", "content", "tool_use_result"],
        IrFamily::RateLimit => &["info"],
        IrFamily::Opaque => &["vendor_type", "vendor_subtype", "digest", "source_line"],
    }
}

/// What a projection of these events would look like, without building an IR nobody can read
/// back.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectionReport {
    /// How many events landed in each family, keyed by the family's IR name.
    pub families: BTreeMap<&'static str, u32>,
    /// How many events are control-plane and map to no family.
    pub control_plane: u32,
    /// Every required field a family needed and the event could not supply, as
    /// `(event name, field)`.
    pub gaps: Vec<(&'static str, &'static str)>,
    /// The retained transcript, from which `transcript_digest` and `source_line` are filled.
    /// `None` means the projection cannot fill either, which is a defect in the adapter and not
    /// a property of the run (design § 8.4 O8).
    pub transcript: Option<TranscriptRef>,
}

impl ProjectionReport {
    /// How many events were projected in total.
    #[must_use]
    pub fn total(&self) -> u32 {
        self.families.values().sum::<u32>() + self.control_plane
    }
}

/// Project a run's events, structurally.
///
/// Total by construction: every event either increments a family or the control-plane count, so
/// [`ProjectionReport::total`] equals the number of events given. A field a family needs and the
/// event does not carry is a **gap**, listed rather than defaulted — a projection that filled a
/// missing field with a zero would report its blindest case as its best one.
#[must_use]
pub fn project(events: &[Event]) -> ProjectionReport {
    let mut report = ProjectionReport::default();
    for event in events {
        let Some(family) = ir_family(event) else {
            report.control_plane += 1;
            continue;
        };
        *report.families.entry(family.as_str()).or_default() += 1;

        if let Event::SessionStarted { transcript, .. } = event {
            report.transcript = Some(transcript.clone());
        }

        // `usage` folds into `run_outcome` and carries only the usage half of it, so it is
        // checked against what it actually contributes rather than against the terminal
        // record's whole field set.
        let required: &[&str] = if matches!(event, Event::Usage { .. }) {
            &["usage"]
        } else {
            required_ir_fields(family)
        };
        let Ok(serde_json::Value::Object(fields)) = serde_json::to_value(event) else {
            report
                .gaps
                .push((event.name(), "the event did not serialize"));
            continue;
        };
        for field in required {
            if !fields.contains_key(*field) {
                report.gaps.push((event.name(), field));
            }
        }
    }
    report
}
