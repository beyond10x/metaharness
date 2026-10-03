//! One repeated CLI option reaches the shared public `RunSpec` without becoming a file glob.
use clap::Parser as _;
use metaharness_cli::{Cli, Verb};

#[test]
fn repeated_process_directories_reach_the_shared_run_spec() {
    let cli = Cli::try_parse_from([
        "metaharness",
        "run",
        "b10x",
        "--substrate-embedded",
        "--process-write-subtree",
        "target",
        "--process-write-subtree",
        "generated",
        "--write-scope",
        "**=allowed",
    ])
    .unwrap();
    let Verb::Run(args) = cli.command else {
        panic!("run");
    };
    assert_eq!(args.spec.process_write_subtree, ["target", "generated"]);
    assert_eq!(args.spec.write_scope, ["**=allowed"]);
    assert!(metaharness::check_spec(&args.spec).is_ok());
}

#[test]
fn absent_process_declaration_is_empty_and_unsupported_use_is_named() {
    for kind in ["b10x", "codex", "claude"] {
        let cli = Cli::try_parse_from(["metaharness", "run", kind]).unwrap();
        let Verb::Run(args) = cli.command else {
            panic!("run");
        };
        assert_eq!(args.spec.process_write_subtree, [] as [String; 0]);
        let cli = Cli::try_parse_from([
            "metaharness",
            "run",
            kind,
            "--process-write-subtree",
            "target",
        ])
        .unwrap();
        let Verb::Run(args) = cli.command else {
            panic!("run");
        };
        let error =
            metaharness::check_spec(&args.spec).expect_err("confinement required, b10x only");
        assert!(
            error.to_string().contains("--process-write-subtree"),
            "{error}"
        );
    }
}
