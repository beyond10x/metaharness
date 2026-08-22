//! The `metaharness` binary.
//!
//! Everything is in the library beside this file, so an integration test can read the real verb
//! surface instead of a copy of it. This is the process boundary and nothing else.

use clap::Parser as _;

fn main() {
    std::process::exit(metaharness_cli::execute(metaharness_cli::Cli::parse()));
}
