---
format: aep.planning-md/3
id: story:b10x-api-key-from-keychain
kind: story
status: draft
title: The b10x kind names its API key by a secret name
revision: 3
---
# Story: the b10x kind names its API key by a secret name

## Scheduling

Draft, approved 2026-10-22 by `b10x-chef-orchestrator` for after wave 7 and release 0.13.0; it changes
neither's scope. Source: a takt trial of 2026-10-21 (harness-arm changes, row 3), which needs it for the
takt harness arm on a hosted route. Replaces `story:b10x-api-key-from-file` (archived): the operator
decided on 2026-10-22 that the key lives in the operator's secret store (keychain backend), never in a file.

Waits on a harness release whose loop accepts `--api-key-secret <namespace>/<name>` (filed in the harness
repository the same day). Until that release exists and `metaharness-b10x` pins it, this story cannot be
built.

## Outcome

`metaharness run b10x --credentials api-key` can name the key by a secret name, a namespace and a name
such as `llm/opencode-go-api-key`. metaharness passes only that name to the loop as
`--api-key-secret <namespace>/<name>`, the loop resolves it from the secret store, and neither metaharness
nor the child environment ever holds the key.

## Context

- Today `CredentialSource::ApiKey` on the b10x kind always reads a variable:
  `crates/metaharness/src/builder.rs:1963-1972` ends in `launch.from_environment(B10X_API_KEY_VARIABLE)`,
  and `B10X_API_KEY_VARIABLE` is `OPENAI_API_KEY` (`builder.rs:2122`). The key then sits in the child's
  environment, which takt's rule forbids.
- `Credential` in `crates/metaharness-b10x/src/launch.rs:138-170` has a file form and a variable form for
  the key and nothing that names a secret; its doc comment says "Four variants and no fifth". A fifth
  variant for the secret name renders `--api-key-secret`, and that comment changes with it.
- The subscription route already carries a credential reference on `RunSpec`
  (`crates/metaharness-protocol/src/spec.rs:333-337`); the secret name takes the same shape: a name in an
  argv, never a value.

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

## Scope

`crates/metaharness-protocol/src/spec.rs` (`RunSpec`), `crates/metaharness/src/builder.rs` (the b10x
credential branch), `crates/metaharness-b10x/src/launch.rs` (`Credential` and the argv), the harness pin
(invariant 10), `website/docs/reference/cli.md`, an amendment to `docs/design/metaharness-protocol-v0.1.md`,
`CHANGELOG.md` (to be confirmed by the story scoper).
