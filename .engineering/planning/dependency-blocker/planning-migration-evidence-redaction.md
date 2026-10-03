---
format: aep.planning-md/3
id: dependency-blocker:planning-migration-evidence-redaction
kind: dependency-blocker
status: cleared
title: Planning migration needs supported redaction of historical personal paths
relations:
- blocks: epic:github-issue-repair
- blocks: story:ess-specification-and-hardening
revision: 3
transitions:
- {from: "open", to: "cleared", at: "2026-10-03T00:09:06Z", actor: "human:timo", revision: 3}
---
## Observed refusal

The requested schema migration successfully verified old and new planning answers. The bot commit then exited 1:

```
b10x-gates: common checks failed: 2 finding(s); private details remain local
b10x-gates: bot Git operation refused
```

The private hook findings name rule personal-paths at line 11 in these migrated evidence files:

- .engineering/evidence/migration-plan/aep-runtime-extraction/20260909T135542Z-000-2039bb6c30eb.json
- .engineering/evidence/migration-plan/aep-runtime-extraction/20260909T141404Z-000-a4e82627cb69.json

Both offending values are historical local log references, preserved by migration, rather than new evidence. No matched private text is included here.

## What clears this

A supported AEP migration/evidence-redaction operation, or an explicit operator instruction permitting a narrowly reviewed correction of only these two reference strings while preserving the original record privately. Do not weaken Gates, modify privacy policy, drop evidence, bypass hooks or change recorded test results. AEP 0.68.0 plan store and artifact evidence help expose migration and append-only evidence creation, but no redaction operation.

## Handoff

The migrated store and requested ESS adoption/hardening story remain in the managed issue-repair worktree. No new commit or GitHub write succeeded; all six issues remain open. Next owner: the operator for the evidence-handling decision, then the coding agent to commit the permitted migration and resume issue implementation.

## Resolution

The operator approved the pending narrow correction and implementation wave on 2026-10-03. The two historical reference strings now use the home-relative prefix ~/; their suffixes, test results, timestamps and every other field are unchanged. Original bytes are retained in a private owner-only evidence directory outside Git and in the prior managed-worktree archive. This is the explicitly authorized exception to append-only evidence, not a Gates exception. No other evidence was rewritten.
