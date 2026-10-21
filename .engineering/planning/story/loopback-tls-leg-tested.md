---
format: aep.planning-md/3
id: story:loopback-tls-leg-tested
kind: story
status: implemented
title: The loopback's upstream TLS leg is exercised by a test
revision: 7
transitions:
- {from: "draft", to: "proposed", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 3}
- {from: "proposed", to: "active", at: "2026-10-21T12:00:26Z", actor: "human:timo", revision: 4}
- {from: "active", to: "implemented", at: "2026-10-21T18:43:46Z", actor: "human:timo", revision: 7, decided_on: {"recorded":{"test_result":1,"review_outcome":2,"ess_conformance_coverage_v1":1}}}
---
# Story: the loopback's upstream TLS leg is exercised by a test

## Scheduling

Wave 5 unit 5 (`approval-record:adr-0096-wave-5`), filed 2026-10-21 from the repository review of that date
(finding 5).

## Outcome

The rustls leg of the credential loopback, the one that carries the real credential upstream, runs in the
suite against a local TLS fake upstream, so a regression in it fails a test rather than a live session.

## Context

- At filing, `crates/metaharness/src/loopback.rs:51-54` (base `ca40260`) labelled the TLS leg "built, not
  verified": every vector ran against a plain-HTTP fake upstream, and the comment said the leg flips to
  verified when V-LP2 runs one live session through it.
- That comment read V-LP2 as unrun, which was stale. `docs/design/loopback-provider-v0.1.md:62` records
  V-LP2 as **verified, 2026-08-23** (run `claude-1469713`): the upstream accepted the custody-attached
  OAuth bearer on a live `--credentials loopback` run, the child held only the placeholder
  (`credential_source: "none"`, H4/H6 imposed) and the model answered; `:63` records V-LP3 verified in
  the same run. The gap this story closes is the suite's: no test exercised the rustls leg. (Corrected
  2026-10-21 at the wave-5 close, from the unit's implementor report.)
- A local TLS upstream with a test certificate authority needs no network and no credential.

## Acceptance

- A test starts a TLS fake upstream on loopback with a test CA, points the loopback at it, sends a request
  through, and asserts the upstream received the injected credential header and the child received the
  response bytes.
- A test with an untrusted certificate asserts the loopback refuses the upstream and the credential is not
  sent.
- The module comment's "unexercised" bullet is updated to what the tests now cover.

## Delivered

Commit `5c4de3f`, merged `5b6d2b4` into `integration/adr-0096-wave-5`; one `aep:adversary` pass
(`review-result:adr0096-w5-loopback-tls-adversary-1`, CONFIRMED, one introduced warning); implementor gate
81 test binaries, 1014 passed, 0 failed, 49 ignored, conformance 103/103. Vector 7 runs a rustls fake
upstream on 127.0.0.1 under a certificate authority the test generates, trusts it, and checks the custody
token arrives past the handshake and the response bytes reach the child. Vector 8 presents three untrusted
upstreams (an issuer outside the compiled-in roots, an issuer outside the roots the seam was given, a
trusted issuer that signed for another name) and checks the handshake fails before any request, so the
token never crosses. The test CA reaches the proxy through `start_trusting`, which exists only under
`cfg(test)`; production TLS configuration is unchanged. The adversary's pass found that no unit test
compiles the `cfg(not(test))` trust line; the coordinator kept
`crates/metaharness/tests/adversary_w5u5_tls_production.rs`, which drives the production build through
`start`, and stated that coverage in the `loopback.rs` module comment and the vector 8 doc. Dev-dependency
`rcgen` 0.14.10 (21 lockfile packages, MIT and/or Apache-2.0, no non-dev change on Linux). No amendment
(a39 unused).

Whole gate at the wave-6 head: task check, one step at a time, on `integration/adr-0096-wave-6` at
`5ddebbd`: 6 steps exit 0, 96 test binaries, 1128 passed, 0 failed, 49 ignored, conformance 103/103 on ESS
0.67.0.

Deviations: the pass's pre-existing dependency warning on the TLS crates was closed by wave 5 unit 7
(commit `ceffb6d`, merge `3bc0de4`, no story). Its two pre-existing notes stay open: `forwarded` is counted
only after a response head (`loopback.rs:546`), and a close-delimited body truncated without `close_notify`
reaches the child as a clean close (`loopback.rs:1144`). Not covered: the compiled-in roots accepting a real
vendor chain. The independent review notes the production test has no positive control for an empty root
store (`story:adversary-tests-have-positive-controls`, draft).
