//! Adversary case, pass 2, wave 3 of 2026-10-20, unit `story:b10x-adapter-on-harness-over-llm`.
//!
//! Harness 0.13.3 answered a path outside the workspace in two ways: *resolves outside the
//! workspace and was refused* when something was there, and *No such file or directory* when
//! nothing was. The second reply told the model whether a path outside its workspace exists. At
//! 0.15.0 `b10x-tool-fs` and `b10x-tool-code-editor` judge a missing path by where its deepest
//! existing ancestor lands, so both answers are the same refusal. Observed on both releases with
//! the same calls (adversary pass 2 probe); none of the implementor's 70 calls named an outside
//! path. This pins the 0.15.0 behaviour so a later pin cannot reopen it unnoticed.

use metaharness_tools::{Catalogue, INVOKE_VERB, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

fn invoke(server: &mut Server, entry: &str, arguments: &Value) -> (bool, String) {
    let answer = server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": INVOKE_VERB, "arguments": {"name": entry, "arguments": arguments}
            }}),
        )
        .expect("answered");
    (
        answer["result"]["isError"] == true,
        answer["result"]["content"][0]["text"]
            .as_str()
            .expect("one text block")
            .to_owned(),
    )
}

#[test]
fn a_missing_path_outside_the_workspace_is_refused_exactly_as_an_existing_one() {
    let parent = tempfile::tempdir().expect("a parent");
    let workspace = parent.path().join("workspace");
    std::fs::create_dir(&workspace).expect("a workspace");
    std::fs::write(parent.path().join("exists.txt"), "outside\n").expect("seed");
    let mut server = Server::new(Verbs::new(Catalogue::of(
        LocalOperations::unconfined(&workspace, Vec::new()).expect("opens"),
    )));
    let absent = format!("{}/absent.txt", parent.path().display());
    let present = format!("{}/exists.txt", parent.path().display());

    for (entry, key, extra) in [
        ("file_read", "path", json!({})),
        ("dir_list", "path", json!({})),
        ("find", "path", json!({"glob": "*"})),
        ("file_edit", "path", json!({"old": "outside", "new": "x"})),
    ] {
        let mut replies = Vec::new();
        for path in [
            "../absent.txt",
            "../exists.txt",
            absent.as_str(),
            present.as_str(),
        ] {
            let mut arguments = extra.clone();
            arguments[key] = json!(path);
            let (failed, said) = invoke(&mut server, entry, &arguments);
            assert!(failed, "{entry} {path} was served: {said}");
            replies.push((path.to_owned(), said));
        }
        for (path, said) in &replies {
            assert_eq!(
                said,
                &format!("`{path}` resolves outside the workspace and was refused"),
                "{entry}: the reply tells a missing outside path from an existing one"
            );
        }
    }
    assert_eq!(
        std::fs::read_to_string(parent.path().join("exists.txt")).expect("the outside file"),
        "outside\n"
    );
}
