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

use clap::{Args, Parser, Subcommand};
use metaharness::protocol::{Kind, RunSpec, VectorOutcome};
use metaharness::{Input, Metaharness, Refusal, RunExit};

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

/// `conformance <kind>`.
#[derive(Args, Debug)]
pub struct ConformanceArgs {
    /// Which harness.
    #[arg(value_enum)]
    pub kind: Kind,
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
        Verb::Conformance(args) => conformance(args.kind),
        Verb::Project(_) => refuse(&Refusal::NotInThisMilestone {
            verb: "project",
            // Q9: `trace-ir/1` is a Serialize-only Rust type with no published schema, so a
            // document written here has no reader. The projection is an in-process value until
            // that changes (design D6a).
            missing: "a readable `trace-ir/1` document form, which is gated on Q9",
        }),
        Verb::Audit(_) => refuse(&Refusal::NotInThisMilestone {
            verb: "audit",
            missing: "an event stream reader for a transcript metaharness did not produce, \
                      which arrives with the real spawner",
        }),
        Verb::Doctor(_) => refuse(&Refusal::NotInThisMilestone {
            verb: "doctor",
            missing: "the vendor binary, which this build never spawns",
        }),
    }
}

/// `run` — the whole point, and the one verb this build cannot finish.
fn run(spec: RunSpec) -> i32 {
    match Metaharness::from_spec(spec).start(Input::FromSpec) {
        Ok(_) => {
            // Unreachable while there is no spawner, and written as a real branch rather than
            // an `unreachable!()` so the day the spawner lands, this is the line that changes.
            eprintln!("the run started and this build has no loop to drive it");
            RunExit::Broken.code()
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

fn conformance(kind: Kind) -> i32 {
    let vectors = match metaharness::conformance_vectors(kind) {
        Ok(vectors) => vectors,
        Err(refusal) => return refuse(&refusal),
    };
    for vector in &vectors {
        println!("{}", render_vector(vector));
    }
    let failed = vectors.iter().filter(|vector| !vector.passed).count();
    println!(
        "{} vectors, {failed} failed — no model, no network, no credential",
        vectors.len()
    );
    if failed == 0 {
        RunExit::Ok.code()
    } else {
        RunExit::Gap.code()
    }
}

/// One vector's line. The detail is printed on a failure because the whole point of a vector is
/// that a failure says what differed.
fn render_vector(vector: &VectorOutcome) -> String {
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
