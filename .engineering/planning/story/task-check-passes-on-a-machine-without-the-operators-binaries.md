---
format: aep.planning-md/1
id: story:task-check-passes-on-a-machine-without-the-operators-binaries
kind: story
status: draft
title: task check passes on a machine that has none of the operator's binaries
summary: 'The Gate workflow (gate.yml, 2026-09-15) ran task check on a clean runner four times: run 34911935585 failed because b10x-harness was not on the run''s constructed PATH; run 34912326156 because it was installed into ~/.cargo/bin instead of ~/.local/bin; run 34912870204 because metaharness_preflight never checked the PATH (fixed in aa673e3); run 34913587922 because crates/metaharness-cli/tests/aep_resume.rs:70 (legacy_launch_resumes_without_spending_or_losing_configuration) is refused with ''steps.yaml cannot produce evidence this task''s plan will demand, and --allow-evidence-gap was given: test_result …; static_analysis …'' although the same test passes on the operator''s machine with AEP pinned at 28abe09b. cargo test stops at the first failing target, so later targets are unmeasured. Until every target passes on the runner, the Gate is red on every push.'
revision: 1
---
<!-- Starting point for a `story` artifact, seeded by `aep artifact new story <name>`.
     No frontmatter here on purpose: the `---` block is written by the CLI from the id, kind, status
     and relations you gave it, and a second copy in this file would be the one that went stale.
     Delete the italic guidance as you fill each section. -->

# Story: <name>

## Outcome

*What is true for whom once this has shipped, in one sentence. If it names a component rather than a
person, it is a task — say what changes for someone.*

## Context

*Why this is worth doing now, and what it depends on. Link the epic or specification it comes from
rather than restating it; the `derived_from` relation already carries the edge.*

## Acceptance

*The conditions under which this is done, each one something a person or a check can observe. "Works
correctly" is not one of them.*

## Out of Scope

*What a reasonable reader would expect to be included and is not — the boundary that stops this
story quietly becoming an epic.*

## Ambiguities

*Each gap this story found and did not close, classified. `inferable` — the answer is already
written down, so give the `path:line` or the artifact id that settles it.
`requires-stakeholder-input` — nobody here can decide it, so name who does, and raise that entry as
a `decision-blocker` with a `blocks` edge to this story, or it is a sentence somebody improvises
later.*

## Open Questions

*Anything still undecided belongs in `## Ambiguities` above, classified and with its citation or its
decider. Keep this section for a question that is neither — an unowned question is a story that
stalls without anybody noticing.*
