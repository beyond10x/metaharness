//! The run loop: events out, commands in, decisions correlated.
//!
//! Everything in § 7.7 lives here, and each rule is a named method rather than a comment:
//!
//! | rule | where |
//! |---|---|
//! | 1 — the decision is written before any control is applied | [`Run::abandon_pending`] writes every pending decision before the control line |
//! | 2 — metaharness's deadline is strictly less than the vendor's | [`metaharness_deadline_ms`] |
//! | 3 — a decision correlates to one request and cannot be replayed | [`request_digest`], and `UNKNOWN_CALL` / `TOO_LATE` in [`Run::decide`] |
//! | 4 — `interrupt` is a legal answer to a pending decision | [`Run::abandon_pending`], reached from `interrupt` and `halt` |
//! | 5 — several decisions may be pending and may be answered out of order | the outbox drains before any deadline is checked, and a deadline is armed at delivery |
//!
//! **Synchronous does not mean one decision at a time.** [`Run::next_event`] hands over every
//! currently-pending `tool.requested` before an answer to any of them is due, and each one's
//! budget starts when the embedder is handed it — not when the vendor asked. Without both, a
//! single-threaded policy deciding call A would burn call B's budget and metaharness would emit
//! `deadline` denies the embedder never chose (finding F15).

use std::collections::{BTreeMap, VecDeque};
use std::io::ErrorKind;

use metaharness_protocol::{
    Capabilities, Command, CommandOutcome, CommandSupport, DecidedBy, Decision, DecisionCensus,
    DecisionMode, Digest, Emission, Event, EventLine, EventStream, Frame, Operation, RefusalCode,
    Refused, RunSpec, Seam, TranscriptRef,
};
use serde_json::Value;

use crate::clock::Clock;
use crate::process::HarnessProcess;
use metaharness_protocol::HarnessSeam;

/// Short codes for the warnings this crate raises, so an embedder matches a code and not prose.
pub mod warning {
    /// The run is in `frame` mode and no frame is in force, so nothing narrowed this call.
    ///
    /// Emitted once. Denying every call because there is no admitted set would make the default
    /// invocation do nothing and bill for it; allowing every call would *grant*, which on this
    /// wire overrides a stricter rule in the vendor's own settings. So the call is neither
    /// denied nor granted: metaharness abstains and the warning says so (amendment a3).
    pub const NO_FRAME_IN_FORCE: &str = "NO_FRAME_IN_FORCE";
    /// The window closed on a call nobody decided. The vendor has moved on; a later
    /// `tool.decide` for it is `TOO_LATE`.
    pub const PENDING_CALL_ABANDONED: &str = "PENDING_CALL_ABANDONED";
    /// The same `call_id` came back with a different input. Refused by name, never silently
    /// approved under the earlier decision.
    pub const REQUEST_MUTATED: &str = "REQUEST_MUTATED";
    /// A tool was called that no operation in the v0.1 vocabulary renders to, so the frame has
    /// no way to admit it (design § 7.8).
    pub const UNCOVERED_TOOL: &str = "UNCOVERED_TOOL";
}

/// The vendor hook timeout metaharness assumes when the adapter's plan states none.
///
/// A guess, and it is safe in the only direction that matters: a guess that is too **high**
/// makes metaharness's own deadline too late and hands the ambiguity back to the vendor, so the
/// number is deliberately modest.
pub const DEFAULT_VENDOR_HOOK_TIMEOUT_MS: u64 = 30_000;

/// How far below the vendor's timeout metaharness's own deadline sits.
pub const DEADLINE_MARGIN_MS: u64 = 5_000;

