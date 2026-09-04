# The W4.1 verifiers

One check per decomposed task, written in the `establish_verifiers` state of run `W4-1/1` — before
`run-agents.sh`, the two prompts, the two trace documents, `fixtures/` or the README section
existed. **They are red, and red is the product.** A test that passes before the code exists is a
test of nothing, and the `establish_verifiers → implement` transition is guarded on `test.exists`
precisely so the order cannot be argued about afterwards.

```bash
./run-checks.sh                          # every check
./run-checks.sh trace-documents readme   # only those
```

Nothing here calls the Claude API.

## What a run needs

| Handle | What it names | Default |
|---|---|---|
| `AEP_REPO` | the subject checkout: the fixture, the artifact tree, and the two **charter documents** | `~/beyond10x/aep` |
| `AGENTPLUGINS_REPO` | the checkout the `aep-plan` plugin comes from — `E1`, `E2` and `E4` read its two files | `~/beyond10x/agentplugins` |
| `aep` on `PATH` | the CLI `T1`–`T5` judge transcripts with and `E4` creates a story with | — |
| `EVAL_CHARTER_SPEC_DIR` | the eval corpus inside `AEP_REPO`, if it ever moves | `$AEP_REPO/conformance/eval` |
| `EVAL_CHARTER_SPEC_DECOMPOSER` / `_REVIEWER` | one charter document each, by path, overriding both of the above | — |

**The two charter documents are not in this repository, and a copy of them here would be the
defect.** They are AEP's own eval corpus —
`conformance/eval/decomposer-charter/expectations.trace.yaml` and its `plan-reviewer-charter`
sibling — and `crates/edge/aep-cli/tests/eval_corpus.rs` replays every case in it inside *that*
repository's `task check`. Nothing under `evals/` runs in this repository's gate (`AGENTS.md`
invariant 5), so a second copy here would be a copy nothing checks. `T1`–`T8` read the canonical
files and say, in the row's own reason, which path they could not read and what to set when they
cannot.

```bash
AEP_REPO=~/beyond10x/aep AGENTPLUGINS_REPO=~/beyond10x/agentplugins ./run-checks.sh
```

| Check | Decides | Rows |
|---|---|---|
| `check-scratch-fixture.sh` | `task:agent-eval-scratch-fixture` | F1–F9 |
| `check-decomposes-edge-examples.sh` | `task:decomposes-edge-examples` | E1–E4 |
| `check-trace-documents.sh` | `task:agent-eval-trace-documents` | T1–T8 |
| `check-decomposer-stage.sh` | `task:agent-eval-decomposer-stage` | S1–S8 |
| `check-reviewer-stage.sh` | `task:agent-eval-reviewer-stage` | V1–V8 |
| `check-runner-verdict.sh` | `task:agent-eval-runner-verdict` | R1–R9 |
| `check-offline-mode.sh` | `task:agent-eval-offline-mode` | O1–O8 |
| `check-readme.sh` | `task:agent-eval-readme` | M1–M7 |
| `check-live-evidence.sh` | `task:agent-eval-live-evidence` | L1–L8 |

The row ids are the tasks' own. A check reports every id it declares, exactly once, on every path.

## What is green, and what the red rows are still waiting for

Measured 2026-09-04, against agentplugins `0.7.0` and AEP `0.51.0`: **11 pass, 58 fail,
0 broken**, exit 1. The eleven are `E1`, `E2`, `E4` and `T1`–`T8` — every row whose subject exists.
The fifty-eight are not a regression and not a mystery; they are the four things `W4-1/1` decomposed
and never built, plus two pins the repository split moved out of reach:

| Rows | Why they are red |
|---|---|
| `F1`–`F9`, `S3`–`S8`, `V3`–`V8`, `R2`–`R9`, `O1`–`O8` (37) | `../run-agents.sh` does not exist. It never has, in this repository or in AEP's history: `W4-1/1` wrote these verifiers in `establish_verifiers`, and the runner, the two prompts and `../fixtures/` were never implemented. |
| `S1`, `S2`, `V1`, `V2`, `R1`, `L1`, `L2`, `L3`, `L5`, `L6`, `L8` (11) | the recordings `contracts/evidence-manifest.txt` names are absent. They are the output of three **paid** live runs, which is why they are recordings and not something a check could produce. |
| `M1`–`M7` (7) | `../README.md` has no section describing a runner that does not exist. |
| `E3`, `L4` (2) | `contracts/pre-task-blobs.txt` pins `b83c623` under `integrations/claude-code/`. That revision is unreachable in every AEP checkout, and those paths moved to the `agentplugins` repository. There is no honest re-pin: the blob is in a history neither repository still has. |
| `L7` (1) | it reads `git status --porcelain` in the **subject** checkout, so it is red whenever anyone has that checkout dirty — which is a fact about the machine, not about this eval. |

