//! Conformance tiers, and what each one costs.
//!
//! Three of the four tiers need no model, no network and no credential (design D13), which is
//! what makes the adapter's promises a tested claim rather than a paragraph in a document.

use serde::{Deserialize, Serialize};

/// Which tier a vector belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConformanceTier {
    /// Launch vectors: the argv and the child environment the adapter would construct for a
    /// given spec, against a recorded expectation. Free.
    C1,
    /// Replay vectors: recorded vendor transcripts in, expected event stream out, byte-exact
    /// JSONL. Free.
    C2,
    /// Control vectors: a scripted fake vendor speaking the vendor's own wire, driven through
    /// allow, deny, replace, deadline expiry, cancel-instead-of-decide, an unknown call and a
    /// decision after the window closed. Free, and this is the tier that carries the safety
    /// argument.
    C3,
    /// One live run with a deliberate denial in it. Costs money and network, and is **never**
    /// part of the default gate.
    C4,
}

impl ConformanceTier {
    /// The tiers that run with no model, no network and no credential.
    pub const FREE: [ConformanceTier; 3] = [
        ConformanceTier::C1,
        ConformanceTier::C2,
        ConformanceTier::C3,
    ];

    /// The tier's name, as a report prints it.
    #[must_use]
    pub fn as_str(&self) -> &'static str {
        match self {
            ConformanceTier::C1 => "C1",
            ConformanceTier::C2 => "C2",
            ConformanceTier::C3 => "C3",
            ConformanceTier::C4 => "C4",
        }
    }

    /// Whether this tier needs a model call.
    #[must_use]
    pub fn needs_a_model(&self) -> bool {
        matches!(self, ConformanceTier::C4)
    }
}

/// What one vector did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VectorOutcome {
    /// The vector's id, stable enough to cite in a report.
    pub id: String,
    /// Which tier it belongs to.
    pub tier: ConformanceTier,
    /// Whether the observed result matched the complete expectation.
    pub passed: bool,
    /// What was seen — the whole point of a vector is that a failure says what differed.
    pub detail: String,
}

impl VectorOutcome {
    /// A vector that matched its expectation.
    #[must_use]
    pub fn passed(id: impl Into<String>, tier: ConformanceTier) -> Self {
        Self {
            id: id.into(),
            tier,
            passed: true,
            detail: String::new(),
        }
    }

    /// A vector that did not, and what differed.
    #[must_use]
    pub fn failed(id: impl Into<String>, tier: ConformanceTier, detail: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            tier,
            passed: false,
            detail: detail.into(),
        }
    }

    /// A vector that held, carrying a named gap the reader must still see (CT-3).
    ///
    /// `passed` with a non-empty `detail`: not a failure, because the observed state is a known,
    /// recorded fact and reddening the contract over it would teach operators to ignore red —
    /// and not a silent pass, because a version pair that disagrees is exactly the finding Q18
    /// existed for. Consumers render the detail as a warning.
    #[must_use]
    pub fn passed_with_warning(
        id: impl Into<String>,
        tier: ConformanceTier,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            tier,
            passed: true,
            detail: detail.into(),
        }
    }

    /// Whether this outcome is a pass that carries a named gap.
    #[must_use]
    pub fn is_warning(&self) -> bool {
        self.passed && !self.detail.is_empty()
    }
}

/// How one row of the contract's authoring shape is answered.
///
/// Two variants and no third, because the only two honest answers are "these vectors fill it" and
/// "nothing fills it, and here is why". A row left silently empty would make the checklist read
/// green on an adapter that owes the same thing as every other one and does not deliver it — the
/// same reason [`VectorOutcome::passed_with_warning`] exists.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Obligation {
    /// Filled by these vector ids. Every one must appear in that adapter's own conformance run,
    /// in the row's tier, passing.
    Filled(&'static [&'static str]),
    /// Not filled, in the words that say what stands in for it and what a consumer therefore
    /// cannot read off the contract record.
    Gap(&'static str),
}

