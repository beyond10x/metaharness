---
id: hermetic
title: The hermetic contract
sidebar_position: 3
---

# The hermetic contract

**A hermetic run shares credentials with the operator and nothing else.** No ambient plugins, no
account-level MCP servers, no inherited environment, no settings file you forgot about.

Twelve rows, each **imposed** and each **asserted**. Two things about that are the whole point.

## 1. A row is asserted, and not every assertion is equally strong

| Assertion | What the evidence is | Strength |
|---|---|---|
| `record` | Read out of the vendor's own record. | Strongest available without a live run. |
| `launch` | A value metaharness holds before spawning — the argv, the child environment, an ancestor walk. | Strong about what was *imposed*, silent about what the harness then did. |
| `effect` | Not directly assertable; the evidence is other rows holding. | Weakest, and labelled as such. |

metaharness also emits its own attestation of what it imposed. **That is not evidence.** It exists
so a reader can see the intent beside the outcome and notice when they disagree.

## 2. Gating is per row, not global

Two rows are unobservable as a property of the **mechanism** rather than of the run. If any `unk`
failed a strict run, every strict run would fail forever. Those two are advisory: evaluated,
reported, printed — and not counted against the exit code.

## The twelve rows

| Row | Control | Assertion | Severity |
|---|---|---|---|
| **H1a** | Config home is scratch — plugins are exactly the declared set | `record` | gating |
| **H1b** | Config home is scratch — the output style is the default | `record` | gating |
| **H2** | Settings sources are excluded | `launch` | **advisory** |
| **H3** | The environment is constructed, not inherited | `launch` | gating |
| **H4** | No API key unless the run declared one | `record` | gating |
| **H5** | The MCP surface is exactly what the launch gave | `record` | gating |
| **H6** | Credentials are one file, copied | `effect` | **advisory** |
| **H7** | The working directory is ours | `record` | gating |
| **H8** | Hooks and customizations are not skipped | `launch` | gating |
| **H9** | The vendor version is the pinned one | `record` | gating |
| **H10** | Governing documents cannot move under the run | `record` | gating |
| **H11** | No memory file outside the copied tree is discoverable | `launch` | gating |

H1a and H1b are split because they **fail independently**, and one unknown must not mask the
other.

H2 and H6 are advisory because of the mechanism: the absence of allow rules that would shadow the
seam is not observable in any record, and a credential copy leaves no trace of its own — its
evidence is H1a, H4 and H5 holding.

## Three verdicts, not two

| Verdict | Meaning |
|---|---|
| `ok` | The control held. |
| `gap` | The control did not hold. Exit 1 when the row is gating. |
| `unk` | **Nobody found out.** Exit 3 when the row is gating. |

`unk` is not a softer `gap`. A crashed suite is not a failing suite, and absence of evidence is
not hermeticity. This distinction is load-bearing enough that getting it wrong has already cost
a real defect: an early build treated *"this run pinned no documents"* as *"nobody found out
whether the documents moved"*, which made `--hermetic strict` unpassable.

The attestation knows a third state beside imposed and unavailable, for one row only: under
[`--cwd <DIR> --scratch-cwd`](reference/cli.md#--scratch-cwd-the-embedders-disposable-copy) H7 is
**`declared`** — the embedder declared the directory as its disposable scratch copy — and the floor
reads it `ok` only when the record's cwd is the one planned. H11 stays measured.

H7 compares **directories, not spellings**: the record's cwd and the planned one are the same
directory when they are equal or when both are absolute paths that resolve, links followed, to the
same path. A loop that records its canonical root therefore matches a directory metaharness created
under a `TMPDIR` reached through a link. Nothing is resolved against metaharness's own process: a
relative name, or one that reaches `/proc` or `/dev/fd` (such as `/proc/self/cwd`), is compared as
spelled.

On the b10x kind the record's cwd is the directory the loop itself reported in its opening record
(`b10x-harness` 0.15.1), never metaharness's plan. A strict or `--audit` b10x run whose other
gating rows hold exits 0 when that is the directory metaharness created or the declared copy, and
1 when it is another directory. When the loop names none — under `--substrate <socket>`, or for a
directory whose name is not UTF-8 — the record's cwd is `null` and H7 is `unk`, *"the loop did not
report its working directory"*, so the run exits 3. A strict `--cwd` run without the declaration
exits 1, on H7 and H11; bare `--hermetic` without `--audit` owes no verdict and exits 0.

## The three modes

| `--hermetic` | Behaviour |
|---|---|
| `off` *(default)* | No hermetic controls imposed. The run inherits the operator's world. |
| `on` — i.e. bare `--hermetic` | The controls are imposed and the verdict is reported. |
| `strict` | The controls are imposed and a gating row that is not `ok` fails the run. |

## Ambient inputs metaharness does not claim to remove

The attestation carries an `ambient_inputs` list: things the run is exposed to that metaharness
**reports** rather than claims to have stripped. The named one is **git status** — the vendor's
own flag description says it is placed in the system prompt.

Naming these is the honest alternative to a contract that quietly overstates its reach.

## What the contract does not claim

Metaharness does not place Claude Code or Codex inside an outer Substrate sandbox. Sandbox
inversion was rejected for production implementation; the neutral scripted `ProcessEnvelope`
contract retained from `0.4.0` is evidence vocabulary, not a runtime provider. The vendor's own
launch remains responsible for its confinement, while Metaharness observes and mediates calls.

The agent-visible execution context says `confinement=inner-harness` and
`substrate=not-used-by-metaharness` on those paths. Network isolation and outer mount enforcement
therefore remain unclaimed; absence is `unk`, never a successful confinement result.
