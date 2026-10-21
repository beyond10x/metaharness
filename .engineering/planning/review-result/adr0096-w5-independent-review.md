---
format: aep.planning-md/3
id: review-result:adr0096-w5-independent-review
kind: review-result
status: active
title: Independent review of wave 5, 10 slices
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:claude-h11-memory-walk-shapes
- reviews: story:hook-program-quotes-channel-paths
- reviews: story:session-close-decides-absent-fields
- reviews: story:caller-supplied-catalogue-on-every-kind
- reviews: story:loopback-tls-leg-tested
revision: 1
---
unit: wave 5 of the ADR 0096 work (units 1–5 and 7 at `3bc0de4`, the head of `integration/adr-0096-wave-5`; unit 4's uncommitted tree against base `6ad241c`), independent review in ten read-only slices, 2026-10-21
verdict: no blocker; 52 findings recorded here, 20 warnings and 32 notes
cases: none executed (read-only: nothing built, run or edited)
origin: introduced 44 / pre-existing 8 / undecided 0
wrote-outside-worktree: the slice files under metaharness-wave-adr0096/review in the operator's cache directory, saved by the coordinator because the reviewers return findings as text
needs-coordinator: triage below, taken by the coordinator; s7-1, s7-2, s9-1 and s10-5 went into unit 4's fix round 2

## Slices

| slice | covers |
|---|---|
| s1 | `crates/metaharness/src/loopback.rs` and the unit-5 production TLS test |
| s2, s3, s4 | H11 (`audit.rs`, `builder.rs` walk, Claude canary table, research page), the scratch-path guards, the session close specification |
| s5 | the Codex hook quoting and the C1 fixtures |
| s7, s8, s9 | unit 4's caller catalogue (`caller.rs`, `server.rs`, `run.rs`, `builder.rs`, vectors, fixtures) |
| s10 | the tests and manifests the wave added |

Slice s6 and finding s10-6 concern later work and are not recorded here. The coordinator re-checked s7-2,
s2-6 and s3-2 by reading and confirmed them; s10-1 duplicates s7-1 and s10-4 duplicates s7-5.

## Disposition (coordinator)

- Fixed in unit 4's fix round 2 (landed in commit `1bdc5b2`): s7-1 and s10-1 (`caller.rs:77`), s10-5 (CHANGELOG
  count), s7-2 (argument check, `crates/metaharness-tools/src/caller.rs:206`, called at `server.rs:172`),
  s9-1 (the vector observes the copy the plan serves, `crates/metaharness-claude/src/vectors.rs:915`,
  `:945`), s8-1 (a40 § 6 and `cli.md` reworded to the real order).
- By design: s9-2 (`server.rs:158` refuses an entry nothing in the run performs).
- Deliberate: s10-3.
- Became stories: s2-1, s3-1, s4-1, s2-2, s4-5, s2-4, s4-4, s4-2 → `story:h11-claims-match-what-was-measured`;
  s4-3, s5-1 → `story:vendor-claims-labelled`; s10-2, s1-5 → `story:adversary-tests-have-positive-controls`;
  s8-3, s8-4, s7-5, s10-4, s7-3 → `story:catalogue-road-leftovers`.
- Open, no story: s1-1, s1-2, s1-3, s1-4, s1-6, s1-7, s2-3, s2-5, s2-6, s2-7, s3-2, s3-3, s3-4, s3-5, s4-6,
  s4-7, s4-8, s5-2, s5-3, s7-4, s8-2, s9-3, s9-4, s9-5, s9-6, s10-7, s10-8.

## Findings

