//! The sealed process-envelope request and its measured result.
//!
//! These values describe confinement without implementing it. A process runner supplied by an
//! embedder receives a sealed request, and returns measurements from the child boundary. Keeping
//! the vocabulary here lets every adapter ask for the same thing without depending on the
//! substrate that implements it (sandbox-inversion design v0.1).

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::Digest;

/// Whether a mounted path is writable by the child.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MountAccess {
    /// The child may read the path but cannot change it.
    ReadOnly,
    /// The child may read and change the path.
    ReadWrite,
}

/// One executable staged into the envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StagedExecutable {
    /// Digest of the staged file's bytes.
    pub digest: Digest,
    /// Absolute path at which the child sees the executable.
    pub mounted_path: String,
}

/// The only credential-shaped channel visible to the child.
///
/// This identifies transport, never secret bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CredentialChannel {
    /// The child receives no credential channel.
    None,
    /// A socket mounted into the child's namespace.
    UnixSocket {
        /// Absolute path at which the child sees the socket.
        mounted_path: String,
    },
    /// A per-run placeholder carried in one named environment variable.
    PlaceholderEnvironment {
        /// The variable's name. Its value is deliberately absent from this value.
        key: String,
    },
}

/// Network reach admitted by an envelope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "reach", rename_all = "snake_case")]
pub enum NetworkPolicy {
    /// No network destination is reachable.
    None,
    /// Exactly one model proxy is reachable.
    ModelProxy {
        /// The proxy endpoint as mounted or addressed inside the envelope.
        endpoint: String,
    },
}

/// Bounds imposed on the child process tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessBounds {
    /// Maximum number of processes in the tree, including the root child.
    pub processes: u32,
    /// Maximum elapsed time in milliseconds.
    pub wall_time_ms: u64,
    /// Maximum combined output bytes retained from the child.
    pub output_bytes: u64,
}

/// A harness-neutral request for one confined child process.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEnvelopeRequest {
    /// Runtime roots the resolved executable needs, mounted read-only.
    pub runtime_roots: Vec<String>,
    /// The one workspace root visible to the child.
    pub workspace_root: String,
    /// Workspace-relative or absolute subtrees the child may change.
    pub writable_subtrees: Vec<String>,
    /// Private state root for this run.
    pub scratch_root: String,
    /// Executables staged into the envelope, with their content digests.
    pub executables: Vec<StagedExecutable>,
    /// Constructed non-secret environment. Credential values do not belong here.
    pub environment: BTreeMap<String, String>,
    /// Reference to the credential channel, never its contents.
    pub credential_channel: CredentialChannel,
    /// Network reach admitted to the child.
    pub network: NetworkPolicy,
    /// Process, time and output bounds.
    pub bounds: ProcessBounds,
}

impl ProcessEnvelopeRequest {
    /// Canonicalise and seal this request before it crosses the process-envelope port.
    #[must_use]
    pub fn seal(mut self) -> SealedProcessEnvelope {
        self.runtime_roots.sort();
        self.runtime_roots.dedup();
        self.writable_subtrees.sort();
        self.writable_subtrees.dedup();
        self.executables
            .sort_by(|left, right| left.mounted_path.cmp(&right.mounted_path));
        let digest = request_digest(&self);
        SealedProcessEnvelope {
            request: self,
            digest,
        }
    }
}

/// A request whose digest fixes every value the envelope provider receives.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SealedProcessEnvelope {
    request: ProcessEnvelopeRequest,
    digest: Digest,
}

impl SealedProcessEnvelope {
    /// The immutable request.
    #[must_use]
    pub fn request(&self) -> &ProcessEnvelopeRequest {
        &self.request
    }

    /// Digest that enters the run record.
    #[must_use]
    pub fn digest(&self) -> &Digest {
        &self.digest
    }

    /// Recompute the digest and reject a deserialised value changed after sealing.
    #[must_use]
    pub fn digest_intact(&self) -> bool {
        self.digest == request_digest(&self.request)
    }
}

/// Facts measured from inside the child boundary.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessEnvelopeMeasurement {
    /// Every path mounted into the child and its effective access.
    pub mounts: BTreeMap<String, MountAccess>,
    /// Every path on which the child has write access.
    pub writable_paths: BTreeSet<String>,
    /// Environment keys visible to the child. Values are deliberately not evidence.
    pub environment_keys: BTreeSet<String>,
    /// Mounted executable path to measured content digest.
    pub executable_digests: BTreeMap<String, Digest>,
    /// Effective network reach.
    pub network: NetworkPolicy,
    /// Effective process, time and output bounds.
    pub bounds: ProcessBounds,
    /// Actual working directory seen by the child.
    pub cwd: String,
}

/// One exact request/result disagreement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnvelopeMismatch {
    /// Stable field name for machines and people.
    pub field: String,
    /// Canonical expected value.
    pub expected: String,
    /// Canonical measured value.
    pub measured: String,
}

/// What the process-envelope port established.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "verdict", rename_all = "snake_case")]
pub enum EnvelopeAssessment {
    /// Every measured field equals the sealed request.
    Matched,
    /// Measurements exist and disagree by these named fields.
    Mismatch {
        /// All disagreements, in stable field order.
        differences: Vec<EnvelopeMismatch>,
    },
    /// The port returned no measurement. Absence is not a pass.
    Withheld,
}

