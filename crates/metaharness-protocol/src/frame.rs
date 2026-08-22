//! The workflow frame: what the embedder says this step is, as a typed value.
//!
//! The frame is presented on every turn, not once per session, because the embedder's per-state
//! tool set changes at every transition (design § 5, from `aep-driver`'s `StepContext`). A frame
//! that could only be set per session would turn a per-step guarantee into a per-session one.

use std::collections::BTreeSet;
use std::fmt;

use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};

/// A hex-encoded SHA-256 digest.
///
/// Used for the frame (§ 5.1) and for the raw transcript (§ 8.4 O8). A digest rather than a copy
/// so an event can cite the exact value in force without repeating it, and so a value mutated
/// after the model saw it is detectable rather than silent.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Digest(String);

impl Digest {
    /// The digest of these bytes.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        Self(format!("{:x}", hasher.finalize()))
    }

    /// The digest as it appears on the wire.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for Digest {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// Which workflow governs this run, at which pinned version.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowRef {
    /// The workflow's id, as the embedder names it.
    pub id: String,
    /// The version pinned for the life of the run (design § 8.1 H10).
    pub version: String,
}

/// One state of a workflow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NodeRef {
    /// The state's id.
    pub id: String,
}

/// Which step of which node, and which attempt at it.
///
/// A step is the *embedder's* unit of work and a turn is the *vendor's*; conflating them is how
/// a per-step guarantee quietly becomes a per-session one (design § 4.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StepRef {
    /// The workflow this step belongs to.
    pub workflow: String,
    /// The state the run is in.
    pub state: String,
    /// The step's index within the run.
    pub index: u32,
    /// Which attempt at this step, counting from 1.
    pub attempt: u32,
}

/// One thing that has already been established, on its own line.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceLine {
    /// The claim, verbatim.
    pub text: String,
    /// Which document established it, so the model is not asked to trust an unattributed line.
    pub source: Option<String>,
}

/// One requirement, carried verbatim and never summarised.
///
/// Verbatim because a driver that summarised here would be the only place the summary existed
/// (design § 5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Line {
    /// The requirement, in the words of the document that asked for it.
    pub text: String,
    /// The document that asked. `None` is legal and is worse; it is not forbidden here because
    /// forbidding it would push embedders into inventing a source.
    pub asked_by: Option<String>,
}

/// What a step owes before it may end.
///
/// A step that owes nothing says so; a step whose handoff is unstated is a step nobody can fail
/// (design § 5.1).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "handoff", rename_all = "snake_case")]
pub enum Handoff {
    /// This step owes nothing, deliberately.
    None,
    /// This step owes a named artifact.
    Artifact {
        /// What the artifact is called.
        name: String,
        /// What kind of artifact it is, in the embedder's own vocabulary.
        kind: Option<String>,
    },
    /// This step owes a structured answer against a schema.
    StructuredAnswer {
        /// The schema the answer is checked against, as the embedder names it.
        schema: String,
    },
}

/// One harness-neutral operation.
///
/// The vocabulary is deliberately small and closed (design D7). The frame names operations; the
/// adapter renders them into vendor tool names and **never re-decides** what an admission
/// implies — a second harness that decided `file.write` admitted a shell would be a weakening
/// the protocol had no way to notice.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Operation {
    /// Read a file.
    #[serde(rename = "file.read")]
    FileRead,
    /// Create or overwrite a file.
    #[serde(rename = "file.write")]
    FileWrite,
    /// Change part of an existing file.
    #[serde(rename = "file.edit")]
    FileEdit,
    /// List a directory.
    #[serde(rename = "dir.list")]
    DirList,
    /// Search the tree.
    #[serde(rename = "search")]
    Search,
    /// Run a shell command.
    #[serde(rename = "shell")]
    Shell,
    /// Fetch something over the web.
    #[serde(rename = "web.read")]
    WebRead,
    /// Load a skill body into the conversation.
    #[serde(rename = "skill.load")]
    SkillLoad,
    /// Spawn a subagent. Not admitted by default on any adapter (design § 5.2): a subagent's
    /// tool set is derived by nothing in these decisions and would be a route around the
    /// per-step admission.
    #[serde(rename = "subagent.spawn")]
    SubagentSpawn,
    /// Maintain a task list.
    #[serde(rename = "task.todo")]
    TaskTodo,
    /// Call one tool on one MCP server.
    #[serde(rename = "mcp.call")]
    McpCall {
        /// The server, as the launch configuration named it.
        server: String,
        /// The tool on that server.
        tool: String,
    },
}

