//! The `metaharness` binary's verb surface.
//!
//! **This crate holds no protocol logic.** It parses into
//! [`metaharness::protocol::RunSpec`] and hands that value to the library unchanged, which is
//! what makes the two faces one protocol: a flag the library cannot express cannot be added, and
//! an option the CLI cannot express cannot be introduced (design D11). The rule is enforced
//! mechanically in `tests/anti_drift.rs` rather than stated here, because the first statement of
//! it was decorative and the design's own two surfaces had already drifted apart (finding F16).
//!
//! It is a library beside the binary for one reason: an integration test cannot import a `main`,
//! and the anti-drift assertion has to read the real [`Cli`] rather than a copy of it.
//!
//! # Exit codes
//!
//! Exactly § 9.4, for every verb:
//!
//! | code | meaning |
//! |---|---|
//! | `0` | it ran, and every gating verdict is `ok` |
//! | `1` | a gating verdict is `gap`, or the auditor exited `1`, or a conformance vector failed |
//! | `2` | metaharness could not do its job. **Never a verdict about the run** |
//! | `3` | nobody found out — a gating row is `unk`, or the auditor exited `3`, or the harness died without a record. The harness itself may have exited `0`: the report then says `child exit: 0` beside `control verdict: unk` |

mod forward;

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use metaharness::protocol::{Kind, RunSpec, VectorOutcome};
use metaharness::{
    Input, Metaharness, ProcessAuditor, ProjectionForm, Refusal, Run, RunExit, VendorSource,
};

/// One interface to many agent harnesses.
#[derive(Parser, Debug)]
#[command(name = "metaharness", version, about)]
pub struct Cli {
    /// Which verb.
    #[command(subcommand)]
    pub command: Verb,
}

/// What the binary can do.
#[derive(Subcommand, Debug)]
pub enum Verb {
    /// Execute AEP-governed work through a concrete harness.
    Aep {
        /// AEP operation.
        #[command(subcommand)]
        command: Box<AepCommand>,
    },
    /// Run a harness session: events as JSON lines on stdout, commands as JSON lines on stdin.
    ///
    /// Boxed because the run options dwarf every other verb's, and the enum is sized by its
    /// largest variant.
    Run(Box<RunArgs>),
    /// What an adapter says it can do: declared tiers, pinned versions, operation rendering.
    ///
    /// Exists so an embedder can refuse early rather than discovering mid-run that a tier is
    /// absent (design § 9.2).
    Capabilities(CapabilitiesArgs),
    /// The free conformance vectors: no model, no network, no credential (design § 8.5).
    Conformance(ConformanceArgs),
    /// Project an event stream into `trace-ir/1`.
    Project(ProjectArgs),
    /// Judge a transcript offline.
    Audit(AuditArgs),
    /// The installed vendor version against the adapter's pin.
    Doctor(DoctorArgs),
    /// Serve the owned tool surface over MCP on stdio, for a harness launched with `--tools ""`.
    ///
    /// Not a verb an operator types. `metaharness run --tool-surface owned` writes an
    /// `--mcp-config` naming *this binary and this subcommand*, so the server the vendor starts is
    /// the one already installed — design § 7.5, strategy C.
    McpServe(McpServeArgs),
}

/// Operations supplied by the AEP integration.
#[derive(Subcommand, Debug)]
pub enum AepCommand {
    /// Walk a governed run, inspect it, or evaluate a harness.
    Drive {
        /// Driver operation.
        #[command(subcommand)]
        command: metaharness_aep::drive::DriveCommand,
    },
}

/// `run` carries the library's option struct and nothing else.
///
/// A `flatten` and not a copy: every long flag below comes from `RunSpec`, and
/// `tests/anti_drift.rs` asserts the two sets are equal.
#[derive(Args, Debug)]
pub struct RunArgs {
    /// The run, as the one options type.
    #[command(flatten)]
    pub spec: RunSpec,
}

/// `capabilities <kind> [--render]`.
#[derive(Args, Debug)]
pub struct CapabilitiesArgs {
    /// Which harness.
    #[arg(value_enum)]
    pub kind: Kind,
    /// Print the neutral-operation → vendor-tool table instead of the whole descriptor.
    ///
    /// A rendering that only exists inside a run cannot be asserted on before one (design § 8.4
    /// O6).
    #[arg(long)]
    pub render: bool,
}

/// `conformance <kind> [--contract]`.
#[derive(Args, Debug)]
pub struct ConformanceArgs {
    /// Which harness.
    #[arg(value_enum)]
    pub kind: Kind,
    /// Emit the run as a `contract_result` record (design `adapter-contract-v0.1.md`, CT-1)
    /// instead of the human vector lines — the adapter's conformance as a contract between the
    /// vendor and `metaharness.event/1`, in the shape `AEP` reads.
    #[arg(long)]
    pub contract: bool,
}

