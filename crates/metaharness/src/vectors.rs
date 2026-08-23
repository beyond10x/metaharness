//! C3 — the control vectors, model-free.
//!
//! **Each vector is one stimulus and its complete observable expectation, including the typed
//! refusal** (design § 8.5). "Complete" is the word that does the work: a vector that asserted
//! only the happy event would pass while the decision never reached the child, so every vector
//! below compares the whole ordered event trace **and** the whole ordered list of lines written
//! to the child.
//!
//! They are driven through [`crate::ScriptedProcess`] and [`crate::ScriptedSeam`] rather than
//! through one adapter's transcript format, because what they assert is metaharness's own
//! control machinery — § 7.7's five ordering rules and § 6.1's refusal codes. A vector that went
//! red because a vendor record changed shape would be reporting a C2 defect under a C3 name; the
//! vendor-wire half is the adapter's own C1 and C2, and `metaharness conformance` runs all three.
//!
//! This tier is free. That is the whole argument for it: it makes the seam's promises a tested
//! claim rather than a paragraph in a document.

use metaharness_protocol::{
    Command, CommandOutcome, ConformanceTier, ContractObligations, Decision, DecisionMode, Event,
    Kind, RefusalCode, VectorOutcome,
};

use crate::builder::{Input, Metaharness};
use crate::clock::ManualClock;
use crate::refusal::Refusal;
use crate::run::{Run, deadline_reason, decider_name};
use crate::scripted::{ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.240","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login"}"#;
const CALL: &str =
    r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#;
const RESULT: &str = r#"{"emit":"tool.result","call_id":"t1","is_error":false}"#;
const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// The seven control vectors of § 8.5's C3 tier.
///
/// Returned as data rather than asserted here, so the binary can print them and a test can
/// require every one of them to pass — the same values, read twice.
#[must_use]
pub fn control_vectors() -> Vec<VectorOutcome> {
    vec![
        vector_allow(),
        vector_deny(),
        vector_replace(),
        vector_deadline_expiry(),
        vector_cancel_instead_of_decide(),
        vector_unknown_call(),
        vector_too_late(),
        vector_observe(),
    ]
}

/// One vector's whole observation: the ordered event trace, and the ordered lines written out.
struct Observed {
    trace: Vec<String>,
    written: Vec<String>,
}

impl Observed {
    fn differs_from(&self, trace: &[&str], written: &[&str]) -> Option<String> {
        let seen_trace: Vec<&str> = self.trace.iter().map(String::as_str).collect();
        if seen_trace != trace {
            return Some(format!("trace was {seen_trace:?}, expected {trace:?}"));
        }
        let seen_written: Vec<&str> = self.written.iter().map(String::as_str).collect();
        if seen_written != written {
            return Some(format!(
                "written to the child was {seen_written:?}, expected {written:?}"
            ));
        }
        None
    }
}

fn started(
    script: Vec<ScriptStep>,
    decisions: DecisionMode,
) -> Result<(Run, ScriptedLog), Refusal> {
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(script, log.clone());
    let mut seams = ScriptedSeams;
    let run = Metaharness::new(Kind::Claude)
        .with_decisions(decisions)
        .start_with_clock(
            Input::Prompt("vector".to_string()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )?;
    Ok((run, log))
}

fn note(event: &Event) -> String {
    match event {
        Event::ToolRequested {
            decision_required, ..
        } => format!("tool.requested decision_required={decision_required}"),
        Event::ToolDecided {
            decision,
            decided_by,
            ..
        } => format!(
            "tool.decided {} by={}",
            match decision {
                Decision::Allow => "allow",
                Decision::Deny { .. } => "deny",
                Decision::Replace { .. } => "replace",
                Decision::Abstain => "abstain",
            },
            decider_name(*decided_by)
        ),
        Event::CommandResult { outcome, .. } => match outcome {
            CommandOutcome::Ok { .. } => "command.result ok".to_string(),
            CommandOutcome::Refused { refused } => {
                format!("command.result refused {}", refused.code.as_str())
            }
        },
        Event::SessionEnded { census, .. } => format!(
            "session.ended allowed={} denied={} replaced={} abstained={}",
            census.allowed, census.denied, census.replaced, census.abstained
        ),
        Event::Warning { code, .. } => format!("warning {code}"),
        other => other.name().to_string(),
    }
}

/// Drive one run to the end, letting `policy` answer the first pending call.
fn observe(
    mut run: Run,
    log: &ScriptedLog,
    mut policy: impl FnMut(&mut Run, &str) -> std::io::Result<()>,
) -> std::io::Result<Observed> {
    let mut trace = Vec::new();
    while let Some(line) = run.next_event()? {
        trace.push(note(&line.event));
        if let Event::ToolRequested {
            call_id,
            decision_required: true,
            ..
        } = &line.event
        {
            let call_id = call_id.clone();
            policy(&mut run, &call_id)?;
        }
    }
    Ok(Observed {
        trace,
        written: log.written(),
    })
}

fn outcome(
    id: &str,
    observed: std::io::Result<Observed>,
    trace: &[&str],
    written: &[&str],
) -> VectorOutcome {
    match observed {
        Err(error) => VectorOutcome::failed(id, ConformanceTier::C3, error.to_string()),
        Ok(observed) => match observed.differs_from(trace, written) {
            None => VectorOutcome::passed(id, ConformanceTier::C3),
            Some(detail) => VectorOutcome::failed(id, ConformanceTier::C3, detail),
        },
    }
}

fn ask_script() -> Vec<ScriptStep> {
    vec![
        ScriptStep::line(INIT),
        ScriptStep::line(CALL),
        ScriptStep::awaiting("t1"),
        ScriptStep::line(RESULT),
        ScriptStep::line(END),
    ]
}

fn vector_allow() -> VectorOutcome {
    let id = "c3/allow-lets-the-call-run";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    let observed = observe(run, &log, |run, call_id| {
        run.send(Command::ToolDecide {
            call_id: call_id.to_string(),
            decision: Decision::Allow,
        })
        .map(|_| ())
    });
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided allow by=embedder",
            "command.result ok",
            "tool.result",
            "session.ended allowed=1 denied=0 replaced=0 abstained=0",
        ],
        &[r#"{"call_id":"t1","decision":{"decision":"allow"}}"#],
    )
}

fn vector_deny() -> VectorOutcome {
    let id = "c3/deny-carries-a-reason-the-model-is-told";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    let observed = observe(run, &log, |run, call_id| {
        run.send(Command::ToolDecide {
            call_id: call_id.to_string(),
            decision: Decision::Deny {
                reason: "this step admits no shell".to_string(),
            },
        })
        .map(|_| ())
    });
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided deny by=embedder",
            "command.result ok",
            "tool.result",
            "session.ended allowed=0 denied=1 replaced=0 abstained=0",
        ],
        &[
            r#"{"call_id":"t1","decision":{"decision":"deny","reason":"this step admits no shell"}}"#,
        ],
    )
}

