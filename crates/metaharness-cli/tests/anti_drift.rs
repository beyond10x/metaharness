//! The anti-drift test (design D11, finding F16).
//!
//! **Scoped to `run`.** `project` and `audit` carry `--events`, `--transcript` and `--to`, which
//! are not `RunSpec` fields and never will be — a test that claimed to cover "the CLI" could not
//! have meant those verbs. What is asserted is the one thing D11 promises: the `run`
//! subcommand's long-flag set equals the set derived from `RunSpec`, exactly.
//!
//! The first statement of this rule was a paragraph, and the design's own two surfaces had
//! already drifted apart while it was being written. That is the argument for a mechanical test
//! rather than against it.

use std::collections::BTreeSet;

use clap::{Args as _, Command, CommandFactory as _, Parser as _};
use metaharness::protocol::RunSpec;
use metaharness_cli::Cli;

/// The long flags of a command, minus the two `clap` adds for itself.
fn long_flags(command: &Command) -> BTreeSet<String> {
    command
        .get_arguments()
        .filter_map(clap::Arg::get_long)
        .filter(|long| *long != "help" && *long != "version")
        .map(ToString::to_string)
        .collect()
}

#[test]
fn the_run_verbs_long_flags_are_exactly_the_ones_derived_from_the_options_type() {
    let cli = Cli::command();
    let run = cli
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "run")
        .expect("the run verb exists");

    let from_the_binary = long_flags(run);
    let from_the_library = long_flags(&RunSpec::augment_args(Command::new("derived")));

    assert_eq!(
        from_the_binary, from_the_library,
        "a flag the library cannot express cannot be added, and an option the CLI cannot express \
         cannot be introduced"
    );
}

#[test]
fn the_run_verb_carries_every_flag_the_design_lists_for_it() {
    let cli = Cli::command();
    let run = cli
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "run")
        .expect("the run verb exists");
    let flags = long_flags(run);
    for expected in [
        "hermetic",
        "frame",
        "decisions",
        "tool-surface",
        "credentials",
        "model",
        "max-turns",
        "max-budget-usd",
        "plugin-dir",
        "strict-version",
        "audit",
        "spec",
        "auditor",
    ] {
        assert!(flags.contains(expected), "--{expected} is missing");
    }
}

#[test]
fn the_run_verb_takes_the_kind_as_a_positional_and_the_prompt_as_short_p() {
    let cli = Cli::command();
    let run = cli
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "run")
        .expect("the run verb exists");
    assert!(
        run.get_positionals()
            .any(|argument| argument.get_id() == "kind")
    );
    assert!(
        run.get_arguments()
            .any(|argument| argument.get_short() == Some('p'))
    );
}

/// `project` and `audit` are deliberately outside the rule, and the test says so out loud so
/// nobody "fixes" them into it.
#[test]
fn the_verbs_outside_the_rule_carry_flags_run_spec_does_not_have() {
    let cli = Cli::command();
    let project = cli
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "project")
        .expect("the project verb exists");
    let derived = long_flags(&RunSpec::augment_args(Command::new("derived")));
    assert!(long_flags(project).contains("to"));
    assert!(!derived.contains("to"));
}

#[test]
fn every_verb_the_design_names_is_on_the_surface() {
    let cli = Cli::command();
    let verbs: BTreeSet<&str> = cli.get_subcommands().map(clap::Command::get_name).collect();
    for expected in [
        "run",
        "capabilities",
        "conformance",
        "project",
        "audit",
        "doctor",
    ] {
        assert!(verbs.contains(expected), "{expected} is missing");
    }
}

#[test]
fn the_command_line_parses_the_designs_own_example_invocation() {
    let cli = Cli::try_parse_from([
        "metaharness",
        "run",
        "claude",
        "--hermetic",
        "strict",
        "-p",
        "tidy the imports",
        "--decisions",
        "ask",
        "--tool-surface",
        "native",
        "--credentials",
        "operator-login",
        "--model",
        "sonnet",
        "--max-turns",
        "30",
        "--max-budget-usd",
        "5",
        "--plugin-dir",
        "plugins/one",
        "--strict-version",
        "--audit",
        "--spec",
        "eval/expectations.trace.yaml",
        "--auditor",
        "protocol observe trace check",
        "--",
        "--advisory",
        "billed-to-the-session",
    ])
    .expect("the design's own invocation parses");

    let metaharness_cli::Verb::Run(args) = cli.command else {
        panic!("expected the run verb");
    };
    assert_eq!(args.spec.kind.as_str(), "claude");
    assert_eq!(
        args.spec.hermetic,
        metaharness::protocol::HermeticMode::Strict
    );
    assert_eq!(args.spec.prompt.as_deref(), Some("tidy the imports"));
    assert_eq!(args.spec.max_turns, Some(30));
    assert!(args.spec.strict_version);
    assert!(args.spec.audit);
    assert_eq!(
        args.spec.auditor.as_deref(),
        Some("protocol observe trace check")
    );
    assert_eq!(
        args.spec.auditor_args,
        vec![
            "--advisory".to_string(),
            "billed-to-the-session".to_string()
        ]
    );
}

#[test]
fn hermetic_alone_means_on_and_the_default_is_off() {
    let bare = Cli::try_parse_from(["metaharness", "run", "claude"]).expect("parses");
    let metaharness_cli::Verb::Run(args) = bare.command else {
        panic!("expected run")
    };
    assert_eq!(args.spec.hermetic, metaharness::protocol::HermeticMode::Off);

    let flagged =
        Cli::try_parse_from(["metaharness", "run", "claude", "--hermetic"]).expect("parses");
    let metaharness_cli::Verb::Run(args) = flagged.command else {
        panic!("expected run")
    };
    assert_eq!(args.spec.hermetic, metaharness::protocol::HermeticMode::On);
}
