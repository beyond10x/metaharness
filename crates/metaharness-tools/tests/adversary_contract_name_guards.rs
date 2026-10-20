//! Adversary cases, wave 3 of 2026-10-20, unit `story:metaharness-tools-adopts-tool-contract`.
//!
//! Amendment a32 and `CHANGELOG.md` `[Unreleased]` say a `tool-contract/1` command name "stands
//! for" the harness entry harness maps it to, and `metaharness mcp-serve` performs a `tool_invoke`
//! naming one as that entry. The contract those names come from (tools `0.2.0`, `60d4d0e3`,
//! `contracts/code-editor/v1alpha1/contract.yaml`) gives `code.write` and `code.edit` an optional
//! `expected_sha256` input with `preconditions: [expected_sha256]` and an outcome
//! `precondition-failed` whose error reads *"The file's digest is not `expected_sha256`; nothing
//! was changed."* (lines 52, 78, 97, 119, 143).
//!
//! Harness `0.15.1` builds every `WriteInput` and `EditInput` with `expected_sha256: None`
//! (`f0870131:crates/harness-catalogue/src/local.rs:520,543`) and its catalogue never reads the
//! field (`catalogue.rs:476-483`). The story's Out of scope declines to publish the contract schema
//! for exactly that reason: "advertising it would promise a guard nobody runs". These cases drive
//! the served surface with the contract name **and** the contract's own guard, carrying a digest
//! the file does not have, and assert the contract's outcome: nothing changed, and the caller is
//! told so. On the base (`34c427b`) both names were unknown and refused, so the file stayed as it
//! was and both cases passed; after the change the call is performed and the guard is dropped.

use metaharness_tools::{Catalogue, INVOKE_VERB, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

/// The SHA-256 of the empty string: a real digest, and not the one `README.md` has.
const STALE_DIGEST: &str = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";

const ORIGINAL: &str = "hello harness\n";

fn workspace() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary tree");
    std::fs::write(dir.path().join("README.md"), ORIGINAL).expect("a file");
    dir
}

/// The server `metaharness mcp-serve --writable` builds (`metaharness-cli/src/lib.rs:332,341`).
fn writable(root: &std::path::Path) -> Server {
    Server::new(Verbs::new(Catalogue::of(
        LocalOperations::unconfined(root, Vec::new()).expect("opens"),
    )))
}

fn invoke(server: &mut Server, entry: &str, arguments: &Value) -> Value {
    server
        .handle(
            &json!({"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
                "name": INVOKE_VERB, "arguments": {"name": entry, "arguments": arguments}
            }}),
        )
        .expect("answered")
}

fn on_disk(dir: &tempfile::TempDir) -> String {
    std::fs::read_to_string(dir.path().join("README.md")).expect("still a file")
}

#[test]
fn a_contract_write_carrying_a_digest_the_file_does_not_have_changes_nothing() {
    let dir = workspace();
    let mut server = writable(dir.path());

    let answer = invoke(
        &mut server,
        "code.write",
        &json!({"path": "README.md", "text": "replaced\n", "expected_sha256": STALE_DIGEST}),
    );

    assert_eq!(
        on_disk(&dir),
        ORIGINAL,
        "`code.write` with a stale `expected_sha256` is `precondition-failed` under \
         tool-contract/1 and changes nothing; the served surface wrote the file. Answer: {answer}"
    );
    assert_eq!(answer["result"]["isError"], true, "{answer}");
}

#[test]
fn a_contract_edit_carrying_a_digest_the_file_does_not_have_changes_nothing() {
    let dir = workspace();
    let mut server = writable(dir.path());

    let answer = invoke(
        &mut server,
        "code.edit",
        &json!({"path": "README.md", "old": "hello", "new": "bye", "expected_sha256": STALE_DIGEST}),
    );

    assert_eq!(
        on_disk(&dir),
        ORIGINAL,
        "`code.edit` with a stale `expected_sha256` is `precondition-failed` under \
         tool-contract/1 and changes nothing; the served surface edited the file. Answer: {answer}"
    );
    assert_eq!(answer["result"]["isError"], true, "{answer}");
}
