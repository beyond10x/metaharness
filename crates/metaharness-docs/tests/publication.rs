//! Exercise the actual CLI and its complete static publication output.
use std::{fs, path::PathBuf, process::Command};

fn scratch(label: &str) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("metaharness-docs-{}-{label}", std::process::id()));
    fs::create_dir_all(&root).unwrap();
    root
}

#[test]
fn checked_site_builds_all_thirteen_docs_and_exact_provenance() {
    let out = scratch("publication");
    let commit = "1234567890123456789012345678901234567890";
    let result = Command::new(env!("CARGO_BIN_EXE_metaharness-docs"))
        .args(["build", "--out", out.to_str().unwrap(), "--commit", commit])
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    for page in [
        "",
        "quickstart/",
        "hermetic/",
        "control-seam/",
        "frames/",
        "status/",
        "protocol/events/",
        "protocol/commands/",
        "harnesses/claude/",
        "harnesses/codex/",
        "harnesses/b10x/",
        "reference/cli/",
        "reference/library/",
    ] {
        let html = fs::read_to_string(out.join(format!("docs/{page}index.html"))).unwrap();
        assert!(html.contains("Skip to content"));
        assert!(html.contains("/metaharness/docs/harnesses/b10x/"));
        assert!(!html.contains("<script"));
    }
    let provenance: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".well-known/b10x-site.json")).unwrap()).unwrap();
    assert_eq!(
        provenance,
        serde_json::json!({"schema":"b10x-project-site/v1", "repository":"metaharness", "commit":commit, "baseUrl":"/metaharness/"})
    );
    assert!(out.join("index.html").is_file());
    assert!(out.join("styles.css").is_file());
    fs::remove_dir_all(out).unwrap();
}

#[test]
fn check_runs_the_complete_site_without_output() {
    let result = Command::new(env!("CARGO_BIN_EXE_metaharness-docs"))
        .arg("check")
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}",
        String::from_utf8_lossy(&result.stderr)
    );
    assert!(String::from_utf8_lossy(&result.stdout).contains("13 documentation pages"));
}

#[test]
fn published_inventory_matches_every_legacy_heading_and_only_public_files() {
    let out = scratch("inventory");
    let commit = "1234567890123456789012345678901234567890";
    metaharness_docs::build(&out, commit).unwrap();
    let inventory: serde_json::Value =
        serde_json::from_slice(&fs::read(out.join(".well-known/b10x-routes.json")).unwrap())
            .unwrap();
    assert_eq!(inventory["schema"], "b10x-project-routes/v1");
    assert_eq!(inventory["commit"], commit);
    assert_eq!(inventory["repository"], "metaharness");
    assert_eq!(inventory["baseUrl"], "/metaharness/");
    let routes = inventory["routes"].as_array().unwrap();
    assert_eq!(routes.len(), 14);
    let legacy: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/legacy-anchors.json")).unwrap();
    assert_eq!(legacy["pages"].as_object().unwrap().len(), 13);
    for (path, ids) in legacy["pages"].as_object().unwrap() {
        let published = routes
            .iter()
            .find(|route| route["path"] == path.as_str())
            .unwrap();
        let anchors = published["anchors"].as_array().unwrap();
        for id in ids.as_array().unwrap() {
            assert!(anchors.contains(id), "{path}#{id}");
        }
    }
    let mut files = Vec::new();
    collect_files(&out, &out, &mut files);
    assert_eq!(files.len(), 19);
    assert!(files.iter().all(|file| {
        std::path::Path::new(file)
            .extension()
            .is_some_and(|ext| ext == "html")
            || [
                "styles.css",
                "img/logo.svg",
                ".nojekyll",
                ".well-known/b10x-site.json",
                ".well-known/b10x-routes.json",
            ]
            .contains(&file.as_str())
    }));
    fs::remove_dir_all(out).unwrap();
}

fn collect_files(root: &std::path::Path, directory: &std::path::Path, files: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            collect_files(root, &path, files);
        } else {
            files.push(
                path.strip_prefix(root)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned(),
            );
        }
    }
}
