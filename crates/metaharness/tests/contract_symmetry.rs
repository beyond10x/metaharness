//! CT-4 — one authoring shape every adapter fills, checked against what it really produced.
//!
//! The milestone's sentence is *"a new adapter's contract is a checklist, not a fresh invention"*
//! (design `docs/design/adapter-contract-v0.1.md`). The checklist is
//! [`metaharness::protocol::ContractObligations`]: four rows, no `Default`, no optional field, so a
//! declaration cannot be written without answering all of them, and `contract_obligations(kind)`
//! will not compile for a third adapter until that adapter has one.
//!
//! What makes it a contract rather than a comment is this file. Each adapter's declaration is
//! compared against its **own** `conformance_vectors()` output and the `provider` string its
//! `contract_result` record carries: a row that names a vector the run does not produce, produces
//! in another tier, or produces red is an unmet obligation, and so is a row declared unfilled
//! without saying why.
//!
//! The two adapters answer identically **since 2026-08-23**, and how they got there is the whole
//! argument for the shape. On its first run codex filled three rows and named its missing launch
//! face; before the shape existed that absence was invisible — the codex contract record read
//! `checked: 10`, `failed: 0`, and said nothing at all about a face it never tested. Filling the
//! row is what moved the record to `checked: 17`, deliberately and with the diff read.

use metaharness::protocol::{ContractObligations, Kind, Obligation};
use metaharness::{ADAPTERS, conformance_vectors, contract_obligations, contract_result};

/// Every kind this build carries. The length assertion below is what keeps it honest.
const KINDS: [Kind; 3] = [Kind::Claude, Kind::Codex, Kind::B10x];

/// Read one adapter's declaration, its real vectors and its real provider string, and report what
/// the declaration promised and the run did not deliver.
fn unmet(kind: Kind) -> Vec<String> {
    let declared: ContractObligations = contract_obligations(kind).expect("the adapter exists");
    let vectors = conformance_vectors(kind).expect("the adapter exists");
    let record = contract_result(kind, &vectors).expect("the record is built");
    let provider = record["provider"]
        .as_str()
        .expect("the record carries a provider")
        .to_string();
    declared.unmet(&vectors, &provider)
}

/// The claude adapter fills the shape, and every vector it names is really there and really green.
#[test]
fn the_claude_adapter_fills_the_contract_authoring_shape() {
    let unmet = unmet(Kind::Claude);
    assert!(unmet.is_empty(), "{unmet:#?}");
}

/// The codex adapter likewise — all four rows filled since the launch face was recorded, and
/// nothing declared that the run does not deliver.
#[test]
fn the_codex_adapter_fills_the_contract_authoring_shape() {
    let unmet = unmet(Kind::Codex);
    assert!(unmet.is_empty(), "{unmet:#?}");
}

/// The direct-provider adapter fills every applicable row. Its hook row remains a named N/A:
/// adding a hook would violate the observe-only boundary rather than improve contract coverage.
#[test]
fn the_b10x_adapter_fills_every_applicable_contract_row() {
    let unmet = unmet(Kind::B10x);
    assert!(unmet.is_empty(), "{unmet:#?}");
    let declared = contract_obligations(Kind::B10x).expect("the adapter exists");
    let Obligation::Gap(reason) = declared.recorded_hook_input else {
        panic!("b10x must not claim a hook input for a seam it does not have")
    };
    assert!(reason.contains("not applicable"), "{reason}");
}

/// The asymmetry CT-4 found, now closed — and pinned closed, so re-opening it is deliberate too.
///
/// Until 2026-08-23 the codex launch row was a **named gap**: no `fixtures/c1/`, a launch plan
/// pinned by unit tests that redden this workspace's suite and never the record a consumer reads.
/// Filling it moved `checked` from 10 to 17, which is the only way a count is allowed to move. What
/// replaces the gap test is this: **no** adapter may answer the launch row with a gap now that both
/// can answer it with vectors, because the next adapter inherits the standard the last one set.
#[test]
fn no_adapter_answers_the_launch_row_with_a_gap_any_more() {
    for kind in KINDS {
        let declared = contract_obligations(kind).expect("the adapter exists");
        let Obligation::Filled(ids) = declared.launch else {
            panic!(
                "{}'s launch row is a gap again. If that is deliberate, the reason belongs in the \
                 declaration and this test's sentence has to change with it — but the standard \
                 both adapters meet today is a recorded launch expectation, not a unit test",
                kind.as_str()
            );
        };
        assert!(
            !ids.is_empty(),
            "{} declares its launch face filled by no vector at all",
            kind.as_str()
        );
    }
}

/// No adapter reaches a consumer without a declaration.
///
/// `contract_obligations` is exhaustive over [`Kind`], so the compiler already refuses a third
/// adapter with no declaration; what this adds is that the declaration is *about that adapter* —
/// a copied row set naming the wrong id is exactly the "fresh invention" CT-4 is against — and
/// that this file's own list of kinds did not fall behind `ADAPTERS`.
#[test]
fn every_adapter_declares_through_the_same_shape() {
    assert_eq!(
        KINDS.len(),
        ADAPTERS.len(),
        "an adapter was added and CT-4's per-adapter checks were not extended to it"
    );
    for kind in KINDS {
        let declared = contract_obligations(kind).expect("the adapter exists");
        assert_eq!(
            declared.adapter,
            kind.as_str(),
            "the declaration names a different adapter than the one it is dispatched for"
        );
        assert_eq!(declared.rows().len(), 4, "the shape is the same four rows");
    }
}

/// An obligation the run does not deliver is reported by name, not by a bare false.
///
/// The checklist's whole value is that a failure says which row, which vector id and which of the
/// three ways it went wrong — otherwise the next adapter's author is sent back to read this file.
#[test]
fn an_unmet_obligation_names_the_row_and_the_vector() {
    use metaharness::protocol::{ConformanceTier, VectorOutcome};

    let declared = ContractObligations {
        adapter: "pi",
        launch: Obligation::Filled(&["c1-nothing-produces-this"]),
        recorded_wire: Obligation::Filled(&["wrong-tier"]),
        recorded_hook_input: Obligation::Filled(&["is-red"]),
        version_pair: Obligation::Gap("   "),
    };
    let vectors = vec![
        VectorOutcome::passed("wrong-tier", ConformanceTier::C3),
        VectorOutcome::failed("is-red", ConformanceTier::C2, "the vendor moved"),
    ];
    let unmet = declared.unmet(&vectors, "pi");
    let report = unmet.join("\n");

    assert!(
        report.contains("c1-nothing-produces-this") && report.contains("no such vector"),
        "{report}"
    );
    assert!(
        report.contains("`wrong-tier` is declared as C2 and the run reports it as C3"),
        "{report}"
    );
    assert!(
        report.contains("`is-red` failed — the vendor moved"),
        "{report}"
    );
    assert!(
        report.contains("declared unfilled without saying why"),
        "a gap that names no reason is an absence wearing a type: {report}"
    );
    assert!(
        report.contains("pi <pinned version>"),
        "a provider with no pin does not say which binary the contract is about: {report}"
    );
    assert_eq!(unmet.len(), 5, "{report}");
}

/// The record-level rows: a run that checked nothing asserts nothing, whatever its declaration says.
#[test]
fn a_run_that_checked_nothing_is_an_unmet_obligation_on_its_own() {
    let declared = contract_obligations(Kind::Claude).expect("the adapter exists");
    let unmet = declared.unmet(&[], "claude 2.1.240");
    assert!(
        unmet.iter().any(|gap| gap.contains("checked is 0")),
        "{unmet:#?}"
    );
}
