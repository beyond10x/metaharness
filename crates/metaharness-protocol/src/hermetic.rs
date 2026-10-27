//! The hermetic contract: twelve rows, each imposed and each asserted.
//!
//! Two things about this module are the whole point:
//!
//! * **A row is asserted either from the vendor's own record or from a value metaharness holds
//!   before spawning, and those are not the same strength**, so [`HermeticRow::assertion`] says
//!   which (design § 8.1).
//! * **Gating is per row, not global.** Two rows are unobservable as a property of the
//!   *mechanism* rather than of the run; if any `unk` failed a strict run, every strict run
//!   would fail forever (design § 8.1, finding F3). Those two are [`Severity::Advisory`]:
//!   evaluated, reported, printed, and not counted against the exit code. H9 is the third,
//!   by decision rather than mechanism: a vendor version off the pin is reported, never refused
//!   (amendment a62).

use serde::{Deserialize, Serialize};

use crate::plugin::InstalledPlugin;
use crate::spec::DecisionMode;

/// How hermetic a run is asked to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum HermeticMode {
    /// No hermetic controls imposed. The run inherits the operator's world.
    Off,
    /// The controls are imposed and the verdict is reported.
    On,
    /// The controls are imposed and a gating row that is not `ok` fails the run.
    Strict,
}

/// Whether a row's verdict moves the exit code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// A `gap` is exit 1 and an `unk` is exit 3.
    Gating,
    /// Evaluated, reported, printed — and it does not move the exit code.
    Advisory,
}

/// What a row's evidence is, and therefore how strong it is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Assertion {
    /// Read out of the vendor's own record. The strongest form available without a live run.
    Record,
    /// A value metaharness holds before spawning — the argv, the child environment, an ancestor
    /// walk. Strong about what was imposed, silent about what the harness then did.
    Launch,
    /// Not directly assertable at all; the evidence is the effect of other rows.
    Effect,
}

/// One verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    /// The control held.
    Ok,
    /// The control did not hold.
    Gap,
    /// Nobody found out. Not a softer `gap`: a crashed suite is not a failing suite, and
    /// absence of evidence is not hermeticity (design § 2.1, § 9.4).
    Unk,
}

/// One row of the hermetic contract.
///
/// The identifiers are the design's own, so a report and the document can be read side by side.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HermeticRow {
    /// Config home is scratch — plugins are exactly the declared set.
    H1a,
    /// …and the output style is the default. Split from H1a because they fail independently and
    /// one unknown must not mask the other (finding F11).
    H1b,
    /// Settings sources excluded.
    H2,
    /// The environment is constructed, not inherited.
    H3,
    /// No API key unless the run declared one.
    H4,
    /// The MCP surface is exactly what the launch gave.
    H5,
    /// Credential carriage is exactly what the run declared.
    H6,
    /// The working directory is ours.
    H7,
    /// Hooks and customizations are not skipped.
    H8,
    /// The vendor version is the pinned one.
    H9,
    /// Governing documents cannot move under the run.
    H10,
    /// No memory file outside the copied tree is discoverable.
    H11,
}

impl HermeticRow {
    /// Every row, in the design's order.
    pub const ALL: [HermeticRow; 12] = [
        HermeticRow::H1a,
        HermeticRow::H1b,
        HermeticRow::H2,
        HermeticRow::H3,
        HermeticRow::H4,
        HermeticRow::H5,
        HermeticRow::H6,
        HermeticRow::H7,
        HermeticRow::H8,
        HermeticRow::H9,
        HermeticRow::H10,
        HermeticRow::H11,
    ];

