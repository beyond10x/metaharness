//! `METAHARNESS_LIVE` is documented as the guard of the commands that check it, and no wider.
//!
//! The variable is read by `metaharness aep drive run` and `metaharness aep drive resume` for a
//! step map with an `llm` step (`crates/metaharness-aep/src/drive/spending.rs`), by
//! `metaharness aep drive eval run` (`crates/metaharness-aep/src/eval.rs`), by `aep-eval native
//! --spend` and `aep-eval driven --spend` (`crates/metaharness-aep-eval/src/lib.rs`) and by the
//! ignored live tests. `metaharness run` does not read it: it is the embedder's launch command and
//! spawns the harness with the operator's credentials whatever the variable holds. A page that
//! states the variable as the guard of live runs in general tells a reader that a plain `run` is
//! safe to type, so every published page that names the variable says that `run` does not check it.

use std::fs;
use std::path::{Path, PathBuf};

const VARIABLE: &str = "METAHARNESS_LIVE";
const RUN_IS_UNGUARDED: &str = "plain `metaharness run` does not check `METAHARNESS_LIVE`";
const CHECKING_COMMAND: &str = "`metaharness aep drive run`";

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

fn markdown_under(directory: &Path, found: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(directory)
        .unwrap_or_else(|error| panic!("reading {}: {error}", directory.display()))
        .map(|entry| entry.expect("a directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            markdown_under(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "md") {
            found.push(path);
        }
    }
}

/// `README.md` and every page of the published site, relative to the workspace root.
fn documents() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut found = vec![root.join("README.md")];
    markdown_under(&root.join("website/docs"), &mut found);
    found
}

/// The text with every run of whitespace collapsed to one space, so a sentence wrapped across
/// lines still matches.
fn flattened(path: &Path) -> String {
    fs::read_to_string(path)
        .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn relative(path: &Path) -> String {
    path.strip_prefix(workspace_root())
        .unwrap_or(path)
        .display()
        .to_string()
}

#[test]
fn every_page_that_names_the_live_variable_says_a_plain_run_does_not_check_it() {
    let mut unscoped = Vec::new();
    for path in documents() {
        let text = flattened(&path);
        if !text.contains(VARIABLE) {
            continue;
        }
        if !text.contains(RUN_IS_UNGUARDED) {
            unscoped.push(format!(
                "{}: missing \"{RUN_IS_UNGUARDED}\"",
                relative(&path)
            ));
        }
        if !text.contains(CHECKING_COMMAND) {
            unscoped.push(format!(
                "{}: names no checking command such as {CHECKING_COMMAND}",
                relative(&path)
            ));
        }
    }
    assert!(
        unscoped.is_empty(),
        "a page names {VARIABLE} as a guard without scoping it to the commands that check it:\n{}",
        unscoped.join("\n")
    );
}

#[test]
fn the_readme_and_the_status_page_name_plain_run_beside_the_live_variable() {
    let root = workspace_root();
    for page in ["README.md", "website/docs/status.md"] {
        let text = flattened(&root.join(page));
        assert!(text.contains(VARIABLE), "{page} no longer names {VARIABLE}");
        assert!(
            text.contains(RUN_IS_UNGUARDED),
            "{page} names {VARIABLE} without saying \"{RUN_IS_UNGUARDED}\""
        );
    }
}