impl Operation {
    /// Every operation in the closed v0.1 vocabulary that takes no parameter.
    ///
    /// [`Operation::McpCall`] is absent because it is parameterised by server and tool, so there
    /// is no finite list of it to publish.
    pub const PARAMETERLESS: [Operation; 10] = [
        Operation::FileRead,
        Operation::FileWrite,
        Operation::FileEdit,
        Operation::DirList,
        Operation::Search,
        Operation::Shell,
        Operation::WebRead,
        Operation::SkillLoad,
        Operation::SubagentSpawn,
        Operation::TaskTodo,
    ];

    /// The key an operation sorts by: its wire name, then the MCP coordinates where there are
    /// any.
    ///
    /// This — not variant order — is [`Operation`]'s `Ord`, and the choice is a wire fact
    /// (§ 5.5): an [`OperationSet`] serializes in this order, the sealed digest is over that
    /// serialization, and *"operations sorted by their `op` name"* is a rule an external
    /// producer can follow without reading this enum. A derived variant order would make the
    /// canonical form depend on this file.
    fn sort_key(&self) -> (&'static str, &str, &str) {
        match self {
            Operation::McpCall { server, tool } => ("mcp.call", server.as_str(), tool.as_str()),
            other => (other.name(), "", ""),
        }
    }

    /// The operation's wire name.
    #[must_use]
    pub fn name(&self) -> &'static str {
        match self {
            Operation::FileRead => "file.read",
            Operation::FileWrite => "file.write",
            Operation::FileEdit => "file.edit",
            Operation::DirList => "dir.list",
            Operation::Search => "search",
            Operation::Shell => "shell",
            Operation::WebRead => "web.read",
            Operation::SkillLoad => "skill.load",
            Operation::SubagentSpawn => "subagent.spawn",
            Operation::TaskTodo => "task.todo",
            Operation::McpCall { .. } => "mcp.call",
        }
    }
}

impl Ord for Operation {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

impl PartialOrd for Operation {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Strictly the operations a step admits.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OperationSet(BTreeSet<Operation>);

impl OperationSet {
    /// A step that admits nothing — the classify step of the routing example (design § 10.4),
    /// whose only job is a judgement and which therefore holds no tool.
    #[must_use]
    pub fn empty() -> Self {
        Self(BTreeSet::new())
    }

    /// The set of exactly these operations.
    #[must_use]
    pub fn of<I: IntoIterator<Item = Operation>>(operations: I) -> Self {
        Self(operations.into_iter().collect())
    }

    /// Whether this operation is admitted here.
    #[must_use]
    pub fn admits(&self, operation: &Operation) -> bool {
        self.0.contains(operation)
    }

    /// The operations, in a stable order.
    pub fn iter(&self) -> impl Iterator<Item = &Operation> {
        self.0.iter()
    }

    /// How many operations are admitted.
    #[must_use]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether nothing is admitted.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<'a> IntoIterator for &'a OperationSet {
    type Item = &'a Operation;
    type IntoIter = std::collections::btree_set::Iter<'a, Operation>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

/// The enumerated set a routing step chooses from.
///
/// Data in the frame rather than something the model remembers, because an entity list the model
/// recalled is an entity list the model can hallucinate having read (design § 10.4).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EntityList {
    /// Where the list came from, so a stale list is attributable.
    pub source: String,
    /// The members, verbatim.
    pub members: Vec<String>,
}

/// What the embedder says this step is.
///
/// The field set is a superset of the embedder's own step context (design § 2.3): a field
/// metaharness omitted would be information the embedder lost by adopting it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Frame {
    /// Which workflow, at which pinned version.
    pub workflow: WorkflowRef,
    /// The state the run is in.
    pub node: NodeRef,
    /// Which step of that node, and which attempt.
    pub step: StepRef,
    /// What has already been established, one line each.
    pub prior: Vec<EvidenceLine>,
    /// What must hold while here.
    pub obligations: Vec<Line>,
    /// What does not hold yet on a way out, prefixed with where it goes.
    ///
    /// Present because of a recorded failure: a run in which the model was never told what the
    /// next state wanted, wrote neither, and was refused for work already paid for (§ 5.1).
    pub reaching: Vec<Line>,
    /// The nodes reachable from here.
    pub next: Vec<NodeRef>,
    /// What this step must produce before it may end.
    pub handoff: Handoff,
    /// Strictly the operations admitted here.
    pub operations: OperationSet,
    /// The enumerated set a routing step chooses from, when there is one.
    pub entities: Option<EntityList>,
    /// SHA-256 over the canonical form of everything above. Set by [`Frame::seal`].
    pub digest: Digest,
}

impl Frame {
    /// The digest this frame's contents imply, ignoring whatever [`Frame::digest`] currently
    /// holds.
    ///
    /// # Panics
    ///
    /// If the frame does not serialize, which cannot happen for the types above and would be a
    /// defect in this crate rather than in a caller's input.
    #[must_use]
    pub fn computed_digest(&self) -> Digest {
        let mut value = serde_json::to_value(self).expect("a Frame serializes");
        if let Some(object) = value.as_object_mut() {
            object.remove("digest");
        }
        // `serde_json::Value`'s map is ordered, so this byte form is canonical for a given
        // content — two processes that build the same frame agree on its digest.
        let bytes = serde_json::to_vec(&value).expect("a serde_json::Value serializes");
        Digest::of(&bytes)
    }

