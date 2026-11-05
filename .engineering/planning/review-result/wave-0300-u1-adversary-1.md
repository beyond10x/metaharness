---
format: aep.planning-md/3
id: review-result:wave-0300-u1-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on the substrate 0.20.0 pin and the cgroup root by prefix
summary: 'CONFIRMED: no disagreement with the probe; mount mutant closed by 3 cases; citation corrected in b832d1c; two pre-existing findings filed as drafts'
relations:
- reviews: story:substrate-host-0-20-confinement-only
- reviews: story:eval-cgroup-root-check-by-prefix
revision: 1
---
unit: story:substrate-host-0-20-confinement-only and story:eval-cgroup-root-check-by-prefix, impl/substrate-0-20-and-root-prefix at 1b4876c (base 4fa5164, merged f59a3af); cases 7ddb9b4; correction b832d1c (merged e60dd9f)
verdict: CONFIRMED
cases: executed 53→56, red 0 against the tree (3 red against a mutated copy)
origin: introduced 2 / pre-existing 2 / undecided 0
wrote-outside-worktree: wave scratch only

No input was found on which the eval's below-the-root check and Substrate's probe decide differently. Inputs
tried: `..`, `//`, a trailing slash, a relative root, a hybrid host, the `0::/` line and a missing `0::` line.

The suite missed one real gap: nothing pinned the mount. `crates/metaharness-aep-eval/tests/adversary_u1_cgroup_root.rs`
now runs the built `aep-eval preflight` with a real cgroup ancestor as the root, named plainly, with `..` and
through a symlink. The three cases go red when `CGROUP_MOUNT` is mutated.

**Contract drift, fixed in b832d1c.** The citation named the eval's own Substrate 0.20.0. The probe that
decides runs in the harness's built-in 0.14.0. The comment and the CHANGELOG now name both.

**Free preflight at 0.20.0.** EXIT=0, verbatim: `confined lifecycle preflight: decision-blocker starts open;
the `run` call came from a scripted endpoint on 127.0.0.1; no model contacted`. Seven tools were published.

**Lock test.** It reads the workspace `Cargo.lock` through `include_str!` and matches whole `name =` lines. Its
four names cover both features. The lock lost 128 names and gained none.

**Pre-existing findings filed as drafts:**
- `story:eval-cgroup-root-made-absolute` (lib.rs:378)
- `story:eval-names-an-undelegated-cgroup-root` (lib.rs:1330, out of scope)

```findings
[
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":1358,"category":"mutant","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"No case pinned the mount below_cgroup_root passes, so a CGROUP_MOUNT of /sys/fs/cgroup/unified kept the suite green (w8u7 18/18); tests/adversary_u1_cgroup_root.rs now fails on it."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":378,"category":"judgement","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"The cgroup root is never made absolute, so in the paid driven arm the eval checks a relative root against its own cwd while the harness, spawned in the project cwd, probes another path."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":1360,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The mirrored rule cites Substrate 0.20.0, but the deciding probe runs in the harness's built-in Substrate 0.14.0 (0b9d50c); the rule is the same today, and a later pin bump would track the wrong copy."},
{"file":"crates/metaharness-aep-eval/src/lib.rs","line":1330,"category":"out-of-scope","severity":"note","verdict":"INFEASIBLE","origin":"pre-existing","message":"The eval copies only the probe's prefix step, so a root that holds the eval but is not delegated skips the scope and fails later as a missing run tool, not by naming the root."}
]
```
