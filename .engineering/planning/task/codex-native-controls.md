---
format: aep.planning-md/3
id: task:codex-native-controls
kind: task
status: active
title: Observe native Codex cancellation and controls without credentials
relations:
- decomposes: story:current-adapter-compatibility
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 3}
---
## Acceptance

Extend the explicitly selected native fixture tests to drive the production Metaharness binary cancellation command and supported per-call denial, assert final closure/process evidence and absent denied effects, and record declared-control observations from actual native records. Keep unsupported/unexercised controls explicit. No operator credentials, external provider or paid request. Do not change pins until the compatibility matrix is reviewed. Rust only. Coordinator owns this serial extension to the Codex fixture unit.

## Scope

crates/metaharness-codex/tests/native_fixture.rs and docs/research/2026-10-03-native-codex-fixture.md. Production behavior changes require a failing regression and design amendment before implementation.