/// `project <events.jsonl> [<events.jsonl>] [--from <format>] [--to trace-ir] [--out <f>]
/// [--html <f>]`.
///
/// The stream is a **positional**, and up to two of them, because the viewer's whole subject is
/// two runs read beside each other and a flag repeated twice reads as an accident
/// (`docs/design/runs-side-by-side-v0.1.md` § 2).
#[derive(Args, Debug)]
pub struct ProjectArgs {
    /// The event stream to project. Two of them under `--html`, one otherwise.
    #[arg(value_name = "EVENTS", required = true)]
    pub events: Vec<std::path::PathBuf>,
    /// Read every record given as a vendor record of this format, through the adapter's own
    /// reader, instead of as a `metaharness.event/1` stream (design amendment a28):
    /// `claude-code-stream-json`, `codex-rollout`, `harness-json` or `claude-code-session` (a Claude
    /// Code session as stored on disk, with the sub-agent files beside it, amendment a63).
    ///
    /// A string the library parses rather than a `clap` enum, so an unknown name is refused once,
    /// by the library, in the same words on both faces (design D11).
    #[arg(long, value_name = "FORMAT")]
    pub from: Option<String>,
    /// The target form: `trace-ir` (the `trace-ir/1` document) or `trace-ir-summary` (its
    /// `trace-ir-summary/1`, design amendment a42). A summary is one run's: under
    /// `trace-ir-summary` exactly one event stream is accepted.
    ///
    /// A string the library parses rather than a `clap` enum, so an unknown form is refused once,
    /// by the library, in the same words on both faces (design D11).
    #[arg(long, default_value = "trace-ir")]
    pub to: String,
    /// Write the document (or its summary, under `--to trace-ir-summary`) here instead of to
    /// stdout.
    #[arg(long, value_name = "FILE")]
    pub out: Option<std::path::PathBuf>,
    /// Render one or two runs as a static two-column page at this path.
    ///
    /// One file, no server, no network: the alignment is computed here and the page carries it,
    /// so the two runs read the same in a browser and inside whatever embeds it.
    #[arg(long, value_name = "FILE")]
    pub html: Option<std::path::PathBuf>,
}

/// `audit --transcript <f> [--events <f>] [--spec <s>] [--auditor <p>]`.
#[derive(Args, Debug)]
pub struct AuditArgs {
    /// The raw vendor transcript to judge.
    #[arg(long, value_name = "FILE")]
    pub transcript: std::path::PathBuf,
    /// The event stream that went with it.
    #[arg(long, value_name = "FILE")]
    pub events: Option<std::path::PathBuf>,
    /// The expectation document.
    #[arg(long, value_name = "FILE")]
    pub spec: Option<std::path::PathBuf>,
    /// The external auditor, as an argv prefix.
    #[arg(long, value_name = "PREFIX")]
    pub auditor: Option<String>,
}

/// `mcp-serve --workspace <dir> [--writable] [--allow-program <p>]...`.
#[derive(Args, Debug)]
pub struct McpServeArgs {
    /// The only directory the served tools can see.
    #[arg(long, value_name = "DIR", default_value = ".")]
    pub workspace: std::path::PathBuf,
    /// Let the served tools change the tree.
    ///
    /// Off by default and asked for by name, because nothing under this confines the process: the
    /// bound on an effect is the `tools` crates' own path arithmetic. A run that wants the
    /// effects actually confined is a b10x run against substrate, not this.
    #[arg(long)]
    pub writable: bool,
    /// A program the served `run` may start. Repeatable, and an empty set publishes no `run`.
    ///
    /// Declared rather than open, because an argv whose program could be anything is the shell
    /// this surface exists to remove. Refused beside `--catalogue`, whose server publishes no
    /// `run`.
    #[arg(long, value_name = "PROGRAM")]
    pub allow_program: Vec<String>,
    /// A caller's `tool-contract/1` catalogue to serve instead of the three verbs (design
    /// amendment a40).
    ///
    /// Each entry is listed as its own tool under its `mcp` rendering, and only an entry this
    /// binary performs does anything when called. Read and checked before the first request; a
    /// file that is not a catalogue is refused by name.
    #[arg(long, value_name = "FILE")]
    pub catalogue: Option<std::path::PathBuf>,
}

/// `doctor <kind>`.
#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Which harness.
    #[arg(value_enum)]
    pub kind: Kind,
}

/// Do what the parsed command line says, and report the process exit code.
///
/// Returns the code rather than calling `exit` so every verb's code is a value a test reads.
#[must_use]
pub fn execute(cli: Cli) -> i32 {
    match cli.command {
        Verb::Aep { command } => {
            let AepCommand::Drive { command } = *command;
            let code = metaharness_aep::execute(command);
            if code == std::process::ExitCode::SUCCESS {
                0
            } else if code == std::process::ExitCode::from(2) {
                2
            } else if code == std::process::ExitCode::from(3) {
                3
            } else {
                1
            }
        }
        Verb::Run(args) => run(args.spec),
        Verb::Capabilities(args) => capabilities(&args),
        Verb::Conformance(args) => conformance(args.kind, args.contract),
        Verb::Project(args) => project(&args),
        Verb::Audit(_) => refuse(&Refusal::NotInThisMilestone {
            verb: "audit",
            // Not the spawner any more — that exists. What is missing is the launch facts the
            // hermetic floor is evaluated against (the planned cwd, the declared plugins, the
            // adapter's pin), which a transcript metaharness did not launch cannot carry.
            missing: "the launch facts the hermetic floor compares a record against, which a \
                      transcript metaharness did not itself launch does not carry",
        }),
        Verb::Doctor(args) => doctor(args.kind),
        Verb::McpServe(args) => mcp_serve(&args),
    }
}

