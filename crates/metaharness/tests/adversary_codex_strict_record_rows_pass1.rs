//! Adversary pass 1 on `story:codex-strict-record-rows` (amendment a34).
//!
//! The codex seam now holds a hook's `tool.requested` while the rollout reader holds the opening
//! record, and its `finish` releases what is still held. The seam's own unit tests show the held
//! call is *emitted*. These cases drive the same seam through the real run loop and ask what the
//! loop does with it: is the call decided (or abandoned, with its warning), the way the loop decides
//! every hook call that reaches it through `push_line`?
//!
//! Synthetic and free: a scripted child, no process, no credential, no model.

use metaharness::protocol::{Command, CredentialSource, DecisionMode, Event, Kind, Seam};
use metaharness::{
    EventPoll, HarnessProcess, Input, LaunchPlanView, ManualClock, Metaharness, ProcessPoll,
    ProcessRunner, Run, ScriptedLog, ScriptedRunner, warning,
};

const META: &str = r#"{"timestamp":"2026-10-21T00:00:00.000Z","type":"session_meta","payload":{"id":"s-1","session_id":"s-1","cli_version":"0.153.4","cwd":"/w"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-10-21T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const HOOK_INPUT: &str = r#"{"hook_event_name":"PreToolUse","tool_name":"Bash","tool_use_id":"call-7","tool_input":{"command":"ls"}}"#;

fn hook_line() -> String {
    metaharness_codex::hook_request_line("k7", HOOK_INPUT)
}

fn events_of(run: &Run) -> Vec<Event> {
    run.events().to_vec()
}

fn hook_call_presented(events: &[Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            Event::ToolRequested { call_id, seam: Seam::Hook, .. } if call_id == "call-7"
        )
    })
}

fn decided(events: &[Event]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, Event::ToolDecided { call_id, .. } if call_id == "call-7"))
}

fn names(events: &[Event]) -> Vec<&'static str> {
    events.iter().map(Event::name).collect()
}

/// A child that writes `lines`, then either sits idle (blocked on nothing the loop can see) or
/// reports that a hook is waiting on a decision (`WouldBlock`), the way `CodexProcess::poll_line`
/// does after `BLOCK_GRACE_POLLS` idle polls with an unanswered hook request.
struct Child {
    lines: Vec<String>,
    then_would_block: bool,
    killed: bool,
}

impl HarnessProcess for Child {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        match self.poll_line()? {
            ProcessPoll::Line(line) => Ok(Some(line)),
            ProcessPoll::Ended => Ok(None),
            ProcessPoll::Idle => Err(std::io::Error::new(std::io::ErrorKind::WouldBlock, "idle")),
        }
    }

    fn poll_line(&mut self) -> std::io::Result<ProcessPoll> {
        if self.killed {
            return Ok(ProcessPoll::Ended);
        }
        if !self.lines.is_empty() {
            return Ok(ProcessPoll::Line(self.lines.remove(0)));
        }
        if self.then_would_block {
            return Err(std::io::Error::new(
                std::io::ErrorKind::WouldBlock,
                "a PreToolUse hook is waiting on a decision metaharness has not written",
            ));
        }
        Ok(ProcessPoll::Idle)
    }

    fn write_line(&mut self, _line: &str) -> std::io::Result<()> {
        Ok(())
    }

    fn kill(&mut self) -> std::io::Result<()> {
        self.killed = true;
        Ok(())
    }

    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        Ok(None)
    }
}

struct Runner {
    lines: Vec<String>,
    then_would_block: bool,
}

impl ProcessRunner for Runner {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(&mut self, _plan: &LaunchPlanView<'_>) -> std::io::Result<Box<dyn HarnessProcess>> {
        Ok(Box::new(Child {
            lines: std::mem::take(&mut self.lines),
            then_would_block: self.then_would_block,
            killed: false,
        }))
    }
}

