---
format: aep.planning-md/3
id: story:independent-documentation
kind: story
status: active
title: Publish independent Mantle-style Metaharness documentation
scope:
- confidence: cited
  path: .github/workflows
- confidence: cited
  path: AGENTS.md
- confidence: cited
  path: Cargo.lock
- confidence: cited
  path: Cargo.toml
- confidence: cited
  path: README.md
- confidence: cited
  path: Taskfile.yml
- confidence: cited
  path: b10x.docs.yaml
- confidence: inferred
  path: crates/metaharness-docs
- confidence: cited
  path: website
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T19:49:37Z", actor: "human:timo", revision: 4}
- {from: "proposed", to: "active", at: "2026-10-03T19:49:37Z", actor: "human:timo", revision: 5}
---
## Acceptance
Metaharness publishes a Mantle-style site from its own main at /metaharness/, preserving all 13 documentation pages and existing deep links, with exact source provenance, while Atlas and Website stop owning its documentation content and retain working redirects.

## Approved implementation
The operator approved the complete independent documentation plan and instructed "Implement the plan". This authorizes implementation, bot commits and publication, coordinated main merges, and live documentation verification; no software release is needed. Objectives O3/O6: make the harness comparison and governance interfaces independently discoverable and usable.

Use a Rust/clap static builder, Markdown content, static HTML/CSS and the immutable project-site publisher used by Mantle. Preserve all 13 pages, heading anchors, code and warnings. Keep /metaharness/docs/ routes and include b10x in navigation. Build emits b10x-project-site/v1 provenance. Credential-free build; bot-only deployment. Retire unified-source integration in Atlas and Website with old /docs/metaharness/ redirects only after live independent publication succeeds.

## Scope
Cited: website/, Cargo.toml, Cargo.lock, Taskfile.yml, .github/workflows/, AGENTS.md, README.md, b10x.docs.yaml. Inferred: new crates/metaharness-docs/.
Coordinated owning repositories: atlas documentation roster/catalog and website source roster, discovery data and compatibility redirects. Coordinator writes all AEP records. Documentation tooling introduces no product runtime entity.

## Verification
All 13 pages and navigation, fragments/assets; broken-link, duplicate-route, invalid-provenance and determinism tests; mobile/desktop and keyboard review; repository gate; affected Atlas/Website checks; live provenance and old URL redirects. Preserve current shared documentation until the replacement is live.

## Execution record
AEP implementing 0.19.2, approved wave of one cross-repository outcome with bounded source units. No decomposition panel: one story, no sibling acceptance units. Coordinator integrates workflows/publication and AEP; implementation workers use isolated managed trees and package-scoped checks. Default concurrency limit four; initial site build baseline previously measured at 1.4 GiB for the Metaharness full gate. All task scratch under ~/.cache/metaharness-independent-docs; each worker uses its own named target directory. No unrelated trees are cleaned.
