//! Every way metaharness says "I could not do my job", as one type.
//!
//! One enum rather than one per call site, because they all mean the same thing to a caller:
//! **exit `2`**. `2` is not a verdict about the run — it is metaharness reporting that no verdict
//! was possible, and a caller that treated it as a red run would be reading a setup failure as
//! evidence (design § 9.4).
//!
//! Every variant names the missing thing. A refusal that only says "unsupported" makes the
//! reader open the source to find out what to install.

use std::fmt;
use std::path::PathBuf;

use metaharness_protocol::{CommandOutcome, Emission, Event, Kind, RefusalCode, Refused};

/// Why metaharness could not do its job. Always exit `2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// This kind has no adapter in this build.
    NoAdapter {
        /// The kind, by the name the CLI spells.
        kind: String,
    },
    /// `--frame <file>` was given and the file could not be read.
    ///
    /// The library resolves the path, never the CLI — parsing a frame document in the binary
    /// would be protocol logic in the CLI, and D11 exists to forbid that (design § 9.3,
    /// correction 3; the format itself landed as amendment a5).
    FrameUnreadable {
        /// The path that was asked for.
        path: PathBuf,
        /// What the filesystem said.
        detail: String,
    },
    /// `--frame <file>` was read and is not a well-formed, sealed `metaharness.frame/1` document.
    ///
    /// Covers a missing or unknown format tag, a shape that is not a frame, and a digest that
    /// does not describe the contents — each named in the detail, verbatim from the parser.
    FrameInvalid {
        /// The document that was refused.
        path: PathBuf,
        /// What [`metaharness_protocol::FrameDocError`] said.
        detail: String,
    },
    /// Both an in-memory frame and `--frame <file>` were given.
    ///
    /// Refused by name rather than resolved by precedence: whichever one silently won, the other
    /// would be a frame the embedder believed was in force and was not.
    FrameConflict {
        /// The file that competed with the in-memory frame.
        path: PathBuf,
    },
    /// `--decisions observe` was given together with a frame.
    ///
    /// Refused rather than composed, on finding F9's rule: a frame whose text reaches the model
    /// while nothing enforces it tells the model *"strictly only these operations"* and makes it
    /// false. Observe mode enforces nothing by construction, so the two together are a frame that
    /// is advertised and not applied — which is worse than either alone.
    ObserveWithFrame,
    /// `--tool-surface owned` was given for a kind that has no vendor surface to replace.
    ///
    /// Strategy C is **built** now: `metaharness-tools` implements read, write, edit, list, search
    /// and a bounded `run`, and `mcp-serve` publishes them as three verbs. What it needs from the
    /// vendor is a way to remove the built-in tools and add ours, and only Claude Code has one
    /// (`--tools ""` plus `--mcp-config`).
    ///
    /// Q1 — per-step tool re-listing — never fires: its own recorded resolution is that strategy C
    /// is per-session, and an evaluation arm with one fixed toolset for a whole run never re-lists.
    ToolSurfaceOwned {
        /// The kind that has no such surface.
        kind: Kind,
    },
    /// `--prices <card>` was given for a kind that prices its own runs.
    ///
    /// Claude Code and codex read rates from a catalogue their service delivers and report the
    /// figure themselves; metaharness passes that figure through and computes nothing (design
    /// § 4.1, D4). A card handed to one of them could only be ignored — and an operator who
    /// supplied one would believe the run was priced at the rates they declared when it was
    /// priced at the vendor's.
    PricesUnsupported {
        /// The kind that prices itself.
        kind: Kind,
    },
    /// `--substrate`, `--substrate-embedded` or `--cgroup-root` was given for a kind whose tools
    /// are not ours to confine.
    ///
    /// substrate stands behind the catalogue the b10x loop publishes. Claude Code and codex reach
    /// the filesystem through their own tools, so a socket configured for one of them would be
    /// accepted, never consulted, and would read as containment nobody applied.
    ConfinementUnsupported {
        /// The kind that brings its own tools.
        kind: Kind,
    },
    /// `--write-scope` or `--context` was given for a kind that carries them another way.
    ///
    /// Not because a vendor arm should be unbounded. A scope reaches it as `Frame.subjects`, sealed
    /// into the frame's digest and adjudicated at its hook seam; a flag here would be a second,
    /// unsealed copy of the same rule, and the two would disagree the first time one moved.
    ScopeUnsupported {
        /// The kind whose scope travels in the frame.
        kind: Kind,
    },
    /// The adapter refused to plan the launch.
    Launch {
        /// What the adapter said, verbatim.
        detail: String,
    },
    /// The adapter cannot honour a command this run's configuration will need.
    ///
    /// Raised at run start rather than at the call, so a run that will fail on control fails
    /// before it spends money (design § 6.1).
    Control {
        /// One entry per refused command: its wire name and the refusal.
        refusals: Vec<(String, Refused)>,
    },
    /// A verb that exists on the surface and is not built in this milestone.
    NotInThisMilestone {
        /// The verb.
        verb: &'static str,
        /// What it needs that does not exist yet.
        missing: &'static str,
    },
    /// `--spec` was given and `--auditor` was not.
    ///
    /// A refusal and not a skip: a specification nobody checked reads exactly like a
    /// specification that passed (design § 9.4).
    SpecWithoutAuditor,
    /// `--auditor` was given and `--spec` was not, so there is nothing to check against.
    AuditorWithoutSpec,
    /// The expectation document could not be read.
    SpecUnreadable {
        /// Which document.
        path: PathBuf,
        /// What the filesystem said.
        detail: String,
    },
    /// The auditor could not be invoked.
    AuditorNotInvokable {
        /// The argv metaharness constructed.
        argv: Vec<String>,
        /// What the platform said.
        detail: String,
    },
    /// The auditor ran and produced no verdict rows.
    ///
    /// A setup failure, never a contradiction: a verdict table with nothing in it would
    /// otherwise go green — or red — while checking nothing (design § 9.4, finding F2).
    NoVerdictRows {
        /// The argv that produced nothing.
        argv: Vec<String>,
    },
    /// A line of an event stream `project` was pointed at could not be read.
    ///
    /// **By name, and never by producing a shorter document.** A stream that lost a line silently
    /// would make a reader report *the tool was never called* when what happened is that the
    /// reader stopped being able to see tool calls — design D4's failure, one level up.
    ProjectionUnreadable {
        /// The stream.
        path: PathBuf,
        /// Which 1-based line, or `0` when the whole file was unreadable.
        line: usize,
        /// What the framing reader said, verbatim.
        detail: String,
    },
    /// The viewer was given something other than one or two runs.
    ///
    /// Two columns is the shape `docs/design/runs-side-by-side-v0.1.md` § 2 decided. A third
    /// would have to be aligned against something nobody decided, and silently dropping it would
    /// leave a reader comparing two runs believing they had asked about three.
    ViewerColumnCount {
        /// How many were given.
        given: usize,
    },
    /// A declared `--plugin` could not be resolved against this machine (amendment a16).
    ///
    /// The detail is the adapter's own sentence, which names the `claude plugin marketplace add`
    /// and `claude plugin install` to run **once, deliberately, outside a run**. That is where the
    /// network reach belongs: a launch that fetched would be unpinnable at 2.1.258 and would reach
    /// out from inside the boundary the hermetic floor exists to draw.
    MarketplacePlugin {
        /// What the adapter said, verbatim.
        detail: String,
    },
    /// `--plugin` was given for a kind that has no marketplace.
    ///
    /// Refused by name rather than accepted and ignored, on the same rule `--prices` carries: an
    /// operator who declared a plugin would otherwise believe the run had one.
    MarketplacePluginUnsupported {
        /// The kind with no marketplace.
        kind: Kind,
    },
    /// Something the platform refused.
    Io {
        /// What it said.
        detail: String,
    },
}