/// `project` — an event stream into a `trace-ir/1` document, or two of them into one page.
///
/// Amendment a15 lifted the refusal this verb used to carry. What Q9 still gates is a *reader*
/// outside this repository, not a writer here: the document is written, tagged and byte-stable,
/// and `aep observe trace check` consumes the event stream this projects **from** rather than the
/// document (`docs/design/runs-side-by-side-v0.1.md` P4).
///
/// Under `--from <format>` (amendment a28) every record is read through the adapter's production
/// reader instead; the library picks the reader and states the one rule a wrong format is refused
/// by, and nothing vendor-specific is parsed here.
fn project(args: &ProjectArgs) -> i32 {
    let form = match args.to.parse::<ProjectionForm>() {
        Ok(form) => form,
        Err(refusal) => return refuse(&refusal),
    };
    if form == ProjectionForm::TraceIrSummary && args.html.is_some() {
        return refuse(&Refusal::Io {
            detail: format!(
                "--to {} with --html is refused: the page renders trace-ir/1 documents, and a \
                 caller who asked for a summary and got a page would cite the wrong artifact",
                metaharness::TRACE_IR_SUMMARY_FORM
            ),
        });
    }
    if form == ProjectionForm::TraceIrSummary && args.events.len() != 1 {
        return refuse(&Refusal::SummaryStreamCount {
            given: args.events.len(),
        });
    }
    let source = match args.from.as_deref().map(str::parse::<VendorSource>) {
        Some(Ok(source)) => Some(source),
        Some(Err(refusal)) => return refuse(&refusal),
        None => None,
    };

    let mut documents = Vec::with_capacity(args.events.len());
    for path in &args.events {
        let projected = match source {
            Some(source) => metaharness::project_vendor_record(path, source),
            None => metaharness::project_file(path),
        };
        match projected {
            Ok(document) => documents.push(document),
            Err(refusal) => return refuse(&refusal),
        }
    }

    if let Some(page) = &args.html {
        let html = match metaharness::render_page(&documents) {
            Ok(html) => html,
            Err(refusal) => return refuse(&refusal),
        };
        if let Err(error) = std::fs::write(page, html) {
            return refuse(&Refusal::Io {
                detail: format!("{} could not be written: {error}", page.display()),
            });
        }
        return RunExit::Ok.code();
    }

    // Without `--html` this verb projects **one** run: a document is one run's record, and a
    // second stream given here would have to be written somewhere nobody named.
    let [document] = documents.as_slice() else {
        return refuse(&Refusal::ViewerColumnCount {
            given: documents.len(),
        });
    };
    let rendered = match form {
        ProjectionForm::TraceIr => document.render(),
        ProjectionForm::TraceIrSummary => metaharness::protocol::summarize(document).render(),
    };
    let rendered = match rendered {
        Ok(rendered) => rendered,
        Err(error) => {
            return refuse(&Refusal::Io {
                detail: error.to_string(),
            });
        }
    };
    match &args.out {
        Some(path) => {
            if let Err(error) = std::fs::write(path, rendered) {
                return refuse(&Refusal::Io {
                    detail: format!("{} could not be written: {error}", path.display()),
                });
            }
        }
        None => print!("{rendered}"),
    }
    RunExit::Ok.code()
}

/// `mcp-serve` — the owned tool surface, on stdin and stdout, until the client goes away.
fn mcp_serve(args: &McpServeArgs) -> i32 {
    use metaharness_tools::{CallerCatalogue, Catalogue, LocalOperations, Server, Verbs, serve};

    // A server handed a catalogue publishes its entries and no `run`, so programs named for `run`
    // would start nothing (design amendment a40). Refused on the flags alone, before any file is
    // read, as `run claude` refuses the same pair.
    if args.catalogue.is_some() && !args.allow_program.is_empty() {
        return refuse(&Refusal::CatalogueWithAllowProgram {
            programs: args.allow_program.clone(),
        });
    }

    // Next, so a file that is not a catalogue is refused before the workspace is opened.
    let caller = match args
        .catalogue
        .as_deref()
        .map(CallerCatalogue::read)
        .transpose()
    {
        Ok(caller) => caller,
        Err(error) => return refuse(&Refusal::Catalogue { error }),
    };

    if !args.writable && !args.allow_program.is_empty() {
        return refuse(&Refusal::Io {
            detail: "`--allow-program` without `--writable` names programs nothing will start;                      add `--writable` or drop them"
                .to_owned(),
        });
    }

    let operations = if args.writable {
        LocalOperations::unconfined(&args.workspace, args.allow_program.clone())
    } else {
        LocalOperations::new(&args.workspace)
    };
    let operations = match operations {
        Ok(operations) => operations,
        Err(detail) => return refuse(&Refusal::Io { detail }),
    };

    let verbs = Verbs::new(Catalogue::of(operations));
    let mut server = match caller {
        Some(caller) => Server::with_catalogue(verbs, caller),
        None => Server::new(verbs),
    };
    let stdin = std::io::stdin();
    match serve(&mut server, stdin.lock(), std::io::stdout().lock()) {
        Ok(()) => 0,
        // The client hung up or the pipe broke. Exit `2`: metaharness could not do its job, and
        // this is never a verdict about the run the tools were serving.
        Err(error) => refuse(&Refusal::Io {
            detail: error.to_string(),
        }),
    }
}

