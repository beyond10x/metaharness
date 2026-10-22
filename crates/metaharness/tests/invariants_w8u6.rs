//! Independent invariant review of wave 8 unit 6 (`story:b10x-api-key-from-keychain`, amendment a50, D42).
//!
//! No vendor process, no store, no credential: the lock file is read as text, and the tool server
//! is the production one over a temporary workspace.

use metaharness_tools::{Catalogue, INVOKE_VERB, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

const LOCK: &str = include_str!("../../../Cargo.lock");

/// Brief item 1: metaharness never opens the secret store, so nothing in the workspace's lock is a
/// Secret Service, keyring or D-Bus client. A harness crate that started pulling the loop's store
/// client into this workspace would put the store one call away from metaharness.
#[test]
fn the_lock_resolves_no_secret_store_client() {
    const CLIENTS: [&str; 9] = [
        "zbus",
        "zvariant",
        "secret-service",
        "keyring",
        "oo7",
        "dbus",
        "zbus-secret-service-keyring-store",
        "llm-credentials",
        "secrets-core",
    ];
    let names: Vec<&str> = LOCK
        .lines()
        .filter_map(|line| line.strip_prefix("name = \""))
        .filter_map(|rest| rest.strip_suffix('"'))
        .collect();
    assert!(names.len() > 50, "the lock was read: {}", names.len());
    for client in CLIENTS {
        assert!(
            !names.contains(&client),
            "{client} is in the workspace lock"
        );
    }
}

/// Brief item 4: one harness revision, the 0.18.0 commit, and one tools source, tag 0.4.0.
#[test]
fn the_lock_names_harness_330e2cde_and_tools_0_4_0_and_nothing_else() {
    let sources: std::collections::BTreeSet<&str> = LOCK
        .lines()
        .filter_map(|line| line.strip_prefix("source = \"git+"))
        .collect();
    let harness: Vec<&&str> = sources
        .iter()
        .filter(|source| source.contains("beyond10x/harness"))
        .collect();
    let tools: Vec<&&str> = sources
        .iter()
        .filter(|source| source.contains("beyond10x/tools"))
        .collect();
    assert_eq!(
        harness,
        [
            &"https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde6299e4e55b05664001e73903959a076f\""
        ],
    );
    assert_eq!(
        tools,
        [
            &"https://github.com/beyond10x/tools?tag=0.4.0#6017fccc5a0110f0f8f1da5f0a56570825cf95b5\""
        ],
    );
}

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary tree");
    std::fs::write(dir.path().join("README.md"), "hello harness\n").expect("a file");
    dir
}

fn invoke(server: &mut Server, name: &str, arguments: &Value) -> Value {
    server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": {"name": INVOKE_VERB, "arguments": {"name": name, "arguments": arguments}},
        }))
        .expect("answered")
}

/// Brief item 5 (D42): the server takes out a null `expected_sha256` before the call. No digest
/// that is present and not null, of any JSON kind and on either spelling of the entry, ends in a
/// write: the file is byte-for-byte what it was.
#[test]
fn no_present_digest_ends_in_an_unchecked_write() {
    let digests = [
        json!("ca8380141fe6ae546862de4378a80805c5c86eaf8f67a6b91cea383f1bee7527"),
        json!("0000000000000000000000000000000000000000000000000000000000000000"),
        json!(""),
        json!("null"),
        json!(false),
        json!(0),
        json!({}),
        json!([]),
        json!([null]),
        json!({"value": null}),
    ];
    for digest in digests {
        for (name, arguments) in [
            (
                "code.write",
                json!({"path": "README.md", "text": "replaced\n", "expected_sha256": digest}),
            ),
            (
                "code.edit",
                json!({"path": "README.md", "old": "hello", "new": "bye", "expected_sha256": digest}),
            ),
            (
                "file_write",
                json!({"path": "README.md", "text": "replaced\n", "expected_sha256": digest}),
            ),
            (
                "file_edit",
                json!({"path": "README.md", "old": "hello", "new": "bye", "expected_sha256": digest}),
            ),
        ] {
            let dir = tree();
            let operations = LocalOperations::unconfined(dir.path(), Vec::new()).expect("opens");
            let mut server = Server::new(Verbs::new(Catalogue::of(operations)));
            let answer = invoke(&mut server, name, &arguments);
            assert_eq!(
                std::fs::read_to_string(dir.path().join("README.md")).expect("on disk"),
                "hello harness\n",
                "{name} with expected_sha256 {digest} wrote without the check: {answer}"
            );
            assert_eq!(
                answer["result"]["isError"], true,
                "{name} with {digest}: {answer}"
            );
        }
    }
}

/// D42's other half, the control: a null digest on the contract name is the call without the
/// field, and only on the contract name; on harness's own entry name the null is not taken out,
/// and harness refuses the undeclared field with nothing written.
#[test]
fn a_null_digest_is_dropped_on_the_contract_name_only() {
    let dir = tree();
    let operations = LocalOperations::unconfined(dir.path(), Vec::new()).expect("opens");
    let mut server = Server::new(Verbs::new(Catalogue::of(operations)));
    let answer = invoke(
        &mut server,
        "file_write",
        &json!({"path": "README.md", "text": "replaced\n", "expected_sha256": null}),
    );
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).expect("on disk"),
        "hello harness\n"
    );
    let answer = invoke(
        &mut server,
        "code.write",
        &json!({"path": "README.md", "text": "replaced\n", "expected_sha256": null}),
    );
    assert_ne!(answer["result"]["isError"], true, "{answer}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).expect("on disk"),
        "replaced\n"
    );
}
