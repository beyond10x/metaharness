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
    /// vendor is a way to remove the built-in tools and add ours. Claude Code has one (`--tools
    /// ""` plus `--mcp-config`); Codex has one over `codex app-server`, used only for a caller's
    /// catalogue (amendment a66), so on Codex the bare owned surface is refused.
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
    /// `--write-scope` was given for a kind that carries it another way.
    ///
    /// Not because a vendor arm should be unbounded. A scope reaches it as `Frame.subjects`, sealed
    /// into the frame's digest and adjudicated at its hook seam; a flag here would be a second,
    /// unsealed copy of the same rule, and the two would disagree the first time one moved.
    /// `--context` was refused here too until design amendment a35, which gave it every kind.
    ScopeUnsupported {
        /// The kind whose scope travels in the frame.
        kind: Kind,
    },
    /// A `--context` file this run cannot be handed (design amendment a35).
    ///
    /// Raised before anything is spawned, and naming the file: a run handed a smaller context
    /// than it declared is one nobody can reproduce from the declaration.
    ContextFile {
        /// The path as declared.
        path: PathBuf,
        /// Why it cannot be handed over.
        why: ContextRefusal,
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
    /// Auditor arguments were given (`-- <args>`, or [`crate::Metaharness::with_auditor_arg`])
    /// and `--auditor` was not (design amendment a43).
    ///
    /// They go to the auditor after everything metaharness adds, so with no auditor nothing
    /// receives them, and an expectation document named among them would be checked by nobody.
    AuditorArgsWithoutAuditor,
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
    /// `project --to trace-ir-summary` was given something other than one event stream (design
    /// amendment a42 point 5).
    ///
    /// A summary is one run's: it is a function of one document, and a second stream would have
    /// to be summarized into a file nobody named or silently dropped. The viewer's refusal does
    /// not fit, because it tells the caller to give one *or two*.
    SummaryStreamCount {
        /// How many were given.
        given: usize,
    },
    /// `project --from <format>` could not be honoured (design amendment a28): the format is not
    /// one this build has a reader for, or the record is not that format.
    ///
    /// The second is decided by one rule, stated in a28 and repeated in the detail: **the
    /// reader's first event is `session.started`.** The readers raise no error of their own — a
    /// line they cannot read is `opaque` (design D4) — so without this rule a record handed to
    /// the wrong reader would project as a column of opaque nodes that reads exactly like a run
    /// whose record was unreadable.
    ProjectionSource {
        /// The `--from` value as given.
        source: String,
        /// The record, where one was read before the refusal.
        path: Option<PathBuf>,
        /// What was wrong, verbatim.
        detail: String,
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
    /// The run's scratch root is not UTF-8 (design amendment a37).
    ///
    /// The hook program, the hook `command`, the Codex `config.toml` and the b10x loop's argv and
    /// environment all carry paths under it, and a path that is not UTF-8 has no faithful
    /// rendering in any of them. Raised on every kind. Refused before the
    /// spawn rather than rendered through the lossy display form, which would name a directory
    /// that does not exist. Its code string is [`SCRATCH_PATH_NOT_UTF8`].
    ScratchPathNotUtf8 {
        /// The scratch root as the platform handed it over.
        path: PathBuf,
    },
    /// The Claude hook `command` contains `${CLAUDE_PLUGIN_ROOT}` or `${CLAUDE_PLUGIN_DATA}`
    /// (design amendment a37 § 6).
    ///
    /// The pinned Claude Code refuses to spawn such a settings hook before any shell reads it, so
    /// the run would go ahead with no seam. Raised on the Claude kind only. Its code string is
    /// [`SCRATCH_PATH_NAMES_PLUGIN_VARIABLE`].
    ScratchPathNamesPluginVariable {
        /// The hook program's path, which the `command` quotes.
        path: PathBuf,
        /// The literal the `command` contains.
        variable: &'static str,
    },
    /// `--scratch-name` is not 1 to 64 bytes of ASCII letters, digits, `.`, `_` and `-`, or is
    /// `.` or `..`, or ends in `.lock` in any case (design amendment a72 § 2).
    ///
    /// Raised by [`crate::check_spec`], so before anything is created, on every kind. Its code
    /// string is [`SCRATCH_NAME_INVALID`].
    ScratchNameInvalid {
        /// The name as given.
        name: String,
        /// What is wrong with it.
        why: &'static str,
    },
    /// Another run holds the `--scratch-name` (design amendment a72).
    ///
    /// A named run holds an exclusive lock on the name's lock file for its whole life, and this
    /// run took nothing: the other run's root is untouched. Its code string is
    /// [`SCRATCH_NAME_HELD`].
    ScratchNameHeld {
        /// The name.
        name: String,
        /// The lock file the other run holds.
        lock: PathBuf,
    },
    /// Something other than this user's own directory or lock file is where a named run puts its
    /// root or its lock (design amendment a72): a symbolic link, a file, or an entry another user
    /// owns.
    ///
    /// Refused and left untouched: following or removing it would act on a path this run did not
    /// make. Its code string is [`SCRATCH_ROOT_FOREIGN`].
    ScratchRootForeign {
        /// The entry in the way.
        path: PathBuf,
        /// What it is.
        found: &'static str,
    },
    /// `--catalogue <file>` could not be used as a catalogue (design amendment a40): unreadable,
    /// not a `tool-contract/1` document, or one that format's checks refuse.
    ///
    /// Read once, before the frame is resolved and before anything is spawned, so a run never
    /// starts on a catalogue nobody could read. The code string leads the message: one of
    /// [`CATALOGUE_UNREADABLE`], [`CATALOGUE_NOT_TOOL_CONTRACT`] or [`CATALOGUE_MALFORMED`].
    Catalogue {
        /// What the reader said, by name.
        error: metaharness_tools::CatalogueError,
    },
    /// `--catalogue` was given for a kind that cannot carry it (design amendment a40).
    ///
    /// Claude Code offers a catalogue **instead of** its own tools over MCP, and Codex over
    /// `codex app-server` (amendment a66). Only the b10x loop is left: it publishes its own
    /// toolset in-process. Refused by name, with the code [`CATALOGUE_NOT_CARRIED`], rather than
    /// accepted and ignored.
    CatalogueNotCarried {
        /// The kind that cannot carry it.
        kind: Kind,
    },
    /// A Codex catalogue entry is published as `mcp` or under the `mcp__` prefix, which Codex's
    /// app-server reserves and refuses at `thread/start` (`codex-rs/app-server/src/
    /// request_processors/thread_processor.rs:362` at `rust-v0.160.0`; design amendment a66
    /// § review corrections).
    ///
    /// Refused by name before the scratch tree, the spawn and the credential copy. Code
    /// [`CATALOGUE_NAME_RESERVED`].
    CatalogueNameReserved {
        /// The entry's published name.
        name: String,
    },
    /// Two Codex catalogue entries are published under one name, which Codex's app-server
    /// refuses at `thread/start` (`thread_processor.rs:365-371` at `rust-v0.160.0`; design
    /// amendment a66 § review corrections).
    ///
    /// Refused by name before the scratch tree, the spawn and the credential copy. Code
    /// [`CATALOGUE_NAME_DUPLICATE`].
    CatalogueNameDuplicate {
        /// The name published twice.
        name: String,
    },
    /// `--catalogue` was given with `--tool-surface native` (design amendment a40).
    ///
    /// The catalogue is served by this binary's tool server, which only the owned surface starts.
    /// Code [`CATALOGUE_NEEDS_OWNED_SURFACE`].
    CatalogueNeedsOwnedSurface,
    /// `--allow-program` was given with `--catalogue` (design amendment a40).
    ///
    /// The programs are what the served `run` may start, and a server handed a catalogue
    /// publishes that catalogue's entries and nothing else. No entry is performed as `run`, so the
    /// programs would be accepted and do nothing. Code [`CATALOGUE_WITH_ALLOW_PROGRAM`].
    CatalogueWithAllowProgram {
        /// The programs the run named.
        programs: Vec<String>,
    },
    /// The scratch copy of the catalogue, read back after it was written and before the spawn, is
    /// not the catalogue that was checked (design amendment a40).
    ///
    /// The server reads that copy and never the caller's file, so a copy that reads back as
    /// another catalogue, or not at all, would serve something nobody checked. Code
    /// [`CATALOGUE_COPY_DIFFERS`].
    ///
    /// The copy is named as `catalogue.json` in the run's scratch tree and never by its path,
    /// which lies under the operator's `TMPDIR`. The check runs after the scratch files are
    /// written, so the message says what had started rather than that nothing had.
    CatalogueCopyDiffers {
        /// How it differs: the reader's code and problems, or that it reads back as another
        /// catalogue. Never a path.
        detail: String,
        /// A credential proxy had been started for the run; it stops as the refusal returns.
        proxy_started: bool,
        /// The vendor binary had been run with `--version`, for `--strict-version`.
        version_probed: bool,
    },
    /// Something the platform refused.
    Io {
        /// What it said.
        detail: String,
    },
}

/// The stable code string [`Refusal::CatalogueNotCarried`] carries in its message (design
/// amendment a40). A launch-condition code, like [`SCRATCH_PATH_NOT_UTF8`].
pub const CATALOGUE_NOT_CARRIED: &str = "catalogue-not-carried";

/// The stable code string [`Refusal::CatalogueNameReserved`] carries in its message (design
/// amendment a66 § review corrections).
pub const CATALOGUE_NAME_RESERVED: &str = "catalogue-name-reserved";

/// The stable code string [`Refusal::CatalogueNameDuplicate`] carries in its message (design
/// amendment a66 § review corrections).
pub const CATALOGUE_NAME_DUPLICATE: &str = "catalogue-name-duplicate";

/// The stable code string [`Refusal::CatalogueNeedsOwnedSurface`] carries in its message (design
/// amendment a40).
pub const CATALOGUE_NEEDS_OWNED_SURFACE: &str = "catalogue-needs-owned-surface";

/// The stable code string [`Refusal::CatalogueWithAllowProgram`] carries in its message (design
/// amendment a40).
pub const CATALOGUE_WITH_ALLOW_PROGRAM: &str = "catalogue-with-allow-program";

/// The stable code string [`Refusal::CatalogueCopyDiffers`] carries in its message (design
/// amendment a40).
pub const CATALOGUE_COPY_DIFFERS: &str = "catalogue-copy-differs";

/// The stable code strings [`Refusal::Catalogue`] carries, one per way a file is not a catalogue
/// (design amendment a40). Defined by `metaharness-tools`, whose reader raises them.
pub use metaharness_tools::{
    CATALOGUE_MALFORMED, CATALOGUE_NOT_TOOL_CONTRACT, CATALOGUE_UNREADABLE,
};

/// The stable code string [`Refusal::ScratchPathNotUtf8`] carries in its message (design
/// amendment a37).
///
/// A launch-condition code, not a protocol [`RefusalCode`]: those answer *"what happened to this
/// command"*, and this refusal happens before any command exists.
pub const SCRATCH_PATH_NOT_UTF8: &str = "scratch-path-not-utf8";

/// The stable code string [`Refusal::ScratchPathNamesPluginVariable`] carries in its message
/// (design amendment a37 § 6). Defined by the Claude adapter, whose vendor reads the literals.
pub use metaharness_claude::SCRATCH_PATH_NAMES_PLUGIN_VARIABLE;

/// The stable code string [`Refusal::ScratchNameInvalid`] carries in its message (design
/// amendment a72). A launch-condition code, like [`SCRATCH_PATH_NOT_UTF8`].
pub const SCRATCH_NAME_INVALID: &str = "scratch-name-invalid";

/// The stable code string [`Refusal::ScratchNameHeld`] carries in its message (design amendment
/// a72).
pub const SCRATCH_NAME_HELD: &str = "scratch-name-held";

/// The stable code string [`Refusal::ScratchRootForeign`] carries in its message (design
/// amendment a72).
pub const SCRATCH_ROOT_FOREIGN: &str = "scratch-root-foreign";

impl Refusal {
    /// The Claude adapter's launch refusal, as one of these.
    ///
    /// A refusal with its own code keeps it; every other one is [`Refusal::Launch`] in the
    /// adapter's words.
    pub(crate) fn from_claude_launch(refusal: metaharness_claude::LaunchRefusal) -> Self {
        match refusal {
            metaharness_claude::LaunchRefusal::ScratchPathNamesPluginVariable {
                path,
                variable,
            } => Refusal::ScratchPathNamesPluginVariable { path, variable },
            other => Refusal::Launch {
                detail: other.to_string(),
            },
        }
    }
}

/// Why a `--context` file is refused (design amendment a35), one variant per refusal the story's
/// acceptance names, plus the two a file's bytes can raise.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextRefusal {
    /// The run names no `--cwd`, so there is no tree to read the file from.
    WithoutCwd,
    /// The path names no file: empty, or only `.` components.
    Empty,
    /// The path is absolute.
    Absolute,
    /// The path has a `..` component.
    Escapes,
    /// The path is not UTF-8.
    PathNotUtf8,
    /// The path is, or passes through, a symbolic link.
    Symlink,
    /// The path names something other than a regular file.
    NotRegularFile,
    /// The file could not be opened or read: absent, or the platform said no.
    Unreadable {
        /// What the platform said.
        detail: String,
    },
    /// The content is not UTF-8 text without NUL bytes.
    NotText {
        /// Where the content stopped being text.
        detail: String,
    },
    /// The file is not tracked by git in the `--cwd` tree.
    Untracked {
        /// What git said, or why it could not say.
        detail: String,
    },
    /// The list declares more files than [`metaharness_protocol::CONTEXT_FILES_MAX`]; this path
    /// is the first one past the cap.
    CountCap {
        /// How many the run declared.
        declared: usize,
    },
    /// With this file the list passes [`metaharness_protocol::CONTEXT_BYTES_MAX`].
    ByteCap {
        /// The running total, this file included.
        bytes: usize,
    },
}