/// How long the loop waits on a steering command before it looks at the run again.
const STEER_POLL: Duration = Duration::from_millis(50);

/// `run` — the whole point: spawn the harness, put its events on stdout, take steering on stdin.
///
/// A named run's vendor child leads a process group of its own, so the terminal's signals stop
/// at metaharness: `SIGINT`, `SIGTERM` and `SIGHUP` are forwarded to the group, and once the run
/// has been dropped, its root and locks with it, metaharness ends by the signal (design amendment
/// a72 § 5).
fn run(spec: RunSpec) -> i32 {
    if spec.scratch_name.is_none() {
        return run_once(spec, None);
    }
    let forwarding = match forward::GroupForwarding::install() {
        Ok(forwarding) => forwarding,
        Err(error) => {
            return refuse(&Refusal::Io {
                detail: format!("the signals of a named run could not be taken over: {error}"),
            });
        }
    };
    let code = run_once(spec, Some(&forwarding));
    forwarding.finish();
    code
}

/// One run from start to drop, telling `forwarding` its process group once it has started.
fn run_once(spec: RunSpec, forwarding: Option<&forward::GroupForwarding>) -> i32 {
    let mut run = match Metaharness::from_spec(spec).start(Input::FromSpec) {
        Ok(run) => run,
        Err(refusal) => return refuse(&refusal),
    };
    if let Some(forwarding) = forwarding {
        forwarding.lead(run.process_group());
    }
    if let Err(error) = drive(&mut run, &steering()) {
        // A run that broke mid-flight is exit `2` and not a verdict: metaharness could not do
        // its job, and the events already printed are what there is to go on.
        return refuse(&Refusal::Io {
            detail: error.to_string(),
        });
    }
    verdict(&run)
}

/// Read steering commands off stdin, on their own thread.
///
/// A thread because the loop cannot block on stdin and on the child at the same time, and the
/// child is the one that must not be kept waiting: **the run clock keeps elapsing while a
/// decision is pending** (design § 7.6).
///
/// Lines are read as bytes, so a line that is not UTF-8 is handed on like any other and answered
/// (amendment a44). Reading stops at the end of input or on an I/O error, and nowhere else.
fn steering() -> Receiver<Vec<u8>> {
    steering_from(std::io::stdin())
}

/// [`steering`] over any input, so a test can hand it bytes stdin would carry.
fn steering_from(input: impl std::io::Read + Send + 'static) -> Receiver<Vec<u8>> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut input = std::io::BufReader::new(input);
        let mut line = Vec::new();
        loop {
            line.clear();
            match std::io::BufRead::read_until(&mut input, b'\n', &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if sender.send(trim_line_end(&line)).is_err() {
                        break;
                    }
                }
            }
        }
    });
    receiver
}

/// One steering line without its line ending and trailing blanks: what `str::trim_end` leaves of
/// a line that is UTF-8, and the line without trailing ASCII whitespace when it is not.
fn trim_line_end(line: &[u8]) -> Vec<u8> {
    match std::str::from_utf8(line) {
        Ok(text) => text.trim_end().as_bytes().to_vec(),
        Err(_) => line.trim_ascii_end().to_vec(),
    }
}

/// Events out, commands in, until the run ends.
///
/// Every line already read is handed to the run before every poll. That is what gets a line read
/// while the run waits for its child to exit answered before `stream.closed`: the run writes the
/// marker on a later poll than the one that saw the stream end (amendment a44 § 4).
fn drive(run: &mut Run, commands: &Receiver<Vec<u8>>) -> std::io::Result<()> {
    let mut version_warned = false;
    loop {
        while let Ok(line) = commands.try_recv() {
            steer(run, &line)?;
        }
        match run.poll_event()? {
            metaharness::EventPoll::Event(line) => {
                emit(&line);
                warn_off_pin_once(&line, &mut version_warned);
            }
            metaharness::EventPoll::Progress => (),
            metaharness::EventPoll::Ended => return Ok(()),
            metaharness::EventPoll::Idle | metaharness::EventPoll::DecisionPending => {
                // Both provider silence and an unanswered decision must yield to
                // stdin. Never spend the whole decision budget inside next_event.
                match commands.recv_timeout(STEER_POLL) {
                    Ok(line) => steer(run, &line)?,
                    Err(RecvTimeoutError::Timeout) => (),
                    // Disconnection returns immediately; retain a bounded sleep
                    // rather than busy-spinning a synthetic/custom idle process.
                    Err(RecvTimeoutError::Disconnected) => std::thread::sleep(STEER_POLL),
                }
            }
        }
    }
}

/// Apply one steering line, and answer it.
///
/// A line that will not parse is **not** silently dropped: design D9 says a command that can be
/// ignored is a control surface that cannot be tested, so the refusal goes out as an event under
/// the id the caller used, or under `-` when the line was too broken to carry one (amendment
/// a44). The answer is [`Run::send_line`]'s, the one an embedder gets for the same line: the
/// binary holds no protocol logic of its own (design D11).
fn steer(run: &mut Run, line: &[u8]) -> std::io::Result<()> {
    run.send_line(line).map(drop)
}

