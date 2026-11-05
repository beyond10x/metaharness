//! `story:substrate-host-0-20-confinement-only`: the eval takes Substrate's confinement adapters
//! and nothing else, so the workspace lock carries neither the git stack Substrate's `git` feature
//! pulls in nor the kubernetes client its `kubernetes` feature pulls in.
//!
//! The lock is read as text and matched by whole `name = "<crate>"` lines, so a crate whose name
//! merely starts with one of these (`gix-hash`, `git2-curl`) neither satisfies nor trips the check.

const LOCK: &str = include_str!("../../../Cargo.lock");

/// Every package name `lock` resolves, one per whole `name = "<crate>"` line.
fn names_in(lock: &str) -> Vec<&str> {
    lock.lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .collect()
}

#[test]
fn the_lock_resolves_neither_the_git_nor_the_kubernetes_stack_of_substrate() {
    const ABSENT: [&str; 4] = [
        "git2",
        "gix",
        "curl-sys",
        "b10x-substrate-kubernetes-client",
    ];
    let names = names_in(LOCK);
    assert!(names.len() > 50, "the lock was read: {}", names.len());
    assert!(
        names.contains(&"b10x-substrate-host"),
        "the lock no longer resolves b10x-substrate-host, so this check reads nothing"
    );
    let present: Vec<&str> = ABSENT
        .into_iter()
        .filter(|crate_name| names.contains(crate_name))
        .collect();
    assert!(
        present.is_empty(),
        "the workspace lock resolves {present:?}"
    );
}

/// The matcher is exact: a line naming a crate that only starts with one of the four is not it.
#[test]
fn the_lock_matcher_takes_whole_name_lines_only() {
    let lock = "[[package]]\nname = \"gix-hash\"\n\n[[package]]\nname = \"git2-curl\"\nversion = \"1\"\n\n[[package]]\nname = \"gix\"\n";
    let names = names_in(lock);
    assert_eq!(names, ["gix-hash", "git2-curl", "gix"]);
    assert!(!names.contains(&"git2"));
}
