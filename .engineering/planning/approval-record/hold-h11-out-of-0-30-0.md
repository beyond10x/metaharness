---
format: aep.planning-md/3
id: approval-record:hold-h11-out-of-0-30-0
kind: approval-record
status: approved
title: U2 is held out of 0.30.0; the native canary story is accepted
summary: 0.30.0 ships U1 alone; h11-not-ok-on-unprobed-memory depends on the deeper-ancestor canary and lands after it
tags:
- non-interactive
relations:
- decides: story:h11-not-ok-on-unprobed-memory
- decides: story:claude-memory-canary-deeper-ancestors
- informed_by: approval-record:accept-drafts-0-30-0
revision: 2
transitions:
- {from: "draft", to: "approved", at: "2026-11-05T11:38:22Z", actor: "human:timo", revision: 2}
---
## U2 is held out of 0.30.0, and the native canary story is accepted (2026-11-05)

`b10x-chef-orchestrator` decided this under the delegated grant. It ran non-interactively, as a sub-agent of
the orchestrator.

**Decision:**
- `story:h11-not-ok-on-unprobed-memory` is held out of the 0.30.0 wave. 0.30.0 ships U1 alone:
  `story:substrate-host-0-20-confinement-only` and `story:eval-cgroup-root-check-by-prefix`.
- The held story stays active. It depends on `story:claude-memory-canary-deeper-ancestors` and lands after
  it, reading `unk` only for surfaces that are still unprobed.
- `story:claude-memory-canary-deeper-ancestors` is filed and accepted for the next metaharness wave.

**Why:**
- On a machine whose `TMPDIR` sits under `HOME`, the verdict route makes every Claude H11 read `unk`, so
  every strict Claude run exits 3. That brings back what design amendment a36 § 4 reverted, so it is a
  design-level change.
- Keeping the surfaces above the parent `ok` without a measurement would rest on inference.

**What changes:**
- This supersedes the verdict-route clause of `approval-record:accept-drafts-0-30-0`, which said
  "Re-measuring the canary is not part of this", for this story.
- The U2 branch `impl/h11-unprobed-memory` stays unmerged. Its uncommitted work is kept as a patch in the
  wave's scratch.
