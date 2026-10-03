//! Credential-free static documentation builder.
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(about = "Build and validate the independent Metaharness documentation")]
struct Cli {
    #[command(subcommand)]
    action: Action,
}

#[derive(Subcommand)]
enum Action {
    /// Check the complete site, local links, anchors and assets without writing output.
    Check,
    /// Build a static site with exact source commit provenance.
    Build {
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        commit: String,
    },
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    match cli.action {
        Action::Check => metaharness_docs::check()?,
        Action::Build { out, commit } => metaharness_docs::build(&out, &commit)?,
    }
    Ok(())
}
