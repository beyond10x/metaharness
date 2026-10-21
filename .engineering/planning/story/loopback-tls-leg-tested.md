---
format: aep.planning-md/3
id: story:loopback-tls-leg-tested
kind: story
status: draft
title: The loopback's upstream TLS leg is exercised by a test
revision: 1
---
# Story: the loopback's upstream TLS leg is exercised by a test

## Scheduling

Draft, filed 2026-10-21 from the repository review of that date (finding 5). Wave 5 candidate.

## Outcome

The rustls leg of the credential loopback, the one that carries the real credential upstream, runs in the
suite against a local TLS fake upstream, so a regression in it fails a test rather than a live session.

## Context

- `crates/metaharness/src/loopback.rs:51` labels the TLS leg "built, not verified": every vector runs
  against a plain-HTTP fake upstream, and the comment says it flips to verified when V-LP2 runs one live
  session through it.
- The store has no artifact naming V-LP2 (`grep -rln V-LP2 .engineering/planning`: no match), so the gap
  is tracked only by that comment.
- A local TLS upstream with a test certificate authority needs no network and no credential.

## Acceptance

- A test starts a TLS fake upstream on loopback with a test CA, points the loopback at it, sends a request
  through, and asserts the upstream received the injected credential header and the child received the
  response bytes.
- A test with an untrusted certificate asserts the loopback refuses the upstream and the credential is not
  sent.
- The module comment's "unexercised" bullet is updated to what the tests now cover.