impl fmt::Display for ContextRefusal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ContextRefusal::WithoutCwd => f.write_str(
                "a context file is read from the --cwd tree and this run names none; without \
                 --cwd the child starts in an empty scratch directory where no tracked file can \
                 exist. Name the tree with --cwd",
            ),
            ContextRefusal::Empty => f.write_str("the path names no file"),
            ContextRefusal::Absolute => f.write_str(
                "the path is absolute; a context file is named relative to the --cwd tree, so the \
                 declaration cannot reach outside it",
            ),
            ContextRefusal::Escapes => f.write_str(
                "the path has a `..` component; a context file is named beneath the --cwd tree and \
                 never climbs out of it",
            ),
            ContextRefusal::PathNotUtf8 => f.write_str(
                "the path is not UTF-8, and the record and the prompt carry it as text",
            ),
            ContextRefusal::Symlink => f.write_str(
                "the path is or passes through a symbolic link; the file is opened beneath the \
                 --cwd tree without following one (openat2, RESOLVE_BENEATH | \
                 RESOLVE_NO_SYMLINKS), so the file the declaration names is the file that is read",
            ),
            ContextRefusal::NotRegularFile => f.write_str(
                "it is not a regular file; a context file is a file's bytes, and a directory, FIFO, \
                 socket or device either has none or blocks",
            ),
            ContextRefusal::Unreadable { detail } => write!(f, "it could not be read: {detail}"),
            ContextRefusal::NotText { detail } => write!(
                f,
                "its content is not UTF-8 text without NUL bytes ({detail}); it travels in the \
                 prompt, one argv string, and a lossy conversion would render bytes other than the \
                 ones digested"
            ),
            ContextRefusal::Untracked { detail } => write!(
                f,
                "it is not tracked by git in the --cwd tree ({detail}); a context file is one the \
                 tree's own index names"
            ),
            ContextRefusal::CountCap { declared } => write!(
                f,
                "the run declares {declared} context files and the count cap is {} (design \
                 amendment a35)",
                metaharness_protocol::CONTEXT_FILES_MAX
            ),
            ContextRefusal::ByteCap { bytes } => write!(
                f,
                "with it the context files reach {bytes} bytes, past the byte cap of {} counted \
                 over each file's content and rendered path; the cap keeps the prompt under \
                 Linux's 128 KiB single-argument limit (design amendment a35)",
                metaharness_protocol::CONTEXT_BYTES_MAX
            ),
        }
    }
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
                 ours. Claude Code has one — `--tools \"\"` plus `--mcp-config`. Codex has one \
                 over codex app-server, used only for a caller's catalogue: give --catalogue \
                 (design amendment a66). b10x already publishes exactly this catalogue \
                 in-process, so there is nothing to replace. Use --tool-surface native",
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
                "--write-scope is refused for {}: a scope reaches it as Frame.subjects, sealed \
                 into the frame's digest and adjudicated at its hook seam. A flag here would be a \
                 second copy of the same rule that nothing seals, and the two would disagree the \
                 first time one moved. The b10x loop takes one because it has no seam: its \
                 published toolset is its policy, so the scope travels to the tools",
                kind.as_str(),
            ),
            Refusal::ContextFile { path, why } => {
                write!(f, "--context {} is refused: {why}", path.display())
            }
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
            Refusal::AuditorArgsWithoutAuditor => f.write_str(
                "auditor arguments after -- were given without --auditor: they are passed to the \
                 auditor after everything metaharness adds, and with no auditor nothing receives \
                 them, so an expectation document named there would be checked by nobody. Name \
                 the auditor with --auditor '<argv prefix>', or drop the arguments",
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
            Refusal::SummaryStreamCount { given } => write!(
                f,
                "--to trace-ir-summary summarizes one run's document and was given {given} event \
                 streams: a summary is a function of one document, and a second would be written \
                 nowhere or dropped. Give exactly one event stream, and run the verb once per run"
            ),
            Refusal::ProjectionSource {
                source,
                path,
                detail,
            } => match path {
                Some(path) => write!(
                    f,
                    "project --from {source} is refused for {}: {detail}. Refused rather than \
                     projected, because the readers turn what they cannot read into opaque nodes \
                     and a document of those reads exactly like a run whose record was unreadable",
                    path.display()
                ),
                None => write!(f, "project --from {source} is refused: {detail}"),
            },
            Refusal::MarketplacePlugin { detail } => write!(f, "{detail}"),
            Refusal::MarketplacePluginUnsupported { kind } => write!(
                f,
                "--plugin names a marketplace plugin and {} has no marketplace this build can \
                 resolve one from. Refused by name rather than ignored: an operator who declared a \
                 plugin would otherwise believe the run had one",
                kind.as_str()
            ),
            Refusal::ScratchPathNotUtf8 { path } => write!(
                f,
                "{SCRATCH_PATH_NOT_UTF8}: the scratch root {} is not UTF-8, so the hook \
                 program, the hook command, the vendor's settings and the loop's arguments have \
                 no faithful way to name it (design amendment a37). Run with a TMPDIR whose path \
                 is UTF-8",
                path.as_os_str().as_encoded_bytes().escape_ascii()
            ),
            Refusal::ScratchPathNamesPluginVariable { path, variable } => write!(
                f,
                "{SCRATCH_PATH_NAMES_PLUGIN_VARIABLE}: the Claude hook program path {} contains \
                 {variable}, and the pinned Claude Code refuses to spawn a settings hook whose \
                 command names a plugin variable when the hook is not a plugin's, so the seam \
                 would never be consulted (design amendment a37). Run with a TMPDIR whose path \
                 does not contain it",
                path.display()
            ),
            Refusal::ScratchNameInvalid { name, why } => write!(
                f,
                "{SCRATCH_NAME_INVALID}: --scratch-name {name:?} is refused: {why}. The name is \
                 1 to 64 bytes of ASCII letters, digits, ., _ and -, not . or .. and not ending \
                 in .lock in any case, because it is one entry of the scratch root's parent \
                 beside its lock file: <temp dir>/metaharness-u<EUID>/<NAME> (design amendment \
                 a72). Nothing was created"
            ),
            Refusal::ScratchNameHeld { name, lock } => write!(
                f,
                "{SCRATCH_NAME_HELD}: a run under scratch name {name:?} is still going: it holds \
                 the lock {} for as long as it runs, and two runs in one scratch root would \
                 overwrite each other's config home, hook channel and transcript (design \
                 amendment a72). Nothing was touched. Wait for that run to end, or give this run \
                 another name",
                lock.display()
            ),
            Refusal::ScratchRootForeign { path, found } => write!(
                f,
                "{SCRATCH_ROOT_FOREIGN}: {} is {found}, where a named run keeps its private \
                 parent, its scratch root or its lock. Only a directory or lock file this user \
                 left there is taken over; anything else is refused rather than followed or \
                 removed, and was left as it is (design amendment a72). Remove it, or give the \
                 run another name",
                path.display()
            ),
            Refusal::Catalogue { error } => write!(f, "{error}"),
            Refusal::CatalogueNotCarried { kind } => write!(
                f,
                "{CATALOGUE_NOT_CARRIED}: --catalogue is refused for {}: Claude Code offers a \
                 catalogue instead of its own tools (`--tools \"\"` plus this binary's tool \
                 server) and Codex does over codex app-server (design amendment a66), while the \
                 b10x loop publishes its own toolset in-process and takes no catalogue file. \
                 Refused by name rather than ignored: a run that offered something other than \
                 the catalogue it was handed is not the run that was asked for (design amendment \
                 a40). Nothing was started",
                kind.as_str(),
            ),
            Refusal::CatalogueNameReserved { name } => write!(
                f,
                "{CATALOGUE_NAME_RESERVED}: --catalogue publishes an entry as `{name}` on codex, \
                 and codex app-server reserves `mcp` and every name that starts with `mcp__`: \
                 0.160.0 refuses it at thread/start (thread_processor.rs:362), so the run could \
                 not offer this catalogue (design amendment a66 § review corrections). Rename \
                 the entry's `mcp` rendering. Nothing was started"
            ),
            Refusal::CatalogueNameDuplicate { name } => write!(
                f,
                "{CATALOGUE_NAME_DUPLICATE}: --catalogue publishes two entries as `{name}` on \
                 codex, and codex app-server refuses a name offered twice at thread/start \
                 (thread_processor.rs:365-371 at 0.160.0), so the run could not offer this \
                 catalogue (design amendment a66 § review corrections). Give each entry its own \
                 `mcp` rendering. Nothing was started"
            ),
            Refusal::CatalogueNeedsOwnedSurface => write!(
                f,
                "{CATALOGUE_NEEDS_OWNED_SURFACE}: --catalogue was given with --tool-surface \
                 native: the catalogue is served by metaharness's own tool server, which only the \
                 owned surface starts, so under the native surface it would be read and never \
                 offered (design amendment a40). Drop --tool-surface native; --catalogue implies \
                 owned. Nothing was started"
            ),
            Refusal::CatalogueWithAllowProgram { programs } => write!(
                f,
                "{CATALOGUE_WITH_ALLOW_PROGRAM}: --allow-program {} was given with --catalogue: \
                 the programs are what the served `run` may start, and a server handed a \
                 catalogue publishes that catalogue's entries and nothing else, none of which is \
                 performed as `run`, so they would be accepted and do nothing (design amendment \
                 a40). Drop --allow-program. Nothing was started",
                programs.join(", ")
            ),
            Refusal::CatalogueCopyDiffers {
                detail,
                proxy_started,
                version_probed,
            } => {
                write!(
                    f,
                    "{CATALOGUE_COPY_DIFFERS}: the scratch copy of the --catalogue document \
                     (catalogue.json in the run's scratch tree), read back before the spawn, is \
                     not the catalogue that was checked: {detail}. The server reads that copy, so \
                     the run would serve something nobody checked (design amendment a40). No \
                     vendor session was started. The run's scratch files had been written, and \
                     its scratch tree is removed as this refusal returns"
                )?;
                if *version_probed {
                    write!(
                        f,
                        "; the vendor binary had been run with --version for --strict-version"
                    )?;
                }
                if *proxy_started {
                    write!(
                        f,
                        "; a credential proxy had been started, and it stops as this refusal \
                         returns"
                    )?;
                }
                Ok(())
            }
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
