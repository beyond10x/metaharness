//! `--audit`: the built-in floor.
//!
//! **The floor is not delegable.** The twelve rows of § 8.1 are claims about metaharness's own
//! imposition and they must fail even where no auditor is installed — a hermeticity that only
//! holds when somebody remembered to pass a spec file is not a promise (design D12).
//!
//! Two honesties are load-bearing here and both cost a line each time:
//!
//! * **A missing field is `unk`, never a zero and never a pass.** A bound that read a missing
//!   MCP list as an empty one would report its blindest case as its best one (design § 2.1).
//! * **The attestation is metaharness's own claim about its own actions and it is not
//!   independent evidence** (design § 8.3). The launch-asserted rows below are read out of it,
//!   so they say what metaharness *did*, not what the harness then did. The independent evidence
//!   is the vendor's opening record, and the two sit side by side so a reader can notice when
//!   they disagree.
//!
//! H2 and H6 are **advisory**: evaluated, reported, printed, and they do not move the exit code.
//! Their unobservability is a property of the mechanism rather than of the run, and if any `unk`
//! failed a strict run then every strict run would have failed forever (finding F3).

use std::fmt::Write as _;

use metaharness_protocol::{
    Assertion, CredentialSource, DecidedBy, Decision, DecisionCensus, Event, HermeticAttestation,
    HermeticRow, RowVerdict, RunSpec, Severity, Verdict,
};

use crate::auditor::AuditorVerdict;
use crate::run::{decider_name, seam_name};

/// What the floor needs that no event carries.
#[derive(Debug, Clone, Copy)]
pub struct FloorInputs<'a> {
    /// The run's own spec, which says what was asked for.
    pub spec: &'a RunSpec,
    /// The versions the adapter was written against (design § 8.4 O1).
    pub pinned_versions: &'a [String],
    /// The directory metaharness planned to run in, for H7's comparison.
    pub planned_cwd: Option<&'a str>,
    /// The plugin directories the run declared, for H1a's comparison.
    pub declared_plugins: &'a [String],
}

/// How the exit code came out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunExit {
    /// The session ran and every gating verdict is `ok`.
    Ok,
    /// A gating verdict is `gap`, or the auditor exited `1`.
    Gap,
    /// metaharness itself could not do its job. Never a verdict about the run.
    Broken,
    /// Nobody found out. Not a softer [`RunExit::Gap`]: a crashed suite is not a failing suite,
    /// and submitting a failing verdict for something that never ran fabricates an observation.
    NoVerdict,
}

impl RunExit {
    /// The process exit code.
    #[must_use]
    pub fn code(self) -> i32 {
        match self {
            RunExit::Ok => 0,
            RunExit::Gap => 1,
            RunExit::Broken => 2,
            RunExit::NoVerdict => 3,
        }
    }
}

/// What `metaharness run` exits with when no audit was asked for.
///
/// **Never `1`.** Without an audit there is no verdict to contradict, and two exit-code tables
/// for one verb is how a caller comes to treat `0` as "it was fine" (design § 9.4).
#[must_use]
pub fn exit_without_audit(saw_terminal_record: bool) -> RunExit {
    if saw_terminal_record {
        RunExit::Ok
    } else {
        RunExit::NoVerdict
    }
}

/// The floor's verdict, plus whatever the external auditor said.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditReport {
    /// The twelve rows, in the design's order.
    pub rows: Vec<RowVerdict>,
    /// What metaharness's own seam did. **Always printed**: a report that hides "0 denials"
    /// reads as clean when it may mean nothing was ever attempted (design § 9.4).
    pub census: DecisionCensus,
    /// The external auditor's verdict, when one ran.
    pub auditor: Option<AuditorVerdict>,
    /// Whether the harness produced a terminal record at all.
    pub saw_terminal_record: bool,
}

