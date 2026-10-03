---
format: aep.planning-md/3
id: task:release-source-path-remapping
kind: task
status: implemented
title: Remap build-source paths in release executables
relations:
- decomposes: story:bot-release-publication
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-10-03T12:44:33Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-10-03T12:44:33Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-10-03T12:53:40Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1}}}
---
## Problem

The exact 0.9.0 release build succeeds but both Linux executables fail Gates personal-path validation. The release workflow does not remap Rust build-source paths. Compressed archives are not scanner inputs; extracted executables are.

## Authorized repair and acceptance

Prepare deterministic source-path remapping for the runner home and checked-out workspace in the existing release build. Validate a rebuilt Linux executable with Gates scan-artifact and retain its digest and scanner result. This does not qualify macOS, waive Gates refusals, move the annotated release tag, or authorize a second PR. Publication still depends on the release-artifact-validation blocker and a delivery route consistent with the operator's one-PR requirement.

## Remaining embedded fixture paths

A local Linux release rebuilt with path remapping still failed Gates with 16 personal-path findings. Inspection traced these to the runtime conformance vectors' previously redacted golden capture paths, rather than compiler source paths. Further neutralize only those already-redacted paths under `/fixture/operator`, preserving record order, versions, tool calls, identifiers and compact serialization. Update derived path slugs and the Codex hook expectation; regenerate expected event streams through each adapter's existing generator. Record the redaction in each fixture's provenance README, preserving the original capture facts. Full conformance and the artifact scanner must pass on the rebuilt executable.

## Prepared repair verified on 2026-10-03

The integration branch now sets release RUSTFLAGS to remap the runner home and workspace to neutral build paths. The remaining embedded golden fixture paths were further redacted under `/fixture/operator`, with derived slugs and Codex hook expectations updated. Capture versions, record order, tool calls and identifiers are unchanged. Generated event expectations were rebuilt with workspace feature resolution; the standalone adapter generator had changed one floating-point serialization, and regenerating with the released CLI's feature graph restored it. Fixture READMEs document the redaction and correct regeneration command.

`task check` passed after these changes. Workflow YAML parsed and the extracted package shell step passed `bash -n`. The rebuilt x86_64 Linux release executable reports metaharness 0.9.0 and has SHA256 ff55489feea8dace8e6d15aaba93863e2068d018ce2ccb5ff190467902f92e04. Gates 0.1.12 scan-artifact passed on those exact bytes: input_bytes=30027456, privacy_bytes=21843309, extracted_bytes=16713220, sections=34, scanner_invocations=1. The exact executable's offline conformance passed Claude 27/27, Codex 17/17 and b10x 7/7 vectors.

This is local x86_64 Linux qualification of the prepared repair, not replacement evidence for the existing tag's CI archives. All four release platforms still need rebuilt, verified, scanned archives from an agreed source delivery. Gates Mach-O support remains unavailable, the single integration PR is already merged, and the annotated 0.9.0 tag has not moved. The release blocker stays open; no release publication or issue closure is claimed.
