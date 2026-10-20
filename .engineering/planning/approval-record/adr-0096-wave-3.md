---
format: aep.planning-md/3
id: approval-record:adr-0096-wave-3
kind: approval-record
status: draft
title: 'Wave 3 of 2026-10-20: b10x adapter on harness 0.15.0, the loop''s cwd, tools contract and crates'
tags:
- wave
refs:
- provider: atlas
  reference: adr-0097
- provider: atlas
  reference: adr-0098
relations:
- decides: story:b10x-adapter-on-harness-over-llm
- decides: story:b10x-record-carries-the-loops-cwd
- decides: story:metaharness-tools-adopts-tool-contract
- decides: story:metaharness-tools-on-tools-crates
- decides: story:b10x-h7-from-the-loops-cwd
revision: 4
---
## Status

Proposed 2026-10-20 by session `metaharness`; approved the same day by `b10x-chef-orchestrator` under
the owner's standing exception for ADR 0096–0101 work, with one change (below). Skill:
aep:implementing 0.23.2, wave mode. Integration tree `wt-4d5f44e5ec8f`, branch
`integration/adr-0096-wave-3`, base `3d697011` (`main`, Release 0.10.0).

## Units

Sequential: each unit shares files with the next. Final shape after the re-scope (D5, D6) and the
follow-up taken in as unit 2b.

| unit | story | commit | merge | adversary passes |
|---|---|---|---|---|
| 1 | `story:b10x-adapter-on-harness-over-llm` | `40283285` | `0a7b3885` | 2 (`review-result:adr0096-w3-adapter-adversary-1`, `-2`) |
| 2 | `story:b10x-record-carries-the-loops-cwd` | `0aa85270` | `d595b42` | 2 (`review-result:adr0096-w3-loops-cwd-adversary-1`, `-2`) |
| 2b | `story:b10x-h7-from-the-loops-cwd` | `2b03d47` | `34c427b` | 2 (`review-result:adr0096-w3-h7-from-cwd-adversary-1`, `-2`) |
| 3 | `story:metaharness-tools-adopts-tool-contract` | `9f4d6a0` | `6ec311c` | 2 (`review-result:adr0096-w3-contract-names-adversary-1`, `-2`) |
| — | `story:metaharness-tools-on-tools-crates` | — | — | closed on evidence after unit 1 (D5) |