impl AuditReport {
    /// How many gating rows are `gap`.
    #[must_use]
    pub fn gating_gaps(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.severity == Severity::Gating && row.verdict == Verdict::Gap)
            .count()
    }

    /// How many gating rows are `unk`.
    #[must_use]
    pub fn gating_unknowns(&self) -> usize {
        self.rows
            .iter()
            .filter(|row| row.severity == Severity::Gating && row.verdict == Verdict::Unk)
            .count()
    }

    /// The exit code, exactly § 9.4.
    ///
    /// A `gap` outranks an `unk` where both are present: a row that definitely failed is a fact,
    /// and reporting "nobody found out" over it would hide the one thing that was found out.
    #[must_use]
    pub fn exit(&self) -> RunExit {
        let auditor_code = self.auditor.as_ref().and_then(|verdict| verdict.exit_code);
        if self.gating_gaps() > 0 || auditor_code == Some(1) {
            return RunExit::Gap;
        }
        if !self.saw_terminal_record || self.gating_unknowns() > 0 || auditor_code == Some(3) {
            return RunExit::NoVerdict;
        }
        RunExit::Ok
    }

    /// The report a person reads.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::new();
        out.push_str("hermetic verdict (the attestation half is metaharness's own claim, not independent evidence)\n");
        for row in &self.rows {
            let gates = match row.severity {
                Severity::Gating => "gating",
                Severity::Advisory => "advisory",
            };
            let assertion = match row.row.assertion() {
                Assertion::Record => "record",
                Assertion::Launch => "launch",
                Assertion::Effect => "effect",
            };
            let _ = writeln!(
                out,
                "  {:<4} {:<3} {:<8} {:<6} {} — {}",
                row.row.id(),
                verdict_word(row.verdict),
                gates,
                assertion,
                row.row.control(),
                row.detail
            );
        }
        let _ = writeln!(
            out,
            "decision census: allowed={} denied={} replaced={}",
            self.census.allowed, self.census.denied, self.census.replaced
        );
        for (seam, count) in &self.census.by_seam {
            let _ = writeln!(out, "  by seam    {seam}: {count}");
        }
        for (decider, count) in &self.census.by_decider {
            let _ = writeln!(out, "  by decider {decider}: {count}");
        }
        if self.census.allowed + self.census.denied + self.census.replaced == 0 {
            out.push_str(
                "  nothing was adjudicated: a census of zero cannot distinguish enforcement \
                 holding from nothing being attempted\n",
            );
        }
        if let Some(auditor) = &self.auditor {
            let _ = writeln!(
                out,
                "auditor {:?}: exit {:?}, {} verdict rows",
                auditor.argv, auditor.exit_code, auditor.verdict_rows
            );
        }
        out
    }
}

fn verdict_word(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Ok => "ok",
        Verdict::Gap => "gap",
        Verdict::Unk => "unk",
    }
}

fn row(row: HermeticRow, verdict: Verdict, detail: impl Into<String>) -> RowVerdict {
    RowVerdict {
        row,
        verdict,
        severity: row.severity(),
        detail: detail.into(),
    }
}

/// The decision census for these events.
///
/// Read from the terminal record when there is one, because that is the number the run itself
/// published; derived from the `tool.decided` events otherwise, so a run that died before its
/// terminal record still reports what it decided.
#[must_use]
pub fn decision_census(events: &[Event]) -> DecisionCensus {
    for event in events {
        if let Event::SessionEnded { census, .. } = event {
            return census.clone();
        }
    }
    let mut census = DecisionCensus::default();
    for event in events {
        let Event::ToolDecided {
            decision,
            decided_by,
            seam,
            ..
        } = event
        else {
            continue;
        };
        match decision {
            Decision::Allow => census.allowed += 1,
            Decision::Deny { .. } => census.denied += 1,
            Decision::Replace { .. } => census.replaced += 1,
            Decision::Abstain => census.abstained += 1,
        }
        *census
            .by_seam
            .entry(seam_name(*seam).to_string())
            .or_default() += 1;
        *census
            .by_decider
            .entry(decider_name(*decided_by).to_string())
            .or_default() += 1;
    }
    census
}