/// The adapter's off-pin warning, on stderr, once per run and in every hermetic mode
/// (amendment a62).
///
/// The event is already on stdout; stderr is where an operator who reads no event stream looks.
/// A binary off the pin is run, not refused, so this line is the only sign of it such a reader
/// gets before the H9 row of an audit.
fn warn_off_pin_once(line: &metaharness::protocol::EventLine, warned: &mut bool) {
    if let metaharness::protocol::Event::Warning { code, message } = &line.event
        && code == metaharness::protocol::warning_code::VERSION_OUTSIDE_PIN
        && !*warned
    {
        *warned = true;
        eprintln!("metaharness: {message}");
    }
}

/// One event line on stdout.
fn emit(line: &metaharness::protocol::EventLine) {
    match serde_json::to_string(line) {
        Ok(json) => println!("{json}"),
        // An event that cannot be rendered is still an event that happened; saying so on stderr
        // is better than a stream that silently skips one.
        Err(error) => eprintln!("metaharness: an event could not be rendered: {error}"),
    }
}

/// The floor, the auditor and the exit code.
fn verdict(run: &Run) -> i32 {
    if !run.wants_audit() {
        return run.exit(None).code();
    }
    match run.audit(&mut ProcessAuditor) {
        Ok(report) => {
            // Always printed, and on stderr so it never mixes into the event stream a consumer
            // is parsing. A report that hides "0 denials" reads as clean when it may mean
            // nothing was ever attempted (design § 9.4).
            eprintln!("{}", report.render());
            run.exit(Some(&report)).code()
        }
        Err(refusal) => refuse(&refusal),
    }
}

/// `doctor` — the installed vendor version against the adapter's pin, for free.
///
/// Two questions, and the second is the one that catches a producer who never bumps a version:
/// does the binary the spawn will execute still accept every flag the adapter can send it. A pin
/// that reads `0.1.0` on both sides of an interface change blesses a launch that dies on argument
/// parsing, before any of the child's own code runs and therefore before it can say why.
fn doctor(kind: Kind) -> i32 {
    let installed = match metaharness::installed(kind) {
        Ok(installed) => installed,
        Err(refusal) => return refuse(&refusal),
    };
    println!("{}", installed.render());
    let faults = match metaharness::flag_surface(kind) {
        Ok(Some(faults)) => faults,
        // The vendor kinds have no flag surface worth checking; see `flag_surface`.
        Ok(None) => Vec::new(),
        Err(refusal) => return refuse(&refusal),
    };
    if faults.is_empty() {
        println!(
            "{}: every flag the adapter sends is one the binary declares",
            installed.adapter
        );
    } else {
        for fault in &faults {
            println!("{}: {}", installed.adapter, fault.render());
        }
    }
    if installed.on_pin() && faults.is_empty() {
        RunExit::Ok.code()
    } else {
        // A gap, not a refusal: the question was answered, and the answer is that a run launched
        // now would not work. `2` would say metaharness could not find out.
        RunExit::Gap.code()
    }
}

fn capabilities(args: &CapabilitiesArgs) -> i32 {
    let descriptor = match metaharness::capabilities(args.kind) {
        Ok(descriptor) => descriptor,
        Err(refusal) => return refuse(&refusal),
    };
    if args.render {
        for (operation, tool) in &descriptor.rendering {
            match tool {
                Some(tool) => println!("{operation:<16} -> {tool}"),
                // A `None` is a fact worth publishing rather than an omission: the vendor has no
                // tool for that operation, and a frame that admits it will get nothing.
                None => println!("{operation:<16} -> (no vendor tool)"),
            }
        }
        return RunExit::Ok.code();
    }
    match serde_json::to_string_pretty(&descriptor) {
        Ok(json) => {
            println!("{json}");
            RunExit::Ok.code()
        }
        Err(error) => refuse(&Refusal::Io {
            detail: error.to_string(),
        }),
    }
}

fn conformance(kind: Kind, contract: bool) -> i32 {
    let vectors = match metaharness::conformance_vectors(kind) {
        Ok(vectors) => vectors,
        Err(refusal) => return refuse(&refusal),
    };
    let failed = vectors.iter().filter(|vector| !vector.passed).count();
    if contract {
        // The record and nothing else on stdout, so a consumer pipes it straight into the
        // evidence shape it already reads. The exit code still carries the verdict. A warning
        // vector goes to stderr: the record's shape is shared vocabulary and does not grow a
        // field here, but a named gap that contract mode swallowed would be a silent pass
        // (CT-3's acceptance clause is exactly this line).
        for vector in vectors.iter().filter(|vector| vector.is_warning()) {
            eprintln!("metaharness: {}", render_vector(vector));
        }
        match metaharness::contract_result(kind, &vectors) {
            Ok(record) => println!("{record}"),
            Err(refusal) => return refuse(&refusal),
        }
    } else {
        for vector in &vectors {
            println!("{}", render_vector(vector));
        }
        println!(
            "{} vectors, {failed} failed — no model, no network, no credential",
            vectors.len()
        );
    }
    if failed == 0 {
        RunExit::Ok.code()
    } else {
        RunExit::Gap.code()
    }
}

