---
format: aep.planning-md/3
id: review-result:issue-repair-local-boundaries
kind: review-result
status: active
title: Offline boundary review, coordinator fallback
relations:
- reviews: story:current-aep-runtime
- reviews: story:managed-workspace-admission
- reviews: story:scripted-b10x-run-does-not-need-the-binary
- reviews: story:governed-codex
- reviews: story:uncapped-spending-policy
- reviews: story:ess-specification-and-hardening
revision: 1
---
approve

Scope: offline source changes on the single integration branch, including ESS unit b5ab0727 over 6839f5de. This is a coordinator review fallback after worker quota exhaustion, not independent review and not approval of live compatibility or issue closure.

- AEP: exact 0.68.0 matching remains required; mismatched eval binary is refused. Updated command and scratch schema fixtures do not broaden capability admission. Full-gate-only CLI failures were corrected without removing assertions. Restored a historical quoted `protocol` name that a mechanical rename had incorrectly changed.
- Workspace: the shared component predicate matches the pinned substrate syntax; canonicalization and invalid requested-confinement refusal remain. No-model managed-checkout discovery preserved identity. The real owned-tools ESS target rejects an outside symlink; this proves that boundary, not process isolation.
- Scripted runners: executable requirements default true; only the explicit scripted runner opts out. Missing real binaries and strict unobserved versions still refuse. No CI vendor installation remains necessary for this gate.
- Governed Codex: supported shell mapping reaches the real engine with current frame coordinates. Malformed requests, patches and unsupported operations are denied; no Claude Skill exemption applies. Resume preserves model and endpoint; unsuccessful exits and interruptions cannot complete a step. Vendor fixtures live in the adapter, including conformance inputs.
- Uncapped spending: mode requires explicit authority and live opt-in. Admission persistence precedes spawn. Failed persistence and corrupt ordering refuse; resume preserves mode/reference and reads legacy finite launch terms. Unknown cost stays absent. A planted skipped-persist defect failed the production-linked regression and was restored; ESS independently reads the retained file rather than trusting intended counts.
- ESS: every selected expectation executes. Native report conformance_status is checked, suite freshness is checked, and all 24 cases pass with zero refusals/skips. The planted spec verdict error and production persistence error both fail named obligations. All ten emitted guard-negate mutants were killed by the same target. Exact Binary64 remains separately validated, not replaced by a lossy surrogate. Other hardening techniques are explicitly unclaimed.

Final integrated task check exited 0: 744 passed, 13 existing ignored live tests. No source finding remains in this bounded review. Live current-version qualification, the native unsupported-model probe, remote CI and independent agent review are not supplied by this verdict. Logs are under local cache metaharness-issue-repair/{aep-runtime,workspace12,scripted16,governed13,spending14,ess-final} and final-integration-task-check.log.

```findings
[]
```
