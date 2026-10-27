//! Whether a vendor binary would pass the hermetic floor, answered before any session.
//!
//! The floor reads H9 and H11 from an opening record, which only exists once a session has been
//! paid for. Both rows rest on the binary's version alone: H9 on the adapter's pin, H11 on the
//! memory canary recorded for that version. [`qualify`] asks the binary for its `--version`, the
//! one invocation it makes, and reads those rows through the same functions the floor uses, so a
//! caller learns that a binary would stop a strict run before it spends a session finding out.
//!
//! **Only the rows a version decides are here.** H1a, H1b, H4, H5 and H7 read a record this
//! function never has; H2, H3 and H8 are the launch's own impositions and do not vary with the
//! binary. A row missing from [`Qualification::rows`] was not evaluated, and says nothing.

use std::path::PathBuf;

use metaharness_protocol::{HermeticMode, HermeticRow, Kind, RowVerdict, Verdict};

use crate::audit::{memory_held_to_canary, pin_row, row};
use crate::builder::{VERSION_PROBE_DEADLINE, VersionProbe, vendor_version};
use crate::doctor::{pinned_versions, resolve_on};
use crate::refusal::Refusal;

/// One floor row as a binary's version decides it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QualifiedRow {
    /// The row, its verdict, its severity and what was read, as the floor words it.
    pub row: RowVerdict,
    /// Whether this row stops a run under the mode [`qualify`] was asked about: a gating row that
    /// is not `ok`, under [`HermeticMode::Strict`]. Under any other mode the verdict is reported
    /// and no row gates.
    pub gates: bool,
}

/// What [`qualify`] found out about one binary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Qualification {
    /// The adapter the binary was read as.
    pub kind: Kind,
    /// The binary that was run, resolved on the child's `PATH`.
    pub program: PathBuf,
    /// The version token its `--version` printed, or `None` when it printed nothing that reads as
    /// one, failed, or did not exit in time. Each row says which.
    pub version: Option<String>,
    /// The versions the adapter is pinned to.
    pub pinned: Vec<String>,
    /// The mode the rows were read under.
    pub hermetic: HermeticMode,
    /// Every row the version decides, in the design's order.
    pub rows: Vec<QualifiedRow>,
    /// Whether `--strict-version` refuses this binary before the spawn: its version is not one of
    /// [`Qualification::pinned`], or is not known. H9 itself is advisory on every kind (a62), so
    /// this is the only place an off-pin binary stops a run on its version alone.
    pub strict_version_refuses: bool,
}

impl Qualification {
    /// The rows that stop a run under [`Qualification::hermetic`].
    pub fn failing(&self) -> impl Iterator<Item = &QualifiedRow> {
        self.rows.iter().filter(|row| row.gates)
    }

    /// Whether no row stops a run under [`Qualification::hermetic`].
    ///
    /// This is the floor's reading and not `--strict-version`'s: an off-pin Codex binary is
    /// qualified here, because H9 is advisory, and refused only by
    /// [`Qualification::strict_version_refuses`].
    #[must_use]
    pub fn qualified(&self) -> bool {
        self.failing().next().is_none()
    }
}

/// The `PATH` a spawned child of `kind` gets, the one `program` is resolved on.
fn child_path(kind: Kind, home: Option<&str>) -> String {
    match kind {
        Kind::Claude => metaharness_claude::child_path(home),
        Kind::Codex => metaharness_codex::child_path(home),
        Kind::B10x => metaharness_b10x::child_path(home),
    }
}

/// Ask `program` for its version, make no model call, and report the floor rows that version
/// decides.
///
/// `program` is resolved on the `PATH` a spawned child of `kind` gets, built from this process's
/// `HOME` (the way [`crate::installed`] and the launch resolve it); a name containing a `/` is
/// that path. It is run once, as `program --version`, in a constructed environment holding that
/// `PATH` alone and within a deadline, so it reads no operator config home and holds no
/// credential.
///
/// The rows are the ones [`crate::Run::hermetic_floor`] reads for an opening record on the same
/// version: H9 for every kind, and H11 for a kind whose launch records a memory canary (Claude
/// Code). H11 assumes the launch's own imposition, a scratch working directory, which is what
/// the floor holds to the canary; a version with no canary reads `unk` there (a62).
///
/// # Errors
///
/// [`Refusal::Io`] when `program` resolves to nothing on that `PATH`; the detail names the
/// program and the `PATH` searched. A binary that runs but reports no version is not an error: it
/// is a [`Qualification`] whose rows are `unk`.
pub fn qualify(
    kind: Kind,
    program: &str,
    hermetic: HermeticMode,
) -> Result<Qualification, Refusal> {
    let home = std::env::var("HOME").ok();
    let path = child_path(kind, home.as_deref());
    let resolved = resolve_on(program, &path)?;
    let env = std::collections::BTreeMap::from([("PATH".to_owned(), path)]);
    let probe = vendor_version(&resolved, &env, VERSION_PROBE_DEADLINE);
    Ok(qualification(kind, resolved, &probe, hermetic))
}

