//! Source-backed admission and exact native argv for explicit process write directories.
use metaharness_b10x::{B10xLaunch, argv, validate_process_write_subtrees};

fn strings(paths: &[&str]) -> Vec<String> {
    paths.iter().map(|p| (*p).to_owned()).collect()
}

#[test]
fn only_explicit_process_directories_reach_native_argv() {
    let launch = B10xLaunch::new("http://fixture.invalid", "fixture", "/work", "fixture")
        .with_write_scope("**=allowed");
    assert!(
        !argv(&launch)
            .iter()
            .any(|arg| arg == "--process-write-subtree")
    );
    let declared = launch
        .with_process_write_subtree("target")
        .with_process_write_subtree("generated");
    let argv = argv(&declared);
    let paths: Vec<&str> = argv
        .windows(2)
        .filter(|pair| pair[0] == "--process-write-subtree")
        .map(|pair| pair[1].as_str())
        .collect();
    assert_eq!(paths, ["target", "generated"]);
    assert!(
        metaharness_b10x::emitted_flags().contains(&("--process-write-subtree".to_owned(), true))
    );
}

#[test]
fn admission_matches_native_directory_bounds_without_interpreting_file_globs() {
    for paths in [
        vec![],
        strings(&["target"]),
        strings(&["target", "generated"]),
        strings(&["target", "target"]),
        strings(&["target-a", "target"]),
        strings(&["build output", "nested/out"]),
    ] {
        assert!(validate_process_write_subtrees(&paths).is_ok(), "{paths:?}");
    }
    for path in [
        "",
        "/",
        "/tmp",
        ".",
        "..",
        "../target",
        "target/../other",
        "target/./other",
        "target//other",
        "target/",
        "target\\other",
        "target\0other",
        "target/**",
        "out?",
        "[out]",
        "{out}",
    ] {
        let refused =
            validate_process_write_subtrees(&strings(&[path])).expect_err("invalid declaration");
        assert!(refused.contains("--process-write-subtree"), "{refused}");
    }
    for paths in [
        strings(&["target", "target/debug"]),
        strings(&["target/debug", "target"]),
        (0..65).map(|n| format!("out{n}")).collect(),
        vec![vec!["x"; 65].join("/")],
    ] {
        assert!(
            validate_process_write_subtrees(&paths).is_err(),
            "{paths:?}"
        );
    }
    assert!(
        validate_process_write_subtrees(&(0..64).map(|n| format!("out{n}")).collect::<Vec<_>>())
            .is_ok()
    );
    assert!(validate_process_write_subtrees(&[vec!["x"; 64].join("/")]).is_ok());
}
