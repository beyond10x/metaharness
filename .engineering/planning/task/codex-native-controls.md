---
format: aep.planning-md/3
id: task:codex-native-controls
kind: task
status: implemented
title: Observe native Codex cancellation and controls without credentials
relations:
- decomposes: story:current-adapter-compatibility
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T10:50:43Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-03T12:02:42Z", actor: "human:timo", revision: 5}
---
## Acceptance

Extend the explicitly selected native fixture tests to drive the production Metaharness binary cancellation command and supported per-call denial, assert final closure/process evidence and absent denied effects, and record declared-control observations from actual native records. Keep unsupported/unexercised controls explicit. No operator credentials, external provider or paid request. Do not change pins until the compatibility matrix is reviewed. Rust only. Coordinator owns this serial extension to the Codex fixture unit.

## Scope

crates/metaharness-codex/tests/native_fixture.rs and docs/research/2026-10-03-native-codex-fixture.md. Production behavior changes require a failing regression and design amendment before implementation.

## Reviewed native result

The extended suite passed6tests/11actualprocesses against the polling candidate inside an owned loopback-only namespace. Independent read-only review found a startup/cancellation race; the fixture now waits for both the provider request and actual versioned session.started. The focused corrected halt/interrupt test passed again (1test,1.57seconds) against the deadline-aware driver. Rust1.99 target clippy passed. ProductionCLI ask denial correlates the real call and prevents the marker side effect. Source integrated from85e81f17. Final combined-driver rerun remains required after integration changes.
