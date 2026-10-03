---
format: aep.planning-md/3
id: review-result:independent-docs-website-parity
kind: review-result
status: active
title: Website Rust verifier parity review
summary: Independent comparison found three introduced parity gaps; coordinator restored quarantine, bootstrap and ambient-conflict behavior before publication.
owner: agent:plan_parallel
relations:
- reviews: story:independent-documentation
revision: 1
---
# Website Rust verifier parity review

```findings
[
{"file":"tools/website/src/verify.rs","line":495,"category":"contract-drift","severity":"blocker","verdict":"NEEDS-CHANGE","origin":"introduced","message":"Quarantine checks omit data, source-assets and updates/field-notes prefixes required by the prior verifier."},
{"file":"tools/website/src/verify.rs","line":341,"category":"contract-drift","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Bootstrap inference uses permission flag instead of validated empty-lock mode."},
{"file":"tools/website/src/verify.rs","line":117,"category":"boundary","severity":"warning","verdict":"CONFIRMED","origin":"introduced","message":"Ambient source-set mode clears conflicting inputs rather than refusing them outside explicit publication layouts."}
]
```
