//! `--plugin` on the command line: where the refusals land, and how early.
//!
//! Amendment a16, `docs/design/runs-side-by-side-v0.1.md` § 3. Nothing here spawns: the unpinned
//! refusal happens **at parse**, before `execute` is ever called, and the kind refusal happens in
//! `check_spec`, before a launch is planned.

use clap::Parser as _;
use metaharness::protocol::{Kind, RunSpec};
use metaharness_cli::{Cli, Verb};

fn plugin_argv(spelling: &str) -> Vec<String> {
    ["metaharness", "run", "claude", "--plugin", spelling]
        .iter()
        .map(ToString::to_string)
        .collect()
}

/// G1 — a pinned spelling parses onto the one options type, so the library and the binary express
/// the same thing (design D11, and `tests/anti_drift.rs` asserts the sets are equal).
#[test]
fn a_pinned_plugin_reaches_the_run_spec() {
    let cli = Cli::try_parse_from(plugin_argv("bdfinst/agentic-dev-team@dev-team@1.4.0"))
        .expect("a pinned spelling parses");
    let Verb::Run(args) = cli.command else {
        panic!("expected the run verb");
    };
    assert_eq!(args.spec.plugin.len(), 1);
    assert_eq!(args.spec.plugin[0].repo, "bdfinst/agentic-dev-team");
    assert_eq!(args.spec.plugin[0].name, "dev-team");
    assert_eq!(args.spec.plugin[0].pin, "1.4.0");
}

/// G1 — **an unpinned one never becomes a `RunSpec` at all.** The earliest possible refusal: the
/// value parser rejects it, so nothing downstream has to remember to check.
#[test]
fn an_unpinned_plugin_is_refused_at_parse_and_the_message_says_why() {
    let error = Cli::try_parse_from(plugin_argv("bdfinst/agentic-dev-team@dev-team"))
        .expect_err("an unpinned spelling is refused");
    let said = error.to_string();
    assert!(said.contains("no pin"), "{said}");
    assert!(said.contains("<repo>@<name>@<version-or-commit>"), "{said}");
}

/// The other kinds have no marketplace, and a declaration this build could not act on is refused
/// by name rather than accepted and ignored.
#[test]
fn a_marketplace_plugin_is_refused_by_name_on_a_kind_that_has_no_marketplace() {
    for kind in [Kind::Codex, Kind::B10x] {
        let mut spec = RunSpec::new(kind);
        spec.plugin
            .push("owner/repo@thing@1.0.0".parse().expect("parses"));
        let refusal = metaharness::check_spec(&spec).expect_err("refused");
        assert!(
            matches!(
                refusal,
                metaharness::Refusal::MarketplacePluginUnsupported { .. }
            ),
            "{kind:?}: {refusal}"
        );
        assert!(refusal.to_string().contains("no marketplace"));
    }

    let mut claude = RunSpec::new(Kind::Claude);
    claude
        .plugin
        .push("owner/repo@thing@1.0.0".parse().expect("parses"));
    assert!(
        metaharness::check_spec(&claude).is_ok(),
        "claude is the one that takes one"
    );
}

/// The `run` verb's flag set is derived from `RunSpec`, so `--plugin` arriving there is not a
/// second surface — but a reader of this file should not have to go and check.
#[test]
fn the_run_verb_carries_the_flag() {
    use clap::CommandFactory as _;
    let cli = Cli::command();
    let run = cli
        .get_subcommands()
        .find(|subcommand| subcommand.get_name() == "run")
        .expect("the run verb exists");
    assert!(
        run.get_arguments()
            .filter_map(clap::Arg::get_long)
            .any(|long| long == "plugin")
    );
}
