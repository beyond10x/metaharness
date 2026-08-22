//! The `metaharness` binary.
//!
//! `metaharness run <kind> …` drives one harness session: protocol events on stdout as JSON
//! lines, steering commands on stdin. The verb surface is decided by the protocol design and
//! this is the placeholder that keeps `--help` honest about what exists.

use clap::{Parser, Subcommand};

/// One interface to many agent harnesses.
#[derive(Parser)]
#[command(name = "metaharness", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Verb,
}

/// What the binary can do.
#[derive(Subcommand)]
enum Verb {
    /// Run a harness session. Unimplemented: the protocol design is not yet accepted.
    Run,
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Verb::Run => {
            eprintln!(
                "unimplemented: the protocol design (docs/design/metaharness-protocol-v0.1.md) \
                 is not yet accepted"
            );
            std::process::exit(2);
        }
    }
}
