---
format: aep.planning-md/3
id: approval-record:accept-drafts-0-29-0
kind: approval-record
status: approved
title: Five drafts accepted and one rejected for the 0.29.0 wave
summary: Hook refusals, seam abstain, H11 claims, catalogue remainder, aep-eval scope move to active; the pinned-binary draft is rejected (a62)
tags:
- non-interactive
relations:
- decides: story:claude-hook-refusals-reach-the-record
- decides: story:claude-seam-abstain-is-recorded-as-vendor-decided
- decides: story:h11-claims-match-what-was-measured
- decides: story:catalogue-road-leftovers
- decides: story:aep-eval-scope-through-substrate
- decides: story:claude-seam-runs-only-a-pinned-binary
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T03:32:35Z", actor: "human:timo", revision: 2}
---
## Five drafts accepted, one rejected (2026-11-05)

`b10x-chef-orchestrator` decided this under the delegated grant. Run non-interactively, as a sub-agent of
the orchestrator. Quote:

> Accept the 5 drafts: claude-hook-refusals-reach-the-record, claude-seam-abstain-is-recorded-as-vendor-decided,
> h11-claims-match-what-was-measured, catalogue-road-leftovers (as rewritten), aep-eval-scope-through-substrate.
> Reject claude-seam-runs-only-a-pinned-binary, citing a62. The other 12 stay drafts.

- The five move to active. `story:catalogue-road-leftovers` is rewritten to what is still open at `9192a90`;
  its type check had already shipped. `story:h11-claims-match-what-was-measured` has its line references
  refreshed.
- `story:claude-seam-runs-only-a-pinned-binary` is rejected. Amendment a62 (release 0.20.0) decided that an
  off-pin vendor version is reported and never refused, and `--strict-version` stays the opt-in refusal.
- This record accepts and rejects drafts. It does not approve a wave; `approval-record:wave-0-29-0` does.
