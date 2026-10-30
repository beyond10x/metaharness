---
format: aep.planning-md/3
id: review-result:adversary-c5-unit-b-pass-3
kind: review-result
status: active
title: Adversary pass 3 on wave 2026-10-30-c5 unit B
summary: named scratch root fix aadcfdf, 4 red cases
relations:
- reviews: story:scratch-root-stable-by-declared-name
revision: 1
---
unit: c5-ub-adv3, story:scratch-root-stable-by-declared-name, fix aadcfdf with cases c3563bf on adv/c5-ub-3
verdict: red
cases: executed 25→31, red 4
origin: introduced 4
wrote-outside-worktree: ~/.cache/metaharness-c5/unit-b-adv3/ (logs)
needs-coordinator: no

```findings
[
{"file":"crates/metaharness/src/process.rs","line":316,"category":"property","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"The group SIGKILL is sent after the leader was reaped, when nothing holds the id any more, so it kills whatever later leads a group under that reused pid; kill the group before the reap, while the zombie leader still holds the id"},
{"file":"crates/metaharness-cli/src/forward.rs","line":47,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"SIGQUIT is not forwarded, so Ctrl-\\ ends metaharness without dropping the run: the vendor's group outlives it with the name's locks released, and the next run under the name takes over a root they still write into"},
{"file":"crates/metaharness-cli/src/forward.rs","line":47,"category":"acceptance","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Ctrl-Z (SIGTSTP) stops metaharness but not a named run's vendor group, which keeps running the session unobserved"},
{"file":"crates/metaharness-cli/src/forward.rs","line":80,"category":"contract-drift","severity":"note","verdict":"INFEASIBLE","origin":"introduced","message":"After a named run, signal-hook's handler stays installed with no action, so SIGINT, SIGTERM and SIGHUP are ignored for the rest of an in-process caller of execute rather than returned to their default handlers"}
]
```

Could not break: the lock guard on every exit path of take_in (fchmod error path included), the
iterative walk (links removed unfollowed, other users' directories refused), the (dev, ino) climb
check, signals before the child exists or after the group is gone, no handler for unnamed runs and
none in the library. Coordinator decisions for fix 3: kill the group before the reap, on the
normal end path too; forward SIGQUIT; job control for SIGTSTP and SIGCONT; restore SIG_DFL.