None of these is softened, marked advisory or removed. A red row here means the thing it names is
missing, which is the only thing a verifier written before its subject can usefully say.

## Three rules these checks hold themselves to

**A missing deliverable is a red row, never an absent one.** `red_all` in `lib.sh` puts every
declared row in the table with one shared reason under it. A check that reported nothing when its
subject did not exist would go green in `run-checks.sh` for having no failures, which is the same
defect as a gate that was switched off.

**A live-only row is asserted against a recording, never skipped.** `S1`, `S2`, `V1`, `V2`, `R1` and
`L1`–`L6` are claims about a run that costs money. They are checked against the files
[`contracts/evidence-manifest.txt`](./contracts/evidence-manifest.txt) names — the same recordings
the specification's Acceptance Criteria already demand ("shown by a recorded run, not argued"). A
recording that is missing is red, so the live half of the case stays visible between releases
instead of quietly falling out of the table.

**Discrimination is checked, not assumed.** `S3`–`S5`, `V4`–`V6`, `T3`–`T5`, `R2`–`R5`, `O4`, `O5`,
`O7`, `L2` and `L3` each break exactly one thing and require exactly the right row to move. Most of
them also assert that *nothing else* moved, which is what catches an assertion written so loosely
that any mutation reddens it.

## What is in here besides checks

| Path | What it is |
|---|---|
| [`contracts/`](./contracts) | the handles, ids, record shapes and pinned revisions the checks read |
| [`transcripts/`](./transcripts) | small, deliberately-broken transcripts — inputs to `T3`–`T5` and `V6` |
| `lib.sh` | rows, reasons, scratch directories, and the two parsers for a verdict table |

`contracts/` is where a check and an implementation meet. `contracts/interface.md` fixes the flags
and environment variables the tasks leave to the implementer; `contracts/verdict-rows.txt` fixes the
D and P ids the runner's table must name; `contracts/trace-expectations.txt` fixes the expectation
ids R12 states as kinds only. Changing a name there is a real change — it moves what the checks
assert — and that is why it is a file rather than a convention.

`contracts/trace-expectations.txt` is read in **both** directions, and the second one is the reason
it can be trusted. Forwards: every row it names must exist in the charter document, with that kind,
that tool, that matcher, gating. Backwards: every **gating** expectation the charter document
declares must be named in it. A hand-maintained list read only forwards is green while the subject
adds a bound nobody here has ever looked at, and that is what the list did until 2026-09-04 — it
named seven ids per document, none of which the canonical documents carry, and the forward pass had
no way to say so once the documents were missing entirely.

`transcripts/` is **not** `../fixtures/`. These are hand-written inputs; `../fixtures/` holds the
live run's own transcripts and is what `--offline` replays.

## The conflict these verifiers surfaced, and how the owner settled it

`T7` is written literally, as its task states it: *every `tool.absent` is paired with a `tool.called`
over the same tool in the same document*. Applied to R12's decomposer row `tool.absent — Write,
file_path contains .engineering/planning`, it demands a `tool.called` over `Write` — which a correct
decomposer run never produces, and which R14 itself argues is the mark of a bound that cannot fail.
The row was left as written rather than quietly softened, and resolving it was called the eval
owner's call.

The owner resolved it. AEP's `conformance/eval/decomposer-charter/case.yaml` drops that
expectation and writes down why: *"The charter grants `[Read, Grep, Glob, Bash]`, so `Write` is never
offered and the row is true of every possible run — indistinguishable from a check that was switched
off … a vacuous row is worse than a missing one because it reads like coverage."* Both canonical
documents now carry exactly one `tool.absent` tool, `Bash`, with a `tool.called` over `Bash` beside
it, so `T7` passes on the pairing it was written to demand rather than on a softened form of it.
`contracts/trace-expectations.txt` follows that decision and cites it; it did not make it.