/// The twelve rows of § 8.1, evaluated from the emitted events.
///
/// The record-asserted rows read `session.started`'s own fields; the launch-asserted ones read
/// the attestation block the adapter put in that same event. No row is filled from
/// configuration: absence of evidence is not hermeticity.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn hermetic_floor(events: &[Event], inputs: &FloorInputs<'_>) -> Vec<RowVerdict> {
    let started = events
        .iter()
        .find(|event| matches!(event, Event::SessionStarted { .. }));

    let Some(Event::SessionStarted {
        harness_version,
        credential_source,
        output_style,
        cwd,
        plugins,
        mcp_servers,
        inputs_digest,
        hermetic,
        ..
    }) = started
    else {
        return HermeticRow::ALL
            .into_iter()
            .map(|which| {
                row(
                    which,
                    Verdict::Unk,
                    "there is no opening record, so nothing about this run was observed",
                )
            })
            .collect();
    };

    let mut rows = Vec::with_capacity(HermeticRow::ALL.len());

    // H1a — plugins are exactly the declared set.
    rows.push(match plugins {
        None => row(
            HermeticRow::H1a,
            Verdict::Unk,
            "the opening record carries no plugin list, and a missing list is not an empty one",
        ),
        Some(loaded) => {
            let mut names: Vec<String> = loaded
                .iter()
                .filter_map(|plugin| plugin.name.clone())
                .collect();
            names.sort();
            let mut declared = inputs.declared_plugins.to_vec();
            declared.sort();
            if names == declared {
                row(
                    HermeticRow::H1a,
                    Verdict::Ok,
                    format!("loaded exactly the declared set: {names:?}"),
                )
            } else {
                row(
                    HermeticRow::H1a,
                    Verdict::Gap,
                    format!("declared {declared:?} and the record says {names:?}"),
                )
            }
        }
    });

    // H1b — the output style is the default.
    rows.push(match output_style.as_deref() {
        None => row(
            HermeticRow::H1b,
            Verdict::Unk,
            "the opening record carries no output style",
        ),
        Some("default" | "null" | "") => row(
            HermeticRow::H1b,
            Verdict::Ok,
            "the output style is the default",
        ),
        Some(other) => row(
            HermeticRow::H1b,
            Verdict::Gap,
            format!("the output style is {other:?}, which is the operator's and not the default"),
        ),
    });

    // H2 — settings sources excluded. Launch, and advisory.
    rows.push(attested(HermeticRow::H2, hermetic));

    // H3 — the environment is constructed, not inherited. Launch.
    rows.push(attested(HermeticRow::H3, hermetic));

    // H4 — no API key unless the run declared one.
    rows.push(match credential_source.as_deref() {
        None => row(
            HermeticRow::H4,
            Verdict::Unk,
            "the opening record does not say where the credential came from",
        ),
        Some(observed) => {
            let normalized = observed.to_ascii_lowercase().replace(['_', ' '], "-");
            let expected = match inputs.spec.credentials {
                CredentialSource::OperatorLogin => "login",
                CredentialSource::ApiKey => "api",
                CredentialSource::None => "none",
            };
            if normalized.contains(expected) {
                row(
                    HermeticRow::H4,
                    Verdict::Ok,
                    format!("the record says {observed:?} and the run declared {expected:?}"),
                )
            } else {
                row(
                    HermeticRow::H4,
                    Verdict::Gap,
                    format!(
                        "the run declared {expected:?} and the record says {observed:?}; the \
                         vendor's own word for a source varies, so this row matches the family \
                         and prints what it saw"
                    ),
                )
            }
        }
    });

    // H5 — the MCP surface is exactly what the launch gave. A list, never a count.
    rows.push(match mcp_servers {
        None => row(
            HermeticRow::H5,
            Verdict::Unk,
            "the opening record carries no MCP server list, which is unk and never zero: a \
             server the session cannot authenticate to still exists and is still named",
        ),
        Some(servers) if servers.is_empty() => row(
            HermeticRow::H5,
            Verdict::Ok,
            "the record lists no MCP server, and the launch configured none",
        ),
        Some(servers) => {
            let names: Vec<&str> = servers
                .iter()
                .map(|server| server.name.as_deref().unwrap_or("<unnamed>"))
                .collect();
            row(
                HermeticRow::H5,
                Verdict::Gap,
                format!("the launch configured no MCP server and the record lists {names:?}"),
            )
        }
    });

    // H6 — credentials are one file, copied. Effect, and advisory.
    rows.push(attested(HermeticRow::H6, hermetic));

    // H7 — the working directory is ours.
    rows.push(match (cwd.as_deref(), inputs.planned_cwd) {
        (None, _) => row(
            HermeticRow::H7,
            Verdict::Unk,
            "the opening record carries no working directory",
        ),
        (Some(_), None) => row(
            HermeticRow::H7,
            Verdict::Unk,
            "metaharness did not record the directory it planned, so the record has nothing to \
             be compared against",
        ),
        (Some(observed), Some(planned)) if observed == planned => row(
            HermeticRow::H7,
            Verdict::Ok,
            format!("the session ran in {observed}, which metaharness created"),
        ),
        (Some(observed), Some(planned)) => row(
            HermeticRow::H7,
            Verdict::Gap,
            format!("metaharness planned {planned} and the session ran in {observed}"),
        ),
    });

    // H8 — hooks and customizations are not skipped. Launch.
    rows.push(attested(HermeticRow::H8, hermetic));

    // H9 — the vendor version is the pinned one.
    rows.push(match harness_version.as_deref() {
        None => row(
            HermeticRow::H9,
            Verdict::Unk,
            "the opening record carries no harness version",
        ),
        Some(observed) if inputs.pinned_versions.iter().any(|pin| pin == observed) => row(
            HermeticRow::H9,
            Verdict::Ok,
            format!("the vendor is {observed}, which the adapter is pinned to"),
        ),
        Some(observed) => row(
            HermeticRow::H9,
            Verdict::Gap,
            format!(
                "the vendor is {observed} and the adapter is pinned to {:?}; a verdict that \
                 changed because the reader changed must be visible as such",
                inputs.pinned_versions
            ),
        ),
    });

    // H10 — governing documents cannot move under the run.
    rows.push(match inputs_digest {
        None => row(
            HermeticRow::H10,
            Verdict::Unk,
            "the opening record carries no digest of the copied input tree",
        ),
        Some(digest) => row(
            HermeticRow::H10,
            Verdict::Ok,
            format!("the copied tree is {digest}"),
        ),
    });

    // H11 — no memory file outside the copied tree is discoverable. Launch.
    rows.push(attested(HermeticRow::H11, hermetic));

    rows.sort_by_key(|verdict| verdict.row);
    rows
}

