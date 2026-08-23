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
//! The two adapters do not answer identically today, and that is the finding, not a defect in the
//! shape: claude fills all four rows, codex fills three and names its missing launch face. Before
//! the shape existed that absence was invisible — the codex contract record read `checked: 10`,
//! `failed: 0`, and said nothing at all about a face it never tested.

use metaharness::protocol::{ContractObligations, Kind, Obligation};
use metaharness::{ADAPTERS, conformance_vectors, contract_obligations, contract_result};

/// Every kind this build carries. The length assertion below is what keeps it honest.
const KINDS: [Kind; 2] = [Kind::Claude, Kind::Codex];

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

/// The codex adapter likewise — three rows filled, one row a named gap, and nothing declared that
/// the run does not deliver.
#[test]
fn the_codex_adapter_fills_the_contract_authoring_shape() {
    let unmet = unmet(Kind::Codex);
    assert!(unmet.is_empty(), "{unmet:#?}");
}

/// Today's asymmetry, pinned so that closing it is a deliberate act.
///
/// Codex has no `fixtures/c1/` and therefore no launch vector; its launch plan is pinned by unit
/// tests, which redden this workspace's suite and never the record a consumer reads. Recording that
/// as a **named** gap rather than an absence is the same rule the version pair follows: never a
/// silent pass. When the row is filled, this test goes red and is deleted with the gap.
#[test]
fn the_codex_launch_row_is_a_named_gap_rather_than_a_silent_absence() {
    let declared = contract_obligations(Kind::Codex).expect("the adapter exists");
    let Obligation::Gap(reason) = declared.launch else {
        panic!(
            "the codex launch row is filled now — delete this test, and check that the record's \
             `checked` count and every vector-count pin moved with it"
        );
    };
    assert!(
        reason.contains("C1") && reason.contains("src/launch.rs"),
        "the gap names what stands in for the missing vectors: {reason}"
    );
    // And the claude adapter is the row's worked example, which is what makes the gap actionable.
    let claude = contract_obligations(Kind::Claude).expect("the adapter exists");
    let Obligation::Filled(ids) = claude.launch else {
        panic!("the claude launch row is the filled one the codex gap points at");
    };
    assert!(!ids.is_empty());
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