/// One vector's line. The detail is printed on a failure because the whole point of a vector is
/// that a failure says what differed — and on a warning, because a named gap that is not
/// rendered is a silent pass wearing a different type (CT-3).
fn render_vector(vector: &VectorOutcome) -> String {
    if vector.is_warning() {
        return format!(
            "warn {} {} — {}",
            vector.tier.as_str(),
            vector.id,
            vector.detail
        );
    }
    let mark = if vector.passed { "pass" } else { "FAIL" };
    if vector.passed {
        format!("{mark} {} {}", vector.tier.as_str(), vector.id)
    } else {
        format!(
            "{mark} {} {} — {}",
            vector.tier.as_str(),
            vector.id,
            vector.detail
        )
    }
}

/// Print a refusal and give back exit `2`.
///
/// Every refusal is `2` and none of them is a verdict: a caller that treated `2` as a red run
/// would be reading a setup failure as evidence (design § 9.4). The run-start control refusals
/// are printed as **event lines** as well as prose, because the design says they are emitted.
fn refuse(refusal: &Refusal) -> i32 {
    for emission in refusal.emissions() {
        if let Ok(json) = serde_json::to_string(&emission.event) {
            println!("{json}");
        }
    }
    eprintln!("metaharness: {refusal}");
    RunExit::Broken.code()
}

#[cfg(test)]
mod steering_tests {
    use super::*;
    use metaharness::protocol::{
        Command, CommandLine, CommandOutcome, CredentialSource, DecidedBy, DecisionMode, Event,
        RefusalCode, Refused, parse_command_bytes, parse_command_line,
    };
    use metaharness::{
        HarnessProcess, LaunchPlanView, ManualClock, ProcessRunner, ScriptStep, ScriptedLog,
        ScriptedRunner, ScriptedSeams,
    };
    use std::sync::mpsc::{Sender, channel};

    /// One line on the channel `drive` reads steering from.
    type Steered = Vec<u8>;

    /// The child, which puts these steering lines on the channel `drive` reads stdin from the
    /// first time it goes quiet, as a controller does once it has seen what it must answer.
    struct AtDecision {
        process: Box<dyn HarnessProcess>,
        sender: Option<Sender<Steered>>,
        lines: Vec<Steered>,
    }

    impl HarnessProcess for AtDecision {
        fn next_line(&mut self) -> std::io::Result<Option<String>> {
            self.process.next_line()
        }
        fn poll_line(&mut self) -> std::io::Result<metaharness::ProcessPoll> {
            let result = self.process.poll_line();
            if (matches!(result, Ok(metaharness::ProcessPoll::Idle))
                || result
                    .as_ref()
                    .is_err_and(|error| error.kind() == std::io::ErrorKind::WouldBlock))
                && let Some(sender) = self.sender.take()
            {
                for line in &self.lines {
                    sender.send(line.clone()).unwrap();
                }
            }
            result
        }
        fn write_line(&mut self, line: &str) -> std::io::Result<()> {
            self.process.write_line(line)
        }
        fn kill(&mut self) -> std::io::Result<()> {
            self.process.kill()
        }
        fn wait(&mut self) -> std::io::Result<Option<i32>> {
            self.process.wait()
        }
    }

    struct AtDecisionRunner {
        script: ScriptedRunner,
        lines: Vec<Steered>,
        sender: Option<Sender<Steered>>,
    }

