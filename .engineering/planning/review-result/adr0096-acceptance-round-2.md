---
format: aep.planning-md/3
id: review-result:adr0096-acceptance-round-2
kind: review-result
status: active
title: Acceptance critic, round 2 (ADR 0096-0098 drafts)
owner: metaharness
relations:
- reviews: epic:session-format
- reviews: story:trace-ir-domain
- reviews: story:project-reads-vendor-transcripts
- reviews: epic:loom-agent-work-through-metaharness
- reviews: story:caller-supplied-catalogue-on-every-kind
- reviews: story:decision-returns-caller-produced-result
- reviews: story:catalogue-changes-within-a-session
- reviews: story:continuity-across-executor-calls
- reviews: story:call-seam-for-harness
- reviews: story:confinement-of-vendor-kinds
- reviews: story:aep-drive-disposition
- reviews: story:governance-unit-stated
- reviews: epic:layer-pins-follow-their-owners
- reviews: story:metaharness-tools-on-tools-crates
- reviews: story:b10x-adapter-on-harness-over-llm
revision: 1
---
needs-revision
story:project-reads-vendor-transcripts — the acceptance joins five independent outcomes with semicolons, and "the three golden records under `crates/*/fixtures/golden/` are the committed fixtures" is already true before the work. It also fixes `metaharness.source` to the vendor format name while the Ambiguities section leaves that question open — .engineering/planning/story/project-reads-vendor-transcripts.md:59
story:confinement-of-vendor-kinds — the acceptance joins four independent outcomes (two new fields, a scripted vector, a `--hermetic strict` exit, no Substrate dependency). "no Substrate dependency enters `Cargo.toml`" is vacuously true before the work, so it cannot show the change happened — .engineering/planning/story/confinement-of-vendor-kinds.md:62
story:aep-drive-disposition — the acceptance reads "The option `decision-blocker:aep-drive-future` records on clearing is carried out". That names no end state that can be read from a file or command. The per-option checks sit in "Procedure, per option", and the acceptance says it will be rewritten once the blocker clears — .engineering/planning/story/aep-drive-disposition.md:69
story:caller-supplied-catalogue-on-every-kind — the acceptance requires each of three kinds to "launch a session whose `offered_tools` equals the catalogue" and also lets a kind "refuse by name". It does not say which kinds must succeed, so all three refusing passes it. Its own Context lists Codex as unverified — .engineering/planning/story/caller-supplied-catalogue-on-every-kind.md:69
story:decision-returns-caller-produced-result — the acceptance pins only "each such kind" that "serves the owned surface", and no kind is named. A run where no kind can deliver a result, with every kind refused by name, meets the acceptance — .engineering/planning/story/decision-returns-caller-produced-result.md:60
story:continuity-across-executor-calls — the acceptance ends "a kind that cannot continue refuses by name before spawn" and never names the kinds that must continue. Refusal on both Claude Code and Codex meets it, although the Context lists `--resume` and `codex exec resume` as unverified vendor claims — .engineering/planning/story/continuity-across-executor-calls.md:58

What I read: 14 of 14 ids with `aep plan artifact show` (3 epics, 11 stories; `kinds` and `lifecycle story` also run). I spot-checked the citations in `story:trace-ir-domain` against the tree, and they hold: `ess_conformance.rs:874` asserts 45, `ess_suite_matches_the_current_specification` exists, `resolve_verb` exists, and the suite format is `ess-conformance/35`.

Approved without findings:
- `story:trace-ir-domain` — observable (at least 22 `metaharness.trace.*` scenarios, `task check` exit 0), with a stated before and after and the `active → implemented` move.
- `story:catalogue-changes-within-a-session` — before and after the step boundary are named.
- `story:call-seam-for-harness` — b10x refuses `--decisions frame` today, so the change is visible.
- `story:governance-unit-stated` — a `grep` for the paragraph shows the change.
- `story:metaharness-tools-on-tools-crates` — the dependency entries are present now and must be absent after.
- `story:b10x-adapter-on-harness-over-llm` — the pin value changes from 0.13.3.

What I could not establish:
- Whether ESS 0.56.0 generates one scenario per outcome for a twelve-outcome command. The story marks this `inferable`, and I did not run it.
- Whether a scripted-envelope replay in `story:confinement-of-vendor-kinds` can show a refused write. This is feasibility, so out of my lane.
- Whether `docs/design/metaharness-protocol-v0.1.md` has an a25 amendment as its latest, as `story:trace-ir-domain` says. I did not check.
- Coupling and scope-overlap questions (shared `run.rs` and `spec.rs`, dependency edges) belong to the design and parallel-safety critics. They did not affect my verdict.

```findings
[
  {"file": ".engineering/planning/story/project-reads-vendor-transcripts.md", "line": 59, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins five independent outcomes with semicolons, and 'the three golden records under crates/*/fixtures/golden/ are the committed fixtures' is already true before the work; it also fixes metaharness.source to the vendor format name while the Ambiguities section leaves that open"},
  {"file": ".engineering/planning/story/confinement-of-vendor-kinds.md", "line": 62, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance joins four independent outcomes (two new attestation fields, a scripted vector, a --hermetic strict exit, no Substrate dependency) and 'no Substrate dependency enters Cargo.toml' is vacuously true before the work"},
  {"file": ".engineering/planning/story/aep-drive-disposition.md", "line": 69, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance 'The option decision-blocker:aep-drive-future records on clearing is carried out' names no observable end state per option; the checks live in 'Procedure, per option' and the acceptance says it is rewritten when the blocker clears"},
  {"file": ".engineering/planning/story/caller-supplied-catalogue-on-every-kind.md", "line": 69, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance requires each of three kinds to launch a session with offered_tools equal to the catalogue and also lets a kind refuse by name, without naming which kinds must succeed, so all three refusing passes it"},
  {"file": ".engineering/planning/story/decision-returns-caller-produced-result.md", "line": 60, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance pins only 'each such kind' that serves the owned surface and names no kind, so a run where every kind is refused by name meets it"},
  {"file": ".engineering/planning/story/continuity-across-executor-calls.md", "line": 58, "category": "acceptance", "severity": "warning", "verdict": "needs-revision", "origin": "introduced", "message": "the acceptance ends 'a kind that cannot continue refuses by name before spawn' and names no kind that must continue, so refusal on every kind meets it while the vendor resume claims stay unverified"}
]
```
