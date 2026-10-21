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
use std::path::{Component, Path, PathBuf};

use metaharness_protocol::{
    Assertion, CloseReason, CredentialSource, DecidedBy, Decision, DecisionCensus, Event,
    HermeticAttestation, HermeticRow, InstalledPlugin, PluginRef, RowVerdict, RunSpec, Severity,
    StreamCompleteness, Verdict, WithheldTool, stream_completeness,
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
    /// The directory metaharness planned to run in, for H7's comparison, which resolves it on the
    /// filesystem when the floor is evaluated ([`hermetic_floor`]).
    pub planned_cwd: Option<&'a str>,
    /// The plugin directories the run declared, for H1a's comparison.
    pub declared_plugins: &'a [String],
    /// The plugins compiled into each pinned vendor binary, each with the version it belongs to and
    /// the one source its opening record gives it, for H1a's comparison (amendment a33). Empty for
    /// a kind that declares none.
    pub builtin_plugins: &'a [BuiltinPlugin],
    /// The exact H4 `how` this kind's launch attests when the child's API-key variable holds the
    /// launch's own placeholder for a declared endpoint, for H4's comparison (amendment a33).
    /// `None` for a kind whose launch never does that.
    pub endpoint_key_posture: Option<&'a str>,
    /// The MCP servers this launch configured, for H5's comparison. Empty on every surface but
    /// `owned`, where it is metaharness's own tool server.
    pub planned_mcp_servers: &'a [String],
    /// The output style this kind's launch wrote, for H1b's comparison (amendment a34). `Some`
    /// where the launch writes one — the Codex launch writes `personality` — and the record must
    /// then name exactly it; `None` where it writes none, and the record must name the default.
    pub planned_output_style: Option<&'a str>,
    /// Why this kind's opening record carries no field for a record-asserted row, as the adapter
    /// states it for its pinned binary (amendment a34). Appended to that row's `unk` detail so the
    /// row says why it is unknown; it never decides a verdict. Empty for a kind that states none.
    pub record_silences: &'a [(HermeticRow, &'a str)],
    /// The memory canary recorded for each pinned version, for H11 (amendment a36). `Some` for a
    /// kind whose H11 rests on a launch control the binary has to honour: an imposed H11 then
    /// reads `ok` only on a record version whose canary kept every shape out. `None` for a kind
    /// whose H11 is not held to a canary; the attestation alone decides it.
    pub memory_canaries: Option<&'a [MemoryCanary]>,
}

/// One pinned version's memory canary, as the adapter records it (amendment a36): how many
/// memory files were planted in and above the cwd, how many the binary sent to a request with
/// the project and local sources on (the control), and how many reached a request through the
/// launch's own argv and environment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemoryCanary {
    /// The pinned version whose binary the canary ran against.
    pub version: String,
    /// Memory files planted, every shape in the cwd and in its parent.
    pub planted: usize,
    /// How many the same binary sent with the sources on. Fewer than `planted` means the probe
    /// did not show the binary reads every shape, so it shows nothing about the ones it missed.
    pub control_sent: usize,
    /// How many reached a request through the launch, over every credential surface probed.
    pub reached_through_launch: usize,
    /// What the canary and the walk do not cover, as the adapter states it; an `ok` H11 ends with
    /// it, so the row never reads as wider than the measurement (a36 § 6). Empty for none.
    pub not_covered: String,
}

