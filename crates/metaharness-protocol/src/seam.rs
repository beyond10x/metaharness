//! The adapter's two-way half of a run: vendor lines in, control lines out.
//!
//! These two traits live in the protocol crate rather than in the library face, because they are
//! the neutral shape an adapter fills in and the library only consumes. Nothing here names a
//! vendor, and this crate depends on no adapter — which is what keeps an embedder from
//! accidentally depending on which harness is inside.

use crate::command::{Command, Decision};
use crate::event::{DecisionCensus, Emission, Seam, TranscriptRef};
use crate::hermetic::HermeticAttestation;

/// How one vendor's records become events and one embedder's decisions become lines.
pub trait HarnessSeam {
    /// Read one line of the vendor's record.
    ///
    /// A record the adapter cannot map returns `Event::Opaque` and is never dropped (design D4):
    /// the failure that costs the most is a checker reporting "the tool was never called" when
    /// what happened is that it stopped being able to see tool calls.
    fn push_line(&mut self, line: &str) -> Vec<Emission>;

    /// Everything the reader owes once the stream has ended.
    fn finish(&mut self) -> Vec<Emission>;

    /// Hand the terminal record metaharness's own decision census before it is emitted.
    ///
    /// Set rather than computed by the reader, because the census counts what *metaharness*
    /// decided and the vendor's record cannot see it (design D6, finding F10).
    fn set_census(&mut self, census: DecisionCensus);

    /// The line that answers one pending call.
    fn decision_line(&self, call_id: &str, decision: &Decision) -> String;

    /// The line that applies one control, or `None` when this command reaches the child by no
    /// line at all.
    fn control_line(&self, command: &Command) -> Option<String>;
}

/// Builds the seam once the launch plan exists.
///
/// A factory rather than a value, because the seam needs two things only the plan can give it:
/// the retained transcript's reference (design § 8.4 O8) and the attestation block that goes
/// into `session.started` (§ 8.3). A caller that had to construct those itself would be
/// constructing metaharness's own claim about metaharness's own actions.
pub trait SeamFactory {
    /// Supply facts learned only after the launch has been resolved.
    ///
    /// Most vendor records state these themselves and their factories ignore this call. A direct
    /// provider loop writes no provider-side session metadata, so its observer needs the exact
    /// executable version metaharness queried and the cwd metaharness created. The default keeps
    /// existing adapters free of a method whose values they do not consume.
    fn observe_launch(&mut self, _harness_version: Option<String>, _model: String, _cwd: String) {}

    /// Build the seam for this transcript, attestation and control tier.
    fn build(
        &mut self,
        transcript: TranscriptRef,
        attestation: HermeticAttestation,
        seam: Seam,
    ) -> Box<dyn HarnessSeam>;
}