/// One adapter's contract obligations, in the one shape every adapter fills (CT-4, design
/// `adapter-contract-v0.1.md`).
///
/// The point is that a new adapter's contract is a **checklist rather than a fresh invention**:
/// the struct has no `Default` and no optional field, so an adapter cannot be declared without
/// answering every row — and [`ContractObligations::unmet`] then checks the answers against what
/// that adapter's `conformance_vectors()` really produced, which is what makes the checklist
/// enforced rather than documentation.
///
/// It is deliberately not a framework. It mints no vector, runs nothing and knows no adapter; it
/// is a declaration and a comparison against the outcomes somebody else produced.
#[derive(Debug, Clone, Copy)]
pub struct ContractObligations {
    /// The adapter id — the word the `contract_result` record's `provider` begins with.
    pub adapter: &'static str,
    /// The launch face: what argv and child environment this adapter would construct, against a
    /// recorded expectation (C1).
    pub launch: Obligation,
    /// The recorded transcript or rollout face: the vendor's own record, replayed byte-exact
    /// against a committed event stream (C2).
    pub recorded_wire: Obligation,
    /// The recorded hook-input face: the raw vendor stdin the seam reads, pinned field by field
    /// and checked against the rendering table (C2).
    pub recorded_hook_input: Obligation,
    /// The version pair: the recorded sample's own version claim against the adapter's pin (C2,
    /// CT-3).
    pub version_pair: Obligation,
}

impl ContractObligations {
    /// The rows, in the order a reader checks them: what the row owes, the tier its vectors
    /// belong to, and how this adapter answered it.
    #[must_use]
    pub fn rows(&self) -> [(&'static str, ConformanceTier, Obligation); 4] {
        [
            (
                "at least one launch vector",
                ConformanceTier::C1,
                self.launch,
            ),
            (
                "a recorded transcript/rollout vector",
                ConformanceTier::C2,
                self.recorded_wire,
            ),
            (
                "a recorded hook-input vector",
                ConformanceTier::C2,
                self.recorded_hook_input,
            ),
            (
                "a golden-version-pair vector",
                ConformanceTier::C2,
                self.version_pair,
            ),
        ]
    }

    /// What this declaration promised and the run did not deliver. Empty is the checklist met.
    ///
    /// `vectors` is the adapter's own conformance outcomes and `provider` the string the
    /// `contract_result` record carries, because two of the obligations are about the record
    /// rather than about any one vector: `checked > 0` (a run that checked nothing also has zero
    /// failures) and a `provider` that names the vendor **and** its pin (the field a consumer
    /// reads to know which binary the contract is about).
    #[must_use]
    pub fn unmet(&self, vectors: &[VectorOutcome], provider: &str) -> Vec<String> {
        let mut gaps = Vec::new();
        if vectors.is_empty() {
            gaps.push(
                "checked is 0: a run that checked nothing also has zero failures, so it asserts \
                 nothing"
                    .to_string(),
            );
        }
        for (owed, tier, obligation) in self.rows() {
            match obligation {
                Obligation::Gap(reason) => {
                    if reason.trim().is_empty() {
                        gaps.push(format!("{owed}: declared unfilled without saying why"));
                    }
                }
                Obligation::Filled([]) => {
                    gaps.push(format!("{owed}: declared filled by no vector at all"));
                }
                Obligation::Filled(ids) => {
                    for id in ids {
                        match vectors.iter().find(|vector| vector.id == *id) {
                            None => gaps.push(format!(
                                "{owed}: the declaration names `{id}`, and the run has no such \
                                 vector"
                            )),
                            Some(vector) if vector.tier != tier => gaps.push(format!(
                                "{owed}: `{id}` is declared as {} and the run reports it as {}",
                                tier.as_str(),
                                vector.tier.as_str()
                            )),
                            Some(vector) if !vector.passed => {
                                gaps.push(format!("{owed}: `{id}` failed — {}", vector.detail));
                            }
                            Some(_) => {}
                        }
                    }
                }
            }
        }
        match provider.split_once(' ') {
            Some((named, pin)) if named == self.adapter && !pin.trim().is_empty() => {}
            _ => gaps.push(format!(
                "provider is {provider:?}; the record must carry `{} <pinned version>`, or a \
                 consumer cannot tell which binary the contract is about",
                self.adapter
            )),
        }
        gaps
    }
}