    /// This frame with its digest set to what its contents imply.
    #[must_use]
    pub fn seal(mut self) -> Self {
        self.digest = self.computed_digest();
        self
    }

    /// The frame as the instruction text the model is shown.
    ///
    /// This function lives in the protocol crate and is shared by every adapter, so two
    /// harnesses cannot describe the same frame differently (design § 5.3). It is one of the two
    /// independent ways the frame reaches the model: the text says what the step is, and the
    /// control seam is what makes it true. Text alone is not enforcement, and a run that can
    /// only inject text is refused rather than served an advisory frame (§ 6, finding F9).
    ///
    /// Obligations and reaching requirements are rendered **verbatim, one line each, naming the
    /// document that asked** — never summarised, because a summary here would be the only place
    /// that summary existed.
    #[must_use]
    pub fn render_instruction(&self) -> String {
        let mut lines: Vec<String> = vec![format!(
            "Workflow {} ({}), state {}, step {} attempt {}.",
            self.workflow.id,
            self.workflow.version,
            self.node.id,
            self.step.index,
            self.step.attempt
        )];

        lines.push(String::new());
        lines.push("Already established:".to_string());
        if self.prior.is_empty() {
            lines.push("- nothing yet.".to_string());
        }
        for line in &self.prior {
            lines.push(match &line.source {
                Some(source) => format!("- {} ({source})", line.text),
                None => format!("- {}", line.text),
            });
        }

        lines.push(String::new());
        lines.push("Must hold here:".to_string());
        if self.obligations.is_empty() {
            lines.push("- nothing is required of this step beyond its handoff.".to_string());
        }
        lines.extend(self.obligations.iter().map(render_line));

        if !self.reaching.is_empty() {
            // Present because of a recorded failure: a run in which the model was never told
            // what the next state wanted wrote neither of the two things it wanted.
            lines.push(String::new());
            lines.push("Not yet satisfied on a way out:".to_string());
            lines.extend(self.reaching.iter().map(render_line));
        }

        if !self.next.is_empty() {
            let names: Vec<&str> = self.next.iter().map(|node| node.id.as_str()).collect();
            lines.push(String::new());
            lines.push(format!("Reachable from here: {}.", names.join(", ")));
        }

        lines.push(String::new());
        lines.push(format!("This step owes: {}", render_handoff(&self.handoff)));

        if let Some(entities) = &self.entities {
            lines.push(String::new());
            lines.push(format!(
                "Choose exactly one of these, enumerated from {}:",
                entities.source
            ));
            lines.extend(entities.members.iter().map(|member| format!("- {member}")));
        }

        lines.push(String::new());
        lines.push("Strictly only these operations are admitted here:".to_string());
        if self.operations.is_empty() {
            // Said out loud rather than left as an empty list, because a step whose only job is
            // a judgement holds no tool and the model must be told that, not left to infer it.
            lines.push("- none. This step calls no tools.".to_string());
        }
        lines.extend(self.operations.iter().map(|operation| match operation {
            Operation::McpCall { server, tool } => format!("- mcp.call {server}/{tool}"),
            other => format!("- {}", other.name()),
        }));

        lines.push(String::new());
        lines.push(format!("Frame {}.", self.digest));
        lines.join("\n")
    }

    /// Whether the digest still describes the contents.
    ///
    /// A frame mutated after the model saw it fails this, which is the point of carrying the
    /// digest at all (design § 5.1).
    #[must_use]
    pub fn digest_intact(&self) -> bool {
        self.digest == self.computed_digest()
    }

