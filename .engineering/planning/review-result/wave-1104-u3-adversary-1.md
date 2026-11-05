---
format: aep.planning-md/3
id: review-result:wave-1104-u3-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:actor-reaches-every-launch
relations:
- reviews: story:actor-reaches-every-launch
revision: 1
---
unit: story:actor-reaches-every-launch, impl/actor-reaches-every-launch at 79f811c (base 5fef7ce, merged e11dbe2 into integration/wave-2026-11-04); cases 9606462
verdict: CONFIRMED (three red cases)
cases: executed 288→291, red 3
origin: introduced 3 / pre-existing 0 / undecided 0
wrote-outside-worktree: wave scratch only

The routes hold: both Codex runners clear the environment and take the plan's map, an operator
`AEP_ACTOR` without `--actor` reaches neither, no caller is newly refused, and the b10x refusal runs
on every start.

Red cases in `crates/metaharness/tests/adversary_u3_pass1.rs`:

- `a_blank_actor_is_refused_or_never_reaches_the_codex_child`: `--actor ""` is admitted and reaches
  the child as a blank `AEP_ACTOR`, which aep refuses at its first store write.
- `the_codex_attestation_names_the_actor_it_set`: the H3 row does not name `AEP_ACTOR`.
- `the_cli_reference_says_b10x_refuses_actor`: the `--actor` row of the CLI reference names no kind.

Decisions: a blank `--actor` is refused in `check_spec` for every kind; the Codex H3 row names the
declared `AEP_ACTOR`; the CLI reference says b10x refuses it. All three go back to the unit as
correction 1.

Filed as drafts, outside the unit: the Claude H3 row has the same gap (pre-existing), and the
CLI reference's `--effort` row says "passed through" where b10x refuses effort since a64.

```findings
[
{"file":"crates/metaharness-codex/src/launch.rs","line":1137,"category":"boundary","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a blank or whitespace --actor is admitted by check_spec and set as a blank AEP_ACTOR in the Codex child, which the subject refuses at every store write instead of the run being refused before it starts"},
{"file":"crates/metaharness-codex/src/launch.rs","line":1588,"category":"contract-drift","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the Codex H3 attestation still lists the child environment as the allowlist plus PATH, TMPDIR and CODEX_HOME while the plan now carries AEP_ACTOR, contrary to RunSpec::actor's promise that the attestation records it"},
{"file":"website/docs/reference/cli.md","line":102,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"the --actor row says the flag is carried to the child for every kind, but check_spec now refuses it on b10x and the row does not say so"}
]
```
