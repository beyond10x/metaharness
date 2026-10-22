//! Adversary cases for wave 8 unit 4 (amendment a48): the three verdict lines an audit prints.
//!
//! No process is started and no model is called. Every report is built by hand or driven through
//! the scripted runner.

use metaharness::protocol::{
    CloseReason, DecisionCensus, DecisionMode, Event, HermeticRow, Kind, ProcessTermination,
    RowVerdict, Severity, StreamCompleteness, Verdict,
};
use metaharness::{
    AuditReport, FakeAuditor, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog,
    ScriptedRunner, ScriptedSeams,
};

const INIT: &str = r#"{"emit":"session.started","harness_version":"2.1.240","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"operator-login"}"#;
const END: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

const GATING: [HermeticRow; 3] = [HermeticRow::H1a, HermeticRow::H4, HermeticRow::H9];
const ADVISORY: [HermeticRow; 2] = [HermeticRow::H2, HermeticRow::H6];
const VERDICTS: [Verdict; 3] = [Verdict::Ok, Verdict::Unk, Verdict::Gap];

/// Every assignment of the three verdicts to `n` rows, `n` from zero.
fn assignments(n: usize) -> Vec<Vec<Verdict>> {
    let mut all = vec![Vec::new()];
    for _ in 0..n {
        all = all
            .into_iter()
            .flat_map(|prefix| {
                VERDICTS.iter().map(move |verdict| {
                    let mut next = prefix.clone();
                    next.push(*verdict);
                    next
                })
            })
            .collect();
    }
    all
}

fn row(row: HermeticRow, verdict: Verdict, severity: Severity) -> RowVerdict {
    RowVerdict {
        row,
        verdict,
        severity,
        detail: "adversary".to_owned(),
    }
}

fn last_three(report: &AuditReport) -> Vec<String> {
    let rendered = report.render();
    let lines: Vec<&str> = rendered.lines().collect();
    lines[lines.len().saturating_sub(3)..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect()
}

/// Item 1 of the brief, exhaustively: zero to three gating rows in every combination of `ok`,
/// `unk` and `gap`, beside zero to two advisory rows in every combination, with and without a
/// terminal record, under four child exits. The expected lines are literals from a48's table, not
/// values read back from `control()` or `exit()`.
#[test]
fn adversary_w8u4_every_gating_combination_folds_gap_over_unk_over_ok_and_advisory_rows_never_move_it()
 {
    let children = [
        (ProcessTermination::Exited { code: 0 }, "child exit: 0"),
        (ProcessTermination::Exited { code: 255 }, "child exit: 255"),
        (
            ProcessTermination::Signaled { signal: 15 },
            "child exit: signal 15",
        ),
        (ProcessTermination::Unknown, "child exit: unknown"),
    ];
    let mut checked = 0usize;
    for gating_count in 0..=GATING.len() {
        for gating in assignments(gating_count) {
            for advisory in assignments(ADVISORY.len()) {
                for terminal in [true, false] {
                    for (child, child_line) in children {
                        let mut rows: Vec<RowVerdict> = gating
                            .iter()
                            .zip(GATING)
                            .map(|(verdict, id)| row(id, *verdict, Severity::Gating))
                            .collect();
                        rows.extend(
                            advisory
                                .iter()
                                .zip(ADVISORY)
                                .map(|(verdict, id)| row(id, *verdict, Severity::Advisory)),
                        );
                        let report = AuditReport {
                            rows,
                            census: DecisionCensus::default(),
                            withheld: None,
                            installed_plugins: Vec::new(),
                            auditor: None,
                            saw_terminal_record: terminal,
                            stream: StreamCompleteness::Complete {
                                events: 1,
                                reason: CloseReason::Completed,
                            },
                            child_exit: child,
                        };
                        let control = if gating.contains(&Verdict::Gap) {
                            "control verdict: gap"
                        } else if gating.contains(&Verdict::Unk) {
                            "control verdict: unk"
                        } else {
                            "control verdict: ok"
                        };
                        let (audit, code) = if gating.contains(&Verdict::Gap) {
                            ("audit verdict: gap (exit 1)", 1)
                        } else if gating.contains(&Verdict::Unk) || !terminal {
                            ("audit verdict: no verdict (exit 3)", 3)
                        } else {
                            ("audit verdict: ok (exit 0)", 0)
                        };
                        assert_eq!(
                            last_three(&report),
                            [child_line, control, audit],
                            "gating {gating:?}, advisory {advisory:?}, terminal {terminal}, \
                             child {child:?}"
                        );
                        assert_eq!(report.exit().code(), code);
                        checked += 1;
                    }
                }
            }
        }
    }
    // (1 + 3 + 9 + 27) gating × 9 advisory × 2 terminal × 4 children.
    assert_eq!(checked, 40 * 9 * 2 * 4);
}

/// a48 § 2: "a stream with no `stream.closed` [prints] `unknown`". No test reaches the
/// no-marker arm of `child_exit` through `Run::audit`: every existing case either drains the run
/// (so a marker exists) or hand-builds `child_exit`. An embedder may audit a run that has not
/// closed yet; replacing `unwrap_or_default()` with `Exited { code: 0 }` there would print
/// `child exit: 0` for a child nothing measured, and only this case would see it.
#[test]
fn adversary_w8u4_a_run_audited_before_its_marker_prints_child_exit_unknown() {
    let mut runner = ScriptedRunner::new(
        vec![ScriptStep::line(INIT), ScriptStep::line(END)],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Frame)
        .start_with_clock(
            Input::Prompt("audited mid-stream".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .expect("the run starts");
    let first = run.next_event().expect("one event is read");
    assert!(first.is_some(), "the opening record is delivered");
    assert!(
        !run.events()
            .iter()
            .any(|event| matches!(event, Event::StreamClosed { .. })),
        "precondition: the marker is not out yet: {:?}",
        run.events()
    );
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    assert_eq!(report.child_exit, ProcessTermination::Unknown);
    let rendered = report.render();
    assert!(
        rendered.lines().any(|line| line == "child exit: unknown"),
        "{rendered}"
    );
    assert!(
        !rendered.lines().any(|line| line == "child exit: 0"),
        "{rendered}"
    );
}
