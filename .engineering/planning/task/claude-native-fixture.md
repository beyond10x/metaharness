---
format: aep.planning-md/3
id: task:claude-native-fixture
kind: task
status: active
title: Observe current native Claude against a credential-free fixture provider
relations:
- decomposes: story:current-adapter-compatibility
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 3}
---
## Acceptance

Add opt-in Rust native fixture tests for actual Claude2.1.288 using the existing production adapter/binary path, an owned loopback Messages provider, fresh homes and CredentialSource::None. Cover success, terminal failure, a supported tool decision, cancellation, model/usage and declared controls where executable; retain unknowns and explicit limits. Never send operator credentials or call paid/external providers. Placeholder authentication is synthetic, not operator identity. Bound all subprocesses and server waits, clean children on panic, retain private evidence outside Git, and keep native invocations ignored in task check. Do not advance pins or claim general hosted compatibility. Record any real production bug before changing behavior; amend the binding design first. Rust only for committed executable code.

## Scope

crates/metaharness-claude/tests/native_fixture.rs, crates/metaharness-claude/Cargo.toml, Cargo.lock only for needed dev dependencies, docs/research/2026-10-03-native-claude-fixture.md. Integration and AEP writes remain coordinator-owned; use a separate managed worktree and bot commits, no push or PR.
