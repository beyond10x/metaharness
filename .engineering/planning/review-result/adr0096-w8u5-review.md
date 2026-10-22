---
format: aep.planning-md/3
id: review-result:adr0096-w8u5-review
kind: review-result
status: active
title: 'Wave 8 unit 5 review: first-user documents agree with the CLI'
relations:
- reviews: story:first-user-documents-agree-with-cli
revision: 1
---
# Review: wave 8 unit 5, first-user documents agree with the CLI

One `aep:adversary` pass over revision 1 (base `7332266`), 2026-10-22, cases in
`crates/metaharness-cli/tests/adversary_w8u5.rs`. Five cases, two red; 21 mutants, 16 caught by the unit's
checks, 3 caught only by the adversary's green cases, 2 by nothing.

Findings and what the coordinator decided:

- Warning (pre-existing): three pages said a bare `--hermetic` run exits on the floor's verdict; with no audit
  requested no floor is evaluated. D50: the pages say the run exits on stream completion and that `--audit` or
  `--hermetic strict` adds the floor's verdict.
- Warning (introduced): design § 9.2 named six verbs while the CHANGELOG and the test said the design names
  the eight the binary carries. D51: § 9.2 names `aep drive` and `mcp-serve`, recorded in a49 § 6.
- Warning (introduced): the original § 4.2 sentence was rewritten rather than marked, and a49 dated Codex
  observe to a25. D52: the sentence is restored with a superseded marker; a49 dates it to `748376f`
  (2026-08-23).
- Notes: a49 § 4 limit 2 missing from the control-seam and codex pages; "Rust 1.98" where CI runs stable; the
  website README page count typed by hand; a malformed `frame.set` refused MALFORMED before
  UNSUPPORTED_CONTROL. D53: each stated or checked.
- Note: mutants M16 (typed page count in the build) and M17 (prose gate summary) passed every check. Revision
  2 generates the build summary and adds a check for each.

Checked and not faulted: the lines a49 cites say what it claims; the counts, mode table, verbs, exit table,
install line, `--cwd`, Taskfile steps and ESS version match the binary; the page count is generated.

Revision 2 by `aep:implementor`: both red cases green. Gate `w8u5r` exit 0 on every step, 1354 passed,
0 failed, 50 ignored; ESS conformance 116/116.

```findings
[
{"file":"website/docs/quickstart.md","line":null,"category":"docs-claim","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"a bare --hermetic run was said to exit on the floor's verdict; revision 2 says it exits on stream completion"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":null,"category":"design-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"section 9.2 named six verbs; revision 2 names the eight, a49 section 6"},
{"file":"docs/design/metaharness-protocol-v0.1.md","line":null,"category":"amendment-record","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"section 4.2 was rewritten and misdated; revision 2 restores it with a marker and dates to 748376f"},
{"file":"website/docs/control-seam.md","line":null,"category":"docs-claim","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"a49 limit 2 missing; stated in revision 2"},
{"file":"crates/metaharness-cli/tests/website_claims.rs","line":null,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"five mutants passed; adversary cases kept and two checks added"},
{"file":"website/README.md","line":null,"category":"docs-claim","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"hand-typed page count; checked in revision 2"}
]
```
