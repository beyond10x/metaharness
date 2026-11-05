---
format: aep.planning-md/3
id: story:adversary-tests-have-positive-controls
kind: story
status: implemented
title: Adversary tests have positive controls
summary: Two kept adversary tests turn red when what they guard stops working.
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- decomposes: epic:session-format
- informed_by: review-result:adr0096-w5-independent-review
scope:
- confidence: cited
  path: crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs
- confidence: cited
  path: crates/metaharness/tests/adversary_w5u5_tls_production.rs
revision: 6
transitions:
- {from: "draft", to: "proposed", at: "2026-11-04T07:33:27Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-04T07:33:27Z", actor: "human:timo", revision: 3}
- {from: "active", to: "implemented", at: "2026-11-05T02:52:36Z", actor: "human:timo", revision: 6, decided_on: {"recorded":{"test_result":1,"verification":1}}}
---
# Story: adversary tests have positive controls

## Scheduling

Draft, filed 2026-10-21 at the close of wave 5 from `review-result:adr0096-w5-independent-review` (s10-2,
s1-5).

## Outcome

Two kept adversary tests fail when the thing they guard stops working, not only when it misbehaves.

## Context

- `crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs:208-230`,
  `a_wrong_expectation_is_never_passed_for_any_shape`, asserts only that the list of wrongly passed shapes is
  empty, and `interpreted()` at `:141` does not check the `ess` exit status, so it would pass if ESS
  evaluated nothing (s10-2; the vacuous pass is inferred).
- `crates/metaharness/tests/adversary_w5u5_tls_production.rs:313-355` checks only that an out-of-roots
  issuer is refused, so a production trust line that trusts no issuer would pass (s1-5; inferred, not run).

## Acceptance

- expected_before: today both tests stay green under a mutant that disables what they guard (ESS evaluating
  nothing; an empty root store). After: each test carries a positive control (a shape ESS must pass and
  a successful `ess` exit; a chain the compiled-in roots must accept, or a canary against
  `webpki_roots::TLS_SERVER_ROOTS`), and each named mutant turns it red.

## Scope

`crates/metaharness-aep/tests/adversary_session_close_absent_fields.rs`,
`crates/metaharness/tests/adversary_w5u5_tls_production.rs`.