/// metaharness's own deadline for one decision, given the vendor's timeout.
///
/// **Strictly less than the vendor's**, for every positive vendor timeout, because on expiry
/// metaharness itself emits `deny` with `decided_by: "deadline"` and that converts a
/// vendor-owned ambiguity into a metaharness-owned refusal (design § 7.7 rule 2). A vendor
/// timeout of `0` leaves no budget at all and this returns `0`: metaharness denies at the first
/// opportunity, which is the same fail-closed answer one millisecond later.
#[must_use]
pub fn metaharness_deadline_ms(vendor_timeout_ms: u64) -> u64 {
    if vendor_timeout_ms > DEADLINE_MARGIN_MS {
        vendor_timeout_ms - DEADLINE_MARGIN_MS
    } else {
        vendor_timeout_ms / 2
    }
}

/// The vendor hook timeout the adapter's hook definition declares, in milliseconds.
///
/// Read out of the plan's hook value rather than configured, because a second knob for it would
/// be a second place the two numbers could disagree — and § 7.7 rule 2's guarantee is a
/// relationship between them. The vendor states the timeout in **seconds**; a hook value that
/// states none yields [`DEFAULT_VENDOR_HOOK_TIMEOUT_MS`].
#[must_use]
pub fn vendor_hook_timeout_ms(hook: &Value) -> u64 {
    fn first_timeout(value: &Value) -> Option<u64> {
        match value {
            Value::Object(fields) => {
                if let Some(seconds) = fields.get("timeout").and_then(Value::as_u64) {
                    return Some(seconds * 1_000);
                }
                fields.values().find_map(first_timeout)
            }
            Value::Array(items) => items.iter().find_map(first_timeout),
            _ => None,
        }
    }
    first_timeout(hook).unwrap_or(DEFAULT_VENDOR_HOOK_TIMEOUT_MS)
}

/// The correlation key's second half: the digest of the request **as presented**.
///
/// A decision cannot be applied to a different input under the same id, and a request mutated
/// after the embedder saw it is a named refusal rather than a silent approval (design § 7.7
/// rule 3).
#[must_use]
pub fn request_digest(call_id: &str, name: &str, input: &Value) -> Digest {
    // `serde_json::Value`'s map is ordered, so this byte form is canonical for a given content
    // and two processes agree on the digest.
    let canonical = serde_json::json!({ "call_id": call_id, "name": name, "input": input });
    Digest::of(
        serde_json::to_vec(&canonical)
            .unwrap_or_default()
            .as_slice(),
    )
}

/// One tool call presented to the embedder and not yet decided.
#[derive(Debug, Clone, PartialEq)]
pub struct PendingCall {
    call_id: String,
    name: String,
    input: Value,
    digest: Digest,
    seam: Seam,
    deadline_ms: u64,
    armed_at_ms: Option<u64>,
}

impl PendingCall {
    /// Which call.
    #[must_use]
    pub fn call_id(&self) -> &str {
        &self.call_id
    }

    /// The vendor's tool name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The input as presented.
    #[must_use]
    pub fn input(&self) -> &Value {
        &self.input
    }

    /// The digest of the request as presented — the other half of the correlation key.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    /// The budget for deciding this call.
    #[must_use]
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// When the budget started, or `None` while the call is still in the outbox.
    ///
    /// Armed at **delivery**, so an embedder that answers in the order it was handed cannot be
    /// timed out by its own queue (design § 7.7 rule 5).
    #[must_use]
    pub fn armed_at_ms(&self) -> Option<u64> {
        self.armed_at_ms
    }
}

/// What the audit floor needs that no event carries.
#[derive(Debug, Clone)]
pub(crate) struct LaunchFacts {
    pub(crate) planned_cwd: Option<String>,
    pub(crate) declared_plugins: Vec<String>,
    pub(crate) pinned_versions: Vec<String>,
    pub(crate) transcript: TranscriptRef,
}

