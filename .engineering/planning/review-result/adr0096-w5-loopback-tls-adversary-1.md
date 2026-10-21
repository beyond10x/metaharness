---
format: aep.planning-md/3
id: review-result:adr0096-w5-loopback-tls-adversary-1
kind: review-result
status: active
title: Adversary, wave 5 unit 5 (loopback TLS leg tested), pass 1
owner: metaharness
refs:
- provider: atlas
  reference: adr-0096
relations:
- reviews: story:loopback-tls-leg-tested
revision: 1
---
unit: story:loopback-tls-leg-tested, pass 1 — uncommitted tree wt-3b79492b7d71 (wave 5 unit 5, base 4a72d8c)
verdict: CONFIRMED (1 introduced warning, 3 pre-existing findings; no blocker)
cases: executed 1014→1018, red 0 on the tree (2 red on a mutant of the production trust line)
origin: introduced 1 / pre-existing 3 / undecided 0
wrote-outside-worktree: 1 directory (metaharness-wave-adr0096/w5/u5-adv1 under the operator's cache directory)
needs-coordinator: a dependency move for the TLS stack (outside this unit), and keep or replace the added test file

## Cases added (kept, `crates/metaharness/tests/adversary_w5u5_tls_production.rs`)

| case | asserts |
|---|---|
| `the_production_build_refuses_an_issuer_outside_the_compiled_in_roots_over_tls13` | through the public `LoopbackProxy::start` only: 502, no token in the body or on the wire, the upstream saw a TLS record and no plaintext, `forwarded == 0` |
| `…_over_tls12` | the same over TLS 1.2 |
| `a_cleartext_answer_on_the_https_leg_is_refused_and_no_credential_crosses` | the same against an upstream answering in cleartext |
| `an_upstream_that_closes_mid_handshake_gets_no_credential` | the same against an upstream closing mid-handshake |

All four green on the tree; the first two go red when the production trust line accepts any issuer, while
the unit's own 16 `loopback::` tests stay green. Suite: 82 test binaries, 1018 passed, 0 failed, 49 ignored.
V7 and V8 held 25/25 idle and 10/10 under load.

## Attacked and not broken

Production cannot reach the test seam (`start_trusting` absent outside `cfg(test)`); the production TLS
configuration is unchanged (identical Debug output: ALPN none, SNI on, early data off, no client
certificate, 121 roots); no token in any 502 body or on the wire; the `Cargo.lock` shift adds 21 packages,
all MIT and/or Apache-2.0, none removed; the CHANGELOG line and module comment are accurate.

## Disposition (coordinator)

- Kept as an adversary test: finding 1 (the four cases above); the coordinator stated the coverage of the
  production line in the `loopback.rs` module comment and the vector 8 doc before the commit (`5c4de3f`).
- Fixed outside this unit: finding 2, by unit 7's dependency move (`ceffb6d`, merged `3bc0de4`).
- Open, no story: findings 3 (`loopback.rs:546`) and 4 (`loopback.rs:1144`).

## Findings

```findings
- file: crates/metaharness/src/loopback.rs
  line: 666
  category: mutant
  severity: warning
  verdict: CONFIRMED
  origin: introduced
  message: "no unit test compiles the cfg(not(test)) trust line; a mutant accepting any issuer on that line kept the 16 loopback:: tests green, V7 and V8 included; only tests/adversary_w5u5_tls_production.rs catches it; kept"
- file: Cargo.lock
  line: 1974
  category: dependency
  severity: warning
  verdict: CONFIRMED
  origin: pre-existing
  message: the TLS stack on the credential leg (rustls, rustls-webpki, webpki-roots) was behind its current releases at the same versions as the base; moved by unit 7 to rustls 0.23.46, rustls-webpki 0.103.16 and webpki-roots 1.0.10
- file: crates/metaharness/src/loopback.rs
  line: 546
  category: contract-drift
  severity: note
  verdict: CONFIRMED
  origin: pre-existing
  message: forwarded is counted only after a response head; an upstream that received the custody token and closed left forwarded at 0, while ProxyReport documents it as relayed upstream; open
- file: crates/metaharness/src/loopback.rs
  line: 1144
  category: correctness
  severity: note
  verdict: INFEASIBLE
  origin: pre-existing
  message: a close-delimited body cut short without close_notify reaches the child as a clean close (RFC 9112 section 9.8); the module comment already discloses it; whether the vendor sends close-delimited bodies is unknown; open
```