fn start(runner: &mut dyn ProcessRunner, decisions: DecisionMode) -> Run {
    Metaharness::new(Kind::Codex)
        .with_decisions(decisions)
        .with_credentials(CredentialSource::None)
        .start_with_clock(
            Input::Prompt("adversary".to_owned()),
            runner,
            &mut metaharness::CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("a synthetic codex run starts")
}

/// A hook call that reached the seam while the opening record was held, and a stream that ended
/// before any record released it. The seam's `finish` delivers the call (bridge unit test
/// `a_held_call_is_delivered_when_the_stream_ends_before_the_opening_record_completes`), but
/// `Run::wind_up` puts `finish`'s emissions straight into the outbox, after `abandon_pending` and
/// without `admit`: under `--decisions observe` the call is presented and never decided. At the
/// base commit the same lines are admitted on arrival and decided by `observe`.
#[test]
fn a_hook_call_released_by_finish_is_decided_like_any_other() {
    let mut runner = ScriptedRunner::of_lines(
        [META.to_owned(), TASK_STARTED.to_owned(), hook_line()],
        ScriptedLog::new(),
    );
    let mut run = start(&mut runner, DecisionMode::Observe);
    run.drain().expect("the synthetic run drains");
    let events = events_of(&run);
    assert!(hook_call_presented(&events), "{:?}", names(&events));
    assert!(
        decided(&events),
        "call-7 was presented on the hook seam and never decided: {:?}",
        names(&events)
    );
}

/// The same held call, ended by the embedder's `halt` instead of the stream: `halt` abandons every
/// pending call with a `tool.decided` deny and a `PENDING_CALL_ABANDONED` warning, but the held
/// call is not pending — `finish` releases it after the abandonment, undecided and unwarned. At the
/// base commit the call is pending by then and is abandoned with both.
#[test]
fn a_halt_while_a_hook_call_is_held_abandons_it_like_any_other() {
    let mut runner = Runner {
        lines: vec![META.to_owned(), TASK_STARTED.to_owned(), hook_line()],
        then_would_block: false,
    };
    let mut run = start(&mut runner, DecisionMode::Ask);
    for _ in 0..10 {
        match run.poll_event().expect("polling the synthetic run") {
            EventPoll::Ended => panic!("the run ended before the halt"),
            EventPoll::Event(_) | EventPoll::Idle | EventPoll::Progress => {}
            EventPoll::DecisionPending => break,
        }
    }
    run.send(Command::Halt {
        reason: "adversary".to_owned(),
    })
    .expect("halt is honoured");
    run.drain().expect("the halted run drains");
    let events = events_of(&run);
    assert!(hook_call_presented(&events), "{:?}", names(&events));
    assert!(
        decided(&events),
        "call-7 was presented on the hook seam and the halt did not abandon it: {:?}",
        names(&events)
    );
    assert!(
        events.iter().any(|event| matches!(
            event,
            Event::Warning { code, message } if code == warning::PENDING_CALL_ABANDONED && message.contains("call-7")
        )),
        "no PENDING_CALL_ABANDONED for call-7: {:?}",
        names(&events)
    );
}

/// The brief's question — a hook that needs its decision before the opening record is released:
/// deadlock or timeout? Neither: the codex child reports `WouldBlock` once a hook has waited
/// `BLOCK_GRACE_POLLS` idle polls (`spawn_codex.rs`), and the run loop, holding no pending call
/// because the seam is holding it, fails the run with "metaharness holds none pending". At the base
/// commit the call is presented under `--decisions ask` and the loop waits out its deadline.
#[test]
fn a_hook_waiting_behind_the_held_opening_record_does_not_fail_the_run() {
    let mut runner = Runner {
        lines: vec![META.to_owned(), TASK_STARTED.to_owned(), hook_line()],
        then_would_block: true,
    };
    let mut run = start(&mut runner, DecisionMode::Ask);
    let mut presented = false;
    for _ in 0..20 {
        match run.poll_event() {
            Ok(EventPoll::Event(line)) => {
                if matches!(&line.event, Event::ToolRequested { call_id, .. } if call_id == "call-7")
                {
                    presented = true;
                }
            }
            Ok(EventPoll::DecisionPending) => break,
            Ok(EventPoll::Ended) => panic!("the run ended while a hook was waiting"),
            Ok(EventPoll::Idle | EventPoll::Progress) => {}
            Err(error) => panic!(
                "the run failed while a hook waited behind the held opening record \
                 (presented to the embedder: {presented}): {error}"
            ),
        }
    }
    assert!(presented, "call-7 was never presented to the embedder");
}
