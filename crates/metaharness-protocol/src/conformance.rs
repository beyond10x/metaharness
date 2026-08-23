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
