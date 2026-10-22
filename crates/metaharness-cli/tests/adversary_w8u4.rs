//! Adversary cases for wave 8 unit 4 (amendment a48): the documents that describe an audited run's
//! exit, held to what the report prints and to what the wire carries.
//!
//! No process is started and no model is called.

use std::fs;
use std::path::{Path, PathBuf};

use metaharness::protocol::ProcessTermination;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

fn read(relative: &str) -> String {
    fs::read_to_string(workspace_root().join(relative))
        .unwrap_or_else(|error| panic!("reading {relative}: {error}"))
}

/// The table row whose first cell starts with `first_cell`.
fn row<'a>(page: &'a str, first_cell: &str, what: &str) -> &'a str {
    page.lines()
        .find(|line| line.starts_with(first_cell))
        .unwrap_or_else(|| panic!("{what} has no row starting {first_cell:?}"))
}

/// README § Status enumerates what `run --audit` prints. Since a48 the report ends with three
/// lines a caller reads the exit from; a row that lists the floor, the census and the `stream:`
/// line and stops there tells a reader of the README that a `3` is all there is.
#[test]
fn adversary_w8u4_the_readme_run_audit_row_names_the_three_verdict_lines() {
    let readme = read("README.md");
    let audit = row(&readme, "| `run --audit` |", "README.md");
    let missing: Vec<&str> = ["`child exit:`", "`control verdict:`", "`audit verdict:`"]
        .into_iter()
        .filter(|line| !audit.contains(line))
        .collect();
    assert!(
        missing.is_empty(),
        "README.md's `run --audit` row does not name {missing:?}: {audit}"
    );
}

/// The binary's own exit table says "Exactly § 9.4, for every verb". § 9.4, the CLI reference and
/// a48 all put the external auditor's exit `3` on the row for `3`; the doc comment does not.
#[test]
fn adversary_w8u4_the_binarys_exit_table_puts_the_auditor_on_the_row_for_three() {
    let source = read("crates/metaharness-cli/src/lib.rs");
    let three = row(&source, "//! | `3` |", "crates/metaharness-cli/src/lib.rs");
    assert!(
        three.contains("auditor"),
        "the doc comment's row for `3` omits the auditor that exited `3`, which § 9.4 and \
         website/docs/reference/cli.md name: {three}"
    );
    let cli = read("website/docs/reference/cli.md");
    let documented = row(&cli, "| `3` |", "website/docs/reference/cli.md");
    assert!(
        documented.contains("auditor exited `3`"),
        "precondition: the page names it: {documented}"
    );
}

/// The events page says what `stream.closed.process` carries. A consumer parses the wire, so every
/// key and every word the serializer writes for `process` must be on the row; the tag key the
/// variant name sits under is part of the wire.
#[test]
fn adversary_w8u4_the_events_page_names_every_wire_token_of_process() {
    let page = read("website/docs/protocol/events.md");
    let closed = row(
        &page,
        "| `stream.closed` |",
        "website/docs/protocol/events.md",
    );
    let mut tokens = Vec::new();
    for termination in [
        ProcessTermination::Exited { code: 0 },
        ProcessTermination::Signaled { signal: 9 },
        ProcessTermination::Unknown,
    ] {
        let value = serde_json::to_value(termination).expect("a termination serializes");
        for (key, field) in value.as_object().expect("an object") {
            tokens.push(key.clone());
            if let Some(word) = field.as_str() {
                tokens.push(word.to_owned());
            }
        }
    }
    tokens.sort();
    tokens.dedup();
    let missing: Vec<String> = tokens
        .iter()
        .filter(|token| !closed.contains(&format!("`{token}`")))
        .cloned()
        .collect();
    assert!(
        missing.is_empty(),
        "the `stream.closed` row does not name the wire tokens {missing:?} of `process` \
         (serialized: {tokens:?}): {closed}"
    );
}
