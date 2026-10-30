---
format: aep.planning-md/3
id: review-result:adversary-c5-unit-b-pass-2
kind: review-result
status: active
title: Adversary pass 2 on wave 2026-10-30-c5 unit B
summary: named scratch root fix 64e3f91, 4 red cases
relations:
- reviews: story:scratch-root-stable-by-declared-name
revision: 1
---
unit: c5-ub-adv2, story:scratch-root-stable-by-declared-name, fix 64e3f91 with cases 10c8741 on adv/c5-ub-2
verdict: red
cases: executed 17→22, red 4
origin: introduced 4
wrote-outside-worktree: wave scratch unit-b-adv2/ (deleted), test entries under the temp dir (removed)
needs-coordinator: yes (process-group kill goes beyond fix decision 2)

```findings
[
{"file":"crates/metaharness/src/process.rs","line":296,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"reap_unreaped kills one pid, so a process the vendor started (hook, tool command) survives the run's drop and writes into the next run's root under the same name, contradicting a72 section 5; adversary_scratch_name_pass2.rs:247 is red"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":5985,"category":"contract-drift","severity":"note","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the x.lock collision is permanent rather than one at a time: x's lock file is left in place, so once x has run, x.lock is refused as scratch-root-foreign with no run in use; adversary_scratch_name_pass2.rs:314 is red"},
{"file":"crates/metaharness/src/builder.rs","line":2406,"category":"boundary","severity":"warning","verdict":"INFEASIBLE","origin":"introduced","message":"empty_directory recurses per level and aborts the process with a stack overflow on a 1500-deep tree on a 2 MiB thread, which std::fs::remove_dir_all removed before the fix; no caller is shown to leave such a tree; adversary_scratch_name_pass2.rs:399 is red"},
{"file":"crates/metaharness/src/builder.rs","line":2024,"category":"concurrency","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"take_in's error paths after take_name_lock close without the explicit unlock, so with another thread spawning, a refused start leaves the name held and the next start reads scratch-name-held instead of the real refusal; adversary_scratch_name_pass2.rs:428 is red 12/300"}
]
```

Could not break: Drop order and explicit unlock, no hang and no kill of a reaped child, the private
parent's checks (link, file, other uid, wider mode), O_NONBLOCK on a FIFO, links inside a leftover.
Coordinator decisions for fix 2: process group for named runs with group kill on drop and CLI
signal forwarding; names ending in `.lock` refused; iterative removal; a lock guard on every path.