fn vector_replace() -> VectorOutcome {
    let id = "c3/replace-runs-the-call-with-a-different-input";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    let observed = observe(run, &log, |run, call_id| {
        run.send(Command::ToolDecide {
            call_id: call_id.to_string(),
            decision: Decision::Replace {
                input: serde_json::json!({ "command": "ls -1" }),
            },
        })
        .map(|_| ())
    });
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided replace by=embedder",
            "command.result ok",
            "tool.result",
            "session.ended allowed=0 denied=0 replaced=1 abstained=0",
        ],
        &[r#"{"call_id":"t1","decision":{"decision":"replace","input":{"command":"ls -1"}}}"#],
    )
}

fn vector_deadline_expiry() -> VectorOutcome {
    let id = "c3/an-unanswered-call-is-denied-by-metaharnesss-own-deadline";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    // The two numbers are read off the run rather than written down, because the vendor's hook
    // timeout is the adapter's to declare and this vector asserts the *relationship* between
    // them — § 7.7 rule 2 — not a particular pair.
    let (budget, vendor) = (run.deadline_ms(), run.vendor_timeout_ms());
    if budget >= vendor {
        return VectorOutcome::failed(
            id,
            ConformanceTier::C3,
            format!(
                "metaharness's deadline is {budget}ms and the vendor's timeout is {vendor}ms;                  rule 2 requires it to be strictly less"
            ),
        );
    }
    // No policy: the embedder never answers, so the child stays blocked and metaharness's own
    // deadline is what unblocks it.
    let observed = observe(run, &log, |_, _| Ok(()));
    let expected_line = serde_json::json!({
        "call_id": "t1",
        "decision": {
            "decision": "deny",
            "reason": deadline_reason(budget, vendor),
        },
    })
    .to_string();
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided deny by=deadline",
            "tool.result",
            "session.ended allowed=0 denied=1 replaced=0 abstained=0",
        ],
        &[expected_line.as_str()],
    )
}

