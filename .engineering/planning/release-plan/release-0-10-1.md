---
format: aep.planning-md/3
id: release-plan:release-0-10-1
kind: release-plan
status: implemented
title: 'Metaharness 0.10.1: 0.10.0''s features against aep 0.68.1, published with scanned assets'
tags:
- release
refs:
- provider: atlas
  reference: adr-0098
relations:
- supersedes: release-plan:release-0-10-0
- delivers: story:strict-hermetic-with-embedder-scratch-cwd
- delivers: epic:session-format
revision: 2
transitions:
- {from: "draft", to: "implemented", at: "2026-10-20T10:47:10Z", actor: "human:timo", revision: 2}
---
## Why this release

0.10.0 (tag object `a659a232`, commit `3d697011`) was tagged on 2026-10-20, but `b10x-gates
scan-artifact` refused its x86_64 Linux executable ("artifact breaks 1 private rule(s): line 21914
private-identifiers"): a sample tracker key compiled in from `aep-cli`'s help text at the aep pin
`6d7a44d` (aep 0.68.0). `b10x-chef-orchestrator` decided (2026-10-20): aep releases 0.68.1 (0.68.0
plus only the sample-key replacements), metaharness moves its pin and releases 0.10.1; 0.10.0's
GitHub Release stays unpublished, its tag stays (takt pins by git).

## Procedure as run (2026-10-20)

1. Managed tree from `main` `3d697011`, branch `release/0.10.1`.
2. Commit `5cec9355` "deps: link aep 0.68.1": the eight aep pins in `crates/metaharness-aep/Cargo.toml`
   from `6d7a44d` to `2481b48395afbaea9ad9b6165dbc0a80b80e1c01` (aep tag 0.68.1, bot-made, verified by
   `b10x-chef-orchestrator`); `Cargo.lock` moved only the 18 aep packages 0.68.0 → 0.68.1;
   `crates/metaharness-aep/src/eval_tests.rs:314` reads `aep_cli::VERSION` instead of the literal
   `0.68.0` (accepted by `b10x-chef-orchestrator`); CHANGELOG `[Unreleased]` entry. Whole gate green.
3. Commit `c2548b3b` "Release 0.10.1": CHANGELOG cut, workspace version 0.10.0 → 0.10.1, lockfile
   members, README status. Whole gate green: 61 test binaries, 836 passed, 0 failed, 31 ignored;
   conformance 87/87 on ESS 0.56.0.
4. https://github.com/beyond10x/metaharness/pull/35, checks Gate, MSRV 1.98, Security and privacy,
   Build documentation green; `main` fast-forwarded through the bot `3d697011..c2548b3b`.
5. Annotated tag `0.10.1` ("Metaharness 0.10.1"): object `8209f17928c37ec084f2a3e5d8bce86400ab509e`,
   commit `c2548b3bf6f44d204dcb76c674a36d74b3364a37`, pushed through the bot.
6. Release workflow run 39407608562 green (four packages and the archive verification); artifacts
   downloaded by run id; four archive names present; `sha256sum --check SHA256SUMS` exit 0; both Linux
   executables pass `b10x-gates scan-artifact` (x86_64 sha256 `00aede62e5a76f1fbdf3f2b29e08a150567106754b9e79e36c721dd2aeb98cb2`,
   aarch64 `b9d03ca61dcf660916da30faa7382fce5e7e9b278126ec60a4a5415d972f318b`); `metaharness --version`
   prints `metaharness 0.10.1`.
7. GitHub Release 418287055 created by `b10x-gates api --method POST --path
   /repos/beyond10x/metaharness/releases` (the path needs the leading `/`; without it the call is
   refused as an invalid path and nothing is created); assets uploaded by `b10x-gates gh --repo
   <tree> -- release upload 0.10.1 <paths>` with paths relative to the tree (absolute home paths in
   argv are refused under the `personal-paths` rule). The release was visible for about 3 minutes
   without assets between the two calls.

## Publication record

| field | value |
|---|---|
| release | 418287055, https://github.com/beyond10x/metaharness/releases/tag/0.10.1, author `b10x-bot[bot]`, not draft, not prerelease |
| tag | object `8209f179…`, commit `c2548b3b…` |
| `metaharness-0.10.1-aarch64-apple-darwin.tar.gz` | 8,621,015 bytes, `sha256:1e1fa0dcc201f23ac3b025d2fb0d4ef385d4acb1f94b2b5563d6673e4f919cf5` |
| `metaharness-0.10.1-aarch64-unknown-linux-gnu.tar.gz` | 9,485,714 bytes, `sha256:afc50e72c2769d1ae19bc9f4e6f342e798330fd87d5d99c4bd362b9a79bdf7ff` |
| `metaharness-0.10.1-x86_64-apple-darwin.tar.gz` | 9,156,671 bytes, `sha256:5b393b593874391e05738403be5c22d55d1b206655f152ae4411045feeeb5fbc` |
| `metaharness-0.10.1-x86_64-unknown-linux-gnu.tar.gz` | 10,103,380 bytes, `sha256:78acfc9d39b976ef55e3348b48602e15cc8537c4527ee4e4a0a1d7c9e0cf2836` |
| `SHA256SUMS` | 460 bytes, `sha256:3e65118994a1947ffbe00f9a3845aeb783e5c1a69108c3dca678adff4f50ab16` |

Every asset uploaded by `b10x-bot[bot]`; the archive digests equal `SHA256SUMS`.

## Not in this release

A paid run; a vendor pin move; the ESS pin (stays 0.56.0); crates.io or any other registry.
