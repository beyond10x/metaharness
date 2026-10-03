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
revision: 7
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

## Migration findings
The source registry validates relationships before rendering. Its only retained Metaharness references are Website's introduces edge and Agentic Principles' informs edge. Remove those unified-registry edges at their owners, preserve ordinary links through redirects, and promote only the affected Agentic Principles source pin. This adds one manifest-only owner change to the coordinated scope. Independent discovery remains Website-owned metadata and does not collect Metaharness content.

## Implementation evidence
Site unit 50c97895f864b84d7a573d8db03d89185050d8cd preserves all 13 pages (13,057 words) and 125 legacy heading IDs. Seven unit and three publication tests pass; repeated complete builds are byte-identical. Browser checks passed all 14 pages at 1440, 768 and 390 pixels with JavaScript disabled, plus keyboard skip navigation. Atlas unit 24043e81489b52c433020f0114829c9c6ac69406 passes 220 unit and 63 integration tests. Website adversary review found three verifier parity gaps (quarantine namespaces, bootstrap-mode inference, ambient source-set conflict handling); all three are corrected before publication. Delivery and live verification remain pending.

## Adversarial closure
The site builder now stages replacements and accepts only a complete owned output inventory, preserving unrelated files and symlinks on refusal. Asset validation checks poster/background URLs and refuses unsupported srcset, inline CSS and stylesheet asset references through Rust tokenization. Final package verification: 11 unit and 6 publication tests pass; the independent adversary reran 9 boundary tests, all green. Trusted runtime probes do not execute candidate scripts and reject altered lock/bootstrap data. Agentic Principles PR 16 merged as b016a34be831af0695f4cc159e0d537b98d38312 with the exact source pin preserved. Stale Gates hooks were refreshed through the supported installer; policy baselines were not changed.