    /// The row's id, as the design prints it.
    #[must_use]
    pub fn id(&self) -> &'static str {
        match self {
            HermeticRow::H1a => "H1a",
            HermeticRow::H1b => "H1b",
            HermeticRow::H2 => "H2",
            HermeticRow::H3 => "H3",
            HermeticRow::H4 => "H4",
            HermeticRow::H5 => "H5",
            HermeticRow::H6 => "H6",
            HermeticRow::H7 => "H7",
            HermeticRow::H8 => "H8",
            HermeticRow::H9 => "H9",
            HermeticRow::H10 => "H10",
            HermeticRow::H11 => "H11",
        }
    }

    /// What the row controls, in one line.
    #[must_use]
    pub fn control(&self) -> &'static str {
        match self {
            HermeticRow::H1a => "config home is scratch — plugins are exactly the declared set",
            HermeticRow::H1b => "config home is scratch — the output style is the default",
            HermeticRow::H2 => "settings sources are excluded",
            HermeticRow::H3 => "the environment is constructed, not inherited",
            HermeticRow::H4 => "no API key unless the run declared one",
            HermeticRow::H5 => "the MCP surface is exactly what the launch gave",
            HermeticRow::H6 => "credential carriage is exactly declared",
            HermeticRow::H7 => "the working directory is ours",
            HermeticRow::H8 => "hooks and customizations are not skipped",
            HermeticRow::H9 => "the vendor version is the pinned one",
            HermeticRow::H10 => "governing documents cannot move under the run",
            HermeticRow::H11 => "no memory file outside the copied tree is discoverable",
        }
    }

    /// Whether this row's verdict moves the exit code.
    ///
    /// H2 and H6 are advisory, and the reason is the mechanism rather than the run: the absence
    /// of allow rules that would shadow the seam is not observable in any record, and credential
    /// carriage leaves no trace of its own — its evidence is H1a, H4 and H5 holding.
    ///
    /// H9 is advisory on every kind for a different reason (amendment a62): a binary off the pin
    /// is a reader that may have moved, not a control that failed. Its verdict still reads `gap`,
    /// names both versions and is printed; the rows that rest on a per-version measurement say
    /// for themselves that the binary was not measured, and `--strict-version` is the refusal.
    #[must_use]
    pub fn severity(&self) -> Severity {
        match self {
            HermeticRow::H2 | HermeticRow::H6 | HermeticRow::H9 => Severity::Advisory,
            _ => Severity::Gating,
        }
    }

    /// What this row's evidence is.
    #[must_use]
    pub fn assertion(&self) -> Assertion {
        match self {
            HermeticRow::H1a
            | HermeticRow::H1b
            | HermeticRow::H4
            | HermeticRow::H5
            | HermeticRow::H7
            | HermeticRow::H9
            | HermeticRow::H10 => Assertion::Record,
            HermeticRow::H2 | HermeticRow::H3 | HermeticRow::H8 | HermeticRow::H11 => {
                Assertion::Launch
            }
            HermeticRow::H6 => Assertion::Effect,
        }
    }

    /// Whether this row may be attested `declared` (design amendment a27).
    ///
    /// H7 and only H7: its control *is* who owns the working directory, which is a fact only the
    /// party that made the directory can state. Every other row is imposed or it is not, and a
    /// declaration on one of them is read by the floor as a `gap` — a build that tried to declare
    /// H11 away would fail its own strict run.
    #[must_use]
    pub fn admits_declaration(&self) -> bool {
        matches!(self, HermeticRow::H7)
    }
}

/// One control the embedder declared and metaharness did not impose (design amendment a27).
///
/// The third state beside imposed and unavailable: an embedder's disposable scratch copy, named
/// with `--cwd <dir> --scratch-cwd`, is a directory metaharness did not create and does not treat
/// as the operator's. The declaration says so; it does not make the row `ok` by itself — the floor
/// still compares the record's cwd against the planned one (invariant 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeclaredControl {
    /// Which row. Only a row whose [`HermeticRow::admits_declaration`] is true belongs here.
    pub row: HermeticRow,
    /// Who declared what, and what metaharness measured about it, in words a reader can act on.
    pub declaration: String,
}

/// One control metaharness imposed, and how.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ImposedControl {
    /// Which row.
    pub row: HermeticRow,
    /// How it was imposed, concretely enough to be checked against the argv.
    pub how: String,
}

/// One control metaharness could not impose, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UnavailableControl {
    /// Which row.
    pub row: HermeticRow,
    /// Why not — a vendor with no such knob, a run that declared it did not want it.
    pub why: String,
}

