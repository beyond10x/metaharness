//! The harness-neutral wire.
//!
//! A run emits [`Event`]s and accepts [`Command`]s, and nothing on this wire names a vendor: the
//! same stream describes a Claude Code session and a Codex session, which is the whole point —
//! an embedder written against this crate cannot accidentally depend on which harness is inside.
//!
//! Everything here is decided by `docs/design/metaharness-protocol-v0.1.md`. Where this crate
//! and that document could disagree, the document wins and the disagreement is a defect here;
//! where the document was found wrong, the correction is recorded in its own register (a `Q`
//! row) and cited at the point of change in this code.
//!
//! # The shape of a line
//!
//! ```text
//! {"format":"metaharness.event/1","seq":7,"run":"r-1","at":"…","event":"tool.requested",…}
//! {"format":"metaharness.command/1","id":"c-3","command":"tool.decide",…}
//! ```
//!
//! # Three rules that are easy to break by accident
//!
//! * **`seq` is assigned in one place** — [`EventStream`] — because a producer that numbered its
//!   own events would be a second place that decides what a verdict cites (design D2).
//! * **`at` is the vendor's recorded timestamp, passed through or absent.** Nothing in this
//!   crate reads a clock, so a run's numbers can be committed and diffed (design D2).
//! * **An absent payload field serializes as `null` and is never skipped.** Absence is the `unk`
//!   verdict, and a field that vanished from the line cannot be told apart from a field nobody
//!   asked for (design § 8.1: absence of evidence is not hermeticity).

mod capability;
mod command;
mod conformance;
mod event;
mod frame;
mod framing;
mod hermetic;
mod projection;
mod seam;
mod spec;

pub use capability::{
    AdapterClass, AdapterId, Capabilities, CommandSupport, Tier, TierStatus, required_commands,
};
pub use command::{Command, CommandOutcome, Decision, RefusalCode, Refused};
pub use conformance::{ConformanceTier, VectorOutcome};
pub use event::{
    DecidedBy, DecisionCensus, Emission, Event, McpServerRef, PermissionDenial, PluginRef,
    RateLimitInfo, Seam, StepOutcome, TranscriptRef, Usage, warning_code,
};
pub use frame::{
    Digest, EntityList, EvidenceLine, FRAME_FORMAT, Frame, FrameDocError, Handoff, Line, NodeRef,
    Operation, OperationSet, StepRef, WorkflowRef,
};
pub use framing::{
    COMMAND_FORMAT, COMMAND_NAMES, CommandLine, EVENT_FORMAT, EVENT_NAMES, EventLine, EventStream,
    FramingError, RunId, parse_command_line, parse_event_line,
};
pub use hermetic::{
    Assertion, HermeticAttestation, HermeticMode, HermeticRow, ImposedControl, RowVerdict,
    Severity, UnavailableControl, Verdict,
};
pub use projection::{
    CONTROL_PLANE_EVENTS, IrFamily, ProjectionReport, ir_family, project, required_ir_fields,
};
pub use seam::{HarnessSeam, SeamFactory};
pub use spec::{CredentialSource, DecisionMode, Kind, RunSpec, ToolSurface};