/// A launch-asserted row, read out of metaharness's own attestation.
///
/// The attestation is a claim and not evidence (§ 8.3): it says what metaharness imposed before
/// spawning, which is strong about the argv and the child environment and silent about what the
/// harness then did with them.
fn attested(which: HermeticRow, attestation: &HermeticAttestation) -> RowVerdict {
    if let Some(imposed) = attestation
        .imposed
        .iter()
        .find(|control| control.row == which)
    {
        return row(
            which,
            Verdict::Ok,
            format!("metaharness imposed it: {}", imposed.how),
        );
    }
    if let Some(unavailable) = attestation
        .unavailable
        .iter()
        .find(|control| control.row == which)
    {
        return row(
            which,
            Verdict::Gap,
            format!("metaharness could not impose it: {}", unavailable.why),
        );
    }
    row(
        which,
        Verdict::Unk,
        "metaharness's attestation says nothing about this row, so nobody found out",
    )
}

/// Whether these events carry a decision the census would count.
///
/// Offered because "did anything get adjudicated at all" is the question a zero census cannot
/// answer on its own.
#[must_use]
pub fn anything_was_adjudicated(events: &[Event]) -> bool {
    events.iter().any(|event| {
        matches!(
            event,
            Event::ToolDecided {
                decided_by: DecidedBy::Embedder | DecidedBy::Frame | DecidedBy::Deadline,
                ..
            }
        )
    })
}

impl crate::run::Run {
    /// Whether this run owes a verdict.
    ///
    /// `--audit` asks for one. **`--hermetic strict` also asks for one**, because the whole
    /// point of `strict` is that a gating row which is not `ok` fails the run, and a strict run
    /// with no floor evaluated would be a mode with no teeth. The design's exit table says a run
    /// **without** `--audit` never exits `1`; that sentence and `strict`'s own definition cannot
    /// both be literal, and this build resolves it in favour of `strict` meaning something. It
    /// is recorded for the design register rather than decided here twice.
    #[must_use]
    pub fn wants_audit(&self) -> bool {
        self.spec().audit || self.spec().hermetic == metaharness_protocol::HermeticMode::Strict
    }

    /// The twelve rows, evaluated from this run's own events.
    #[must_use]
    pub fn hermetic_floor(&self) -> Vec<RowVerdict> {
        let launch = self.launch_facts();
        hermetic_floor(
            self.events(),
            &FloorInputs {
                spec: self.spec(),
                pinned_versions: &launch.pinned_versions,
                planned_cwd: launch.planned_cwd.as_deref(),
                declared_plugins: &launch.declared_plugins,
            },
        )
    }

    /// The floor, the census and — when the run named one — the external auditor's verdict.
    ///
    /// # Errors
    ///
    /// Every auditor refusal in [`crate::Refusal`]: a spec with nobody to check it, an auditor
    /// with nothing to check, an unreadable spec, an auditor that will not run, and an audit
    /// that produced no verdict rows. All of them are exit `2`.
    pub fn audit(
        &self,
        invoker: &mut dyn crate::auditor::AuditorInvoker,
    ) -> Result<AuditReport, crate::Refusal> {
        let transcript = std::path::PathBuf::from(
            self.transcript()
                .path
                .clone()
                .unwrap_or_else(|| "<no retained transcript>".to_string()),
        );
        let auditor = crate::auditor::run_auditor(
            self.spec().spec.as_deref(),
            self.spec().auditor.as_deref(),
            &self.spec().auditor_args,
            &transcript,
            invoker,
        )?;
        Ok(AuditReport {
            rows: self.hermetic_floor(),
            census: decision_census(self.events()),
            auditor,
            saw_terminal_record: self.saw_terminal_record(),
        })
    }

    /// The exit code for this run, with or without an audit.
    #[must_use]
    pub fn exit(&self, report: Option<&AuditReport>) -> RunExit {
        match report {
            Some(report) => report.exit(),
            None => exit_without_audit(self.saw_terminal_record()),
        }
    }
}