    /// The frame as its on-disk document: one JSON object tagged `metaharness.frame/1`.
    ///
    /// Written exactly as the value holds it, digest included — deliberately not resealed on the
    /// way out, because a producer that mutated a sealed frame should be refused by every
    /// consumer rather than silently repaired by this function. Seal before writing.
    ///
    /// # Panics
    ///
    /// If the frame does not serialize, which cannot happen for these types and would be a
    /// defect in this crate rather than in a caller's input.
    #[must_use]
    pub fn to_document(&self) -> String {
        let mut value = serde_json::to_value(self).expect("a Frame serializes");
        if let Some(object) = value.as_object_mut() {
            object.insert(
                "format".to_string(),
                serde_json::Value::String(FRAME_FORMAT.to_string()),
            );
        }
        let mut text =
            serde_json::to_string_pretty(&value).expect("a serde_json::Value serializes");
        text.push('\n');
        text
    }

    /// A frame read back from its on-disk document, digest verified.
    ///
    /// The format tag is checked first (the D2 rule: a document is self-describing or it is
    /// refused), then the shape, then the digest — a document whose digest does not describe its
    /// contents is refused, because an unsealed or edited frame cited by digest downstream would
    /// pin nothing.
    ///
    /// # Errors
    ///
    /// [`FrameDocError`], one variant per way the document fails, each naming what was found.
    pub fn parse_document(text: &str) -> Result<Self, FrameDocError> {
        let mut value: serde_json::Value =
            serde_json::from_str(text).map_err(|error| FrameDocError::NotJson {
                detail: error.to_string(),
            })?;
        let Some(object) = value.as_object_mut() else {
            return Err(FrameDocError::NotJson {
                detail: "the document is JSON but not an object".to_string(),
            });
        };
        match object.remove("format") {
            Some(serde_json::Value::String(tag)) if tag == FRAME_FORMAT => {}
            Some(tag) => {
                return Err(FrameDocError::UnknownFormat {
                    tag: Some(tag.as_str().unwrap_or("<not a string>").to_string()),
                });
            }
            None => return Err(FrameDocError::UnknownFormat { tag: None }),
        }
        let frame: Frame =
            serde_json::from_value(value).map_err(|error| FrameDocError::Invalid {
                detail: error.to_string(),
            })?;
        if !frame.digest_intact() {
            return Err(FrameDocError::DigestMismatch {
                stated: frame.digest.clone(),
                computed: frame.computed_digest(),
            });
        }
        Ok(frame)
    }
}

/// The format tag the on-disk frame document carries.
///
/// On the document itself rather than on a file extension or a handshake, on the same rule as
/// the wire's per-line tags (design D2): a copied or truncated file is still self-describing.
pub const FRAME_FORMAT: &str = "metaharness.frame/1";

/// Every way an on-disk frame document fails to become a [`Frame`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameDocError {
    /// The file is not a JSON object.
    NotJson {
        /// What the parser said.
        detail: String,
    },
    /// The `format` field is missing or names a format this build does not know.
    UnknownFormat {
        /// The tag that was read, or `None` when the field was missing.
        tag: Option<String>,
    },
    /// The object does not have a frame's shape.
    Invalid {
        /// What deserialization said.
        detail: String,
    },
    /// The stated digest does not describe the contents.
    ///
    /// Refused rather than resealed: a frame whose digest lies was either never sealed or was
    /// edited after sealing, and both mean the document cannot be cited by digest.
    DigestMismatch {
        /// The digest the document states.
        stated: Digest,
        /// The digest its contents imply.
        computed: Digest,
    },
}

impl fmt::Display for FrameDocError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FrameDocError::NotJson { detail } => {
                write!(f, "not a JSON object: {detail}")
            }
            FrameDocError::UnknownFormat { tag: Some(tag) } => {
                write!(f, "unknown format tag {tag:?}, expected {FRAME_FORMAT:?}")
            }
            FrameDocError::UnknownFormat { tag: None } => {
                write!(f, "no format tag, expected {FRAME_FORMAT:?}")
            }
            FrameDocError::Invalid { detail } => {
                write!(f, "not a frame: {detail}")
            }
            FrameDocError::DigestMismatch { stated, computed } => write!(
                f,
                "the stated digest {stated} does not describe the contents (computed {computed}); \
                 the frame was never sealed or was edited after sealing"
            ),
        }
    }
}

