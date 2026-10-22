---
format: aep.planning-md/3
id: story:b10x-api-key-from-keychain
kind: story
status: implemented
title: The b10x kind names its API key by a secret name
scope:
- confidence: cited
  path: CHANGELOG.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: crates/metaharness-aep-eval/src/lib.rs
- confidence: cited
  path: crates/metaharness-b10x/Cargo.toml
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/README.md
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/loop.expected.jsonl
- confidence: cited
  path: crates/metaharness-b10x/fixtures/golden/version.txt
- confidence: cited
  path: crates/metaharness-b10x/src/launch.rs
- confidence: cited
  path: crates/metaharness-b10x/src/lib.rs
- confidence: cited
  path: crates/metaharness-b10x/tests/adversary_native_cli_exits.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness-tools/Cargo.toml
- confidence: cited
  path: crates/metaharness-tools/src/tests.rs
- confidence: cited
  path: crates/metaharness/fixtures/golden/contract-result-b10x.json
- confidence: cited
  path: crates/metaharness/src/audit.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/doctor.rs
- confidence: cited
  path: docs/design/metaharness-protocol-v0.1.md
- confidence: cited
  path: docs/research/2026-10-22-b10x-harness-0.17.1-adapter-surface.md
- confidence: cited
  path: spec/coverage.md
- confidence: cited
  path: website/docs/harnesses/b10x.md
- confidence: cited
  path: website/docs/hermetic.md
- confidence: cited
  path: website/docs/reference/cli.md
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-22T02:17:44Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-22T02:17:44Z", actor: "human:timo", revision: 7}
- {from: "active", to: "implemented", at: "2026-10-22T07:46:31Z", actor: "human:timo", revision: 8, decided_on: {"recorded":{"test_result":1,"review_outcome":1}}}
---
# Story: the b10x kind names its API key by a secret name

## Scheduling

Approved 2026-10-22 by `b10x-chef-orchestrator` for wave 8, unit u6, after harness 0.17.1 shipped
`--api-key-secret` (https://github.com/beyond10x/harness/releases/tag/0.17.1, tag on commit
`c5cf4de50685e671e41f0ec85ecf2e3b58da90ee`). It needs the key for a harness arm on a hosted route.
Replaces `story:b10x-api-key-from-file` (archived): the operator decided on 2026-10-22 that the key lives
in the operator's secret store (keychain backend), never in a file.

The pin moves in this story from harness 0.15.1 (`f0870131e58df3e1119860d0a255f46589026d62`) to 0.17.1.
The pin's one-turn `--strict-version` run waits for the orchestrator's word; no live call before it.

## Outcome

`metaharness run b10x --credentials api-key` can name the key by a secret name, a namespace and a name
such as `llm/opencode-go-api-key`. metaharness passes only that name to the loop as
`--api-key-secret <namespace>/<name>`, the loop resolves it from the secret store, and neither metaharness
nor the child environment ever holds the key.

## Context

- Today `CredentialSource::ApiKey` on the b10x kind always reads a variable:
  `crates/metaharness/src/builder.rs:1963-1972` ends in `launch.from_environment(B10X_API_KEY_VARIABLE)`,
  and `B10X_API_KEY_VARIABLE` is `OPENAI_API_KEY` (`builder.rs:2122`). The key then sits in the child's
  environment.
- `Credential` in `crates/metaharness-b10x/src/launch.rs:138-170` has a file form and a variable form for
  the key and nothing that names a secret; its doc comment says "Four variants and no fifth". A fifth
  variant for the secret name renders `--api-key-secret`, and that comment changes with it.
- The subscription route already carries a credential reference on `RunSpec`
  (`crates/metaharness-protocol/src/spec.rs:333-337`); the secret name takes the same shape: a name in an
  argv, never a value.
- Harness side at `c5cf4de` (story scoper, 2026-10-22; line numbers to be re-read by the implementor):
  the flag on `run`, `chat`, `workflow run` and `app-server` (`crates/harness-cli/src/lib.rs:510-519`,
  `:605-616`), conflicting with the other four credential flags; resolution per request through
  `harness-llm` (`crates/harness-llm/src/lib.rs:140-196`); `credential_source` recorded at
  `crates/harness-cli/src/lib.rs:1234-1241`; the catalogue mapping gains `search` and `run`
  (`crates/harness-catalogue/src/local.rs`); the tool-contract crates move to tag `0.4.0`.

## Acceptance

