---
format: aep.planning-md/3
id: task:b10x-native-role
kind: task
status: active
title: Qualify a bounded native read-only scoper role
relations:
- decomposes: story:current-adapter-compatibility
revision: 3
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T11:37:25Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T11:37:25Z", actor: "human:timo", revision: 3}
---

Exercise actual b10x0.13.3 with a prompted read-only scoper role through the production driver and the owned loopback Responses provider. Assert intended role instructions reach the provider, an actual file_read returns owned bytes, the declared tool surface and terminal/version observations are preserved. Do not claim named-agent/delegate loading or governed AEP behavior from this plain prompted role. Keep credentials absent and execution bounded/serialized. Deliberate source/version repin requires exact main-reachable release provenance and full gate. This task inherits the accepted compatibility story and approved release goal. Worker b10x_fixture performs it after the process-write containment extension.