```findings
[
{"file":"crates/metaharness/src/loopback.rs","line":533,"category":"auditability","severity":"warning","origin":"pre-existing","message":"s1-1: a request whose token was written upstream but whose response head never arrived is counted in no counter, the after_401 retry likewise; open"},
{"file":"crates/metaharness/src/loopback.rs","line":760,"category":"security","severity":"warning","origin":"pre-existing","message":"s1-2: request body and header lines are buffered in full before the placeholder check, so a local process controls the allocation; one thread per connection with no cap; open"},
{"file":"crates/metaharness/src/loopback.rs","line":552,"category":"auditability","severity":"warning","origin":"pre-existing","message":"s1-3: relay errors are discarded, so an early-ending relay is recorded as forwarded like a complete one; open"},
{"file":"crates/metaharness/src/loopback.rs","line":642,"category":"auditability","severity":"warning","origin":"pre-existing","message":"s1-4: ProxyReport has no field for a request the proxy answered with 502, so a refused upstream reads the same as no traffic; open"},
{"file":"crates/metaharness/tests/adversary_w5u5_tls_production.rs","line":313,"category":"auditability","severity":"warning","origin":"introduced","message":"s1-5: the production TLS cases check only that an out-of-roots issuer is refused, so a production branch trusting no issuer would pass (inferred, not run); story:adversary-tests-have-positive-controls"},
{"file":"crates/metaharness/src/loopback.rs","line":624,"category":"correctness","severity":"note","origin":"pre-existing","message":"s1-6: an upstream interim 100 Continue would be relayed as the final response; Expect is forwarded; trigger inferred; open"},
{"file":"crates/metaharness/src/loopback.rs","line":440,"category":"performance","severity":"note","origin":"pre-existing","message":"s1-7: concurrent connections are unbounded and a silent connection holds a thread up to 60 s; open"},
{"file":"crates/metaharness/src/audit.rs","line":1074,"category":"security","severity":"warning","origin":"introduced","message":"s2-1: Claude H11 ok is built from the launch attestation and the version-keyed MEMORY_CANARIES constant, not from the run's record (AGENTS.md invariant 3); story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness-claude/src/lib.rs","line":353,"category":"auditability","severity":"warning","origin":"introduced","message":"s2-2: MEMORY_NOT_PROBED names only managed memory, while user memory in the config home was not probed either; the ok row reads wider than the measurement; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness/src/builder.rs","line":1959,"category":"correctness","severity":"warning","origin":"pre-existing","message":"s2-3: the b10x arm renders operator paths (--hooks, --driver, --plugin-dir, the workspace cwd) with lossy display(); the scratch guard covers only the scratch root; open"},
{"file":"crates/metaharness-claude/src/lib.rs","line":354,"category":"auditability","severity":"note","origin":"introduced","message":"s2-4: the canary plants in the cwd and its parent only, while the walk names every ancestor; the ok text does not say higher ancestors were never probed; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness/src/builder.rs","line":2104,"category":"auditability","severity":"note","origin":"introduced","message":"s2-5: collect_markdown drops read_dir and entry errors silently and renders names lossily, so H11 can read ok without naming a file it could not list; open"},
{"file":"crates/metaharness/src/builder.rs","line":865,"category":"ordering","severity":"note","origin":"introduced","message":"s2-6: the plugin-variable refusal is decided in plan_launch after loopback_for, so under loopback credentials the custody is opened before the refusal; untested; open"},
{"file":"crates/metaharness/src/builder.rs","line":2063,"category":"performance","severity":"note","origin":"introduced","message":"s2-7: the walk recurses every .claude/rules tree under every ancestor up to / on each Claude launch, unbounded; open"},
{"file":"crates/metaharness/src/audit.rs","line":883,"category":"auditability","severity":"warning","origin":"introduced","message":"s3-1: H11 ok on Claude decided from a flag passed and a per-version constant; invariant 3 not amended; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness/src/audit.rs","line":1112,"category":"correctness","severity":"note","origin":"introduced","message":"s3-2: the floor's canary guard is not the adapter's kept_out(): planted 0 or control_sent above planted reads ok; latent, the pin test requires 12 and equality; open"},
{"file":"crates/metaharness/src/vectors.rs","line":640,"category":"auditability","severity":"note","origin":"introduced","message":"s3-3: the fold sums launch_none and launch_loopback, so a gap does not say under which credential mode memory leaked; open"},
{"file":"crates/metaharness/src/vectors.rs","line":741,"category":"correctness","severity":"note","origin":"introduced","message":"s3-4: the fold test's last assertion recomputes the map it tests, and no gated test feeds the shipped MEMORY_CANARIES through hermetic_floor; open"},
{"file":"spec/domains/session.yaml","line":144,"category":"auditability","severity":"note","origin":"introduced","message":"s3-5: the comment on positive-terminal-record says an absent is_error is neither verdict, while coalesce(is_error, false) reads it as false; spec and code agree; reword; open"},
{"file":"crates/metaharness/src/audit.rs","line":1072,"category":"auditability","severity":"warning","origin":"introduced","message":"s4-1: the H11 ok detail reads as this run's measurement but is a table entry chosen by the self-reported version; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness-claude/tests/native_fixture.rs","line":1052,"category":"auditability","severity":"warning","origin":"introduced","message":"s4-2: the canary re-probe a36 requires at every pin move is #[ignore] and never gated; the gate-time test accepts any self-consistent row; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness-claude/src/launch.rs","line":3104,"category":"evidence-label","severity":"warning","origin":"introduced","message":"s4-3: a37 section 6's statement that ${CLAUDE_PROJECT_DIR} is neither substituted nor checked is an unlabelled vendor claim with no cited evidence, and the near-miss test checks only that planning succeeds (invariant 4); story:vendor-claims-labelled"},
{"file":"crates/metaharness-claude/src/lib.rs","line":310,"category":"correctness","severity":"warning","origin":"introduced","message":"s4-4: the MEMORY_SHAPES doc says .claude/rules is read at any depth, but the canary plants at depth one only, unlabelled; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness-claude/src/lib.rs","line":326,"category":"correctness","severity":"warning","origin":"introduced","message":"s4-5: the canary plants in the cwd and its parent while the walk and the ok text cover every ancestor; story:h11-claims-match-what-was-measured"},
{"file":"crates/metaharness-claude/src/launch.rs","line":769,"category":"correctness","severity":"note","origin":"introduced","message":"s4-6: the non-UTF-8 refusal checks only scratch_root while context.cwd is rendered with display() into --workspace; unreachable through the builder; open"},
{"file":"crates/metaharness-claude/src/launch.rs","line":1303,"category":"security","severity":"note","origin":"introduced","message":"s4-7: the plugin-variable refusal covers the hook command only; a tool-server path or a declared --cwd outside the scratch root reaches the --mcp-config document unchecked (a37 section 6 discloses it); open"},
{"file":"crates/metaharness-claude/src/launch.rs","line":553,"category":"auditability","severity":"note","origin":"introduced","message":"s4-8: the vendor readings behind the quoting rule and the plugin-variable refusal cite scratch files a repository reader cannot open; open"},
{"file":"crates/metaharness-codex/src/launch.rs","line":1172,"category":"evidence-label","severity":"warning","origin":"introduced","message":"s5-1: the comment states as fact that Codex 0.153.4 runs the hook command through a POSIX shell, which a37 lists as unverified (read from strings); story:vendor-claims-labelled"},
{"file":"crates/metaharness-codex/src/hook.rs","line":155,"category":"correctness","severity":"note","origin":"introduced","message":"s5-2: if Codex's -lc shell is fish, a root containing \\\\ or \\' names another path or does not parse; not a regression; open"},
{"file":"crates/metaharness-codex/src/launch.rs","line":540,"category":"auditability","severity":"note","origin":"introduced","message":"s5-3: LaunchRefusal::PathNotUtf8 has no refusal code in the Codex crate; only direct API callers see it; open"},
{"file":"crates/metaharness-tools/src/caller.rs","line":75,"category":"correctness","severity":"warning","origin":"introduced","message":"s7-1: the scratch copy did not escape U+2028 or U+2029, so a valid catalogue holding one beside a space was refused; fixed in fix round 2"},
{"file":"crates/metaharness-tools/src/caller.rs","line":548,"category":"correctness","severity":"warning","origin":"introduced","message":"s7-2: call arguments were never checked against the entry's input fields, so an unread field was dropped and an optional path fell back to the workspace root; fixed in fix round 2 (caller.rs:206, server.rs:172)"},
{"file":"crates/metaharness-tools/src/caller.rs","line":226,"category":"security","severity":"note","origin":"introduced","message":"s7-3: a FIFO swapped in between the regular-file check and the open makes the open block; story:catalogue-road-leftovers"},
{"file":"crates/metaharness-tools/src/server.rs","line":130,"category":"auditability","severity":"note","origin":"pre-existing","message":"s7-4: the expected_sha256 guard applies to reads too, and its refusal speaks of a write; open"},
{"file":"crates/metaharness-tools/Cargo.toml","line":17,"category":"auditability","severity":"note","origin":"introduced","message":"s7-5: b10x-tool-contract is pinned by tag 0.2.0, not by revision as invariant 10 pins harness; the lock holds 60d4d0e; story:catalogue-road-leftovers"},
{"file":"crates/metaharness/src/builder.rs","line":935,"category":"correctness","severity":"note","origin":"introduced","message":"s8-1: the copy read-back runs after the scratch write, the proxy and the strict-version probe, unlike the stated order; a40 section 6 and cli.md reworded in fix round 2"},
{"file":"crates/metaharness/src/builder.rs","line":935,"category":"security","severity":"note","origin":"introduced","message":"s8-2: the read-back is a point-in-time check by path; the server re-reads the same path with no digest bound to the checked value; open"},
{"file":"crates/metaharness/src/run.rs","line":811,"category":"correctness","severity":"note","origin":"introduced","message":"s8-3: under DecisionMode::Ask a call resolving to no catalogue entry goes to the embedder, which may allow it; the server then refuses, but the record says allowed; story:catalogue-road-leftovers"},
{"file":"crates/metaharness/src/run.rs","line":1018,"category":"auditability","severity":"note","origin":"introduced","message":"s8-4: a call outside the catalogue is denied with the vocabulary reason and warning, not one naming the run's catalogue; story:catalogue-road-leftovers"},
{"file":"crates/metaharness-claude/src/vectors.rs","line":945,"category":"auditability","severity":"warning","origin":"introduced","message":"s9-1: the vector's offered_tools check fed a constant in and read it back; fixed in fix round 2 (vectors.rs:915, :945)"},
{"file":"crates/metaharness-claude/fixtures/c1/caller-catalogue.json","category":"correctness","severity":"note","origin":"introduced","message":"s9-2: the fixture offers fs_stat, which no harness operation maps; by design, server.rs:158 refuses an entry nothing in the run performs"},
{"file":"crates/metaharness-claude/src/launch.rs","category":"auditability","severity":"note","origin":"introduced","message":"s9-3: the attestation and session.started record catalogue names only, no digest of the copy or its profile; open"},
{"file":"crates/metaharness-tools/src/caller.rs","category":"auditability","severity":"note","origin":"introduced","message":"s9-4: the catalogue profile (risk, effects, subjects) is validated and never read afterwards; open"},
{"file":"crates/metaharness-claude/src/launch.rs","category":"security","severity":"note","origin":"introduced","message":"s9-5: plan_launch trusts LaunchContext::catalogue; the read-back guard is builder-only; open"},
{"file":"crates/metaharness-claude/src/launch.rs","category":"security","severity":"note","origin":"introduced","message":"s9-6: a --plugin may bring its own MCP servers or tools beside a catalogue (inferred, not run); open"},
{"file":"crates/metaharness-tools/src/caller.rs","line":75,"category":"correctness","severity":"warning","origin":"introduced","message":"s10-1: duplicate of s7-1; fixed in fix round 2"},
{"file":"crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs","line":208,"category":"auditability","severity":"warning","origin":"introduced","message":"s10-2: a_wrong_expectation_is_never_passed_for_any_shape has no positive control and interpreted() ignores the ess exit status, so it passes if ESS evaluates nothing (inferred); story:adversary-tests-have-positive-controls"},
{"file":"CHANGELOG.md","line":25,"category":"auditability","severity":"note","origin":"introduced","message":"s10-3: the dependency line names no reason; deliberate"},
{"file":"crates/metaharness-tools/Cargo.toml","line":16,"category":"security","severity":"note","origin":"introduced","message":"s10-4: duplicate of s7-5; story:catalogue-road-leftovers"},
{"file":"CHANGELOG.md","line":18,"category":"correctness","severity":"note","origin":"introduced","message":"s10-5: the count of values the reader refuses included U+0085 and omitted U+2028 and U+2029; fixed in fix round 2"},
{"file":"crates/metaharness/tests/adversary_w5u5_tls_production.rs","line":328,"category":"auditability","severity":"note","origin":"introduced","message":"s10-7: assertions at :328 and :350 match refusal wording (certificate) rather than a stable refusal name or code; open"},
{"file":"crates/metaharness-codex/Cargo.toml","line":16,"category":"auditability","severity":"note","origin":"introduced","message":"s10-8: direct version pins (toml 1 here, serde_yaml_ng 0.10 in metaharness-tools) bypass the workspace rule; serde_yaml and unsafe-libyaml in the graph have no recorded review; open"}
]
```
