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

use metaharness_protocol::{CommandOutcome, Emission, Event, RefusalCode, Refused};

/// Why metaharness could not do its job. Always exit `2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// This kind has no adapter in this build.
    NoAdapter {
        /// The kind, by the name the CLI spells.
        kind: String,
    },
    /// `--frame <file>` was given and the on-disk frame format does not exist yet.
    ///
    /// Refused rather than shipped against an undefined format: parsing it in the binary would
    /// be protocol logic in the CLI, and D11 exists to forbid that (design § 9.3, correction 3).
    FrameFile {
        /// The path that was asked for.
        path: PathBuf,
    },
    /// `--tool-surface owned` was given.
    ///
    /// Strategy C means metaharness implements read, write, edit and shell itself, and per-step
    /// re-listing depends on vendor behaviour nobody has driven (Q1). It is opt-in in the design
    /// and unbuilt here (design § 7.5).
    ToolSurfaceOwned,
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
            Refusal::NoAdapter { .. } | Refusal::FrameFile { .. } | Refusal::ToolSurfaceOwned => {
                Some(RefusalCode::UnsupportedControl)
            }
            _ => None,
        }
    }
}

impl fmt::Display for Refusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Refusal::NoAdapter { kind } => write!(
                f,
                "no adapter for {kind} in this build: the {kind} adapter crate is a later \
                 milestone, so the run is refused by name rather than degraded"
            ),
            Refusal::FrameFile { path } => write!(
                f,
                "--frame {} is refused: the on-disk frame format is owed and is not in v0.1, and \
                 shipping against an undefined format would put protocol logic in the binary. \
                 Use the library's Metaharness::with_frame(Frame) with an in-memory value",
                path.display()
            ),
            Refusal::ToolSurfaceOwned => f.write_str(
                "--tool-surface owned is refused: it requires metaharness to implement read, \
                 write, edit and shell itself, and per-step tool re-listing depends on vendor \
                 behaviour nobody has driven (Q1). Use --tool-surface native",
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
