---
format: aep.planning-md/3
id: story:substrate-host-0-20-confinement-only
kind: story
status: active
title: The AEP eval links substrate-host 0.20.0 without its git and kubernetes stacks
summary: Pin substrate-host 0.20.0 by rev with default-features = false; the lock drops git2, gix, curl and the kubernetes client
tags:
- pin
scope:
- confidence: inferred
  path: Cargo.lock
- confidence: inferred
  path: crates/metaharness-aep-eval/Cargo.toml
revision: 5
transitions:
- {from: "draft", to: "proposed", at: "2026-11-05T11:09:21Z", actor: "human:timo", revision: 2}
- {from: "proposed", to: "active", at: "2026-11-05T11:09:22Z", actor: "human:timo", revision: 3}
---
## Scheduling

Filed 2026-11-05 for the 0.30.0 wave, and accepted by `approval-record:accept-drafts-0-30-0`. This is a pin
move that rides with `story:eval-cgroup-root-check-by-prefix` in the same crate.

## Outcome

`metaharness-aep-eval` links `b10x-substrate-host` 0.20.0 with `default-features = false`. The workspace lock
no longer carries the git and kubernetes stacks, which a confinement-only consumer does not use.

## Context

- 0.29.0 pinned `b10x-substrate-host` 0.19.2 by rev (`aeab475`). That release has no features, so the lock
  gained 175 packages, among them gix, git2, curl, openssl, aws-lc and the kubernetes client.
- Substrate 0.20.0 (tag at `a88cc6423164ec586434b786b3b1103cc2a17c33`) has `default = ["git", "kubernetes"]`:
  - `git` (`dep:curl`, `dep:git2`, `dep:gix`) gates only code in `writable_roots.rs`;
  - `kubernetes` (`dep:substrate-kubernetes-client`) gates only the `pod` confinement adapter.
- The eval uses `confinement::open`, `AdapterRequest::Systemd`, `SystemdRequest`, `InnerRequest::Local`,
  `LocalRequest`, `LocalPolicy::Refuse`, `AdapterName::name` and `ADAPTER_UNSERVED`. None of them is
  feature-gated.
- `crates/substrate-host/tests/confinement-public-items.txt` is identical between 0.19.2 and 0.20.0.
- Read from the tags, not built.

## Acceptance

- expected_before: the lock holds `git2`, `gix`, `curl-sys` and `b10x-substrate-kubernetes-client`.
- After:
  - the pin reads `rev = "a88cc6423164ec586434b786b3b1103cc2a17c33", default-features = false`;
  - the lock holds none of those four crates;
  - a test in `metaharness-aep-eval` asserts their absence from `Cargo.lock`;
  - the eval's adapter tests, including the session-bus refusal and the entry-point preflight case, still
    pass.
- The report gives the lock diff by package.