fn vector_cancel_instead_of_decide() -> VectorOutcome {
    let id = "c3/interrupt-is-a-legal-answer-and-the-deny-is-written-first";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    let observed = observe(run, &log, |run, _| {
        run.send(Command::Interrupt {
            reason: "the embedder does not want to decide".to_string(),
        })
        .map(|_| ())
    });
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided deny by=adapter",
            "warning PENDING_CALL_ABANDONED",
            "command.result ok",
            "tool.result",
            "session.ended allowed=0 denied=1 replaced=0 abstained=0",
        ],
        // Rule 1: the decision reaches the child **before** the interrupt. Cancelling first
        // would clear the active call and leave the child waiting on a correlation that no
        // longer exists.
        &[
            r#"{"call_id":"t1","decision":{"decision":"deny","reason":"the run was interrupted before this call was decided, so nothing ran"}}"#,
            r#"{"control":"interrupt"}"#,
        ],
    )
}

fn vector_unknown_call() -> VectorOutcome {
    let id = "c3/a-decision-for-an-unopened-call-is-unknown-call";
    let Ok((run, log)) = started(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        DecisionMode::Ask,
    ) else {
        return refused(id);
    };
    let mut run = run;
    let sent = run.send(Command::ToolDecide {
        call_id: "never-presented".to_string(),
        decision: Decision::Allow,
    });
    let observed = match sent {
        Err(error) => Err(error),
        Ok(CommandOutcome::Refused { refused }) if refused.code == RefusalCode::UnknownCall => {
            observe(run, &log, |_, _| Ok(()))
        }
        Ok(other) => {
            return VectorOutcome::failed(
                id,
                ConformanceTier::C3,
                format!("the command outcome was {other:?}, expected UNKNOWN_CALL"),
            );
        }
    };
    outcome(
        id,
        observed,
        &[
            "command.result refused UNKNOWN_CALL",
            "session.started",
            "session.ended allowed=0 denied=0 replaced=0 abstained=0",
        ],
        &[],
    )
}

fn vector_too_late() -> VectorOutcome {
    let id = "c3/a-second-decision-for-the-same-call-is-too-late";
    let Ok((run, log)) = started(ask_script(), DecisionMode::Ask) else {
        return refused(id);
    };
    let mut second = None;
    let observed = observe(run, &log, |run, call_id| {
        run.send(Command::ToolDecide {
            call_id: call_id.to_string(),
            decision: Decision::Allow,
        })?;
        second = Some(run.send(Command::ToolDecide {
            call_id: call_id.to_string(),
            decision: Decision::Deny {
                reason: "changed my mind".to_string(),
            },
        })?);
        Ok(())
    });
    match &second {
        Some(CommandOutcome::Refused { refused }) if refused.code == RefusalCode::TooLate => {}
        other => {
            return VectorOutcome::failed(
                id,
                ConformanceTier::C3,
                format!("the second decision produced {other:?}, expected TOO_LATE"),
            );
        }
    }
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=true",
            "tool.decided allow by=embedder",
            "command.result ok",
            "command.result refused TOO_LATE",
            "tool.result",
            "session.ended allowed=1 denied=0 replaced=0 abstained=0",
        ],
        // Exactly one line reached the child. A replayed decision that wrote a second one would
        // be the failure rule 3 exists for.
        &[r#"{"call_id":"t1","decision":{"decision":"allow"}}"#],
    )
}