Also on the branch: `04ab7bed` (merge of `main` at 0.10.1), `e4bce26c` (doc fixes relayed from a scout),
`aeb70214` (coordinator correction of unit 1's trailing-slash attribution).

## Proposal and approval

The proposal (2026-10-20, about 11:05) asked for: the opening store commit; per unit one commit and
one merge into the integration branch (formatting-only fix commits where a merge needs one); the
closing store commit; push, PR and fast-forward of `main` through the bot after CI; one harness build
in a managed harness tree for the golden re-record and the native fixture. Not asked: a paid run, a
release, registry publishing.

`b10x-chef-orchestrator`, 2026-10-20 (verbatim): "Wave 3 approved under exception 5 (the commits,
merges, closing commit, PR + bot fast-forward and the one harness build you listed; no paid run, no
release), with one change. harness 0.15.0 deletes b10x-harness-tools and b10x-harness-toolchain
(tools-1: the Catalogue/Verbs/search layer moves to the new b10x-harness-catalogue, which also
re-exports tool_toolchain) and is close (PR #30 CI, then the short llm-port wave). So U1 pins harness
0.15.0, not 0.14.1, and D3 keeps that layer through b10x-harness-catalogue, not b10x-harness-tools; I
send the 0.15.0 sha. Until then, open the store commit and scope U1/U3/U4 against harness's
integration branch wave/2026-10-20-tools-1 (58b29933). D1, D2 and D4 accepted; for D4, file a harness
draft "the loop record carries its cwd" through me if 0.15.0 still writes none."

## Decisions

- D1 (accepted): unit 1 lands before the tools stories; the `depends_on` edge was reversed. No
  harness type crosses `metaharness-tools`/`metaharness-b10x`.
- D2 (accepted): unit 3's direction is contract name in, existing neutral operation out, harness
  names still accepted. The former wording would deny every owned-surface act under a frame
  (`run.rs:966-969`).
- D3 (changed by the approval): the verb/catalogue/search layer comes from `b10x-harness-catalogue`
  (harness 0.15.0), not `b10x-harness-tools`.
- D4 (accepted): unit 2 is narrowed to the null shape; `run b10x --hermetic strict --cwd <copy>
  --scratch-cwd` moves exit 0 → 3 (H7 gating), named in the changelog. Harness `947d0e48` still writes
  no working directory (`event.rs:114-201`; converter `"cwd": Value::Null`, `metaharness.rs:358`); the
  harness draft goes through `b10x-chef-orchestrator` once 0.15.0 is tagged and still writes none.
- D5 (coordinator, after the re-scope): `story:metaharness-tools-on-tools-crates` is superseded by
  unit 1. Harness 0.15.0 deletes `b10x-harness-tools`, so unit 1 must take `b10x-harness-catalogue`,
  whose `LocalOperations` runs `fs` and `code-editor` through the tools crates (harness
  `crates/harness-catalogue/src/local.rs:243-257,499-557`); its doc lines moved to unit 1; it closes
  on evidence read after unit 1 merges. The alternative (unit 1 renames the dependency key back to
  `b10x-harness-tools`) keeps a key named after a deleted crate.
- D6 (coordinator, after the re-scope): unit 3 takes the contract-name mapping from
  `b10x-harness-catalogue` (`harness_operation`, `local.rs:118-143`) with no tools dependency and no
  wait for tools 0.2.0; the served catalogue keeps harness's names, because the b10x run and the
  metaharness server share one catalogue and harness keeps its names model-facing. Contract schemas
  are not published: harness drops `expected_sha256` (`catalogue.rs:476-483`).
  `dependency-blocker:tools-0-2-0-release` was cleared without the tag, its body says why.
- Unit 1's crate swap uses the crate's own name (`harness_catalogue::`); manifest comments that name
  the deleted crate are deleted, not rewritten.

## Log

- Scoping (2026-10-20): four `aep:story-scoper` passes against harness 0.14.1 and tools 0.1.0 /
  `wave/w2`; after the approval, units 1, 3 and 4 re-scoped against harness `947d0e48`
  (`wave/2026-10-20-tools-1`, merged to harness `main` at that commit, verified by
  `b10x-chef-orchestrator`). Relayed and corrected: `b10x-harness-catalogue` re-exports only
  `tool_toolchain::ResolvedProvider` (`lib.rs:65`); the whole crate is re-exported by
  `b10x-harness-substrate` (`lib.rs:86`); metaharness uses neither. Harness `947d0e48` pins tools
  0.1.0 (`046dfb14`), reachable from tools `origin/main`.
- Release 0.10.0 (not part of this wave, recorded here because the wave's branch carries the store):
  tag `0.10.0` (object `a659a232`, commit `3d697011`) pushed; the Release workflow (run 39396148010)
  built all four archives and `SHA256SUMS` verified, but `b10x-gates scan-artifact` refused the
  x86_64-linux executable: "artifact breaks 1 private rule(s): line 21914 private-identifiers". The
  match is a tracker-key example compiled in from aep-cli's help text at the aep pin `6d7a44d`. The
  GitHub Release is not published. `b10x-chef-orchestrator` decided option A: aep replaces the sample
  keys (aep PR 130), metaharness moves its aep pin and releases 0.10.1 (approved: one pin commit and
  the release through the repository's process); 0.10.0's GitHub Release stays unpublished.
- Disk (Timo, 2026-10-20, relayed): disk space and accounts are `b10x-chef-orchestrator`'s; the
  coordinator's 25G build floor was removed from its build wrapper; one cargo build at a time stays.
  The integration tree's build cache (5,272 MiB) was discarded on request at about 11:15.
- Unit 1 `aep:implementor` (tree `wt-1f0ad046d7f7`, base `04ab7bed`; cut once by HTTP 429 and resumed on the same model) returned green: harness pins in both manifests at 0.15.0 `dd5c2a66`, `b10x-harness-catalogue` imported as `harness_catalogue::`, two manifest comments deleted, `Cargo.lock` one harness and one tools revision (tools 0.2.0 `60d4d0e3`), golden re-recorded from a 0.15.0 binary with harness's emulator (`flat-tool`, loopback-only netns), `loop.expected.jsonl` changed only in `harness_version` and `credential_source`, amendment a29, research page; red runs first (b10x lib 3 failed, contract golden 1 failed, native 6 failed), then workspace 836 → 839 passed, 0 failed, 31 ignored, six gate steps exit 0, native fixture 7/7. Relayed facts corrected: the emulator did not move; 0.15.0 has two model-facing reply changes (`dir_list` of a FIFO, capped `find`). The `started` record names no working directory (D4 harness draft sent through `b10x-chef-orchestrator`). Confined process writes unverified on this host (cgroup controllers cpu, memory, pids only). Harness build tree `wt-b6794e62c1f2` finished and gc'd. Agent cost: 300,262 tokens, 40 tool uses, 652 s (after the resume). `aep:adversary` pass 1 dispatched.
- D4 draft routed by `b10x-chef-orchestrator` (2026-10-20): harness files it and implements it with 0.15.1 (the llm 0.8.1 repin), or 0.15.2 if not green by then; the tag carrying `cwd` will be relayed.
- Pace rule (Timo, 2026-10-20 about 15:25, relayed by `b10x-chef-orchestrator`): at most one sub-agent at a time in this session; one unit implemented, reviewed and merged before the next starts.
- `aep:adversary` pass 1 on unit 1 (cut twice by HTTP 429 and once by the coordinator to save quota, resumed on the same model each time): NEEDS-CHANGE, 3 red cases in three new test files; findings all `introduced`: (1) warning, a29/CHANGELOG/research page name two served reply changes where 0.15.0 changes more (`file_edit` overlap now refused, ` (edit 0)` suffix, non-UTF-8 wording, `file_read` `limit: 0` refused, empty-glob `find` and write-onto-directory wording); (2) note, capped `find` also flips `truncated`; (3) note, three comments still describe a 0.9.1 golden; (4) note, the lock test misses source spellings and passes with zero tools packages; (5) note, the fetched-repository comment lists miss tools. Containment, pins, golden provenance and the native fixture held. Recorded as `review-result:adr0096-w3-adapter-adversary-1`. Agent cost: 276,615 tokens on the final resume. Correction round 1 dispatched to the same implementor (finding 5: the stale comment lists deleted, per the no-comments-in-config rule).
- Correction round 1 on unit 1 returned green: all five findings fixed, the adversary's three files 5/5 green unedited; a 70-call side-by-side probe of the 0.13.3 and 0.15.0 crates found 17 moved replies (12 beyond harness's two), listed in a29, the changelog and a 14-row research table; the implementor corrected its own earlier `depth_bound_reached` observation; the lock parser normalises git URL spellings and requires exactly one tools revision, pinned by a parser test shown red under a mutant; the stale repository comment lists deleted. Workspace 839 → 845 passed, 0 failed, 31 ignored; six gate steps exit 0. Agent cost this round: 407,406 tokens, 79 tool uses, 1,590 s. Pass 1 recorded `fixed`; `aep:adversary` pass 2 dispatched.
- `aep:adversary` pass 2 on unit 1 (the last; cut once by HTTP 429, resumed): all five pass-1 findings resolved; one note, `introduced`: ten more served replies move outside the 70 probed calls (a missing outside path now refused like an existing one; `file_edit`/`file_write` with a trailing `/` now applied, also through a link inside the workspace); one green case added; 846 passed. Recorded as `review-result:adr0096-w3-adapter-adversary-2`. Agent cost: 337,244 tokens on the final resume. Coordinator verification: the note answered by a coordinator addendum (second table in the research page, one changelog bullet); the doc-reading adversary tests (2 and 1 passed) and the docs check re-run; outcome recorded `fixed`; `verification` evidence recorded. Unit 1 committed `40283285`, merged `0a7b3885`; its tree finished, archived and gc'd (6,916 MiB freed), branch deleted.
- Coordinator doc correction `aeb70214` on the integration branch: the research addendum and changelog bullet now call the trailing-slash replies the tools contract (confirmed by the tools owner through `b10x-chef-orchestrator`), not a gap; no test asserted the 0.13.3 refusal.
- harness 0.15.1 (tag object `a0a2dfda`, commit `f0870131`, bot; carries the `started` `cwd`, llm 0.8.1) relayed by `b10x-chef-orchestrator`: "Your follow-up to story:b10x-record-carries-the-loops-cwd (H7 from the record) can pin 0.15.1 in your next unit." `story:b10x-h7-from-the-loops-cwd` filed and taken into this wave as unit 2b, after unit 2 and before unit 3 (decided by the coordinator: it shares every file with unit 2, and unit 2's `unk` branch stays for records without `cwd`).
- Unit 2 `aep:implementor` (tree `wt-e432eccdde80`, base `0a7b3885`; cut once by HTTP 429 and resumed) returned green: b10x `session.started.cwd` is `null`, H7 `unk` with "the loop did not report its working directory" (`direct_provider` arm), `B10xSeams::new` lost its third parameter (14 call sites, 9 files), vector checks the 0.15.0 golden's line 1 carries no `cwd`, `loop.expected.jsonl` changed only `cwd` (cmp-proved), adversary test rewritten, amendment a30. Beyond the story: a strict b10x run without `--cwd` also moves exit 0 → 3 (named in the changelog and a30; unit 2b restores an observed H7 on 0.15.1). Red runs first; workspace 846 → 853 passed, 0 failed, 32 ignored; six gate steps exit 0; native lane 8/8 against the 0.15.0 binary. Outside the Scope: one call-site line in `builder.rs:383`, `website/docs/reference/cli.md`, the protocol seam trait doc. Agent cost: 245,093 tokens, 29 tool uses, 1,712 s (after the resume). `aep:adversary` pass 1 dispatched.
- `aep:adversary` pass 1 on unit 2: NEEDS-CHANGE on documents only, no behaviour defect; 2 red cases (scripted) and 4 green native CLI cases in two new test files; findings all `introduced`: F1 warning `cli.md:38` promises strict passes on a declared copy for every kind; F2 note `--hermetic --audit` on b10x also moved 0 → 3; F3 warning a30 and `seam.rs:162` say no released loop writes a `started` cwd while harness 0.15.1 does; F4 note stale a6 comment in `builder.rs:674`; F5 note the a6 H7 row prints the loop reason (a30 decision 2, kept). Recorded as `review-result:adr0096-w3-loops-cwd-adversary-1`. Agent cost: 211,003 tokens, 105 tool uses, 940 s. Correction round 1 dispatched to the same implementor.
- Correction round 1 on unit 2 returned green: all six items fixed (cli.md row qualified; `--hermetic --audit` 0 → 3 named; a30 and every "no released loop" sentence corrected to 0.15.0 none / 0.15.1 writes `cwd`, verified at `f0870131` `event.rs:202-228`; a6 comment; a30 a6 sentence; the vector reads the `started` line, pinned by a new case whose red failed for the wrong reason before). Workspace 853 → 856 passed, 0 failed, 36 ignored; six gate steps exit 0; native lanes 4/4 and 8/8 on the 0.15.0 binary. Agent cost this round: 305,708 tokens, 70 tool uses, 566 s. Pass 1 recorded `fixed`; `aep:adversary` pass 2 dispatched.
- `aep:adversary` pass 2 on unit 2 (the last): all pass-1 findings resolved; one note N1 (`introduced`): every `--audit` b10x run, in any `--hermetic` mode, also moves 0 → 3; one green case added; 857 passed. Recorded as `review-result:adr0096-w3-loops-cwd-adversary-2`. Agent cost: 250,768 tokens, 28 tool uses, 341 s. Coordinator verification: N1 applied as wording in the changelog and a30 decision 3, the doc-reading cases (2 and 1 passed), docs check and fmt re-run; outcome `fixed`; `verification` evidence recorded. Unit 2 committed `0aa85270`, merged `d595b42` (CHANGELOG auto-merged with `aeb70214`); tree finished, archived and gc'd (4,069 MiB freed), branch deleted.
- Unit 2b `aep:implementor` (tree `wt-6d0269524061`, base `d595b42`) returned green: harness pins at 0.15.1 `f0870131`; golden re-recorded from a 0.15.1 binary inside a private mount namespace (workspace at `/scratch/run/work`; a first recording from scratch carried a home path and was discarded); `loop.expected.jsonl` changed `harness_version` and `cwd` only; the seam reads the record's `cwd`; H7 on b10x `ok`/`declared`/`gap`/`unk` from the record; a canonical directory comparison (byte-equal, or both absolute and canonicalize-equal; relative never resolved) applied on every kind; strict and `--audit` b10x return 3 → 0 on a clean run; a6 stays 1 with H7 `gap`; adversary tests of units 1 and 2 and `adversary_project_from.rs` moved to the new state; amendment a31; red runs first (adapter 6 failed, core 6, native 9); workspace 857 → 867 passed, 0 failed, 37 ignored; native 9/9 and 4/4 on 0.15.1. Harness build tree `wt-bd68184105b4` finished and gc'd. Agent cost: 410,927 tokens, 271 tool uses, 2,473 s. `aep:adversary` pass 1 dispatched.
- `aep:adversary` pass 1 on unit 2b (cut once by HTTP 429, resumed): CONFIRMED with one red case and 4 notes, all `introduced`: F1 `same_directory` canonicalizes `/proc/self/cwd`, so a record naming it reads H7 `ok` (against a31's own rule; INFEASIBLE today); F2 the public `hermetic_floor` doc says rows come from events alone; F3 a native helper weakened from `cwd` null to `is_string`; F4 the § 8.1 H7 cell keeps a30's now-false wording. Pins, golden provenance (chroot recording byte-identical), the canonical rule's other edges and the exits held. Recorded as `review-result:adr0096-w3-h7-from-cwd-adversary-1`. Agent cost: 225,058 tokens on the final resume. Correction round 1 dispatched.
- Correction round 1 on unit 2b returned green: F1 `same_directory` now resolves through a component-by-component `realpath`-style function that refuses any prefix reaching `/proc` or `/dev/fd` (12 spellings pinned, all red before); F2 the public doc states H7 reads the filesystem at call time; F3 native helpers pin the exact directory, canonically; F4 the § 8.1 cell marks a30 superseded. The adversary's case green unedited; workspace 869 passed, 0 failed, 37 ignored; six gate steps exit 0; native 9/9 and 4/4 on 0.15.1. Agent cost this round: 453,138 tokens, 54 tool uses, 414 s. Pass 1 recorded `fixed`; `aep:adversary` pass 2 dispatched.
- `aep:adversary` pass 2 on unit 2b (the last): F1-F4 resolved; F5 new (note, INFEASIBLE, `introduced`): the hand-written resolver applies `..` lexically after a file component, so `<copy>/README.md/..` (ENOTDIR for realpath) reads H7 `ok` as the declared copy; one red case and a 14,000-name agreement property with `canonicalize` (green). Recorded as `review-result:adr0096-w3-h7-from-cwd-adversary-2`. Agent cost: 280,125 tokens, 28 tool uses, 466 s. Correction round 2 dispatched (require a directory for a non-link component with more components to come); the coordinator verifies it from the diff and the exit codes, no third attack.
- Correction round 2 on unit 2b (F5): a non-link component with more after it must be a directory, a trailing `/` or `/.` is kept as a checked `.`; nine ENOTDIR shapes pinned red first; both adversary files green unedited; 872 passed, 0 failed, 37 ignored; six gate steps exit 0; native 9/9 and 4/4. Agent cost: 481,732 tokens, 27 tool uses, 259 s. Coordinator verification: diff read (`audit.rs:824`), adversary file hashes unchanged, its own run of the two adversary files and the audit tests (1, 2, 55 passed, exit 0); outcome `fixed`; `verification` evidence recorded. Unit 2b committed `2b03d47`, merged `34c427b`; tree finished, archived and gc'd (7,353 MiB freed), branch deleted.
- Unit 3 `aep:implementor` (tree `wt-c6ce0a7152d6`, base `34c427b`) returned green: `entry_of(name) = harness_operation(name).unwrap_or(name)` used by `resolve_verb`, `subjects_of_verb` and `Server::call` (`tool_invoke`, `tool_describe`); mapped set at 0.15.1 `fs.read`, `fs.list`, `fs.glob`, `code.write`, `code.edit` (harness `local.rs:118-124`), read again at runtime by a class test; served catalogue unchanged (D6); no tools dependency; amendment a32 (a frame judges these calls as their entry where it denied them as `UNCOVERED_TOOL`); 15 new cases, 14 red first; workspace 872 → 887 passed, 0 failed, 37 ignored; six gate steps exit 0. Decided by the coordinator: the implementor's end-to-end frame-scope case (`run_loop.rs`, outside its file list) applied in the unit tree; it fails on base, passes with the change, and showed the subject-scope hole when the translation was removed from `subjects_of_verb` alone. Agent cost: 182,171 tokens, 113 tool uses, 855 s. `aep:adversary` pass 1 dispatched.
- `aep:adversary` pass 1 on unit 3: CONFIRMED, two red cases; F1 warning (`introduced`): a contract-named `code.write`/`code.edit` carrying the contract's `expected_sha256` is performed with the precondition dropped (harness 0.15.1 never reads it), so a stale-digest write lands; F2 note: on a b10x record a contract-named call the 0.15.1 loop refused reads as its entry's operation and subject. Argument spellings, the bare top-level name, withheld entries, `fs.glob`, variants and the end-to-end case held. Recorded as `review-result:adr0096-w3-contract-names-adversary-1`. Agent cost: 150,337 tokens, 72 tool uses, 731 s. Decided by the coordinator: both fixed in this unit (F1: refuse a contract-named invoke that carries `expected_sha256` until harness runs the check; F2: the b10x record path does not translate contract names, because the loop refuses them). Correction round 1 dispatched.
- Correction round 1 on unit 3 returned green: F1 `Server::call` refuses a contract-named invoke carrying `expected_sha256` (any value) with `precondition-failed`, nothing runs; measured on the way: a harness-named `file_write` carrying a stale digest is accepted and the field ignored (`additionalProperties: false` is declared but not enforced on the served path), stated in a32 point 5, reported to `b10x-chef-orchestrator` for harness; F2 a `VerbSurface { Served, HarnessLoop }` parameter: only the served surface translates, a b10x record reads a contract name as Unknown with no subject. 894 passed, 0 failed, 37 ignored; six gate steps exit 0; the adversary file green unedited. Agent cost this round: 262,422 tokens, 67 tool uses, 780 s. Pass 1 recorded `fixed`; `aep:adversary` pass 2 dispatched.
- `aep:adversary` pass 2 on unit 3 (the last): F1 and F2 resolved; P2-1 new, `pre-existing` warning: a b10x run is read as publishing the three verbs while the launched loop publishes flat, so the record drops the subject the loop reported for every b10x call. Recorded as `review-result:adr0096-w3-contract-names-adversary-2`, outcome `escalated` to `story:b10x-record-keeps-the-loops-subjects` (filed; its acceptance case is the adversary's red case, kept and ignored with that reason). Coordinator residue edits: resolve.rs docs and a32 point 4 no longer say the b10x loop binds the verbs. Agent cost: 218,350 tokens, 37 tool uses, 344 s. Coordinator verification: its own run (run_loop 52, metaharness-tools lanes green, the pass-2 file 1 passed 1 ignored, clippy, docs, fmt exit 0); `verification` evidence recorded. Unit 3 committed `9f4d6a0`, merged `6ec311c`; tree finished, archived and gc'd (4,305 MiB freed), branch deleted.

Notes collected during the wave:

- tools 0.2.0 released 2026-10-20 (relayed by b10x-chef-orchestrator, verified by it): tag object cf9db3a8, commit 60d4d0e32f558bb84a01ac8adb581f4f856c1230 (tools main); every crate Apache-2.0; tool_contract::documents carries FS_V1ALPHA1, CODE_EDITOR_V1ALPHA1, PROCESS_V1ALPHA1. Asked harness to pin it before the 0.15.0 cut.
- D5 and D6 accepted by b10x-chef-orchestrator (2026-10-20, about 11:40); harness told that 0.15.0 pins tools 0.2.0.
- Doc fixes relayed from a scout by b10x-chef-orchestrator (approved under exception 5), commit e4bce26c: README harness row "runs and observes" (ADR 0096 L2), AGENTS.md loop sentence, README names the ESS pin. Scout claims corrected: the ESS 0.56.0 pin exists (spec/ess-inputs.yaml:2 requires: ess 0.56.0; gate.yml:67 installs ess-cli 84ee8d38); b10x-harness-tools exists at harness 0.14.1 and is deleted at 947d0e48 (0.15.0), handled by unit 1.
- tools 0.3.0 released (relayed and verified by b10x-chef-orchestrator, 2026-10-20): tag -> e89380d8f65f564d845711ca3ed77d543d9737a8 (tools main, bot-made); adds build-test v1alpha1 and search v1alpha1 contracts with crates. For the wave after wave 3: check whether the served search (still in harness at 0.15.0, local.rs:213-228) should move to the search crate, beside story:claude-code-2-1-304-declared.
- Trailing-slash finding routed by `b10x-chef-orchestrator` to tools as a 0.3.1 fix (file targets with a trailing slash refused, link check on the normalized path, the three cases as tests); metaharness takes it when harness pins a tools release carrying it.
- Correction (same day): the trailing-slash moves are the released tools code-editor contract (semantics.md link/ is link; scenario adversary-trailing-slash), an intended difference from 0.13.3; no tools 0.3.1 for it. Unit 1 docs corrected in commit aeb70214 on the integration branch.
- Harness-named `file_write` schema finding relayed by `b10x-chef-orchestrator` to harness as a story for the release after 0.16.0 (schema lines confirmed at 0.15.1 `catalogue.rs:855,879`); the metaharness refusal of contract-named `code.write`/`code.edit` with `expected_sha256` stands.
- Whole gate on `integration/adr-0096-wave-3` at `6ec311c`, one step at a time: `ess specify validate` over `spec` and `spec/monetary` exit 0, `cargo fmt --check` exit 0, clippy `--workspace --all-targets -D warnings` exit 0, `cargo test --workspace` exit 0 (73 test binaries, 895 passed, 0 failed, 38 ignored), `metaharness-docs check` exit 0; conformance 87/87 on ESS 0.56.0 (official report: passed, 0 failed, 0 skipped). `story:metaharness-tools-on-tools-crates` closing evidence read at the same head: one harness revision `f0870131`, one tools revision `60d4d0e3` with the four `b10x-tool-*` crates, one `b10x-harness-tools` mention left in a dated research page describing the 0.13.3 probe. All five stories moved to `implemented` in the closing commit.