/// One live run: events out, commands in.
///
/// Synchronous and blocking (design D10), because the embedder this exists to serve is
/// synchronous and the studied approval mechanism is a blocking call on a worker thread. An
/// async surface would be a second concurrency model for the seam, and the seam is the thing
/// that must not have two shapes.
pub struct Run {
    spec: RunSpec,
    stream: EventStream,
    bridge: Box<dyn HarnessSeam>,
    process: Box<dyn HarnessProcess>,
    clock: Box<dyn Clock>,
    capabilities: Capabilities,
    operation_of_tool: BTreeMap<String, String>,
    frame: Option<Frame>,
    seam: Seam,
    vendor_timeout_ms: u64,
    deadline_ms: u64,
    pending: Vec<PendingCall>,
    presented: BTreeMap<String, Digest>,
    census: DecisionCensus,
    outbox: VecDeque<Emission>,
    next_command_id: u64,
    finished: bool,
    saw_terminal_record: bool,
    events: Vec<Event>,
    warned_no_frame: bool,
    pub(crate) launch: LaunchFacts,
    pub(crate) scratch: Option<tempfile::TempDir>,
}

impl std::fmt::Debug for Run {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Run")
            .field("kind", &self.spec.kind.as_str())
            .field("pending", &self.pending.len())
            .field("emitted", &self.stream.emitted())
            .field("finished", &self.finished)
            .finish_non_exhaustive()
    }
}

/// Everything [`Run::new`] needs, gathered so the constructor does not take nine arguments.
pub(crate) struct RunParts {
    pub(crate) spec: RunSpec,
    pub(crate) stream: EventStream,
    pub(crate) bridge: Box<dyn HarnessSeam>,
    pub(crate) process: Box<dyn HarnessProcess>,
    pub(crate) clock: Box<dyn Clock>,
    pub(crate) capabilities: Capabilities,
    pub(crate) frame: Option<Frame>,
    pub(crate) seam: Seam,
    pub(crate) vendor_timeout_ms: u64,
    pub(crate) launch: LaunchFacts,
    pub(crate) scratch: Option<tempfile::TempDir>,
}

impl Run {
    pub(crate) fn new(parts: RunParts) -> Self {
        let mut operation_of_tool = BTreeMap::new();
        for operation in &Operation::PARAMETERLESS {
            if let Some(tool) = metaharness_claude::render_operation(operation) {
                operation_of_tool.insert(tool.to_string(), operation.name().to_string());
            }
        }
        Self {
            spec: parts.spec,
            stream: parts.stream,
            bridge: parts.bridge,
            process: parts.process,
            clock: parts.clock,
            capabilities: parts.capabilities,
            operation_of_tool,
            frame: parts.frame,
            seam: parts.seam,
            vendor_timeout_ms: parts.vendor_timeout_ms,
            deadline_ms: metaharness_deadline_ms(parts.vendor_timeout_ms),
            pending: Vec::new(),
            presented: BTreeMap::new(),
            census: DecisionCensus::default(),
            outbox: VecDeque::new(),
            next_command_id: 1,
            finished: false,
            saw_terminal_record: false,
            events: Vec::new(),
            warned_no_frame: false,
            launch: parts.launch,
            scratch: parts.scratch,
        }
    }

    /// The spec this run was started from.
    #[must_use]
    pub fn spec(&self) -> &RunSpec {
        &self.spec
    }

    /// What metaharness's own seam has decided so far.
    #[must_use]
    pub fn census(&self) -> &DecisionCensus {
        &self.census
    }

    /// Every event delivered so far, in order.
    #[must_use]
    pub fn events(&self) -> &[Event] {
        &self.events
    }

    /// The calls presented and not yet decided, in the order they were delivered.
    #[must_use]
    pub fn pending_calls(&self) -> &[PendingCall] {
        &self.pending
    }

    /// The vendor hook timeout this run's deadline was derived from.
    #[must_use]
    pub fn vendor_timeout_ms(&self) -> u64 {
        self.vendor_timeout_ms
    }

    /// metaharness's own per-decision budget, strictly less than the vendor's timeout.
    #[must_use]
    pub fn deadline_ms(&self) -> u64 {
        self.deadline_ms
    }

    /// Whether the harness produced a terminal record.
    ///
    /// The difference between exit `0` and exit `3`: a crashed harness is not a failing run
    /// (design § 9.4).
    #[must_use]
    pub fn saw_terminal_record(&self) -> bool {
        self.saw_terminal_record
    }