/// metaharness's own claim about its own actions.
///
/// **Not evidence.** The independent evidence is the vendor's opening record: the plugin list,
/// the MCP list, the credential source, the cwd, the version. This block exists so a reader can
/// see the intent beside the outcome and notice when they disagree (design § 8.3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HermeticAttestation {
    /// How hermetic the run asked to be.
    pub mode: HermeticMode,
    /// **Who decided every tool call in this run** (design amendment a10).
    ///
    /// Beside `mode` because it is the same kind of claim: metaharness's own posture, stated by
    /// metaharness, reaching `session.started` where a reader can put it beside what the vendor
    /// recorded. It is here rather than left to be inferred from the `tool.decided` events,
    /// because a run in which the model called no tool emits none of those — and *"the model
    /// never called a tool"* and *"metaharness would have allowed anything it called"* are not
    /// the same fact.
    pub decisions: DecisionMode,
    /// What was imposed.
    pub imposed: Vec<ImposedControl>,
    /// What could not be.
    pub unavailable: Vec<UnavailableControl>,
    /// What the embedder declared instead (design amendment a27): H7 under `--scratch-cwd`.
    ///
    /// **Absent on the wire when empty**, unlike `installed_plugins`, and the difference is the
    /// direction of the claim: a vanished plugin list could hide an installation, whereas a
    /// declaration is additive — its absence claims nothing and can conceal nothing. Every stream
    /// written before a27 reads as *nothing was declared*, which is what was true of it.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub declared: Vec<DeclaredControl>,
    /// Inputs metaharness reports and does **not** claim to have removed — git status is the
    /// named one, because the vendor's own flag description says it is in the system prompt
    /// (design § 8.1, H11's second half).
    pub ambient_inputs: Vec<String>,
    /// Every plugin this launch copied into the run's scratch tree, with its digest and where it
    /// came from (crossing #4).
    ///
    /// **Always present, empty when there is none.** A key that vanished on a plugin-less run
    /// would make "this run installed nothing" and "this build does not report installations"
    /// the same bytes, which is the reading § 8.1 refuses everywhere else: absence of evidence is
    /// not a property. `[]` is metaharness saying *none*; there is no third answer, because this
    /// is metaharness's claim about its own copying and it always knows.
    pub installed_plugins: Vec<InstalledPlugin>,
}

impl HermeticAttestation {
    /// An attestation for a run that imposes nothing.
    ///
    /// `decisions` is [`DecisionMode::Frame`] and not the run's, because this constructor is for
    /// readers that have no run — a replay fixture, an offline projection — and the safe
    /// direction for a value nobody supplied is the one that claims the least. `observe` is
    /// reached by a run asking for it, never by a default.
    #[must_use]
    pub fn none(mode: HermeticMode) -> Self {
        Self {
            mode,
            decisions: DecisionMode::Frame,
            imposed: Vec::new(),
            unavailable: Vec::new(),
            declared: Vec::new(),
            ambient_inputs: Vec::new(),
            installed_plugins: Vec::new(),
        }
    }

    /// Whether this row was claimed as imposed.
    #[must_use]
    pub fn claims(&self, row: HermeticRow) -> bool {
        self.imposed.iter().any(|control| control.row == row)
    }

    /// The embedder's declaration for this row, if the attestation carries one (amendment a27).
    #[must_use]
    pub fn declaration(&self, row: HermeticRow) -> Option<&DeclaredControl> {
        self.declared.iter().find(|control| control.row == row)
    }

    /// Whether this row was attested as declared rather than imposed (amendment a27).
    #[must_use]
    pub fn declares(&self, row: HermeticRow) -> bool {
        self.declaration(row).is_some()
    }

    /// Whether this run allowed every call and adjudicated none of them.
    ///
    /// One named predicate rather than a comparison spelled out at each reader, so a consumer
    /// asking *"was this a capture run?"* asks it the same way everywhere — and so a run that
    /// did not ask for observe mode cannot be read as one by an expression that got the
    /// comparison the wrong way round.
    #[must_use]
    pub fn is_observing(&self) -> bool {
        self.decisions == DecisionMode::Observe
    }
}