impl std::error::Error for FrameDocError {}

/// One requirement line, with the document that asked for it.
fn render_line(line: &Line) -> String {
    match &line.asked_by {
        Some(source) => format!("- {} ({source})", line.text),
        None => format!("- {}", line.text),
    }
}

/// What a step owes, in one clause.
fn render_handoff(handoff: &Handoff) -> String {
    match handoff {
        Handoff::None => "nothing.".to_string(),
        Handoff::Artifact { name, kind } => match kind {
            Some(kind) => format!("the artifact {name} (kind {kind})."),
            None => format!("the artifact {name}."),
        },
        Handoff::StructuredAnswer { schema } => {
            format!("a structured answer against the schema {schema}.")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame() -> Frame {
        Frame {
            workflow: WorkflowRef {
                id: "development/default".into(),
                version: "1".into(),
            },
            node: NodeRef {
                id: "implement".into(),
            },
            step: StepRef {
                workflow: "development/default".into(),
                state: "implement".into(),
                index: 3,
                attempt: 1,
            },
            prior: vec![EvidenceLine {
                text: "the specification is approved".into(),
                source: Some("docs/specs/one.md".into()),
            }],
            obligations: vec![Line {
                text: "the suite is red before the implementation".into(),
                asked_by: Some("workflows/development/default.yaml".into()),
            }],
            reaching: vec![Line {
                text: "to verify: the suite is green".into(),
                asked_by: Some("workflows/development/default.yaml".into()),
            }],
            next: vec![NodeRef {
                id: "verify".into(),
            }],
            handoff: Handoff::Artifact {
                name: "the implementation".into(),
                kind: Some("change".into()),
            },
            operations: OperationSet::of([Operation::FileEdit, Operation::Shell]),
            entities: None,
            digest: Digest::of(b""),
        }
    }

    #[test]
    fn sealing_a_frame_makes_its_digest_describe_its_contents() {
        let sealed = frame().seal();
        assert!(sealed.digest_intact());
    }

    /// The digest exists to catch a frame mutated after the model saw it, so a mutation must
    /// break it. A digest that survived an edit would be decoration.
    #[test]
    fn mutating_a_sealed_frame_breaks_its_digest() {
        let mut sealed = frame().seal();
        sealed.operations =
            OperationSet::of([Operation::FileEdit, Operation::Shell, Operation::FileWrite]);
        assert!(!sealed.digest_intact());
    }

    /// Two processes that build the same frame must agree on its digest, or the digest cannot be
    /// cited across a process boundary — which is the only place it is ever cited.
    #[test]
    fn the_digest_is_stable_across_two_equal_frames() {
        assert_eq!(frame().seal().digest, frame().seal().digest);
    }

    #[test]
    fn a_frame_round_trips_through_json() {
        let sealed = frame().seal();
        let line = serde_json::to_string(&sealed).expect("serializes");
        let back: Frame = serde_json::from_str(&line).expect("parses");
        assert_eq!(back, sealed);
        assert!(back.digest_intact());
    }

    #[test]
    fn operations_carry_the_wire_names_the_design_decided() {
        let rendered: Vec<&str> = Operation::PARAMETERLESS
            .iter()
            .map(Operation::name)
            .collect();
        assert_eq!(
            rendered,
            [
                "file.read",
                "file.write",
                "file.edit",
                "dir.list",
                "search",
                "shell",
                "web.read",
                "skill.load",
                "subagent.spawn",
                "task.todo",
            ]
        );
        let mcp = Operation::McpCall {
            server: "s".into(),
            tool: "t".into(),
        };
        let line = serde_json::to_string(&mcp).expect("serializes");
        assert_eq!(line, r#"{"op":"mcp.call","server":"s","tool":"t"}"#);
    }

    #[test]
    fn a_classify_step_admits_nothing() {
        let empty = OperationSet::empty();
        assert!(empty.is_empty());
        assert!(!empty.admits(&Operation::Shell));
    }

    /// Obligations are carried verbatim and name the document that asked, because a driver that
    /// summarised here would be the only place the summary existed.
    #[test]
    fn the_rendered_frame_carries_every_obligation_verbatim_with_its_source() {
        let text = frame().seal().render_instruction();
        assert!(
            text.contains(
                "- the suite is red before the implementation (workflows/development/default.yaml)"
            ),
            "{text}"
        );
        assert!(text.contains("- to verify: the suite is green"), "{text}");
    }

    /// A step that admits nothing says so. An empty list would leave the model to infer the
    /// strictest thing the frame says, which is exactly what it must not have to do.
    #[test]
    fn a_step_that_admits_nothing_says_so_in_words() {
        let mut classify = frame();
        classify.operations = OperationSet::empty();
        let text = classify.seal().render_instruction();
        assert!(text.contains("- none. This step calls no tools."), "{text}");
    }

    /// One function shared by every adapter, so two harnesses cannot describe the same frame
    /// differently (design § 5.3).
    #[test]
    fn two_equal_frames_render_the_same_text() {
        assert_eq!(
            frame().seal().render_instruction(),
            frame().seal().render_instruction()
        );
    }

    /// The rendered text cites the frame it came from, so a frame mutated after the model saw it
    /// is detectable in the transcript and not only in the event stream.
    #[test]
    fn the_rendered_frame_cites_its_own_digest() {
        let sealed = frame().seal();
        assert!(
            sealed.render_instruction().contains(sealed.digest.as_str()),
            "the rendered frame names its digest"
        );
    }

    /// The set's iteration order is the document's canonical order, and it is the wire-name
    /// order — a rule an external producer can follow — not the enum's variant order, which
    /// nobody outside this file can see. The first cross-repository document failed on exactly
    /// this.
    #[test]
    fn operations_iterate_in_wire_name_order_not_variant_order() {
        let set = OperationSet::of([
            Operation::Shell,
            Operation::FileRead,
            Operation::DirList,
            Operation::McpCall {
                server: "a".into(),
                tool: "b".into(),
            },
        ]);
        let names: Vec<String> = set
            .iter()
            .map(|operation| operation.name().to_string())
            .collect();
        assert_eq!(names, ["dir.list", "file.read", "mcp.call", "shell"]);
    }

    // ------------------------------------------------------------ the on-disk document

    #[test]
    fn a_sealed_frame_round_trips_through_its_document() {
        let sealed = frame().seal();
        let text = sealed.to_document();
        assert!(text.contains(FRAME_FORMAT), "the document names its format");
        let back = Frame::parse_document(&text).expect("a sealed document parses");
        assert_eq!(back, sealed);
    }

    /// Written as the value holds it, refused on the way back in: an unsealed frame must not
    /// survive the file boundary, or the digest downstream events cite pins nothing.
    #[test]
    fn an_unsealed_frame_writes_a_document_every_consumer_refuses() {
        let error = Frame::parse_document(&frame().to_document())
            .expect_err("an unsealed document is refused");
        assert!(matches!(error, FrameDocError::DigestMismatch { .. }));
        assert!(error.to_string().contains("never sealed"), "{error}");
    }

    #[test]
    fn a_document_edited_after_sealing_is_refused_naming_both_digests() {
        let sealed = frame().seal();
        let edited = sealed
            .to_document()
            .replace("the suite is red", "the suite is whatever");
        let error = Frame::parse_document(&edited).expect_err("an edited document is refused");
        let FrameDocError::DigestMismatch { stated, computed } = &error else {
            panic!("expected DigestMismatch, got {error:?}");
        };
        assert_eq!(stated, &sealed.digest);
        assert_ne!(stated, computed);
    }

    /// The D2 rule applied to a file: a document that does not say what it is, or says it is
    /// something this build does not know, is refused rather than guessed at.
    #[test]
    fn a_document_without_the_format_tag_is_refused() {
        let mut value = serde_json::to_value(frame().seal()).expect("serializes");
        let untagged = serde_json::to_string(&value).expect("serializes");
        assert!(matches!(
            Frame::parse_document(&untagged),
            Err(FrameDocError::UnknownFormat { tag: None })
        ));

        value
            .as_object_mut()
            .expect("an object")
            .insert("format".into(), "metaharness.frame/2".into());
        let future = serde_json::to_string(&value).expect("serializes");
        assert!(matches!(
            Frame::parse_document(&future),
            Err(FrameDocError::UnknownFormat { tag: Some(tag) }) if tag == "metaharness.frame/2"
        ));
    }

    #[test]
    fn a_document_that_is_not_a_frame_is_refused_as_such() {
        assert!(matches!(
            Frame::parse_document("not json at all"),
            Err(FrameDocError::NotJson { .. })
        ));
        assert!(matches!(
            Frame::parse_document(&format!(r#"{{"format":"{FRAME_FORMAT}","workflow":3}}"#)),
            Err(FrameDocError::Invalid { .. })
        ));
    }
}
