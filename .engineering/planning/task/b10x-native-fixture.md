---
format: aep.planning-md/3
id: task:b10x-native-fixture
kind: task
status: active
title: Observe current native b10x against a credential-free fixture provider
relations:
- decomposes: story:current-adapter-compatibility
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T11:00:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T11:00:27Z", actor: "human:timo", revision: 3}
---
## Acceptance

Use opt-in Rust tests with the real installed b10x-harness0.13.3 and an owned local fixture provider to observe success, failure, observe-only tools, cancellation, model/usage and declared controls where supported. Follow the production Metaharness launch/normalization path, retaining actual binary version and source/pin distinctions. Fresh owned scratch and no real credentials or hosted/paid requests. Bound processes, clean on panic, record unknowns honestly and keep private transcripts out of source. Native runs are serialized with the coordinator and Claude unit; tests stay ignored in task check. Do not advance pins or claim governed AEP role qualification. If a production defect appears, record it and coordinate scope before changing behavior. Rust only.

## Scope

crates/metaharness-b10x/tests/native_fixture.rs, its Cargo.toml, Cargo.lock for required dev dependencies, docs/research/2026-10-03-native-b10x-fixture.md. Parent story owns the machine-readable scope. Separate managed tree, bot commits, no AEP writes, pushes or PRs. One integration PR remains coordinator-owned.