/// Compare a sealed request with optional measured facts.
#[must_use]
pub fn assess_envelope(
    sealed: &SealedProcessEnvelope,
    measurement: Option<&ProcessEnvelopeMeasurement>,
) -> EnvelopeAssessment {
    let Some(measured) = measurement else {
        return EnvelopeAssessment::Withheld;
    };

    let request = sealed.request();
    let mut expected_mounts = BTreeMap::new();
    for root in &request.runtime_roots {
        expected_mounts.insert(root.clone(), MountAccess::ReadOnly);
    }
    expected_mounts.insert(request.workspace_root.clone(), MountAccess::ReadOnly);
    expected_mounts.insert(request.scratch_root.clone(), MountAccess::ReadWrite);
    for path in &request.writable_subtrees {
        expected_mounts.insert(path.clone(), MountAccess::ReadWrite);
    }

    let mut expected_writable = BTreeSet::from([request.scratch_root.clone()]);
    expected_writable.extend(request.writable_subtrees.iter().cloned());
    let expected_environment: BTreeSet<String> = request.environment.keys().cloned().collect();
    let expected_executables: BTreeMap<String, Digest> = request
        .executables
        .iter()
        .map(|executable| (executable.mounted_path.clone(), executable.digest.clone()))
        .collect();

    let mut differences = Vec::new();
    compare(
        &mut differences,
        "mounts",
        &expected_mounts,
        &measured.mounts,
    );
    compare(
        &mut differences,
        "writable_paths",
        &expected_writable,
        &measured.writable_paths,
    );
    compare(
        &mut differences,
        "environment_keys",
        &expected_environment,
        &measured.environment_keys,
    );
    compare(
        &mut differences,
        "executable_digests",
        &expected_executables,
        &measured.executable_digests,
    );
    compare(
        &mut differences,
        "network",
        &request.network,
        &measured.network,
    );
    compare(
        &mut differences,
        "bounds",
        &request.bounds,
        &measured.bounds,
    );
    compare(
        &mut differences,
        "cwd",
        &request.workspace_root,
        &measured.cwd,
    );

    if differences.is_empty() {
        EnvelopeAssessment::Matched
    } else {
        EnvelopeAssessment::Mismatch { differences }
    }
}

fn compare<T: std::fmt::Debug + PartialEq>(
    differences: &mut Vec<EnvelopeMismatch>,
    field: &str,
    expected: &T,
    measured: &T,
) {
    if expected != measured {
        differences.push(EnvelopeMismatch {
            field: field.to_owned(),
            expected: format!("{expected:?}"),
            measured: format!("{measured:?}"),
        });
    }
}

fn request_digest(request: &ProcessEnvelopeRequest) -> Digest {
    // All maps are ordered and `seal` orders every vector, so equal requests have equal bytes.
    Digest::of(&serde_json::to_vec(request).expect("a process-envelope request is serialisable"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> ProcessEnvelopeRequest {
        ProcessEnvelopeRequest {
            runtime_roots: vec!["/runtime/b".into(), "/runtime/a".into()],
            workspace_root: "/workspace".into(),
            writable_subtrees: vec!["/workspace/out".into()],
            scratch_root: "/state".into(),
            executables: vec![StagedExecutable {
                digest: Digest::of(b"tool"),
                mounted_path: "/runtime/bin/tool".into(),
            }],
            environment: BTreeMap::from([("PATH".into(), "/runtime/bin".into())]),
            credential_channel: CredentialChannel::None,
            network: NetworkPolicy::None,
            bounds: ProcessBounds {
                processes: 4,
                wall_time_ms: 30_000,
                output_bytes: 1_000_000,
            },
        }
    }

    fn matching(sealed: &SealedProcessEnvelope) -> ProcessEnvelopeMeasurement {
        let request = sealed.request();
        ProcessEnvelopeMeasurement {
            mounts: BTreeMap::from([
                ("/runtime/a".into(), MountAccess::ReadOnly),
                ("/runtime/b".into(), MountAccess::ReadOnly),
                ("/state".into(), MountAccess::ReadWrite),
                ("/workspace".into(), MountAccess::ReadOnly),
                ("/workspace/out".into(), MountAccess::ReadWrite),
            ]),
            writable_paths: BTreeSet::from(["/state".into(), "/workspace/out".into()]),
            environment_keys: BTreeSet::from(["PATH".into()]),
            executable_digests: BTreeMap::from([(
                "/runtime/bin/tool".into(),
                request.executables[0].digest.clone(),
            )]),
            network: NetworkPolicy::None,
            bounds: request.bounds.clone(),
            cwd: "/workspace".into(),
        }
    }

    #[test]
    fn sealing_is_canonical_and_stable() {
        let left = request().seal();
        let mut reordered = request();
        reordered.runtime_roots.reverse();
        let right = reordered.seal();
        assert_eq!(left.digest(), right.digest());
        assert!(left.digest_intact());
    }

    #[test]
    fn a_matching_measurement_is_exactly_matched() {
        let sealed = request().seal();
        assert_eq!(
            assess_envelope(&sealed, Some(&matching(&sealed))),
            EnvelopeAssessment::Matched
        );
    }

    #[test]
    fn a_wider_write_surface_is_a_named_mismatch() {
        let sealed = request().seal();
        let mut measured = matching(&sealed);
        measured.writable_paths.insert("/workspace".into());
        let EnvelopeAssessment::Mismatch { differences } =
            assess_envelope(&sealed, Some(&measured))
        else {
            panic!("a wider surface must not match");
        };
        assert_eq!(differences.len(), 1);
        assert_eq!(differences[0].field, "writable_paths");
    }

    #[test]
    fn no_measurement_is_withheld_not_matched() {
        let sealed = request().seal();
        assert_eq!(assess_envelope(&sealed, None), EnvelopeAssessment::Withheld);
    }
}
