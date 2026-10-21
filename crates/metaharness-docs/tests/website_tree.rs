//! The `website/` source tree carries no package-manager configuration.
use std::{fs, io::ErrorKind, path::PathBuf};

fn website() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../website")
}

#[test]
fn website_has_no_npm_configuration() {
    assert!(
        !website().join(".npmrc").exists(),
        "website/.npmrc is npm configuration; website/ is static HTML, CSS and docs"
    );
}

#[test]
fn website_gitignore_keeps_only_the_entry_the_docs_builder_writes() {
    let path = website().join(".gitignore");
    let text = match fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == ErrorKind::NotFound => return,
        Err(error) => panic!("{}: {error}", path.display()),
    };
    let entries: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .collect();
    assert_eq!(
        entries,
        ["/build"],
        "website/.gitignore may carry only the /build entry that `metaharness-docs build` writes"
    );
}
