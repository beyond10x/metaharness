---
format: aep.planning-md/3
id: approval-record:delivery-at-green-d65
kind: approval-record
status: draft
title: The unit merges at green; adversary findings land after
relations:
- decides: story:off-pin-vendor-version-is-advisory
revision: 1
---
## Decision

The coordinating session, 2026-10-27 02:26Z (its decision D65, "delivery faster"): "the metaharness
unit merges at green and proceeds to its release without waiting for the adversary; the
adversary's findings land as fix commits (or a patch release) after." And: "builds run in parallel
inside 40G per repository".

## Applied to this story

`story:off-pin-vendor-version-is-advisory`: adversary pass 1 had already returned when D65
arrived. Its medium finding (an off-pin Claude record listing only `@builtin` plugins read H1a
`gap`, so `--hermetic strict` exited 1 for the version alone) and two stale-text findings were
fixed in the same unit before the merge; the rest are notes for Loom's pin hop.
