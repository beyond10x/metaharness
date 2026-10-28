---
format: aep.planning-md/3
id: story:run-spec-document
kind: story
status: active
title: A session request is a RunSpec document, parsed with unknown keys refused and checked per harness kind
summary: Public parse of a metaharness run settings mapping into RunSpec; unknown keys and bad values refused by name; Codex max-turns and max-budget-usd refused in check_spec
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: crates/metaharness-codex/src/launch.rs
- confidence: cited
  path: crates/metaharness-protocol/src/lib.rs
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: spec/conformance.json
- confidence: inferred
  path: spec/domains/session.yaml
- confidence: inferred
  path: spec/ess-inputs.yaml
revision: 10
transitions:
- {from: "draft", to: "proposed", at: "2026-10-28T21:16:54Z", actor: "human:timo", revision: 9}
- {from: "proposed", to: "active", at: "2026-10-28T21:16:55Z", actor: "human:timo", revision: 10}
---
## Outcome

A session request is a document: a caller holding a mapping of `metaharness run` settings (for
example a run file's `harness` section) turns it into a `RunSpec` through a public parse function of
`metaharness-protocol`, and `metaharness::check_spec` refuses, before anything starts, a key that the
spec's harness kind does not take. A key the document does not know is refused with a named error
that carries the key, never dropped; a value that does not read as its key's type is refused with a
named error that carries the key.

## Context

- Consumer: beyond10x/takt `dependency-blocker:metaharness-session-request-document`, which blocks
  takt `story:harness-section-is-metaharness-session-request` (acceptance 1, 4, 5) and
  `story:harness-flags-spell-metaharness-names` (acceptance 1, 3). takt reaches metaharness through
  Loom (`loom-session-executor` takes `metaharness` by tag, loom `crates/loom-session-executor/Cargo.toml:12`).
- `RunSpec` (`crates/metaharness-protocol/src/spec.rs:172`) derives `Serialize`/`Deserialize` with
  Rust field names and no `deny_unknown_fields`; it has no document form. Its documented spelling is
  the `metaharness run <kind>` flag set (`derive(clap::Args)` behind the `clap` feature).
- `check_spec` (`crates/metaharness/src/builder.rs:1860`) refuses cross-field and per-kind
  combinations, but Codex's refusal of `--max-turns` and `--max-budget-usd` is raised only at launch
  (`crates/metaharness-codex/src/launch.rs:919-935`, `unsupported_options`).
- `metaharness-protocol` may depend on `serde`, `serde_json`, `sha2` and optional `clap` only
  (AGENTS.md invariant 1), so the document's text form is JSON and the parse also takes a
  `serde_json::Value`, which a YAML caller produces from its own mapping.
- `effort` is passed through and validated by whoever serves the model (`spec.rs:356-365`); its
  vocabulary stays open, so a document refuses an `effort` that is not a string, not one outside a
  word list.

## Decision

- Keys are the `metaharness run` long flag names (`max-turns`, `strict-version`, ...) plus `kind`,
  the positional argument; values take the flag's own vocabulary (`hermetic: "strict"`), booleans
  are JSON booleans, repeatable flags are arrays. Absent keys take `RunSpec::new(kind)`'s values.
- An admitted key list derived from the same fields as the flag set, held equal by a test, so the
  CLI and the document cannot drift.
- The per-kind refusals that depend on the spec alone move into `check_spec`, called by the
  library, the CLI and the document's callers alike; the adapter keeps the vendor-specific reason
  (AGENTS.md invariant 2).

## Acceptance

1. `RunSpec::from_document_str(r#"{"kind":"codex","effort":"high"}"#)` returns a spec with
   `kind == Codex` and `effort == Some("high")`, all else as `RunSpec::new(Kind::Codex)`; today no
   such function exists.
2. A document with the key `retain-dirr` (or any key outside the admitted list) is refused as
   `RunSpecDocumentError::UnknownKey { key: "retain-dirr" }`, and its `Display` names the key.
3. `{"kind":"claude","max-turns":"many"}` is refused as `RunSpecDocumentError::BadValue` naming
   `max-turns`; a document without `kind`, or that is not a JSON object, is refused by name.
4. `check_spec` on `{"kind":"codex","max-turns":60}` returns a `Refusal` naming `--max-turns`, where
   today it returns `Ok(())`; the same for `max-budget-usd`.
5. A test fails when a `metaharness run` long flag has no document key or a document key has no
   flag.

## Out of Scope

- A word list for `effort`.
- An in-band format tag: the document is a mapping meant to be embedded in a caller's file.
- Loom's executor taking a caller's partial spec over its defaults (beyond10x/loom).

## Scope

- `crates/metaharness-protocol/src/spec.rs` (or a new `spec_document.rs` beside it), its `lib.rs` exports.
- `crates/metaharness/src/builder.rs` (`check_spec`), `crates/metaharness-codex/src/launch.rs`
  (`unsupported_options` made callable from `check_spec`).
- `spec/domains/session.yaml` or a new domain, `spec/ess-inputs.yaml`, `spec/conformance.json`, the
  conformance test that executes it.
- `CHANGELOG.md`, `docs/design/metaharness-protocol-v0.1.md` (an amendment), `website/docs/` reference.
