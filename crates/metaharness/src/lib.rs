//! One interface to many agent harnesses: observable, steerable, hermetic.
//!
//! ```no_run
//! use metaharness::{Input, Metaharness};
//! use metaharness::protocol::{Command, Decision, DecisionMode, Event, Kind};
//!
//! let mut run = Metaharness::new(Kind::Claude)
//!     .with_decisions(DecisionMode::Ask)
//!     .with_prompt("tidy the imports")
//!     .start(Input::FromSpec)?;                 // spawns the real `claude`
//!
//! while let Some(line) = run.next_event()? {
//!     if let Event::ToolRequested { call_id, decision_required: true, .. } = &line.event {
//!         run.send(Command::ToolDecide {
//!             call_id: call_id.clone(),
//!             decision: Decision::Allow,
//!         })?;
//!     }
//! }
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```
//!
//! # What this build does and does not do
//!
//! | | |
//! |---|---|
//! | spec → launch plan → transcript → events → audit | **built**, and exercised end to end through [`ScriptedRunner`] |
//! | driving the real vendor binary | **built.** [`Metaharness::start`] spawns it through [`SpawnRunner`], and the `PreToolUse` hook it installs answers over a real channel |
//! | `Kind::Codex` | **CX-M1**: the rollout reader, declared capabilities, doctor pin and C2 vectors exist; `run codex` is refused by name until a driven spawn (CX-M2) |
//! | `--frame <file>` | **built** (amendment a5): a sealed `metaharness.frame/1` document, resolved by the library at start, refused by name when unreadable, untagged, misshapen or digest-broken |
//! | `--tool-surface owned` | **refused.** Strategy C means metaharness implements the tools itself, and per-step re-listing is unverified vendor behaviour |
//!
//! # Three properties that are easy to break by accident
//!
//! * **`next_event` hands over every currently-pending `tool.requested` before an answer to any
//!   of them is due, and each one's deadline is armed at delivery.** Without both, a
//!   single-threaded policy deciding call A would burn call B's budget and metaharness would
//!   emit `deadline` denies the embedder never chose (design § 7.7 rule 5, finding F15).
//! * **The decision reaches the child before any control does.** Cancelling first clears the
//!   active call and leaves the child waiting on a correlation that no longer exists (rule 1).
//! * **A missing field in the opening record is `unk`, never a zero and never a pass.** A bound
//!   that read a missing MCP list as an empty one would report its blindest case as its best one.
//!
//! Everything here is decided by `docs/design/metaharness-protocol-v0.1.md`. Where this crate
//! and that document could disagree, the document wins and the disagreement is a defect here.

mod audit;
mod auditor;
mod builder;
mod clock;
mod doctor;
mod process;
mod refusal;
mod run;
mod scripted;
mod spawn;
mod spawn_vectors;
mod vectors;

/// The harness-neutral wire: the events, the commands and the one options type.
///
/// Re-exported so an embedder needs one dependency to write a decision policy, and so the
/// binary can `derive` its `run` flags on the same `RunSpec` the library takes.
pub use metaharness_protocol as protocol;

pub use audit::{
    AuditReport, FloorInputs, RunExit, anything_was_adjudicated, decision_census,
    exit_without_audit, hermetic_floor,
};
pub use auditor::{
    AuditorInvoker, AuditorRun, AuditorVerdict, FakeAuditor, ProcessAuditor, auditor_argv,
    count_verdict_rows, run_auditor,
};
pub use builder::{Input, Metaharness, check_spec, start_refusals};
pub use clock::{Clock, ManualClock, SystemClock};
pub use doctor::{Installed, installed};
pub use process::{
    CredentialCopyView, HarnessProcess, LaunchPlanView, ProcessRunner, copy_credentials,
};
pub use refusal::Refusal;
pub use run::{
    DEADLINE_MARGIN_MS, DEFAULT_VENDOR_HOOK_TIMEOUT_MS, PendingCall, Run, deadline_reason,
    decider_name, metaharness_deadline_ms, request_digest, seam_name, vendor_hook_timeout_ms,
    warning,
};
pub use scripted::{
    ScriptStep, ScriptedLog, ScriptedProcess, ScriptedRunner, ScriptedSeam, ScriptedSeams,
};
pub use spawn::{HookChannel, SpawnRunner, SpawnedProcess};
// The seam's neutral traits live in the protocol crate and its Claude half in the
// adapter crate; both are re-exported here so an embedder needs one import.
pub use metaharness_claude::{ClaudeSeam, ClaudeSeams};
pub use metaharness_protocol::{HarnessSeam, SeamFactory};
pub use spawn_vectors::spawn_vectors;
pub use vectors::{all_passed, capabilities, conformance_vectors, control_vectors};

/// The adapter ids this build carries, in the order the CLI lists them.
///
/// Published as a value so a caller can ask what exists rather than discovering by refusal.
pub const ADAPTERS: [&str; 2] = [
    metaharness_claude::ADAPTER_ID,
    metaharness_codex::ADAPTER_ID,
];
