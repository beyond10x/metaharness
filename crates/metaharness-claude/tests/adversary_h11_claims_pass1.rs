//! Adversary pass 1 for `story:h11-claims-match-what-was-measured` (design amendment a36 § 8).
//!
//! The story narrows Claude's H11 claims to what the memory canary measured: each shape planted in
//! the cwd and in its parent, nothing further up. `MEMORY_NOT_PROBED` now says so, and lists memory
//! files above the cwd's parent as unprobed. These cases look for the same surface stated as
//! measured somewhere else a reader or a record meets it: the H11 reason an operator-named `--cwd`
//! run renders, the Claude harness page, and the research page the rows cite.
//!
//! The launch case plans a value: no vendor process, no credential, no network.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use metaharness_claude::{LaunchContext, MEMORY_NOT_PROBED, plan_launch};
use metaharness_protocol::{Digest, HermeticMode, HermeticRow, Kind, RunSpec};

fn context(cwd: &str) -> LaunchContext {
    LaunchContext {
        context_files: Vec::new(),
        catalogue: None,
        scratch_root: PathBuf::from("/scratch/run-9"),
        cwd: PathBuf::from(cwd),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([("HOME".to_string(), "/operator".to_string())]),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"inputs")),
        plugins: Vec::new(),
        marketplace_plugins: Vec::new(),
        loopback: None,
        tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
        managed_settings: Vec::new(),
    }
}

/// A file of the repository, its whitespace folded to single spaces so a sentence reads the same
/// wherever the source wraps it.
fn page(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{relative}: {error}"));
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The H11 reason of a strict run pointed at the operator's own directory (`--cwd` without
/// `--scratch-cwd`) cites the canary result as the reason the launch still keeps memory out. It
/// may cite only what the canary measured: the cwd and its parent. `MEMORY_NOT_PROBED` says memory
/// files above the parent were not probed, so "in and above the cwd" claims the surface the same
/// unit lists as unprobed. Every such run renders this reason in its H11 row.
#[test]
fn an_operator_cwd_h11_reason_claims_the_canary_only_for_the_cwd_and_its_parent() {
    assert!(
        MEMORY_NOT_PROBED.contains("memory files above the cwd's parent"),
        "the premise: the unit lists the ancestors above the parent as unprobed"
    );
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("go".to_string());
    spec.cwd = Some(PathBuf::from("/operator/repo"));
    let plan = plan_launch(&spec, &context("/operator/repo")).expect("an operator cwd plans");
    let why = plan
        .attestation
        .unavailable
        .iter()
        .find(|control| control.row == HermeticRow::H11)
        .map(|control| control.why.clone())
        .expect("H11 is attested unavailable under an operator cwd");
    assert!(
        why.contains("2.1.304, 2.1.305 and 2.1.309"),
        "the reason cites the canary result: {why}"
    );
    assert!(
        !why.contains("above the cwd out of every request"),
        "the reason says memory files above the cwd were kept out at the pins, and the canary \
         probed only the cwd and its parent: {why}"
    );
    assert!(
        why.contains("parent"),
        "the reason names how far up the canary reached: {why}"
    );
}

/// The Claude harness page lists memory files above the cwd's parent as not probed, and the same
/// paragraph opens by saying no memory file in the working directory or above it reached the
/// model at each pin. The second is the claim the story narrows.
#[test]
fn the_claude_harness_page_does_not_say_every_ancestor_was_measured() {
    let text = page("website/docs/harnesses/claude.md");
    assert!(
        text.contains("memory files above the cwd's parent"),
        "the premise: the page lists the ancestors above the parent as not probed"
    );
    assert!(
        !text.contains("no memory file in the working directory or above it reached the model"),
        "the page says every ancestor was measured and, in the same paragraph, that ancestors \
         above the parent were not probed"
    );
}

/// The research page the 2.1.304 and 2.1.305 rows cite concludes that no memory file in the cwd
/// or an ancestor reaches the model, and, three paragraphs on, that memory files above the cwd's
/// parent were not probed. The record line the row is held to sits between the two.
#[test]
fn the_cited_research_page_does_not_conclude_for_every_ancestor() {
    let text = page("docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md");
    assert!(
        text.contains("memory files above the cwd's parent"),
        "the premise: the page lists the ancestors above the parent as not probed"
    );
    assert!(
        !text.contains("no memory file of any shape in the cwd or an ancestor reaches the model"),
        "the page concludes for every ancestor and records that only the parent was probed"
    );
}