/// The capture mode, end to end on a scripted stream (R2.5, amendment a10).
///
/// **Nothing is bypassed and nothing is silent.** The hook fires for the call exactly as it does
/// in every other mode, an `allow` is written down the same channel a `deny` would be, and the
/// call leaves a `tool.decided` naming the mode that made it — so an unsteered arm's transcript
/// has the same shape as a steered arm's and the two can be compared expectation by expectation.
///
/// The three things this asserts together, because any one of them alone would pass while the
/// mode was broken: the decision **reached the child** (the written line), the record **names the
/// mode** (`by=observe`, not `by=adapter` or `by=frame`), and the census says **nothing was
/// denied** — a capture run that quietly denied a call would be a treatment, not a measurement.
fn vector_observe() -> VectorOutcome {
    let id = "c3/observe-allows-every-call-and-names-the-mode-that-did";
    let script = vec![
        ScriptStep::line(INIT),
        ScriptStep::line(CALL),
        ScriptStep::line(RESULT),
        ScriptStep::line(END),
    ];
    let Ok((run, log)) = started(script, DecisionMode::Observe) else {
        return refused(id);
    };
    // No policy: the embedder is never asked, which is the point. `decision_required=false` on
    // the request is how the stream says so.
    let observed = observe(run, &log, |_, _| Ok(()));
    outcome(
        id,
        observed,
        &[
            "session.started",
            "tool.requested decision_required=false",
            "tool.decided allow by=observe",
            "tool.result",
            "session.ended allowed=1 denied=0 replaced=0 abstained=0",
        ],
        &[r#"{"call_id":"t1","decision":{"decision":"allow"}}"#],
    )
}

fn refused(id: &str) -> VectorOutcome {
    VectorOutcome::failed(
        id,
        ConformanceTier::C3,
        "the run could not be started, so the vector observed nothing".to_string(),
    )
}

/// Every free vector for this kind: the adapter's C1 and C2, and this crate's C3.
///
/// # Errors
///
/// [`Refusal::NoAdapter`] for a kind this build has no adapter for. A conformance run that
/// silently reported zero vectors would read exactly like one that passed.
pub fn conformance_vectors(kind: Kind) -> Result<Vec<VectorOutcome>, Refusal> {
    match kind {
        Kind::Claude => {
            let mut vectors = metaharness_claude::conformance_vectors();
            vectors.extend(control_vectors());
            // The spawn half of C3: a real process and the real hook program, still with no
            // model, no network and no credential.
            vectors.extend(crate::spawn_vectors::spawn_vectors());
            Ok(vectors)
        }
        Kind::Codex => {
            let mut vectors = metaharness_codex::conformance_vectors();
            // The control tier is metaharness's own machinery and is adapter-neutral, so it is
            // **not** repeated here: the C3 vectors in `control_vectors` drive a scripted process
            // and assert § 7.7's ordering rules once, for every adapter. What is codex-specific is
            // the spawn half — a real process, a real hook program, a real session file — and that
            // is what this adds.
            vectors.extend(crate::spawn_codex_vectors::spawn_vectors());
            Ok(vectors)
        }
    }
}

/// What the adapter for this kind says it can do.
///
/// # Errors
///
/// None today — both kinds have adapters — and the `Result` stays so a future kind without one
/// refuses instead of panicking.
pub fn capabilities(kind: Kind) -> Result<metaharness_protocol::Capabilities, Refusal> {
    match kind {
        Kind::Claude => Ok(metaharness_claude::capabilities()),
        Kind::Codex => Ok(metaharness_codex::capabilities()),
    }
}

/// What the adapter for this kind declares its contract owes (CT-4).
///
/// The dispatch is the enforcement: a new [`Kind`] does not compile until it names a declaration,
/// so "fill the checklist" is the first thing a third adapter is told by the compiler rather than
/// the last thing a reviewer remembers to ask for.
///
/// # Errors
///
/// None today — both kinds have adapters — and the `Result` stays for the same reason
/// [`capabilities`]'s does.
pub fn contract_obligations(kind: Kind) -> Result<ContractObligations, Refusal> {
    match kind {
        Kind::Claude => Ok(metaharness_claude::CONTRACT_OBLIGATIONS),
        Kind::Codex => Ok(metaharness_codex::CONTRACT_OBLIGATIONS),
    }
}

/// Whether every vector in this list passed.
#[must_use]
pub fn all_passed(vectors: &[VectorOutcome]) -> bool {
    !vectors.is_empty() && vectors.iter().all(|vector| vector.passed)
}

/// An adapter's conformance run, as a `contract_result` record.
///
/// The reuse `engineering-protocols`' `contract-testing` principle asks for, made concrete
/// without a dependency crossing the boundary: metaharness emits the *shape* EP defines —
/// `{checked, failed, breaking_changes, provider, consumer}` — so a consumer reads an adapter's
/// conformance as a contract between the vendor and the protocol wire (design
/// `adapter-contract-v0.1.md`, CT-1).
///
/// - `provider` is the vendor and its pin (`codex 0.145.0`) — the side that can move under us;
/// - `consumer` is [`metaharness_protocol::EVENT_FORMAT`], the wire the adapter maps onto;
/// - `checked` is the vector count (the principle's `checked > 0`: a run that checked nothing
///   asserts nothing);
/// - `failed` is any red vector, either face;
/// - `breaking_changes` is the subset in the **vendor-facing tiers (C1, C2)** — a failure there
///   means the *vendor* moved, which is what breaks a consumer. A C3 failure is metaharness's own
///   control machinery regressing: counted in `failed`, never here.
///
/// # Errors
///
/// [`Refusal::NoAdapter`] for a kind with no adapter, surfaced by [`capabilities`].
pub fn contract_result(
    kind: Kind,
    vectors: &[VectorOutcome],
) -> Result<serde_json::Value, Refusal> {
    let pin = capabilities(kind)?.versions_pinned.join(", ");
    let failed = vectors.iter().filter(|vector| !vector.passed).count();
    let breaking_changes = vectors
        .iter()
        .filter(|vector| {
            !vector.passed && matches!(vector.tier, ConformanceTier::C1 | ConformanceTier::C2)
        })
        .count();
    Ok(serde_json::json!({
        "kind": "contract_result",
        "provider": format!("{} {pin}", kind.as_str()),
        "consumer": metaharness_protocol::EVENT_FORMAT,
        "checked": vectors.len(),
        "failed": failed,
        "breaking_changes": breaking_changes,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// CT-1: an adapter's conformance run is a `contract_result` — the shape a consumer reads,
    /// with the pin in the provider and the vendor-tier failures called out as breaking.
    #[test]
    fn conformance_emits_a_contract_result_with_the_pin_in_the_provider() {
        for (kind, expected_provider_prefix) in [(Kind::Claude, "claude "), (Kind::Codex, "codex ")]
        {
            let vectors = conformance_vectors(kind).expect("the adapter exists");
            let record = contract_result(kind, &vectors).expect("a record");
            assert_eq!(record["kind"], "contract_result");
            assert_eq!(record["consumer"], metaharness_protocol::EVENT_FORMAT);
            assert_eq!(record["checked"].as_u64(), Some(vectors.len() as u64));
            // A green run: no failures, so nothing breaking, and checked > 0 (the principle's
            // rule that a run which checked nothing asserts nothing).
            assert_eq!(record["failed"].as_u64(), Some(0));
            assert_eq!(record["breaking_changes"].as_u64(), Some(0));
            assert!(record["checked"].as_u64().unwrap() > 0);
            let provider = record["provider"].as_str().expect("a provider string");
            assert!(
                provider.starts_with(expected_provider_prefix),
                "provider {provider:?} names the vendor"
            );
            // The pin travels in the provider — the fact CX-M2's Q18 is about, made checkable.
            assert!(
                provider.trim_end().len() > expected_provider_prefix.len(),
                "provider {provider:?} carries a version after the vendor"
            );
        }
    }

    /// `breaking_changes` counts only the vendor-facing tiers: a C3 failure is metaharness's own
    /// control machinery regressing, red in `failed` but not a vendor break.
    #[test]
    fn a_c3_failure_is_failed_but_not_breaking() {
        let vectors = vec![
            VectorOutcome::failed("x/c3", ConformanceTier::C3, "internal"),
            VectorOutcome::failed("x/c2", ConformanceTier::C2, "vendor moved"),
        ];
        let record = contract_result(Kind::Claude, &vectors).expect("a record");
        assert_eq!(record["failed"].as_u64(), Some(2));
        assert_eq!(record["breaking_changes"].as_u64(), Some(1));
    }

    /// C3 is the tier that carries the safety argument, and it is free — so every vector in it
    /// runs in the default gate and every one of them must pass.
    #[test]
    fn every_control_vector_passes() {
        let vectors = control_vectors();
        let failures: Vec<(&str, &str)> = vectors
            .iter()
            .filter(|vector| !vector.passed)
            .map(|vector| (vector.id.as_str(), vector.detail.as_str()))
            .collect();
        assert!(failures.is_empty(), "{failures:#?}");
        assert_eq!(vectors.len(), 8);
    }

    /// **A run that did not ask for observe mode never gets it**, asserted at the seam rather than
    /// at the launch: drive the *same* script under every mode and check that `by=observe` appears
    /// in exactly one of them. The launch-side polarity is the claude adapter's own C1 vector;
    /// this is the half that would catch a run loop that reached the observe branch by accident.
    #[test]
    fn only_a_run_that_asked_for_observe_mode_is_decided_by_it() {
        for mode in DecisionMode::ALL {
            let script = vec![
                ScriptStep::line(INIT),
                ScriptStep::line(CALL),
                ScriptStep::awaiting("t1"),
                ScriptStep::line(RESULT),
                ScriptStep::line(END),
            ];
            let (run, log) = started(script, mode).expect("the run starts");
            let observed = observe(run, &log, |run, call_id| {
                run.send(Command::ToolDecide {
                    call_id: call_id.to_string(),
                    decision: Decision::Allow,
                })
                .map(|_| ())
            })
            .expect("the run drains");
            let by_observe = observed
                .trace
                .iter()
                .any(|note| note.contains("by=observe"));
            assert_eq!(
                by_observe,
                mode == DecisionMode::Observe,
                "--decisions {} produced {:?}",
                mode.as_str(),
                observed.trace
            );
        }
    }

    /// The C3 tier must not quietly become a list of C1 rows: every vector this crate publishes
    /// is a control vector.
    #[test]
    fn every_control_vector_is_declared_c3() {
        assert!(
            control_vectors()
                .iter()
                .all(|vector| vector.tier == ConformanceTier::C3)
        );
    }
}