- expected_before: `metaharness run b10x --credentials api-key` with a secret name is refused by clap as
  an unknown flag. After: given `llm/opencode-go-api-key`, the loop's argv carries
  `--api-key-secret llm/opencode-go-api-key`, no `--api-key-env` and no `--api-key-file`, and the child
  environment holds no key variable.
- A secret name that is not `<namespace>/<name>` with both parts non-empty is refused before the spawn,
  naming the value it was given.
- A secret name with any credential source other than `api-key`, or beside a subscription source, is
  refused before the spawn, in the words of the existing two-credentials refusal.
- `--credentials api-key` without a secret name still reads `OPENAI_API_KEY` by name, as before.
- On the Claude and Codex kinds the secret name is refused by name before the spawn until each is designed.
- metaharness never opens the secret store; no event, refusal or record carries the key.
- The tests use a fixture name and never read a real secret store.
- The harness pin is 0.17.1 at every site invariant 10 names, and the lock test still resolves exactly one
  harness revision and one tools revision.

## Decisions (coordinator, 2026-10-22)

- D24: the catalogue mapping change that arrives with the pin (`search`, `run`) is accepted; the tests in
  `crates/metaharness-tools/src/tests.rs` follow the harness.
- D25: the design amendment is a50.
- D26: `b10x-tool-contract` moves from tag `0.2.0` to `0.4.0` in this unit; the one-tools-revision lock
  test requires it.
- D27: a research note `docs/research/2026-10-22-b10x-harness-0.17.1-adapter-surface.md` cites each
  0.17.1 claim as harness file:line at `c5cf4de`; a runtime claim nobody has observed is labelled
  unverified (invariant 4).
- D28: one unit, built after u1 and u4 merge into the integration branch (shared `tests.rs`, `caller.rs`,
  `audit.rs`, `cli.md`).
- D29: the child environment stays cleared. On the secret route only, the loop also receives the
  variables the pinned Secret Service client reads to find the session bus, each cited at its pinned
  version as file:line; nothing else is forwarded. a50 and `website/docs/hermetic.md` state it. Whether
  the store resolves under the floor is unverified until the live run.
- D30: audit H4 accepts the secret route by the exact `credential_source` string the harness records,
  read from its source and asserted in a test.
- D31: goldens that only need the binary's identity (`version.txt`, `--version` banners) are re-recorded
  from the released 0.17.1 Linux archive after its `SHA256SUMS` check, with no model call. A golden that
  needs a model call keeps its 0.15.1 bytes, and the research note says so until the live run.

## Scope

- `crates/metaharness-protocol/src/spec.rs`: `RunSpec` gains `api_key_secret: Option<String>`
  (struct :172, subscription fields :327-344, `RunSpec::new` :611-613).
- `crates/metaharness/src/builder.rs`: `start_b10x` :527, environment insertion :619-633, subscription
  refusal :1898-1907, `b10x_credential` :1925, ApiKey arm :1963-1972, `B10X_API_KEY_VARIABLE` :2122,
  tests near :3700-3725.
- `crates/metaharness-b10x/src/launch.rs`: `Credential` :149 and its doc :140-146, constructors :277,
  :286 and a new `from_secret`, `argv` :514 and the credential match :562-582, `base_environment`
  :53-61 (D29).
- The pin: `crates/metaharness-b10x/src/lib.rs` `PINNED_VERSIONS` :96, `HARNESS_REVISION` :103, tests
  :305-370; `crates/metaharness-b10x/Cargo.toml` :14, :18; `crates/metaharness-tools/Cargo.toml` :14-16;
  `Cargo.lock` (regenerated by cargo).
- `crates/metaharness-tools/src/tests.rs` `MAPPED` :426-436 and :605-625 (D24).
- Goldens and version asserts: `crates/metaharness-b10x/fixtures/golden/` (`version.txt`, `README.md`,
  `loop.expected.jsonl`), `crates/metaharness-b10x/tests/adversary_native_cli_exits.rs:358`,
  `crates/metaharness/fixtures/golden/contract-result-b10x.json` (D31).
- `crates/metaharness/src/doctor.rs` :117-125, :351, :404-419; `crates/metaharness-aep-eval/src/lib.rs`
  :1146-1166, :2180-2186; `crates/metaharness/src/audit.rs` :606-660 (D30).
- `docs/design/metaharness-protocol-v0.1.md` (a50), the research note (D27), `website/docs/reference/cli.md`,
  `website/docs/harnesses/b10x.md`, `website/docs/hermetic.md`, `spec/coverage.md`, `CHANGELOG.md`.
