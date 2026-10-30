//! `--scratch-name` on the run verb (`story:scratch-root-stable-by-declared-name`, design
//! amendment a72): the help names it, and the value reaches `RunSpec::scratch_name` unchanged.
use std::process::Command;

use clap::Parser as _;
use metaharness_cli::{Cli, Verb};

#[test]
fn the_run_verbs_help_names_scratch_name() {
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["run", "--help"])
        .output()
        .expect("the binary runs");
    assert!(output.status.success(), "{output:?}");
    let help = String::from_utf8(output.stdout).expect("UTF-8 help");
    assert!(help.contains("--scratch-name <NAME>"), "{help}");
}

#[test]
fn the_scratch_name_flag_reaches_the_run_spec() {
    let cli = Cli::try_parse_from(["metaharness", "run", "b10x", "--scratch-name", "case-7"])
        .expect("parses");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    assert_eq!(args.spec.scratch_name.as_deref(), Some("case-7"));

    let cli = Cli::try_parse_from(["metaharness", "run", "claude"]).expect("parses");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    assert_eq!(args.spec.scratch_name, None);

    let cli = Cli::try_parse_from(["metaharness", "run", "codex", "--scratch-name", "a/b"])
        .expect("the shape is the library's to refuse, as every face refuses it");
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    let refusal = metaharness::check_spec(&args.spec).expect_err("not one plain component");
    assert!(
        refusal
            .to_string()
            .starts_with(metaharness::SCRATCH_NAME_INVALID),
        "{refusal}"
    );
}
