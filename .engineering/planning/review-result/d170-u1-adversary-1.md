---
format: aep.planning-md/3
id: review-result:d170-u1-adversary-1
kind: review-result
status: active
title: Adversary pass 1 on story:run-spec-document
relations:
- reviews: story:run-spec-document
revision: 1
---
unit: story:run-spec-document, commit 9109f336 on unit/d170-u1-run-spec-document (base ca82f28b); cases d2923ff
verdict: NEEDS-CHANGE (three red cases)
cases: executed 20→31, red 3
origin: introduced 1 / pre-existing 0 / undecided 2
wrote-outside-worktree: wave scratch only
needs-coordinator: whether the per-kind silently dropped keys are in this story's scope

Red: `an_empty_path_is_refused_by_the_document_as_the_command_line_refuses_it` (12 path keys read
`""` into `Some("")` where clap refuses), `b10x_is_refused_a_spend_ceiling_its_launch_drops` (argv
identical with and without `max-budget-usd`, `check_spec` admits it) and
`every_key_a_kinds_launch_reads_nowhere_is_refused_by_check_spec` (actor, hooks, driver on Codex;
hooks, driver on Claude; actor, effort on b10x).

Attacked and not broken: `DOCUMENT_KEYS` against the clap argument set; enum words against clap's
`ValueEnum` spellings; duplicate detection on raw text through escapes and with equal values; key
case and leading dashes; `max-turns` range, sign, fraction and null; a `kind` that is not a harness
kind; every keyed `Display` names its key; `Send + Sync + 'static`; AGENTS.md invariant 1; the CLI
gives the same words for Codex `--max-turns`; no in-code caller builds a Claude or Codex spec the new
refusals reject; ESS `RunSpecDocumentRefusal` matches the Rust variants.

Coordinator routing (fix 178eb5c on the unit branch):

- Finding 1: fixed; the document refuses an empty path as `BadValue`.
- Finding 2: fixed; `check_spec` refuses `--max-budget-usd` on b10x.
- Finding 3: fixed in part. `check_spec` refuses `--hooks` and `--driver` on Claude and Codex and
  `--effort` on b10x. `--actor` on Codex and b10x is filed as story:actor-reaches-every-launch,
  because `aep drive` passes `--actor` to Codex today and a refusal would break it. The adversary
  case was rewritten with that story id in its assertion message.
- Finding 4: accepted as a note; the gate's workspace run unifies the `clap` feature.

```findings
[{"file":"crates/metaharness-protocol/src/spec_document.rs","line":400,"category":"contract-drift","severity":"warning","verdict":"NEEDS-CHANGE","origin":"introduced","message":"An empty string for any of the 12 path keys reads as Some(\"\") while clap's PathBuf parser refuses the same flag value, so a document admits a value the command line refuses, against amendment a64's same-spec claim."},
{"file":"crates/metaharness/src/builder.rs","line":2216,"category":"acceptance","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"undecided","message":"check_spec admits max-budget-usd on b10x while b10x_launch never reads it (argv identical with and without), so a spend ceiling is accepted and silently dropped, against the story Outcome that check_spec refuses a key the kind does not take."},
{"file":"crates/metaharness/src/builder.rs","line":1868,"category":"acceptance","severity":"warning","verdict":"CONFIRMED","origin":"undecided","message":"check_spec admits actor, hooks and driver on codex, hooks and driver on claude, and actor and effort on b10x, keys whose launches read nowhere, so they are dropped in silence contrary to the story Outcome and unlisted in amendment a64 section 5."},
{"file":"crates/metaharness-protocol/tests/run_spec_document.rs","line":439,"category":"judgement","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"The acceptance 5 anti-drift tests are cfg(feature = \"clap\") and are compiled out of a crate-targeted run (11 tests without the feature, 15 with), so only a workspace run exercises them."}]
```
