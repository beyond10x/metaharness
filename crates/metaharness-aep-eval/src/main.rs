//! The paid AEP evaluation runner.

use clap::Parser as _;

fn main() {
    std::process::exit(metaharness_aep_eval::execute(
        metaharness_aep_eval::Cli::parse(),
    ));
}