    /// The retained raw transcript (design § 8.4 O8).
    #[must_use]
    pub fn transcript(&self) -> &TranscriptRef {
        &self.launch.transcript
    }

    pub(crate) fn launch_facts(&self) -> &LaunchFacts {
        &self.launch
    }

    /// The scratch root this run owns, deleted when the run is dropped.
    ///
    /// Held rather than merely created, because a `TempDir` that nobody keeps is removed the
    /// moment it is constructed — and the config home, the copied credential and the retained
    /// transcript all live under it.
    #[must_use]
    pub fn scratch_root(&self) -> Option<&std::path::Path> {
        self.scratch.as_ref().map(tempfile::TempDir::path)
    }

    /// The next event, or `None` when the run is over.
    ///
    /// # Errors
    ///
    /// Whatever the child's stream said, plus one refusal of its own: a child that is blocked on
    /// a decision while metaharness holds none pending is a state neither side can leave, and it
    /// is reported rather than spun on.
    pub fn next_event(&mut self) -> std::io::Result<Option<EventLine>> {
        loop {
            if let Some(emission) = self.outbox.pop_front() {
                return Ok(Some(self.deliver(emission)));
            }
            if self.expire_deadlines()? {
                continue;
            }
            if self.finished {
                return Ok(None);
            }
            let read = self.process.next_line();
            match read {
                Ok(Some(line)) => {
                    self.bridge.set_census(self.census.clone());
                    let emissions = self.bridge.push_line(&line);
                    self.admit(emissions)?;
                }
                Ok(None) => self.wind_up(),
                Err(error) if error.kind() == ErrorKind::WouldBlock => {
                    self.wait_out_earliest_deadline()?;
                }
                Err(error) => return Err(error),
            }
        }
    }

    /// Pump to the end and collect everything, deciding nothing.
    ///
    /// The `frame`-mode shape: the adapter answers every call from the frame, so there is
    /// nothing for the caller to do between events.
    ///
    /// # Errors
    ///
    /// As [`Run::next_event`].
    pub fn drain(&mut self) -> std::io::Result<Vec<EventLine>> {
        let mut lines = Vec::new();
        while let Some(line) = self.next_event()? {
            lines.push(line);
        }
        Ok(lines)
    }

    /// Send one command, under an id metaharness assigns.
    ///
    /// # Errors
    ///
    /// Whatever writing to the child said. A command the adapter cannot honour is **not** an
    /// error: it is a `refused` outcome, because a refusal is a result and silence is not
    /// (design D9).
    pub fn send(&mut self, command: Command) -> std::io::Result<CommandOutcome> {
        let id = format!("c-{}", self.next_command_id);
        self.next_command_id += 1;
        self.send_as(id, command)
    }

    /// Send one command under the id the caller chose — the binary's case, where the id came in
    /// on the command line and its `command.result` must carry the same one.
    ///
    /// # Errors
    ///
    /// As [`Run::send`].
    pub fn send_as(
        &mut self,
        id: impl Into<String>,
        command: Command,
    ) -> std::io::Result<CommandOutcome> {
        let outcome = self.apply(command)?;
        self.outbox
            .push_back(Emission::untimed(Event::CommandResult {
                id: id.into(),
                outcome: outcome.clone(),
            }));
        Ok(outcome)
    }

    fn deliver(&mut self, emission: Emission) -> EventLine {
        if let Event::ToolRequested {
            call_id,
            decision_required: true,
            ..
        } = &emission.event
        {
            let now = self.clock.now_ms();
            if let Some(call) = self
                .pending
                .iter_mut()
                .find(|call| call.call_id == *call_id && call.armed_at_ms.is_none())
            {
                call.armed_at_ms = Some(now);
            }
        }
        let line = self.stream.stamp(emission);
        if matches!(line.event, Event::SessionEnded { .. }) {
            self.saw_terminal_record = true;
        }
        self.events.push(line.event.clone());
        line
    }