/// One plugin compiled into a pinned vendor binary, as the adapter says that binary's opening
/// record spells it (amendment a33).
///
/// **H1a matches all three fields.** The version is the binary the list was read from, so a record
/// on any other version has no list; the name is what the record calls the plugin; the source is
/// the adapter's statement of where a compiled-in one comes from. A record plugin that carries the
/// name with any other source — or with none — is not this built-in: an ambient plugin can be called
/// anything, so a name alone would let one pass as a plugin nobody installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuiltinPlugin {
    /// The pinned version whose binary compiles this plugin in.
    pub version: String,
    /// The plugin's name, as the vendor's record reports it.
    pub name: String,
    /// The exact source the vendor's record reports for it.
    pub source: String,
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
pub fn exit_without_audit(completed_successfully: bool) -> RunExit {
    if completed_successfully {
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
    /// What the run declared and this machine would not admit, as the opening record stated it
    /// (amendment a12). **Always printed**, for the same reason the census is: a reader who sees
    /// no denial cannot otherwise tell *nothing was refused* from *the tool was never there*.
    ///
    /// [`None`] is *the harness did not say* — including a stream with no opening record at all —
    /// and never *nothing was withheld*. `Some(vec![])` is the harness stating that the run got
    /// everything it asked for (invariant 3, amendment a4's rule).
    pub withheld: Option<Vec<WithheldTool>>,
    /// Every plugin metaharness says it installed, from the attestation in the opening record.
    ///
    /// **Always printed, and `plugins: none` when it is empty** (amendment a16). A report that
    /// printed nothing for a plugin-less run would make *this run installed nothing* and *this
    /// build does not report installations* the same bytes, which is the reading § 8.1 refuses
    /// everywhere else.
    ///
    /// It is metaharness's own claim and **not** evidence: whether the harness then loaded any of
    /// them is H1a's row, read out of the vendor's own plugin list.
    pub installed_plugins: Vec<InstalledPlugin>,
    /// The external auditor's verdict, when one ran.
    pub auditor: Option<AuditorVerdict>,
    /// Whether the harness produced a terminal record at all.
    pub saw_terminal_record: bool,
    /// Whether the stream closed itself, and what it said when it did (amendment a17).
    ///
    /// **Always printed, and a stream with no `stream.closed` marker is named `TRUNCATED`.** A
    /// report that said nothing here would leave *this run did X zero times* and *this file stopped
    /// before X could happen* the same bytes, which is the reading invariant 3 refuses everywhere
    /// else — and is why eight `aep observe trace check` reports ended `undecided` on 2026-09-03.
    ///
    /// **It does not gate**, and the reason is what a failure here would mean: this marker is
    /// metaharness's own, written by the loop that produced these events, so its absence in a live
    /// run is a defect in this build rather than a verdict about the run. The reader who needs it
    /// is the offline one, reading a file this process did not produce.
    pub stream: StreamCompleteness,
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
        if !self.saw_terminal_record
            || !matches!(
                self.stream,
                StreamCompleteness::Complete {
                    reason: CloseReason::Completed,
                    ..
                }
            )
            || self.gating_unknowns() > 0
            || auditor_code == Some(3)
        {
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
        // `abstained` is printed beside the other three and not folded into them. It was
        // absent from this line until an owned-surface run reported `allowed=0 denied=0
        // replaced=0` beside three calls that plainly happened — the number the reader needed to
        // see was the one the line did not carry.
        let _ = writeln!(
            out,
            "decision census: allowed={} denied={} replaced={} abstained={}",
            self.census.allowed, self.census.denied, self.census.replaced, self.census.abstained
        );
        for (seam, count) in &self.census.by_seam {
            let _ = writeln!(out, "  by seam    {seam}: {count}");
        }
        for (decider, count) in &self.census.by_decider {
            let _ = writeln!(out, "  by decider {decider}: {count}");
        }
        // Always a line, on the same rule as the census and the plugin list: a report that printed
        // nothing here would make *nothing happened* and *the file stopped* the same bytes.
        let _ = writeln!(out, "{}", self.stream.render());
        if self.census.allowed + self.census.denied + self.census.replaced == 0 {
            if self.census.abstained == 0 {
                out.push_str(
                    "  nothing was adjudicated: a census of zero cannot distinguish enforcement \
                     holding from nothing being attempted\n",
                );
            } else {
                // The two zeroes mean different things and the reader has to be told which one
                // this is. Under `--tool-surface owned` the toolset **is** the policy: metaharness
                // runs the tool, so there is nothing per call to allow or deny, and a run that
                // reported "nothing was adjudicated" would read as enforcement that never fired.
                let _ = writeln!(
                    out,
                    "  {} call(s) were adjudicated by nobody, and that is the configuration \
                     rather than a silence: what the run may do is the published tool list, in \
                     `session.started.offered_tools`",
                    self.census.abstained
                );
            }
        }
        // Always a line, and `none` rather than silence when there is nothing to list: a report
        // that printed nothing here would make "this run installed nothing" and "this build does
        // not report installations" the same bytes (amendment a16).
        if self.installed_plugins.is_empty() {
            out.push_str("plugins: none\n");
        } else {
            for plugin in &self.installed_plugins {
                let _ = writeln!(
                    out,
                    "plugins: {} from {} at {} (sha256:{}) — {}",
                    plugin.name,
                    plugin.source,
                    plugin.installed_at,
                    plugin.digest,
                    plugin.loaded_by
                );
            }
        }
        // Beside the census, because the census cannot answer this: a tool the machine would
        // not admit was never put in front of the model, so no call was ever refused and every
        // denial count stays zero. It is absent from `offered_tools` and `available_operations`
        // in exactly the way a tool nobody wanted is, and the difference between *nothing was
        // refused* and *the tool was never there* is the whole of amendment a12.
        match &self.withheld {
            Some(withheld) if !withheld.is_empty() => {
                let named: Vec<String> = withheld
                    .iter()
                    .map(|entry| format!("{} ({})", entry.tool, entry.reason))
                    .collect();
                let _ = writeln!(out, "withheld: {}", named.join("; "));
            }
            // The harness said it withheld nothing, which is a statement and not a silence.
            Some(_) => out.push_str("withheld: none declared\n"),
            // And a silence is printed as one, never as an empty list: reading it as `[]` would
            // assert that this machine admitted everything it was asked for (invariant 3).
            None => out.push_str("withheld: not stated by the harness\n"),
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

/// An `unk` row's detail, with the adapter's statement of why its record is silent on the row
/// appended when it made one (amendment a34). Appended, never substituted: the floor's own
/// sentence says what is missing, and the adapter's says why this kind's record lacks it.
fn silent(base: &str, which: HermeticRow, silences: &[(HermeticRow, &str)]) -> String {
    match silences.iter().find(|(row, _)| *row == which) {
        Some((_, why)) => format!("{base}: {why}"),
        None => base.to_owned(),
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

/// What the opening record said this run asked for and this machine would not admit.
///
/// [`None`] is *the harness did not say* and never *nothing was withheld*: the two are different
/// runs, and an adapter that read silence as `[]` would be asserting that a machine it never
/// asked admitted everything (invariant 3, amendment a4's rule, design § 4.1 a12). A stream with
/// no opening record flattens to the same `None` for the same reason — nobody stated anything.
#[must_use]
pub fn withheld_tools(events: &[Event]) -> Option<Vec<WithheldTool>> {
    events
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted { withheld, .. } => Some(withheld.clone()),
            _ => None,
        })
        .flatten()
}

/// Every plugin metaharness attested installing, from the opening record.
///
/// Empty when the run installed none **and** when there is no opening record at all. The two are
/// distinguishable from the rest of the report — a run with no `session.started` has no rows read
/// from a record either — and folding them here keeps the printed line one sentence rather than
/// three.
#[must_use]
pub fn installed_plugins(events: &[Event]) -> Vec<InstalledPlugin> {
    events
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted { hermetic, .. } => Some(hermetic.installed_plugins.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

/// The twelve rows of § 8.1, evaluated from the emitted events.
///
/// The record-asserted rows read `session.started`'s own fields; the launch-asserted ones read
/// the attestation block the adapter put in that same event. No row is filled from
/// configuration: absence of evidence is not hermeticity.
///
/// **H7 also reads the filesystem, when this is called.** It compares the record's `cwd` with
/// [`FloorInputs::planned_cwd`] as directories, not spellings: two names that differ are one
/// directory only if both still resolve, links followed, to the same path now (amendment a31). So
/// the verdict depends on the moment of the call: evaluated after the directory is gone — a
/// [`crate::Run`] deletes the scratch directory it created when it is dropped, and an embedder may
/// delete its declared copy — a record that names the directory under another spelling than the
/// plan (the canonical one, where the plan passed through a linked `TMPDIR`) reads `gap` where it
/// read `ok` while the directory existed. Byte-equal names are unaffected. [`crate::Run::hermetic_floor`]
/// and `metaharness run` evaluate it while the run still holds its scratch directory.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn hermetic_floor(events: &[Event], inputs: &FloorInputs<'_>) -> Vec<RowVerdict> {
    let started = events
        .iter()
        .find(|event| matches!(event, Event::SessionStarted { .. }));

    let Some(Event::SessionStarted {
        adapter_class,
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
    rows.push(match (adapter_class.as_str(), plugins) {
        ("direct_provider", None) if inputs.declared_plugins.is_empty() => row(
            HermeticRow::H1a,
            Verdict::Ok,
            "the direct-provider loop has no ambient plugin registry and this run declared no plugin directory",
        ),
        (_, None) => row(
            HermeticRow::H1a,
            Verdict::Unk,
            silent(
                "the opening record carries no plugin list, and a missing list is not an empty one",
                HermeticRow::H1a,
                inputs.record_silences,
            ),
        ),
        (_, Some(loaded)) => plugin_row(
            loaded,
            harness_version.as_deref(),
            inputs.declared_plugins,
            inputs.builtin_plugins,
        ),
    });

    // H1b — the output style is the default, or exactly what the launch wrote (amendment a34).
    //
    // Where the launch writes one — Codex's `personality` — the record has to name that value:
    // what the launch wrote is metaharness's own claim, so it decides nothing by itself, and a
    // record that names nothing stays `unk` (invariant 3). Where it writes none, the record has
    // to name the default, as before.
    rows.push(
        match (
            adapter_class.as_str(),
            output_style.as_deref(),
            inputs.planned_output_style,
        ) {
            ("direct_provider", None, _) => row(
                HermeticRow::H1b,
                Verdict::Ok,
                "the direct-provider loop exposes no output-style setting, so there is no operator style to inherit",
            ),
            (_, None, Some(planned)) => row(
                HermeticRow::H1b,
                Verdict::Unk,
                format!(
                    "the opening record carries no output style, and {planned:?}, which the launch \
                     wrote, is not evidence of one"
                ),
            ),
            (_, None, None) => row(
                HermeticRow::H1b,
                Verdict::Unk,
                "the opening record carries no output style",
            ),
            // The record names what the launch wrote, but the launch attested a layer it does not
            // control: such a layer can change what the model is told while the recorded value
            // stays the launch's (a34), so the match shows nothing and the row is `unk`.
            (_, Some(observed), Some(planned)) if observed == planned => match hermetic
                .unavailable
                .iter()
                .find(|control| control.row == HermeticRow::H1b)
            {
                Some(unavailable) => row(
                    HermeticRow::H1b,
                    Verdict::Unk,
                    format!(
                        "the record's output style is {observed:?}, which is what the launch \
                         wrote, and the launch attested H1b unavailable: {}",
                        unavailable.why
                    ),
                ),
                None => row(
                    HermeticRow::H1b,
                    Verdict::Ok,
                    format!(
                        "the record's output style is {observed:?}, which is what the launch wrote"
                    ),
                ),
            },
            (_, Some(observed), Some(planned)) => row(
                HermeticRow::H1b,
                Verdict::Gap,
                format!(
                    "the launch wrote {planned:?} and the record's output style is {observed:?}, \
                     which is not what the launch wrote"
                ),
            ),
            (_, Some("default" | "null" | ""), None) => row(
                HermeticRow::H1b,
                Verdict::Ok,
                "the output style is the default",
            ),
            (_, Some(other), None) => row(
                HermeticRow::H1b,
                Verdict::Gap,
                format!(
                    "the output style is {other:?}, which is the operator's and not the default"
                ),
            ),
        },
    );

    // H2 — settings sources excluded. Launch, and advisory.
    rows.push(attested(HermeticRow::H2, hermetic));

    // H3 — the environment is constructed, not inherited. Launch.
    rows.push(attested(HermeticRow::H3, hermetic));

    // H4 — no API key unless the run declared one.
    rows.push(match credential_source.as_deref() {
        None => row(
            HermeticRow::H4,
            Verdict::Unk,
            silent(
                "the opening record does not say where the credential came from",
                HermeticRow::H4,
                inputs.record_silences,
            ),
        ),
        // H4 asks one question — **was an API key in use, and was one declared?** — and it is
        // deliberately not "does the record name the credential source the spec named".
        //
        // The record's field is `apiKeySource`, and under an operator login 2.1.239 reports
        // `"none"`: there is no API *key*, because the session authenticates from the copied
        // credential file. The first live run failed here, on a run that was hermetic in every
        // way the row exists to check, because the earlier reading looked for the word "login"
        // in a field that never carries it (amendment a4). No free tier could have caught it —
        // there is no real opening record below C4.
        Some(observed) => {
            let normalized = observed.to_ascii_lowercase().replace(['_', ' '], "-");
            // The row turns on whether the record **names an API key**, not on whether it
            // repeats the word the spec used. `apiKeySource` is a field about keys: 2.1.239
            // writes `"none"` under an operator login and names the variable when a key is in
            // use, and neither of those is the string "operator-login".
            let names_a_key = normalized.contains("api") || normalized.contains("key");
            let declared_a_key = inputs.spec.credentials == CredentialSource::ApiKey;
            // The one key that is not the operator's: the launch's own placeholder for a declared
            // endpoint (amendment a33). Recognised from the run's attestation — the exact sentence
            // this kind's launch writes when it put the placeholder there — and never from the
            // variable's name, which an operator's key shares.
            let placeholder = inputs.endpoint_key_posture.filter(|posture| {
                hermetic
                    .imposed
                    .iter()
                    .any(|control| control.row == HermeticRow::H4 && control.how == *posture)
            });
            match (declared_a_key, names_a_key) {
                (false, true) if placeholder.is_some() => row(
                    HermeticRow::H4,
                    Verdict::Ok,
                    format!(
                        "the record says {observed:?}, and the launch attests that variable holds \
                         its own placeholder for the declared endpoint, not an operator key: {}",
                        placeholder.unwrap_or_default()
                    ),
                ),
                (false, true) => row(
                    HermeticRow::H4,
                    Verdict::Gap,
                    format!(
                        "the run declared no API key and the record says {observed:?}; an \
                         exported key takes precedence over the operator login and may point at \
                         an account with no credits{}",
                        // What the launch said it could not impose, when it said so — managed
                        // settings beside an endpoint run (a33) — so the reason is in the row.
                        hermetic
                            .unavailable
                            .iter()
                            .find(|control| control.row == HermeticRow::H4)
                            .map_or_else(String::new, |control| format!(
                                "; the launch attested H4 unavailable: {}",
                                control.why
                            ))
                    ),
                ),
                (true, false) => row(
                    HermeticRow::H4,
                    Verdict::Gap,
                    format!(
                        "the run declared credentials: api-key and the record says {observed:?}, \
                         which names no key at all"
                    ),
                ),
                _ => row(
                    HermeticRow::H4,
                    Verdict::Ok,
                    format!(
                        "the record says {observed:?}, which agrees with the API key this run \
                         declared"
                    ),
                ),
            }
        }
    });

    // H5 — the MCP surface is exactly what the launch gave. A list, never a count.
    //
    // The comparison is against what the launch configured, which is **not** always nothing:
    // `--tool-surface owned` configures metaharness's own tool server, and a row that read the
    // record's list as foreign by definition would report the run's own tools as a breach. What
    // H5 means is unchanged — *the surface is exactly what the launch gave* — and
    // `--strict-mcp-config` is still what makes the operator's own servers absent from it.
    rows.push(match mcp_servers {
        None => row(
            HermeticRow::H5,
            Verdict::Unk,
            silent(
                "the opening record carries no MCP server list, which is unk and never zero: a \
                 server the session cannot authenticate to still exists and is still named",
                HermeticRow::H5,
                inputs.record_silences,
            ),
        ),
        Some(servers) => {
            let mut observed: Vec<&str> = servers
                .iter()
                .map(|server| server.name.as_deref().unwrap_or("<unnamed>"))
                .collect();
            observed.sort_unstable();
            let mut planned: Vec<&str> = inputs
                .planned_mcp_servers
                .iter()
                .map(String::as_str)
                .collect();
            planned.sort_unstable();

            if observed == planned {
                row(
                    HermeticRow::H5,
                    Verdict::Ok,
                    if planned.is_empty() {
                        "the record lists no MCP server, and the launch configured none".to_owned()
                    } else {
                        format!(
                            "the record lists {observed:?}, which is exactly what the launch \
                             configured"
                        )
                    },
                )
            } else {
                // Named both ways round, because the two failures are different problems: a
                // server the launch did not give is a foreign surface, and one it gave that never
                // appeared is a tool surface the model did not actually have.
                let extra: Vec<&&str> = observed
                    .iter()
                    .filter(|name| !planned.contains(name))
                    .collect();
                let missing: Vec<&&str> = planned
                    .iter()
                    .filter(|name| !observed.contains(name))
                    .collect();
                row(
                    HermeticRow::H5,
                    Verdict::Gap,
                    format!(
                        "the launch configured {planned:?} and the record lists {observed:?} \
                         (unplanned: {extra:?}, absent: {missing:?})"
                    ),
                )
            }
        }
    });

    // H6 — credential carriage is exactly declared. Effect, and advisory.
    rows.push(attested(HermeticRow::H6, hermetic));

    // H7 — the working directory is ours.
    rows.push(match (cwd.as_deref(), inputs.planned_cwd) {
        // A direct-provider record with no working directory — the pinned loop writes none under
        // `--substrate <socket>` or for a root whose name is not UTF-8, and a build older than
        // harness 0.15.1 never does — says whose silence it is (a30, kept by a31). Read before
        // the attestation, so neither a declaration nor an a6 `unavailable` decides it.
        (None, _) if adapter_class == "direct_provider" => row(
            HermeticRow::H7,
            Verdict::Unk,
            "the loop did not report its working directory",
        ),
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
        // The record is the evidence and the attestation says what the directory was (a27): a
        // declared copy is `ok` in the embedder's words, an a6 directory metaharness could not
        // impose is a gap in metaharness's own words, and only a directory nobody named is the one
        // metaharness created. Before a27 this arm read no attestation and printed "which
        // metaharness created" over an operator's tree. The two are compared as directories, not
        // as spellings (a31, `same_directory`).
        (Some(observed), Some(planned)) if same_directory(observed, planned) => {
            if let Some(declared) = hermetic.declaration(HermeticRow::H7) {
                row(
                    HermeticRow::H7,
                    Verdict::Ok,
                    format!(
                        "the session ran in {observed}, which was declared, not created: {}",
                        declared.declaration
                    ),
                )
            } else if let Some(unavailable) = hermetic
                .unavailable
                .iter()
                .find(|control| control.row == HermeticRow::H7)
            {
                row(
                    HermeticRow::H7,
                    Verdict::Gap,
                    format!(
                        "the session ran in {observed}, and metaharness could not impose it: {}",
                        unavailable.why
                    ),
                )
            } else {
                row(
                    HermeticRow::H7,
                    Verdict::Ok,
                    format!("the session ran in {observed}, which metaharness created"),
                )
            }
        }
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
    //
    // **A run that copied no input tree has no governing document that could move**, so the row
    // is satisfied rather than unknown. That is not absence of evidence read as a property: the
    // copied tree is metaharness's own launch input, so whether there is one is something
    // metaharness knows for certain rather than something it failed to observe. Reading it as
    // `unk` made `--hermetic strict` unpassable for every run that pins nothing — including the
    // design's own § 9.2 example — which is finding F3's shape a second time (amendment a4).
    rows.push(match inputs_digest {
        None => row(
            HermeticRow::H10,
            Verdict::Ok,
            "the run pinned no input tree, so there is no governing document that could move \
             under it",
        ),
        Some(digest) => row(
            HermeticRow::H10,
            Verdict::Ok,
            format!("the copied tree is {digest}"),
        ),
    });

    // H11 — no memory file outside the copied tree is discoverable. Launch, held to the memory
    // canary of the version the record reports where the kind records one (amendment a36).
    rows.push(memory_held_to_canary(
        attested(HermeticRow::H11, hermetic),
        harness_version.as_deref(),
        inputs.memory_canaries,
    ));

    rows.sort_by_key(|verdict| verdict.row);
    rows
}

/// H1a over a plugin list the record carried: exactly the declared set, beside the vendor's own
/// compiled-in plugins (amendment a33).
///
/// Every plugin in the record is one of two things or the row is a `gap`:
///
/// * **a built-in of the binary the record reports** — the record's version, the plugin's name
///   **and** its source equal one [`BuiltinPlugin`]. Never the name alone: an ambient plugin can
///   borrow any name, and the source is what says it was compiled in; and never another version's
///   list, because which plugins a binary compiles in is read per binary;
/// * **a declared plugin** — compared by name against what metaharness installed, as before a33.
///
/// A plugin the record gives no name is neither, and is a `gap` rather than a plugin the comparison
/// skips. The row names every built-in it admitted, so a reader sees what the vendor brought along
/// as well as what the run declared.
fn plugin_row(
    loaded: &[PluginRef],
    version: Option<&str>,
    declared: &[String],
    builtins: &[BuiltinPlugin],
) -> RowVerdict {
    let mut admitted: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    let mut unmatched: Vec<String> = Vec::new();
    let mut nameless = false;
    for plugin in loaded {
        let builtin = builtins.iter().find(|builtin| {
            version == Some(builtin.version.as_str())
                && plugin.name.as_deref() == Some(builtin.name.as_str())
                && plugin.source.as_deref() == Some(builtin.source.as_str())
        });
        if let Some(builtin) = builtin {
            admitted.push(builtin.source.clone());
            continue;
        }
        let name = plugin.name.clone();
        nameless |= name.is_none();
        unmatched.push(format!(
            "{} ({})",
            name.as_deref().unwrap_or("no name"),
            plugin.source.as_deref().unwrap_or("no source")
        ));
        names.extend(name);
    }
    admitted.sort();
    names.sort();
    unmatched.sort();
    let mut declared = declared.to_vec();
    declared.sort();
    let version = version.unwrap_or("an unstated version");
    let beside = if admitted.is_empty() {
        String::new()
    } else {
        format!(
            ", beside {} built-in plugin(s) the pinned {version} binary compiles in, each matched on \
             its name and its source: {admitted:?}",
            admitted.len()
        )
    };
    if !nameless && names == declared {
        row(
            HermeticRow::H1a,
            Verdict::Ok,
            format!("loaded exactly the declared set: {names:?}{beside}"),
        )
    } else {
        row(
            HermeticRow::H1a,
            Verdict::Gap,
            format!(
                "declared {declared:?} and the record says {unmatched:?}, none of which is a \
                 built-in the adapter declares for {version} by name and source{beside}"
            ),
        )
    }
}

/// Whether the directory the record names is the directory metaharness planned (H7, a31).
///
/// **Two names are one directory when they are byte-equal, or when both are absolute and resolve
/// (every link followed, `.` and `..` removed, as `std::fs::canonicalize` does) to the same path
/// without passing through a name that means the resolving process.** A loop may record the
/// canonical root its tools act in — harness 0.15.1 does — while the directory metaharness creates
/// is spelled under `TMPDIR`, which may pass through a link (a `TMPDIR` under a linked home is the
/// ordinary case), so comparing spellings would read the same directory as a `gap`.
///
/// Nothing is resolved against metaharness's own process, because that says nothing about where
/// the loop worked: a relative name is never resolved (it would be resolved against metaharness's
/// working directory), and neither is a name whose resolution reaches `/proc` or `/dev/fd` at any
/// step — `/proc/self/cwd`, `/proc/thread-self/…`, `/proc/<pid>/…`, `/dev/fd/<n>`, any `.`, `..` or
/// doubled-slash spelling of them, and any link whose target leads there — because resolved inside
/// metaharness each names metaharness's own working directory, root or file descriptor. Such a
/// name, and one that does not resolve (gone, or never on this machine), is compared as spelled, so
/// nothing here turns two different names into one directory without the filesystem saying so.
///
/// Resolved when the floor is evaluated (see [`hermetic_floor`]), while the run still holds the
/// directory it created; a named `--cwd` is canonical from the launch on.
fn same_directory(observed: &str, planned: &str) -> bool {
    if observed == planned {
        return true;
    }
    match (
        resolve_without_the_reader(Path::new(observed)),
        resolve_without_the_reader(Path::new(planned)),
    ) {
        (Some(observed), Some(planned)) => observed == planned,
        _ => false,
    }
}

/// An absolute name resolved component by component, links followed as `realpath` follows them,
/// or `None` when it is relative, does not resolve, loops, or reaches at any step a directory whose
/// entries mean the resolving process (`/proc`, `/dev/fd`). The check is made on every resolved
/// prefix, not on the spelling, so `/proc/./self`, `/tmp/../proc`, `/dev/stdin` and a link into
/// `/proc` are refused like `/proc/self` itself. A name that goes on past something that is not a
/// directory — `<file>/..`, `<file>/.`, `<file>/`, a link whose target ends in `/` and lands on a
/// file — is `None` too, as `realpath` and the kernel refuse it with `ENOTDIR`.
fn resolve_without_the_reader(name: &Path) -> Option<PathBuf> {
    /// As many links as Linux follows before `ELOOP`.
    const MAX_LINKS: usize = 40;
    fn push_front(pending: &mut Vec<PathBuf>, path: &Path) {
        // `components()` drops a trailing `/` or `/.`, which asks for a directory; it is kept as a
        // `.` that the loop checks once everything before it is resolved.
        let bytes = path.as_os_str().as_encoded_bytes();
        if bytes.ends_with(b"/") || bytes.ends_with(b"/.") {
            pending.push(PathBuf::from("."));
        }
        pending.extend(
            path.components()
                .rev()
                .map(|component| PathBuf::from(component.as_os_str())),
        );
    }
    if !name.is_absolute() {
        return None;
    }
    let mut resolved = PathBuf::new();
    let mut pending = Vec::new();
    push_front(&mut pending, name);
    let mut links = 0;
    while let Some(part) = pending.pop() {
        match part.components().next() {
            Some(Component::RootDir | Component::Prefix(_)) => resolved.push(&part),
            Some(Component::ParentDir) => {
                resolved.pop();
            }
            Some(Component::Normal(_)) => {
                let candidate = resolved.join(&part);
                if ["/proc", "/dev/fd"]
                    .iter()
                    .any(|reader| candidate.starts_with(reader))
                {
                    return None;
                }
                let metadata = std::fs::symlink_metadata(&candidate).ok()?;
                if metadata.file_type().is_symlink() {
                    links += 1;
                    if links > MAX_LINKS {
                        return None;
                    }
                    push_front(&mut pending, &std::fs::read_link(&candidate).ok()?);
                } else if pending.is_empty() || metadata.is_dir() {
                    resolved = candidate;
                } else {
                    // More follows, and only a directory has anything after it (`ENOTDIR`).
                    return None;
                }
            }
            // `.` names the directory resolved so far, and only a directory has one.
            Some(Component::CurDir)
                if !std::fs::metadata(&resolved).is_ok_and(|metadata| metadata.is_dir()) =>
            {
                return None;
            }
            Some(Component::CurDir) | None => {}
        }
    }
    Some(resolved)
}

/// H11's attested verdict, held to the memory canary of the binary that ran (amendment a36).
/// Only an `ok` is held: a canary never turns a `gap` or an `unk` into anything better.
fn memory_held_to_canary(
    attested: RowVerdict,
    version: Option<&str>,
    canaries: Option<&[MemoryCanary]>,
) -> RowVerdict {
    let Some(canaries) = canaries else {
        return attested;
    };
    if attested.verdict != Verdict::Ok {
        return attested;
    }
    let Some(version) = version else {
        return row(
            HermeticRow::H11,
            Verdict::Unk,
            format!(
                "{}; the opening record names no version, so there is no memory canary to hold \
                 the control to",
                attested.detail
            ),
        );
    };
    let Some(canary) = canaries.iter().find(|canary| canary.version == version) else {
        return row(
            HermeticRow::H11,
            Verdict::Gap,
            format!(
                "{}; but there is no memory canary recorded for {version}, so nobody measured \
                 whether this binary honours the control",
                attested.detail
            ),
        );
    };
    let measured = format!(
        "the memory canary recorded for {version}: {} planted, the control sent {}, {} reached a \
         request through the launch",
        canary.planted, canary.control_sent, canary.reached_through_launch
    );
    if canary.reached_through_launch > 0 || canary.control_sent < canary.planted {
        return row(
            HermeticRow::H11,
            Verdict::Gap,
            format!(
                "{}; but {measured}, so the control is not shown to keep every memory shape out",
                attested.detail
            ),
        );
    }
    row(
        HermeticRow::H11,
        Verdict::Ok,
        if canary.not_covered.is_empty() {
            format!("{}; {measured}", attested.detail)
        } else {
            format!("{}; {measured}; {}", attested.detail, canary.not_covered)
        },
    )
}

/// A launch-asserted row, read out of metaharness's own attestation.
///
/// The attestation is a claim and not evidence (§ 8.3): it says what metaharness imposed before
/// spawning, which is strong about the argv and the child environment and silent about what the
/// harness then did with them.
fn attested(which: HermeticRow, attestation: &HermeticAttestation) -> RowVerdict {
    // A declaration is admitted for H7 alone (amendment a27), and H7 is a record row that never
    // reaches this function. A launch row that arrives here declared is a build claiming what it
    // cannot know, and the claim is read as the gap it is — before `imposed`, so it cannot be
    // laundered by also being listed there.
    if let Some(declared) = attestation.declaration(which)
        && !which.admits_declaration()
    {
        return row(
            which,
            Verdict::Gap,
            format!(
                "the attestation declared {}, a row the design admits no declaration for: {}",
                which.id(),
                declared.declaration
            ),
        );
    }
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

/// Whether a run with this spec owes a verdict: `--audit`, or `--hermetic strict` (amendment a3).
///
/// One function for [`crate::run::Run::wants_audit`] and the pairing check `start` makes before
/// the spawn (amendment a43), so the run that is refused for wanting no audit and the run that is
/// never audited are the same run.
pub(crate) fn spec_wants_audit(spec: &metaharness_protocol::RunSpec) -> bool {
    spec.audit || spec.hermetic == metaharness_protocol::HermeticMode::Strict
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
        spec_wants_audit(self.spec())
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
                builtin_plugins: &launch.builtin_plugins,
                endpoint_key_posture: launch.endpoint_key_posture,
                planned_mcp_servers: &launch.planned_mcp_servers,
                planned_output_style: launch.planned_output_style,
                record_silences: launch.record_silences,
                memory_canaries: launch.memory_canaries.as_deref(),
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
            withheld: withheld_tools(self.events()),
            installed_plugins: installed_plugins(self.events()),
            auditor,
            saw_terminal_record: self.saw_terminal_record(),
            stream: stream_completeness(self.events()),
        })
    }

    /// The exit code for this run, with or without an audit.
    #[must_use]
    pub fn exit(&self, report: Option<&AuditReport>) -> RunExit {
        match report {
            Some(report) => report.exit(),
            None => exit_without_audit(
                self.saw_terminal_record()
                    && matches!(
                        stream_completeness(self.events()),
                        StreamCompleteness::Complete {
                            reason: CloseReason::Completed,
                            ..
                        }
                    ),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    //! The one arm of the floor no run reaches on its own: a declaration on a row the design
    //! admits none for (amendment a27, H7 only).

    use metaharness_protocol::{
        DeclaredControl, HermeticAttestation, HermeticMode, HermeticRow, ImposedControl, Verdict,
    };

    use super::attested;

    /// A build that declared H11 away is read as a gap, and listing the row as imposed beside
    /// the declaration does not launder it: the guard runs before `imposed` is consulted.
    #[test]
    fn a_declaration_on_a_row_other_than_h7_is_a_gap_even_when_also_imposed() {
        let mut attestation = HermeticAttestation::none(HermeticMode::Strict);
        attestation.declared.push(DeclaredControl {
            row: HermeticRow::H11,
            declaration: "the embedder declared the walk clean".into(),
        });
        let verdict = attested(HermeticRow::H11, &attestation);
        assert_eq!(verdict.verdict, Verdict::Gap, "{}", verdict.detail);
        assert!(
            verdict.detail.contains("admits no declaration") && verdict.detail.contains("H11"),
            "{}",
            verdict.detail
        );

        attestation.imposed.push(ImposedControl {
            row: HermeticRow::H11,
            how: "the ancestor walk found nothing".into(),
        });
        let laundered = attested(HermeticRow::H11, &attestation);
        assert_eq!(laundered.verdict, Verdict::Gap, "{}", laundered.detail);

        let honest = HermeticAttestation::none(HermeticMode::Strict);
        assert_eq!(attested(HermeticRow::H11, &honest).verdict, Verdict::Unk);
    }
}