impl Refusal {
    /// The `command.result` events a run-start control refusal owes.
    ///
    /// The design says these are **emitted** at run start, and a refusal that only lived in a
    /// return value would be invisible to a caller reading the event stream.
    #[must_use]
    pub fn emissions(&self) -> Vec<Emission> {
        match self {
            Refusal::Control { refusals } => refusals
                .iter()
                .map(|(name, refused)| {
                    Emission::untimed(Event::CommandResult {
                        id: format!("start/{name}"),
                        outcome: CommandOutcome::Refused {
                            refused: refused.clone(),
                        },
                    })
                })
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The refusal code where one applies.
    #[must_use]
    pub fn code(&self) -> Option<RefusalCode> {
        match self {
            Refusal::Control { refusals } => refusals.first().map(|(_, refused)| refused.code),
            Refusal::NoAdapter { .. }
            | Refusal::ToolSurfaceOwned { .. }
            | Refusal::PricesUnsupported { .. }
            | Refusal::ConfinementUnsupported { .. }
            | Refusal::ScopeUnsupported { .. }
            | Refusal::ObserveWithFrame => Some(RefusalCode::UnsupportedControl),
            _ => None,
        }
    }
}

impl fmt::Display for Refusal {
    #[allow(clippy::too_many_lines)]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // **What this used to say stopped being true.** It read "the adapter crate is a
            // later milestone", which was the only way to reach it when it was written. It is now
            // also what `conformance_vectors` raises for an adapter that exists and has no vector
            // suite yet — `b10x`, whose crate drives the driven eval — so the message asserted a
            // missing crate about a crate that is there, and a reader chasing it looked for the
            // wrong thing. It now names what is absent rather than guessing why.
            Refusal::NoAdapter { kind } => write!(
                f,
                "nothing to run for {kind} in this build: either this build has no {kind} adapter, \
                 or that adapter has no free conformance vectors yet — `contract` reports its \
                 obligations row by row. Refused by name rather than reported as zero, which would \
                 read exactly like a pass"
            ),
            Refusal::FrameUnreadable { path, detail } => write!(
                f,
                "the frame document {} could not be read: {detail}",
                path.display()
            ),
            Refusal::FrameInvalid { path, detail } => write!(
                f,
                "the frame document {} is not a sealed metaharness.frame/1 document: {detail}",
                path.display()
            ),
            Refusal::FrameConflict { path } => write!(
                f,
                "both an in-memory frame and --frame {} were given, and whichever silently won, \
                 the other would be a frame the embedder believed was in force. Give exactly one",
                path.display()
            ),
            Refusal::ObserveWithFrame => f.write_str(
                "--decisions observe was given together with a frame: observe mode allows every \
                 call, so the frame's text would reach the model as \"strictly only these \
                 operations\" while nothing enforced it (finding F9). Observe a run with no frame, \
                 or enforce the frame with --decisions frame",
            ),
            Refusal::ToolSurfaceOwned { kind } => write!(
                f,
                "--tool-surface owned is refused for {}: metaharness serves the tools (see \
                 `metaharness mcp-serve`), but it has no way to take {}'s own tools away and add \
                 ours. Claude Code has one — `--tools \"\"` plus `--mcp-config` — and codex \
                 exec does not: dynamicTools is an app-server surface it does not expose. b10x \
                 already publishes exactly this catalogue in-process, so there is nothing to \
                 replace. Use --tool-surface native",
                kind.as_str(),
                kind.as_str(),
            ),
            Refusal::PricesUnsupported { kind } => write!(
                f,
                "--prices is refused for {}: it prices its own runs from a catalogue its service \
                 delivers, and metaharness reports that figure rather than multiplying one out \
                 (design § 4.1, D4). A card here could only be ignored, leaving the run priced at \
                 the vendor's rates while the operator believed it was priced at theirs. The b10x \
                 loop takes one, because nothing behind it returns a price at all",
                kind.as_str(),
            ),
            Refusal::ConfinementUnsupported { kind } => write!(
                f,
                "--substrate, --substrate-embedded and --cgroup-root are refused for {}: substrate \
                 confines the catalogue *we* publish, and {} reaches the filesystem through its \
                 own tools. A socket here would be configured and never consulted, which reads as \
                 containment nobody applied. Only b10x takes one",
                kind.as_str(),
                kind.as_str(),
            ),
            Refusal::ScopeUnsupported { kind } => write!(
                f,
                "--write-scope and --context are refused for {}: a scope reaches it as \
                 Frame.subjects, sealed into the frame's digest and adjudicated at its hook seam. A \
                 flag here would be a second copy of the same rule that nothing seals, and the two \
                 would disagree the first time one moved. The b10x loop takes them because it has \
                 no seam: its published toolset is its policy, so the scope travels to the tools",
                kind.as_str(),
            ),
            Refusal::Launch { detail } => {
                write!(f, "the adapter refused to plan the launch: {detail}")
            }
            Refusal::Control { refusals } => {
                f.write_str("the adapter cannot honour a command this run will need:")?;
                for (name, refused) in refusals {
                    write!(f, " {name} ({}: {})", refused.code.as_str(), refused.reason)?;
                }
                Ok(())
            }
            Refusal::NotInThisMilestone { verb, missing } => write!(
                f,
                "{verb} is not built in this milestone: it needs {missing}. It is declared here \
                 so the verb surface does not change when it arrives"
            ),
            Refusal::SpecWithoutAuditor => f.write_str(
                "--spec was given without --auditor: metaharness embeds no expectation language, \
                 and a specification nobody checked reads exactly like a specification that \
                 passed. Name the auditor with --auditor '<argv prefix>'",
            ),
            Refusal::AuditorWithoutSpec => f.write_str(
                "--auditor was given without --spec: the invocation is \
                 '<prefix…> --spec <spec> --transcript <path>', so there is nothing to check \
                 against",
            ),
            Refusal::SpecUnreadable { path, detail } => write!(
                f,
                "the expectation document {} could not be read: {detail}",
                path.display()
            ),
            Refusal::AuditorNotInvokable { argv, detail } => {
                write!(f, "the auditor could not be invoked ({argv:?}): {detail}")
            }
            Refusal::NoVerdictRows { argv } => write!(
                f,
                "the auditor ({argv:?}) produced no verdict rows: a table with nothing in it is a \
                 setup failure, never a verdict"
            ),
            Refusal::ProjectionUnreadable { path, line, detail } => {
                if *line == 0 {
                    write!(
                        f,
                        "the event stream {} could not be read: {detail}",
                        path.display()
                    )
                } else {
                    write!(
                        f,
                        "line {line} of the event stream {} is not a metaharness.event/1 event \
                         this build knows: {detail}. It is refused rather than skipped, because a \
                         document one event shorter than its stream reads exactly like a run that \
                         did one thing less",
                        path.display()
                    )
                }
            }
            Refusal::ViewerColumnCount { given } => write!(
                f,
                "the viewer renders one or two runs and was given {given}: two columns is the \
                 decided shape, and a third would have to be aligned against a rule nothing \
                 decides. Give one or two event streams"
            ),
            Refusal::MarketplacePlugin { detail } => write!(f, "{detail}"),
            Refusal::MarketplacePluginUnsupported { kind } => write!(
                f,
                "--plugin names a marketplace plugin and {} has no marketplace this build can \
                 resolve one from. Refused by name rather than ignored: an operator who declared a \
                 plugin would otherwise believe the run had one",
                kind.as_str()
            ),
            Refusal::Io { detail } => write!(f, "the platform refused: {detail}"),
        }
    }
}

impl std::error::Error for Refusal {}

impl From<std::io::Error> for Refusal {
    fn from(error: std::io::Error) -> Self {
        Refusal::Io {
            detail: error.to_string(),
        }
    }
}