    fn admit(&mut self, emissions: Vec<Emission>) -> std::io::Result<()> {
        for emission in emissions {
            if matches!(emission.event, Event::ToolRequested { .. }) {
                self.admit_call(emission)?;
                continue;
            }
            let closes_window = matches!(emission.event, Event::TurnEnded { .. });
            self.outbox.push_back(emission);
            if closes_window {
                self.abandon_pending("the turn ended", warning::PENDING_CALL_ABANDONED);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn admit_call(&mut self, emission: Emission) -> std::io::Result<()> {
        let at = emission.at;
        let Event::ToolRequested {
            call_id,
            name,
            input,
            ..
        } = emission.event
        else {
            unreachable!("the caller matched a tool.requested")
        };
        let digest = request_digest(&call_id, &name, &input);

        if let Some(previous) = self.presented.get(&call_id)
            && *previous != digest
        {
            // Design § 2.6 item 3: a request mutated after it was presented is a named refusal,
            // never a silent approval under the decision the embedder gave the first one.
            let seam = self.seam;
            self.outbox.push_back(Emission {
                at,
                event: Event::ToolRequested {
                    call_id: call_id.clone(),
                    name,
                    input,
                    decision_required: false,
                    deadline_ms: None,
                    seam,
                },
            });
            self.warn(
                warning::REQUEST_MUTATED,
                format!(
                    "call {call_id} was presented again with a different input; the correlation \
                     key is the call id plus the digest of the request as presented, so the \
                     earlier decision does not carry over"
                ),
            );
            let call = PendingCall {
                call_id,
                name: String::new(),
                input: Value::Null,
                digest,
                seam,
                deadline_ms: self.deadline_ms,
                armed_at_ms: None,
            };
            return self.write_decision(
                &call,
                Decision::Deny {
                    reason: "this call id was already presented with a different input".to_string(),
                },
                DecidedBy::Adapter,
                None,
            );
        }

        let first_presentation = self
            .presented
            .insert(call_id.clone(), digest.clone())
            .is_none();
        let already_pending = self.pending.iter().any(|call| call.call_id == call_id);

        let ask = self.spec.decisions == DecisionMode::Ask
            && matches!(
                self.capabilities.support("tool.decide"),
                CommandSupport::Honoured
            );

        if ask {
            if !already_pending {
                self.pending.push(PendingCall {
                    call_id: call_id.clone(),
                    name: name.clone(),
                    input: input.clone(),
                    digest,
                    seam: self.seam,
                    deadline_ms: self.deadline_ms,
                    armed_at_ms: None,
                });
            }
            self.outbox.push_back(Emission {
                at,
                event: Event::ToolRequested {
                    call_id,
                    name,
                    input,
                    decision_required: true,
                    deadline_ms: Some(self.deadline_ms),
                    seam: self.seam,
                },
            });
            return Ok(());
        }

        // `frame` mode: the adapter decides from the frame's admitted set, no round trip, and
        // `tool.decided` is emitted all the same — the census counts both modes and a
        // frame-mode run is fully audited (design D5).
        let (decision, by, seam) = self.frame_verdict(&name);
        self.outbox.push_back(Emission {
            at,
            event: Event::ToolRequested {
                call_id: call_id.clone(),
                name: name.clone(),
                input: input.clone(),
                decision_required: false,
                deadline_ms: None,
                seam,
            },
        });
        if !first_presentation {
            // The vendor repeated a call it had already presented, byte for byte. The record
            // keeps the repetition — nothing is dropped — but the decision is not taken twice: a
            // second `tool.decided` would double the census for one call and make the denial
            // count disagree with what happened.
            return Ok(());
        }
        let call = PendingCall {
            call_id,
            name,
            input,
            digest,
            seam,
            deadline_ms: self.deadline_ms,
            armed_at_ms: None,
        };
        self.write_decision(&call, decision, by, None)
    }

    fn frame_verdict(&mut self, tool: &str) -> (Decision, DecidedBy, Seam) {
        let Some(frame) = self.frame.clone() else {
            if !self.warned_no_frame {
                self.warned_no_frame = true;
                self.warn(
                    warning::NO_FRAME_IN_FORCE,
                    "this run is in frame mode and no frame is in force, so nothing narrowed \
                     this call; the record says so rather than claiming a frame decided it"
                        .to_string(),
                );
            }
            // Abstain, not allow. `allow` grants — it bypasses the rest of the vendor's
            // permission pipeline and overrides a stricter rule in the vendor's own settings
            // (§ 6) — so a run that had nothing to narrow with and answered `allow` would be a
            // run that switched the vendor's permission system off by accident. Abstaining
            // claims what is true: metaharness adjudicated nothing here (amendment a3).
            return (Decision::Abstain, DecidedBy::Adapter, Seam::None);
        };
        let Some(operation_name) = self.operation_of_tool.get(tool).cloned() else {
            self.warn(
                warning::UNCOVERED_TOOL,
                format!(
                    "no operation in the v0.1 vocabulary renders to {tool}, so the frame has no \
                     way to admit it"
                ),
            );
            return (
                Decision::Deny {
                    reason: format!(
                        "{tool} is outside the operation vocabulary this step was framed with, \
                         so this step does not admit it"
                    ),
                },
                DecidedBy::Frame,
                self.seam,
            );
        };
        let admitted = frame
            .operations
            .iter()
            .any(|operation| operation.name() == operation_name);
        if admitted {
            (Decision::Allow, DecidedBy::Frame, self.seam)
        } else {
            let legal: Vec<&str> = frame
                .operations
                .iter()
                .map(metaharness_protocol::Operation::name)
                .collect();
            (
                Decision::Deny {
                    reason: format!(
                        "this step admits {legal:?} and {tool} is {operation_name}, which it \
                         does not; a refusal is the answer, not an obstacle"
                    ),
                },
                DecidedBy::Frame,
                self.seam,
            )
        }
    }

    /// Rule 1: the decision reaches the child **before** any control is applied.
    fn write_decision(
        &mut self,
        call: &PendingCall,
        decision: Decision,
        by: DecidedBy,
        latency_ms: Option<u64>,
    ) -> std::io::Result<()> {
        let line = self.bridge.decision_line(&call.call_id, &decision);
        self.process.write_line(&line)?;
        self.count(&decision, call.seam, by);
        self.outbox.push_back(Emission::untimed(Event::ToolDecided {
            call_id: call.call_id.clone(),
            decision,
            decided_by: by,
            seam: call.seam,
            latency_ms,
        }));
        Ok(())
    }

    fn count(&mut self, decision: &Decision, seam: Seam, by: DecidedBy) {
        match decision {
            Decision::Allow => self.census.allowed += 1,
            Decision::Deny { .. } => self.census.denied += 1,
            Decision::Replace { .. } => self.census.replaced += 1,
            Decision::Abstain => self.census.abstained += 1,
        }
        *self
            .census
            .by_seam
            .entry(seam_name(seam).to_string())
            .or_default() += 1;
        *self
            .census
            .by_decider
            .entry(decider_name(by).to_string())
            .or_default() += 1;
    }

    fn warn(&mut self, code: &str, message: String) {
        self.outbox.push_back(Emission::untimed(Event::Warning {
            code: code.to_string(),
            message,
        }));
    }

    fn expire_deadlines(&mut self) -> std::io::Result<bool> {
        if self.pending.is_empty() {
            return Ok(false);
        }
        let now = self.clock.now_ms();
        let mut expired = Vec::new();
        self.pending.retain(|call| {
            let over = call
                .armed_at_ms
                .is_some_and(|armed| now.saturating_sub(armed) >= call.deadline_ms);
            if over {
                expired.push(call.clone());
            }
            !over
        });
        if expired.is_empty() {
            return Ok(false);
        }
        for call in expired {
            let latency = call.armed_at_ms.map(|armed| now.saturating_sub(armed));
            let reason = deadline_reason(call.deadline_ms, self.vendor_timeout_ms);
            self.write_decision(
                &call,
                Decision::Deny { reason },
                DecidedBy::Deadline,
                latency,
            )?;
        }
        Ok(true)
    }

    fn wait_out_earliest_deadline(&mut self) -> std::io::Result<()> {
        let Some(target) = self
            .pending
            .iter()
            .filter_map(|call| call.armed_at_ms.map(|armed| armed + call.deadline_ms))
            .min()
        else {
            return Err(std::io::Error::other(
                "the child is waiting on a decision and metaharness holds none pending; neither \
                 side can leave this state, so it is reported rather than spun on",
            ));
        };
        self.clock.sleep_until_ms(target);
        Ok(())
    }

    fn wind_up(&mut self) {
        self.abandon_pending("the stream ended", warning::PENDING_CALL_ABANDONED);
        self.bridge.set_census(self.census.clone());
        let emissions = self.bridge.finish();
        self.outbox.extend(emissions);
        self.finished = true;
    }

    /// Rule 4: `interrupt` is a legal answer to a pending decision — and rule 1 says the
    /// decision still goes first, because cancelling first clears the active call and leaves the
    /// child waiting on a correlation that no longer exists.
    fn abandon_pending(&mut self, why: &str, code: &str) {
        let abandoned = std::mem::take(&mut self.pending);
        for call in abandoned {
            let latency = call
                .armed_at_ms
                .map(|armed| self.clock.now_ms().saturating_sub(armed));
            let reason = format!("{why} before this call was decided, so nothing ran");
            // A write failure here cannot be reported through a `command.result` that is already
            // being built, and losing the run over a child that has already gone is worse than
            // recording the decision, so the event is emitted either way.
            let line = self.bridge.decision_line(
                &call.call_id,
                &Decision::Deny {
                    reason: reason.clone(),
                },
            );
            let _ = self.process.write_line(&line);
            self.count(
                &Decision::Deny {
                    reason: String::new(),
                },
                call.seam,
                DecidedBy::Adapter,
            );
            self.outbox.push_back(Emission::untimed(Event::ToolDecided {
                call_id: call.call_id.clone(),
                decision: Decision::Deny { reason },
                decided_by: DecidedBy::Adapter,
                seam: call.seam,
                latency_ms: latency,
            }));
            self.outbox.push_back(Emission::untimed(Event::Warning {
                code: code.to_string(),
                message: format!("{} was never decided by the embedder", call.call_id),
            }));
        }
    }

    fn apply(&mut self, command: Command) -> std::io::Result<CommandOutcome> {
        // Well-formedness first, and the adapter second: a command that did not parse is
        // malformed on every adapter, and telling a caller "unsupported" about an input they got
        // wrong sends them to the wrong fix. The adapter-level refusal for a command this run's
        // configuration will need is raised at run start regardless (design § 6.1).
        if let Some(refusal) = malformed(&command) {
            return Ok(refusal);
        }
        if let CommandSupport::Refused(code) = self.capabilities.support(command.name()) {
            return Ok(refused(
                code,
                format!(
                    "the {} adapter cannot honour {}",
                    self.capabilities.adapter.id,
                    command.name()
                ),
            ));
        }
        match command {
            Command::ToolDecide { call_id, decision } => self.decide(&call_id, decision),
            Command::FrameSet { frame } => {
                self.frame = Some(*frame);
                Ok(CommandOutcome::Ok {
                    applies_at: Some("the next turn or step boundary".to_string()),
                })
            }
            Command::Interrupt { .. } => {
                self.abandon_pending("the run was interrupted", warning::PENDING_CALL_ABANDONED);
                self.write_control(&Command::Interrupt {
                    reason: String::new(),
                })?;
                Ok(CommandOutcome::Ok { applies_at: None })
            }
            Command::Halt { .. } => {
                self.abandon_pending("the run was halted", warning::PENDING_CALL_ABANDONED);
                self.write_control(&Command::Halt {
                    reason: String::new(),
                })?;
                self.process.kill()?;
                self.wind_up();
                Ok(CommandOutcome::Ok { applies_at: None })
            }
            other => {
                self.write_control(&other)?;
                Ok(CommandOutcome::Ok { applies_at: None })
            }
        }
    }

    fn write_control(&mut self, command: &Command) -> std::io::Result<()> {
        if let Some(line) = self.bridge.control_line(command) {
            self.process.write_line(&line)?;
        }
        Ok(())
    }

    /// Rule 3: a decision correlates to one request and cannot be replayed.
    fn decide(&mut self, call_id: &str, decision: Decision) -> std::io::Result<CommandOutcome> {
        let Some(index) = self.pending.iter().position(|call| call.call_id == call_id) else {
            return Ok(if self.presented.contains_key(call_id) {
                refused(
                    RefusalCode::TooLate,
                    format!(
                        "{call_id} was decided or its window closed; a decision cannot be replayed"
                    ),
                )
            } else {
                refused(
                    RefusalCode::UnknownCall,
                    format!("{call_id} does not correlate to an open request"),
                )
            });
        };
        let call = self.pending.remove(index);
        let now = self.clock.now_ms();
        let latency = call.armed_at_ms.map(|armed| now.saturating_sub(armed));
        self.write_decision(&call, decision, DecidedBy::Embedder, latency)?;
        Ok(CommandOutcome::Ok { applies_at: None })
    }
}

/// What metaharness says when it denies on its own deadline.
///
/// One function so the vendor's timeout and metaharness's own budget always appear together: a
/// reader of the transcript can check rule 2 held without leaving the line (design § 7.7 rule 2).
#[must_use]
pub fn deadline_reason(budget_ms: u64, vendor_timeout_ms: u64) -> String {
    format!(
        "metaharness's own {budget_ms}ms decision deadline expired before an answer arrived; it \
         is strictly less than the vendor's {vendor_timeout_ms}ms timeout so the refusal is \
         metaharness's and not an ambiguity"
    )
}

/// Whether this command is ill-formed on its own terms, whatever the adapter is.
///
/// Two rules, and each one is a failure that would otherwise reach the model or the vendor:
/// an empty deny reason is a wall where the wire requires an instruction, and a frame whose
/// digest no longer describes it was mutated after it was sealed — which is exactly what the
/// digest exists to catch (design § 5.1, § 6).
fn malformed(command: &Command) -> Option<CommandOutcome> {
    match command {
        Command::ToolDecide { decision, .. } if !decision.is_well_formed() => Some(refused(
            RefusalCode::Malformed,
            "a deny must carry a non-empty reason: the reason is the only part the model can act \
             on, and it is the difference between a wall and an instruction",
        )),
        Command::FrameSet { frame } if !frame.digest_intact() => Some(refused(
            RefusalCode::Malformed,
            "the frame's digest does not describe its contents, so it was mutated after it was \
             sealed",
        )),
        _ => None,
    }
}

fn refused(code: RefusalCode, reason: impl Into<String>) -> CommandOutcome {
    CommandOutcome::Refused {
        refused: Refused::new(code, reason),
    }
}

/// The seam's name, as the census keys it.
#[must_use]
pub fn seam_name(seam: Seam) -> &'static str {
    match seam {
        Seam::Registration => "registration",
        Seam::Hook => "hook",
        Seam::ControlRequest => "control_request",
        Seam::OwnedTool => "owned_tool",
        Seam::None => "none",
    }
}

/// Who decided, as the census keys it.
#[must_use]
pub fn decider_name(by: DecidedBy) -> &'static str {
    match by {
        DecidedBy::Embedder => "embedder",
        DecidedBy::Frame => "frame",
        DecidedBy::Deadline => "deadline",
        DecidedBy::Adapter => "adapter",
    }
}
