---
format: aep.planning-md/3
id: review-result:adr0096-w3-contract-names-adversary-1
kind: review-result
status: active
title: 'Adversary pass 1 on story:metaharness-tools-adopts-tool-contract: 1 warning, 1 note, two red cases'
relations:
- reviews: story:metaharness-tools-adopts-tool-contract
revision: 1
---
unit: story:metaharness-tools-adopts-tool-contract, pass 1 — uncommitted tree wt-c6ce0a7152d6 (branch impl/metaharness-tools-resolves-contract-names, base 34c427b)
verdict: CONFIRMED
cases: executed 888→890, red 2
origin: introduced 2 / pre-existing 0 / undecided 0
wrote-outside-worktree: 5 paths (metaharness-wave-adr0096/w3/u3-adv1 under the operator's cache directory)
needs-coordinator: whether the dropped `expected_sha256` guard holds the unit (warning) or goes to its own story

## Cases added (untracked, `crates/metaharness-tools/tests/adversary_contract_name_guards.rs`)

| case | asserts | now |
|---|---|---|
| `a_contract_write_carrying_a_digest_the_file_does_not_have_changes_nothing` | `tool_invoke code.write {path, text, expected_sha256: <stale>}` leaves the file unchanged and answers `isError: true` (`precondition-failed` in tool-contract/1) | red |
| `a_contract_edit_carrying_a_digest_the_file_does_not_have_changes_nothing` | the same for `code.edit {old, new, expected_sha256}` | red |

```
assertion `left == right` failed: `code.edit` with a stale `expected_sha256` is `precondition-failed` under tool-contract/1 and changes nothing; the served surface edited the file. Answer: {"id":1,"jsonrpc":"2.0","result":{"content":[{"text":"{\n  \"bytes\": 12,\n  \"path\": \"README.md\"\n}","type":"text"}],"isError":false}}
  left: "bye harness\n"
 right: "hello harness\n"
assertion `left == right` failed: `code.write` with a stale `expected_sha256` ... the served surface wrote the file. Answer: {...,"isError":false}}
  left: "replaced\n"
 right: "hello harness\n"
test result: FAILED. 0 passed; 2 failed; 0 ignored
```

Suite with the cases: 888 passed, 2 failed (these), 37 ignored. rustfmt and clippy on the file pass.

## Attacked and not broken

Argument spellings (contract keys `path`, `old`, `new`, `text`, `glob`, `max_results` equal harness's;
subjects and invoke both read `path`); a bare top-level `tools/call {"name":"fs.read"}` is refused (read at
source); a withheld entry under a contract name is refused naming its harness entry; `fs.glob` maps to
`find`, which a frame always denies; case, whitespace and empty variants stay Unknown on both sides; the
end-to-end case cannot pass for the wrong reason; removing the translation from any one of resolve,
subjects, invoke or describe turns a unit test red; a32 and changelog citations match `f0870131`. Not
checked: base behaviour by running it, the b10x path through the binary.

## Findings

```findings
- file: crates/metaharness-tools/src/server.rs
  line: 120
  category: contract-drift
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: a tool_invoke naming code.write or code.edit with the contract's expected_sha256 is performed with the precondition silently dropped, so a stale-digest write lands where tool-contract/1 says nothing changes
- file: crates/metaharness-tools/src/resolve.rs
  line: 102
  category: judgement
  severity: note
  verdict: CONFIRMED
  origin: introduced
  message: on a b10x record a contract-named tool_invoke the 0.15.1 loop refused now reads as its entry's operation and subject, contrary to the rule at resolve.rs:105-107 that a call which never reached a tool puts no effect in the record
```

Agent cost: 150,337 tokens, 72 tool uses, 731 s.