    impl ProcessRunner for AtDecisionRunner {
        fn requires_executable(&self) -> bool {
            false
        }
        fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
            Ok(Box::new(AtDecision {
                process: self.script.start(plan)?,
                sender: self.sender.take(),
                lines: self.lines.clone(),
            }))
        }
    }

    /// The one line a late-steer case sends: this command, framed under the id `late-steer`.
    fn late_steer(command: Command) -> Vec<Steered> {
        vec![serde_json::to_vec(&CommandLine::new("late-steer", command)).unwrap()]
    }

    #[test]
    fn pending_decision_steering_does_not_wait_for_the_entire_deadline() {
        let (sender, commands) = channel();
        let log = ScriptedLog::new();
        let clock = ManualClock::new();
        let mut runner = AtDecisionRunner {
            script: ScriptedRunner::new(
                vec![
                    ScriptStep::line(r#"{"emit":"session.started"}"#),
                    ScriptStep::line(
                        r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
                    ),
                    ScriptStep::awaiting("t1"),
                ],
                log,
            ),
            sender: Some(sender),
            lines: late_steer(Command::Halt {
                reason: "fixture".into(),
            }),
        };
        let mut run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::None)
            .with_decisions(DecisionMode::Ask)
            .start_with_clock(
                Input::Prompt("fixture".into()),
                &mut runner,
                &mut ScriptedSeams,
                Box::new(clock.clone()),
            )
            .unwrap();
        drive(&mut run, &commands).unwrap();
        assert!(
            !run.events().iter().any(|event| matches!(
                event,
                Event::ToolDecided {
                    decided_by: DecidedBy::Deadline,
                    ..
                }
            )),
            "stdin steering was held until the decision deadline"
        );
        assert_eq!(
            clock.reading_ms(),
            0,
            "steering must not consume the decision budget"
        );
    }
    #[test]
    fn quiet_output_delivers_halt_and_interrupt_before_the_next_record() {
        for command in [
            Command::Halt {
                reason: "fixture".into(),
            },
            Command::Interrupt {
                reason: "fixture".into(),
            },
        ] {
            let name = command.name();
            let (sender, commands) = channel();
            let log = ScriptedLog::new();
            let mut runner = AtDecisionRunner {
                script: ScriptedRunner::new(
                    vec![
                        ScriptStep::Idle,
                        ScriptStep::line(r#"{"emit":"session.ended","is_error":false}"#),
                    ],
                    log.clone(),
                ),
                sender: Some(sender),
                lines: late_steer(command),
            };
            let mut run = Metaharness::new(Kind::Claude)
                .with_credentials(CredentialSource::None)
                .with_decisions(DecisionMode::Observe)
                .start_with(
                    Input::Prompt("fixture".into()),
                    &mut runner,
                    &mut ScriptedSeams,
                )
                .unwrap();
            drive(&mut run, &commands).unwrap();
            if name == "interrupt" {
                assert!(
                    !log.killed(),
                    "a wire-bearing seam retains its own interrupt routing"
                );
            }
            assert!(log.written().iter().any(|line| {
                serde_json::from_str::<serde_json::Value>(line).unwrap()["control"] == name
            }));
            assert_eq!(run.events().iter().filter(|event| matches!(event, Event::CommandResult { id, .. } if id == "late-steer")).count(), 1);
            assert_eq!(
                run.events()
                    .iter()
                    .filter(|event| matches!(event, Event::StreamClosed { .. }))
                    .count(),
                1
            );
            assert!(matches!(
                run.events().last(),
                Some(Event::StreamClosed { .. })
            ));
        }
    }

    /// Amendment a44: a steering line that does not parse is answered on the event stream with a
    /// `MALFORMED` refusal in the parser's own words, under the id it carried or under `-`. A
    /// blank line is no command and gets no answer. Silence would leave a controller waiting on
    /// an id that never comes back (design D9).
    #[test]
    fn a_steering_line_that_does_not_parse_is_answered_malformed_under_its_id_or_a_dash() {
        // The flat decision the commands page once showed. `Decision` is internally tagged, so
        // the decoder takes `"decision":{"decision":"allow"}` and refuses this.
        let flat = r#"{"format":"metaharness.command/1","id":"c1","command":"tool.decide","call_id":"t1","decision":"allow"}"#;
        let no_id = r#"{"format":"metaharness.command/1","command":"halt","reason":"no id"}"#;
        let number_id =
            r#"{"format":"metaharness.command/1","id":7,"command":"halt","reason":"a number"}"#;
        let other_format =
            r#"{"format":"metaharness.command/2","id":"c3","command":"halt","reason":"x"}"#;
        let nested = r#"{"format":"metaharness.command/1","id":"c2","command":"tool.decide","call_id":"t1","decision":{"decision":"allow"}}"#;
        // `ö` as one Latin-1 byte, which is not UTF-8: answered under `-`, whatever id it holds.
        let latin1: &[u8] =
            b"{\"format\":\"metaharness.command/1\",\"id\":\"c4\",\"command\":\"steer\",\"text\":\"L\xf6sung\"}";
        let mut lines: Vec<Steered> = [flat, "", "   ", "not json", no_id, number_id, other_format]
            .map(|line| line.as_bytes().to_vec())
            .to_vec();
        lines.push(latin1.to_vec());
        lines.push(nested.as_bytes().to_vec());
        let (sender, commands) = channel();
        let mut runner = AtDecisionRunner {
            script: ScriptedRunner::new(
                vec![
                    ScriptStep::line(r#"{"emit":"session.started"}"#),
                    ScriptStep::line(
                        r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#,
                    ),
                    ScriptStep::awaiting("t1"),
                    ScriptStep::line(r#"{"emit":"session.ended","is_error":false}"#),
                ],
                ScriptedLog::new(),
            ),
            sender: Some(sender),
            lines,
        };
        let mut run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::None)
            .with_decisions(DecisionMode::Ask)
            .start_with_clock(
                Input::Prompt("fixture".into()),
                &mut runner,
                &mut ScriptedSeams,
                Box::new(ManualClock::new()),
            )
            .unwrap();
        drive(&mut run, &commands).unwrap();

        let answered: Vec<(String, CommandOutcome)> = run
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::CommandResult { id, outcome } => Some((id.clone(), outcome.clone())),
                _ => None,
            })
            .collect();
        let malformed = |line: &str| CommandOutcome::Refused {
            refused: Refused::new(
                RefusalCode::Malformed,
                parse_command_line(line)
                    .expect_err("the line does not parse")
                    .to_string(),
            ),
        };
        assert_eq!(
            answered,
            vec![
                ("c1".to_string(), malformed(flat)),
                ("-".to_string(), malformed("not json")),
                ("-".to_string(), malformed(no_id)),
                ("-".to_string(), malformed(number_id)),
                ("c3".to_string(), malformed(other_format)),
                (
                    "-".to_string(),
                    CommandOutcome::Refused {
                        refused: Refused::new(
                            RefusalCode::Malformed,
                            parse_command_bytes(latin1)
                                .expect_err("not UTF-8")
                                .to_string(),
                        ),
                    }
                ),
                ("c2".to_string(), CommandOutcome::Ok { applies_at: None }),
            ],
            "one answer per non-blank line, in the order read, and none for the two blank lines"
        );
        let decided: Vec<&DecidedBy> = run
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::ToolDecided { decided_by, .. } => Some(decided_by),
                _ => None,
            })
            .collect();
        assert_eq!(
            decided,
            vec![&DecidedBy::Embedder],
            "the refused flat line decided nothing; the nested one decided the call"
        );
    }

    /// Amendment a44: the stdin reader reads on past a line that is not UTF-8. It ends at the end
    /// of input and nowhere else, so a `halt` written after such a line is still read. Line
    /// endings and trailing blanks are trimmed as before; a blank line is passed on, and
    /// `send_line` answers it with nothing. A line that is not UTF-8 is passed on byte for byte:
    /// a reader that repaired it would turn the run's "not UTF-8" answer into a JSON error.
    #[test]
    fn the_stdin_reader_reads_on_past_a_line_that_is_not_utf8() {
        let input = b"{\"id\":\"a\"}\r\nL\xf6sung \nsecond\n\n  \t\nlast".to_vec();
        let lines: Vec<Steered> = steering_from(std::io::Cursor::new(input)).iter().collect();
        let expected: [&[u8]; 6] = [
            b"{\"id\":\"a\"}",
            b"L\xf6sung",
            b"second",
            b"",
            b"",
            b"last",
        ];
        assert_eq!(
            lines,
            expected.map(<[u8]>::to_vec),
            "every line is read, in order, to the end of input, and none is rewritten"
        );
    }

    /// The child, which puts these steering lines on the channel `drive` reads stdin from while
    /// the run waits for it to exit: lines a controller wrote as the child went away.
    struct AtWindUp {
        process: Box<dyn HarnessProcess>,
        sender: Option<Sender<Steered>>,
        lines: Vec<Steered>,
    }

    impl HarnessProcess for AtWindUp {
        fn next_line(&mut self) -> std::io::Result<Option<String>> {
            self.process.next_line()
        }
        fn poll_line(&mut self) -> std::io::Result<metaharness::ProcessPoll> {
            self.process.poll_line()
        }
        fn write_line(&mut self, line: &str) -> std::io::Result<()> {
            self.process.write_line(line)
        }
        fn kill(&mut self) -> std::io::Result<()> {
            self.process.kill()
        }
        fn wait(&mut self) -> std::io::Result<Option<i32>> {
            if let Some(sender) = self.sender.take() {
                for line in &self.lines {
                    sender.send(line.clone()).unwrap();
                }
            }
            self.process.wait()
        }
    }

    struct AtWindUpRunner {
        script: ScriptedRunner,
        lines: Vec<Steered>,
        sender: Option<Sender<Steered>>,
    }

    impl ProcessRunner for AtWindUpRunner {
        fn requires_executable(&self) -> bool {
            false
        }
        fn start(&mut self, plan: &LaunchPlanView) -> std::io::Result<Box<dyn HarnessProcess>> {
            Ok(Box::new(AtWindUp {
                process: self.script.start(plan)?,
                sender: self.sender.take(),
                lines: self.lines.clone(),
            }))
        }
    }

    /// Amendment a44 § 4 and a17 rule 1: steering lines that reach the channel while the run
    /// waits for its child to exit are answered, once each, **before** `stream.closed`, and the
    /// marker is the last line.
    #[test]
    fn a_steering_line_read_while_the_run_winds_up_is_answered_before_the_marker() {
        let flat = r#"{"format":"metaharness.command/1","id":"b1","command":"tool.decide","call_id":"ghost","decision":"allow"}"#;
        let ghost = r#"{"format":"metaharness.command/1","id":"g1","command":"tool.decide","call_id":"ghost","decision":{"decision":"allow"}}"#;
        let (sender, commands) = channel();
        let mut runner = AtWindUpRunner {
            script: ScriptedRunner::new(
                vec![
                    ScriptStep::line(r#"{"emit":"session.started"}"#),
                    ScriptStep::line(r#"{"emit":"session.ended","is_error":false}"#),
                ],
                ScriptedLog::new(),
            ),
            lines: [flat, ghost].map(|line| line.as_bytes().to_vec()).to_vec(),
            sender: Some(sender),
        };
        let mut run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::None)
            .with_decisions(DecisionMode::Ask)
            .start_with_clock(
                Input::Prompt("fixture".into()),
                &mut runner,
                &mut ScriptedSeams,
                Box::new(ManualClock::new()),
            )
            .unwrap();
        drive(&mut run, &commands).unwrap();

        let names: Vec<&str> = run.events().iter().map(Event::name).collect();
        assert_eq!(
            names.last(),
            Some(&"stream.closed"),
            "the marker is the last line: {names:?}"
        );
        let answered: Vec<(String, Option<RefusalCode>)> = run
            .events()
            .iter()
            .filter_map(|event| match event {
                Event::CommandResult { id, outcome } => Some((
                    id.clone(),
                    match outcome {
                        CommandOutcome::Refused { refused } => Some(refused.code),
                        CommandOutcome::Ok { .. } => None,
                    },
                )),
                _ => None,
            })
            .collect();
        assert_eq!(
            answered,
            vec![
                ("b1".to_string(), Some(RefusalCode::Malformed)),
                ("g1".to_string(), Some(RefusalCode::UnknownCall)),
            ],
            "both lines are answered, once each: {names:?}"
        );
        assert!(
            metaharness::protocol::stream_completeness(run.events()).is_complete(),
            "{:?}",
            metaharness::protocol::stream_completeness(run.events())
        );
    }
}
