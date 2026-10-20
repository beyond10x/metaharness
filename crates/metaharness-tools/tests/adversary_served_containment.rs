//! Adversary cases, wave 3 of 2026-10-20, unit `story:b10x-adapter-on-harness-over-llm`.
//!
//! At harness 0.15.0 the served `file_read`, `dir_list` and `find` run on `b10x-tool-fs` and
//! `file_write`, `file_edit` on `b10x-tool-code-editor`, while `search` stays in harness. The only
//! containment case in this workspace was a read (`metaharness.boundaries.FileRead`). These drive
//! every served entry at a path that leaves the workspace — by `..`, by an absolute path, through a
//! link to a directory, a link to a file and a link that leads nowhere — through a writable server,
//! and assert that nothing outside was read, listed, found, created or changed.

use std::path::Path;

use metaharness_tools::{Catalogue, INVOKE_VERB, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

struct Fixture {
    workspace: tempfile::TempDir,
    outside: tempfile::TempDir,
}

const SECRET: &str = "outside-marker\n";

fn fixture() -> Fixture {
    let workspace = tempfile::tempdir().expect("a workspace");
    let outside = tempfile::tempdir().expect("an outside directory");
    std::fs::write(workspace.path().join("inside.txt"), "inside-marker\n").expect("seed");
    std::fs::write(outside.path().join("secret.txt"), SECRET).expect("seed");
    let link = |target: &Path, name: &str| {
        std::os::unix::fs::symlink(target, workspace.path().join(name)).expect("a link");
    };
    link(outside.path(), "dir-link");
    link(&outside.path().join("secret.txt"), "file-link");
    link(&outside.path().join("created-by-link.txt"), "dangling-link");
    Fixture { workspace, outside }
}

fn server(root: &Path) -> Server {
    Server::new(Verbs::new(Catalogue::of(
        LocalOperations::unconfined(root, Vec::new()).expect("opens"),
    )))
}

/// `(isError, text)` as the model reads it.
fn invoke(server: &mut Server, entry: &str, arguments: &Value) -> (bool, String) {
    let answer = server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": INVOKE_VERB, "arguments": {"name": entry, "arguments": arguments}
            }}),
        )
        .expect("answered");
    assert!(
        answer.get("error").is_none(),
        "not a protocol error: {answer}"
    );
    (
        answer["result"]["isError"] == true,
        answer["result"]["content"][0]["text"]
            .as_str()
            .expect("one text block")
            .to_owned(),
    )
}

fn outside_untouched(fixture: &Fixture) {
    let mut names: Vec<String> = std::fs::read_dir(fixture.outside.path())
        .expect("the outside directory")
        .map(|entry| {
            entry
                .expect("an entry")
                .file_name()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    names.sort();
    assert_eq!(names, ["secret.txt"], "nothing was created outside");
    assert_eq!(
        std::fs::read_to_string(fixture.outside.path().join("secret.txt")).expect("the secret"),
        SECRET,
        "nothing outside was changed"
    );
}

#[test]
fn no_served_write_or_edit_lands_outside_the_workspace() {
    let fixture = fixture();
    let mut server = server(fixture.workspace.path());
    let outside = fixture.outside.path().display().to_string();
    let outside_name = fixture
        .outside
        .path()
        .file_name()
        .expect("a name")
        .to_string_lossy()
        .into_owned();

    let writes = [
        "../escape.txt".to_owned(),
        format!("../{outside_name}/secret.txt"),
        format!("{outside}/absolute.txt"),
        format!("{outside}/secret.txt"),
        "dir-link/through-dir.txt".to_owned(),
        "dir-link/secret.txt".to_owned(),
        "file-link".to_owned(),
        "file-link/".to_owned(),
        "file-link/.".to_owned(),
        "dangling-link".to_owned(),
        "missing/../../escape.txt".to_owned(),
    ];
    for path in &writes {
        let (failed, said) = invoke(
            &mut server,
            "file_write",
            &json!({"path": path, "text": "written\n"}),
        );
        assert!(failed, "file_write {path} was served: {said}");
    }
    let edits = [
        format!("../{outside_name}/secret.txt"),
        format!("{outside}/secret.txt"),
        "dir-link/secret.txt".to_owned(),
        "file-link".to_owned(),
        "file-link/".to_owned(),
    ];
    for path in &edits {
        let (failed, said) = invoke(
            &mut server,
            "file_edit",
            &json!({"path": path, "old": "outside-marker", "new": "edited"}),
        );
        assert!(failed, "file_edit {path} was served: {said}");
    }
    outside_untouched(&fixture);
}

#[test]
fn no_served_read_list_find_or_search_answers_from_outside_the_workspace() {
    let fixture = fixture();
    let mut server = server(fixture.workspace.path());
    let outside = fixture.outside.path().display().to_string();

    for path in [
        "file-link".to_owned(),
        "dir-link/secret.txt".to_owned(),
        format!("{outside}/secret.txt"),
        "../".to_owned()
            + fixture
                .outside
                .path()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
            + "/secret.txt",
    ] {
        let (failed, said) = invoke(&mut server, "file_read", &json!({"path": path}));
        assert!(failed, "file_read {path} was served: {said}");
        assert!(
            said.contains("outside the workspace"),
            "file_read {path}: {said}"
        );
        assert!(!said.contains("outside-marker"), "file_read {path}: {said}");
    }
    for path in ["..", "dir-link", "/", outside.as_str()] {
        let (failed, said) = invoke(&mut server, "dir_list", &json!({"path": path}));
        assert!(failed, "dir_list {path} was served: {said}");
        assert!(!said.contains("secret.txt"), "dir_list {path}: {said}");
    }
    for path in ["..", "dir-link", outside.as_str()] {
        let (failed, said) = invoke(&mut server, "find", &json!({"glob": "*", "path": path}));
        assert!(failed, "find under {path} was served: {said}");
    }
    let (failed, said) = invoke(&mut server, "find", &json!({"glob": "**"}));
    assert!(!failed, "a find over the workspace is served: {said}");
    assert!(
        !said.contains("secret.txt"),
        "find walked through a link: {said}"
    );

    for path in ["..", "dir-link", outside.as_str()] {
        let (failed, said) = invoke(
            &mut server,
            "search",
            &json!({"pattern": "outside-marker", "path": path}),
        );
        assert!(failed, "search under {path} was served: {said}");
    }
    let (failed, said) = invoke(&mut server, "search", &json!({"pattern": "outside-marker"}));
    assert!(!failed, "a search over the workspace is served: {said}");
    let answer: Value = serde_json::from_str(&said).expect("JSON");
    assert_eq!(
        answer["matches"],
        json!([]),
        "search read through a link: {said}"
    );
    outside_untouched(&fixture);
}
