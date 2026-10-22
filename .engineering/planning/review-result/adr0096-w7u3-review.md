---
format: aep.planning-md/3
id: review-result:adr0096-w7u3-review
kind: review-result
status: active
title: 'Wave 7 unit 3 adversary review: malformed steering line answered by id'
relations:
- reviews: story:malformed-steering-line-answered-by-id
revision: 1
---
# Review: wave 7 unit 3, a steering line that does not parse is answered by id

`aep:adversary` pass 1 over the uncommitted unit 3 change (base `00cd0f8`), 2026-10-21. Two warnings and one
documentation note present before the unit, two notes on the unit's own tests and pages. Two red test files
(`crates/metaharness/tests/adversary_w7u3.rs`, `crates/metaharness-cli/tests/adversary_w7u3.rs`), kept in
the change.

Revision 1 by `aep:implementor` resolved every finding as the coordinator decided: the CLI reads steering
input as bytes and answers a line that is not UTF-8 `MALFORMED` under `-` and keeps reading
(`FramingError::NotUtf8`, `parse_command_bytes`); no event is queued once `stream.closed` is out, and the
marker comes one poll after the child's stream ends so commands handed over before that poll are answered
first (a17, a44 § 4); the events page example decodes; the page test reads every fence whose info string
starts with `json` and compares the quoted reason with the decoder's; `send_line`, `poll_event` and
`drain` are on the library page. No adversary expectation changed; two adversary lines changed for clippy.

The deferred marker made the conformance probe in `crates/metaharness-aep/src/drive/ess_conformance.rs`
stop polling one event early; the coordinator applied the probe change that keeps polling on `Progress`.
The ESS specification is unchanged.

After revision 1: 21 planted mutants, all caught; the reader test compares raw bytes after m16 survived a
lossy comparison.

```findings
[
{"file":"crates/metaharness-cli/src/lib.rs","line":417,"category":"correctness","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"steering() stops reading on the first line that is not UTF-8, so no later line is answered and a later halt is never applied; revision 1 answers that line MALFORMED and keeps reading"},
{"file":"crates/metaharness/src/run.rs","line":593,"category":"protocol","severity":"warning","verdict":"CONFIRMED","origin":"pre-existing","message":"a command.result for a line sent after stream.closed is delivered after the marker (a17), reached by the binary for lines read while wind_up waits on the child; revision 1 queues no event once the marker is out"},
{"file":"website/docs/protocol/events.md","line":13,"category":"docs","severity":"note","verdict":"CONFIRMED","origin":"pre-existing","message":"the events page example does not decode (missing field seq); fixed and held by the page test in revision 1"},
{"file":"crates/metaharness-protocol/tests/commands_page.rs","line":17,"category":"test-coverage","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the page test reads only fences whose info string is exactly json while a44 claims every json block, and does not compare the quoted reason with the decoder; widened in revision 1"},
{"file":"website/docs/reference/library.md","line":93,"category":"docs","severity":"note","verdict":"CONFIRMED","origin":"introduced","message":"the Driving a run table omits the new public Run::send_line; added in revision 1"}
]
```
