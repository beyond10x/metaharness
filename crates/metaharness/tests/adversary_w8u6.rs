//! Adversary cases for `story:b10x-api-key-from-keychain` (wave 8, unit 6): amendment a50 and the
//! coordinator's decisions D41 (harness 0.18.0) and D42 (the null `expected_sha256` taken out
//! before the call), read against amendment a45 § 2 end to end.
//!
//! No vendor process, no credential, no store: the only secret name is the fixture
//! `llm/opencode-go-api-key`, and the tools-server cases run `/bin/echo` in a scratch tree.
#![cfg(unix)]

use std::path::Path;

use metaharness_tools::{CallerCatalogue, Catalogue, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

/// `process.run` as `tool-contract` 0.4.0 releases it (`contracts/process/v1alpha1`), cut to the
/// fields a call reads: `argv`, and `timeout_ms` as `Optional<Integer>`.
const PROCESS_CATALOGUE: &str = r#"format: tool-contract/1
toolset: process
version: v1alpha1
summary: >-
  Run one program from a declared set. Synthetic, written for metaharness's adversary cases; no
  tool in it is offered for anything else.
errors:
  - name: process.InvalidInput
    summary: The input is not the operation's input. Nothing was started.
operations:
  - name: process.run
    summary: Run one declared program with its arguments and answer its exit status and output.
    renderings: {mcp: process_run}
    input:
      - name: argv
        type: List<String>
        description: The program, then its arguments, one item each.
      - name: timeout_ms
        type: Optional<Integer>
        description: The wall-clock bound in milliseconds.
    output:
      - name: exit
        type: Optional<Integer>
        description: The program's exit code.
      - name: stdout
        type: String
        description: The standard output.
    outcomes:
      - {name: ran, summary: The program ran to its own end or to the bound.}
      - {name: invalid-input, error: process.InvalidInput}
    profile:
      effects: [process, write, network, secret]
      consequences: [money, delete, send-external, irreversible, sensitive-data]
      risk: high
      idempotency: non-idempotent
      preconditions: []
      determinism: nondeterministic
      bounds:
        timeout_ms: {default: 600000, max: 600000}
      requires: {binaries: [], env: [], settings: [programs], secrets: []}
      subjects:
        - {field: argv, kind: argv, pattern: "process.run({argv[0]})"}
      placement: local
      capability: workspace.execute
      output: {untrusted: true, failure_as_result: true}
"#;

fn server(root: &Path) -> Server {
    let catalogue = CallerCatalogue::from_text(Path::new("process.yaml"), PROCESS_CATALOGUE)
        .expect("the synthetic process catalogue reads");
    let operations =
        LocalOperations::unconfined(root, vec!["/bin/echo".to_owned()]).expect("opens");
    Server::with_catalogue(Verbs::new(Catalogue::of(operations)), catalogue)
}

fn call(server: &mut Server, name: &str, arguments: &Value) -> (bool, String) {
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments.clone()},
        }))
        .expect("answered");
    let failed = answer["result"]["isError"]
        .as_bool()
        .unwrap_or_else(|| panic!("a tool result, not a JSON-RPC error: {answer}"));
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("one text block")
        .to_owned();
    (failed, text)
}

/// a45 § 2: "On a struct field that is not `required`, null reads as absent and is accepted."
/// `timeout_ms` is `Optional<Integer>` in the released `process.run`, the caller catalogue admits
/// null for it, and the call is performed as harness `run`, whose schema at 330e2cde declares
/// `argv` alone and is closed. D42 takes out a null `expected_sha256` and nothing else, so the
/// same null on `process.run` reaches harness 0.18.0 and is refused as undeclared.
#[test]
fn a_null_timeout_ms_on_process_run_reads_as_absent() {
    let dir = tempfile::tempdir().expect("a temporary tree");
    let mut server = server(dir.path());

    let (failed, text) = call(
        &mut server,
        "process_run",
        &json!({"argv": ["/bin/echo", "control"]}),
    );
    assert!(!failed, "the control call without timeout_ms runs: {text}");
    assert!(text.contains("control"), "the control ran echo: {text}");

    let (failed, text) = call(
        &mut server,
        "process_run",
        &json!({"argv": ["/bin/echo", "nulled"], "timeout_ms": null}),
    );
    assert!(
        !failed,
        "a null optional field reads as absent (a45 § 2), so the call runs as the control did: {text}"
    );
    assert!(text.contains("nulled"), "echo ran: {text}");
}

/// Kills the `split_once` → `rsplit_once` mutant in `validate_api_key_secret`, which no existing
/// input tells apart: the namespace is the first segment, and an empty first segment is refused
/// even when a later slash leaves a non-empty part on each side.
#[test]
fn a_secret_name_whose_first_segment_is_empty_is_refused() {
    assert!(metaharness_b10x::validate_api_key_secret("/llm/opencode-go-api-key").is_err());
}
