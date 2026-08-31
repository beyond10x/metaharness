//! The paid engineering-protocols evaluation runner.

use clap::Parser as _;

fn main() {
    std::process::exit(metaharness_engineering_protocols_eval::execute(
        metaharness_engineering_protocols_eval::Cli::parse(),
    ));
}