fn qualification(
    kind: Kind,
    program: PathBuf,
    probe: &VersionProbe,
    hermetic: HermeticMode,
) -> Qualification {
    let pinned = pinned_versions(kind);
    let version = match probe {
        VersionProbe::Reported(version) => Some(version.clone()),
        VersionProbe::Unreadable | VersionProbe::TimedOut(_) => None,
    };
    let mut rows = match &version {
        Some(version) => rows_for(kind, version, &pinned),
        None => unread_rows(kind, probe),
    };
    rows.sort_by_key(|verdict| verdict.row);
    let strict_version_refuses = version
        .as_ref()
        .is_none_or(|version| !pinned.contains(version));
    let rows = rows
        .into_iter()
        .map(|verdict| QualifiedRow {
            gates: hermetic == HermeticMode::Strict && verdict.gates(),
            row: verdict,
        })
        .collect();
    Qualification {
        kind,
        program,
        version,
        pinned,
        hermetic,
        rows,
        strict_version_refuses,
    }
}

/// The rows the floor reads for an opening record on `version`.
fn rows_for(kind: Kind, version: &str, pinned: &[String]) -> Vec<RowVerdict> {
    let mut rows = vec![pin_row(version, pinned)];
    let canaries = crate::vectors::memory_canaries(kind);
    if !canaries.is_empty() {
        rows.push(memory_held_to_canary(
            row(
                HermeticRow::H11,
                Verdict::Ok,
                "the launch imposes it over a scratch working directory",
            ),
            Some(version),
            Some(&canaries),
        ));
    }
    rows
}

/// The same rows for a binary that told nothing: nobody found out, which is `unk` and never `ok`.
fn unread_rows(kind: Kind, probe: &VersionProbe) -> Vec<RowVerdict> {
    let why = match probe {
        VersionProbe::TimedOut(deadline) => format!(
            "its --version did not exit within {} ms and was killed",
            deadline.as_millis()
        ),
        VersionProbe::Unreadable | VersionProbe::Reported(_) => {
            "its --version failed or printed no readable version".to_owned()
        }
    };
    let mut rows = vec![row(
        HermeticRow::H9,
        Verdict::Unk,
        format!("the binary's version is unknown: {why}"),
    )];
    if !crate::vectors::memory_canaries(kind).is_empty() {
        rows.push(row(
            HermeticRow::H11,
            Verdict::Unk,
            format!("there is no version to hold the memory control to: {why}"),
        ));
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use metaharness_protocol::Severity;

    fn probed(kind: Kind, version: &str, hermetic: HermeticMode) -> Qualification {
        qualification(
            kind,
            PathBuf::from("/fake"),
            &VersionProbe::Reported(version.to_owned()),
            hermetic,
        )
    }

    /// Only a strict run is stopped: the same unmeasured binary is reported under `on` and
    /// `off`, because gating is the mode's decision and not the row's.
    #[test]
    fn only_strict_gates() {
        for (mode, gates) in [
            (HermeticMode::Strict, true),
            (HermeticMode::On, false),
            (HermeticMode::Off, false),
        ] {
            let found = probed(Kind::Claude, "2.1.311", mode);
            assert_eq!(found.qualified(), !gates, "{mode:?}");
        }
    }

    /// H9 never gates, on any kind and any mode (a62), while the refusal is `--strict-version`'s.
    #[test]
    fn h9_is_advisory_and_the_pin_refusal_is_separate() {
        for kind in [Kind::Claude, Kind::Codex, Kind::B10x] {
            let found = probed(kind, "0.0.0-off-the-pin", HermeticMode::Strict);
            let h9 = found
                .rows
                .iter()
                .find(|row| row.row.row == HermeticRow::H9)
                .expect("H9 is read");
            assert_eq!(h9.row.verdict, Verdict::Gap, "{kind:?}");
            assert_eq!(h9.row.severity, Severity::Advisory, "{kind:?}");
            assert!(!h9.gates, "{kind:?}");
            assert!(found.strict_version_refuses, "{kind:?}");
        }
    }

    /// A timed-out probe is a version nobody learned: `unk` rows, a refusal under
    /// `--strict-version`, and the deadline in the words.
    #[test]
    fn a_probe_that_timed_out_is_unk_and_says_so() {
        let found = qualification(
            Kind::Claude,
            PathBuf::from("/fake"),
            &VersionProbe::TimedOut(std::time::Duration::from_millis(300)),
            HermeticMode::Strict,
        );
        assert_eq!(found.version, None);
        assert!(found.strict_version_refuses);
        assert!(found.rows.iter().all(|row| row.row.verdict == Verdict::Unk));
        assert!(found.rows[0].row.detail.contains("300 ms"));
        assert!(found.rows.iter().any(|row| row.gates));
    }
}
