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
//! | `3` | nobody found out — a gating row is `unk`, or the harness died without a record |

use std::sync::mpsc::{Receiver, RecvTimeoutError};
use std::time::Duration;

use clap::{Args, Parser, Subcommand};
use metaharness::protocol::{Kind, RunSpec, VectorOutcome, parse_command_line};
use metaharness::{Input, Metaharness, ProcessAuditor, Refusal, Run, RunExit};

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
    /// Run a harness session: events as JSON lines on stdout, commands as JSON lines on stdin.
    Run(RunArgs),
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
    /// vendor and `metaharness.event/1`, in the shape `engineering-protocols` reads.
    #[arg(long)]
    pub contract: bool,
}

/// `project --events <f> --to trace-ir`.
#[derive(Args, Debug)]
pub struct ProjectArgs {
    /// The event stream to project.
    #[arg(long, value_name = "FILE")]
    pub events: std::path::PathBuf,
    /// The target form.
    #[arg(long, default_value = "trace-ir")]
    pub to: String,
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
    /// bound on an effect is `b10x-harness-tools`' own path arithmetic. A run that wants the
    /// effects actually confined is a b10x run against substrate, not this.
    #[arg(long)]
    pub writable: bool,
    /// A program the served `run` may start. Repeatable, and an empty set publishes no `run`.
    ///
    /// Declared rather than open, because an argv whose program could be anything is the shell
    /// this surface exists to remove.
    #[arg(long, value_name = "PROGRAM")]
    pub allow_program: Vec<String>,
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
        Verb::Run(args) => run(args.spec),
        Verb::Capabilities(args) => capabilities(&args),
        Verb::Conformance(args) => conformance(args.kind, args.contract),
        Verb::Project(_) => refuse(&Refusal::NotInThisMilestone {
            verb: "project",
            // Q9: `trace-ir/1` is a Serialize-only Rust type with no published schema, so a
            // document written here has no reader. The projection is an in-process value until
            // that changes (design D6a).
            missing: "a readable `trace-ir/1` document form, which is gated on Q9",
        }),
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

/// `mcp-serve` — the owned tool surface, on stdin and stdout, until the client goes away.
fn mcp_serve(args: &McpServeArgs) -> i32 {
    use metaharness_tools::{Catalogue, LocalOperations, Server, Verbs, serve};

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

    let mut server = Server::new(Verbs::new(Catalogue::of(operations)));
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
fn run(spec: RunSpec) -> i32 {
    let mut run = match Metaharness::from_spec(spec).start(Input::FromSpec) {
        Ok(run) => run,
        Err(refusal) => return refuse(&refusal),
    };
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
fn steering() -> Receiver<String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut line = String::new();
        loop {
            line.clear();
            match std::io::BufRead::read_line(&mut std::io::stdin().lock(), &mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    if sender.send(line.trim_end().to_string()).is_err() {
                        break;
                    }
                }
            }
        }
    });
    receiver
}

/// Events out, commands in, until the run ends.
fn drive(run: &mut Run, commands: &Receiver<String>) -> std::io::Result<()> {
    loop {
        while let Ok(line) = commands.try_recv() {
            steer(run, &line)?;
        }
        // A call is pending and the embedder owes an answer, so the loop waits on **stdin**
        // rather than on the child: diving back into the run here would spend that call's whole
        // budget without ever looking for the answer that was already on its way.
        if !run.pending_calls().is_empty() {
            match commands.recv_timeout(STEER_POLL) {
                Ok(line) => {
                    steer(run, &line)?;
                    continue;
                }
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {}
            }
        }
        match run.next_event()? {
            Some(line) => emit(&line),
            None => return Ok(()),
        }
    }
}

/// Apply one steering line, and answer it.
///
/// A line that will not parse is **not** silently dropped: design D9 says a command that can be
/// ignored is a control surface that cannot be tested, so the refusal goes out as an event under
/// the id the caller used, or under `-` when the line was too broken to carry one.
fn steer(run: &mut Run, line: &str) -> std::io::Result<()> {
    if line.trim().is_empty() {
        return Ok(());
    }
    match parse_command_line(line) {
        Ok(parsed) => {
            run.send_as(parsed.id, parsed.command)?;
        }
        Err(error) => {
            eprintln!("metaharness: the steering line was refused: {error}");
        }
    }
    Ok(())
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
fn doctor(kind: Kind) -> i32 {
    match metaharness::installed(kind) {
        Ok(installed) => {
            println!("{}", installed.render());
            if installed.on_pin() {
                RunExit::Ok.code()
            } else {
                // A gap, not a refusal: the question was answered, and the answer is the wrong
                // version. `2` would say metaharness could not find out.
                RunExit::Gap.code()
            }
        }
        Err(refusal) => refuse(&refusal),
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