/// One row's verdict, with everything a report needs to print it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RowVerdict {
    /// Which row.
    pub row: HermeticRow,
    /// The verdict.
    pub verdict: Verdict,
    /// Whether it gates.
    pub severity: Severity,
    /// What was seen, in the words a reader needs to act on it.
    pub detail: String,
}

impl RowVerdict {
    /// Whether this verdict should move the exit code.
    #[must_use]
    pub fn gates(&self) -> bool {
        self.severity == Severity::Gating && self.verdict != Verdict::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The list is twelve rows and the design says twelve. A thirteenth added here without the
    /// document changing is the drift this assertion exists to catch.
    #[test]
    fn the_contract_is_twelve_rows() {
        assert_eq!(HermeticRow::ALL.len(), 12);
    }

    /// Exactly three rows are advisory. If a fourth became advisory the strict verdict would
    /// quietly weaken; if H2 or H6 became gating, no strict run could ever pass (finding F3); if
    /// H9 became gating again, a binary off the pin would be refused for its version alone
    /// (amendment a62).
    #[test]
    fn exactly_h2_h6_and_h9_are_advisory() {
        let advisory: Vec<&str> = HermeticRow::ALL
            .iter()
            .filter(|row| row.severity() == Severity::Advisory)
            .map(HermeticRow::id)
            .collect();
        assert_eq!(
            advisory,
            ["H2", "H6", "H9"],
            "story:off-pin-vendor-version-is-advisory: H9 is advisory on every kind"
        );
    }

    /// H3 is a launch assertion and not a record one. The first draft claimed a record for it
    /// and the record it named answers H4 instead (finding F12).
    #[test]
    fn h3_is_asserted_at_launch_not_from_a_record() {
        assert_eq!(HermeticRow::H3.assertion(), Assertion::Launch);
        assert_eq!(HermeticRow::H4.assertion(), Assertion::Record);
    }

    #[test]
    fn an_advisory_row_never_gates() {
        let advisory = RowVerdict {
            row: HermeticRow::H6,
            verdict: Verdict::Unk,
            severity: HermeticRow::H6.severity(),
            detail: "no record can carry this".into(),
        };
        assert!(!advisory.gates());

        let gating = RowVerdict {
            row: HermeticRow::H5,
            verdict: Verdict::Unk,
            severity: HermeticRow::H5.severity(),
            detail: "no MCP list in the opening record".into(),
        };
        assert!(gating.gates());
    }

    /// A declaration is admitted for H7 and nothing else (amendment a27). A second row here would
    /// be a row whose `ok` can be bought with a flag.
    #[test]
    fn only_h7_admits_a_declaration() {
        let admitted: Vec<&str> = HermeticRow::ALL
            .iter()
            .filter(|row| row.admits_declaration())
            .map(HermeticRow::id)
            .collect();
        assert_eq!(admitted, ["H7"]);
    }

    /// The wire shape a27 chose: an empty `declared` list writes no key, so every stream written
    /// before it keeps its bytes, and a stream without the key reads as *nothing was declared*.
    #[test]
    fn an_empty_declared_list_is_absent_on_the_wire_and_absent_reads_as_empty() {
        let none = HermeticAttestation::none(HermeticMode::Strict);
        let json = serde_json::to_string(&none).unwrap();
        assert!(!json.contains("declared"), "{json}");
        let parsed: HermeticAttestation = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, none);

        let mut declared = none;
        declared.declared.push(DeclaredControl {
            row: HermeticRow::H7,
            declaration: "the embedder's copy".into(),
        });
        let json = serde_json::to_string(&declared).unwrap();
        assert!(json.contains("\"declared\""), "{json}");
        let parsed: HermeticAttestation = serde_json::from_str(&json).unwrap();
        assert!(parsed.declares(HermeticRow::H7));
        assert!(!parsed.declares(HermeticRow::H11));
        assert_eq!(
            parsed.declaration(HermeticRow::H7).unwrap().declaration,
            "the embedder's copy"
        );
    }
}
