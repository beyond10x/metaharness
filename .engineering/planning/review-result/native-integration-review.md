---
format: aep.planning-md/3
id: review-result:native-integration-review
kind: review-result
status: active
title: Native fixtures, polling and process-write review
relations:
- reviews: story:silent-stream-steering
- reviews: story:native-process-write-subtrees
- reviews: story:current-adapter-compatibility
revision: 1
---
unit: Codex85e81f17; quiet7e9a310+8bb5f369; process-write692be175+aa2b51fe; integrationa25 pin diff
verdict: PASS after corrections; combined native/gate evidence remains required
cases: reviewer executed zero; read-only static review
origin: introduced findings corrected
wrote-outside-worktree: none
needs-coordinator: final combined-driver tests, generated goldens, source/version pairing and release gates

Independent reviewer plan_parallel found a Codex cancellation readiness race, b10x fixture invalid command tags and unjustified causal attribution, quiet-reader progress throttling, and a leading-hyphen process-directory forwarding edge. Implementations now await actual startup before cancellation, send protocol halt/interrupt with correlated results, retract malformed-command causal evidence, distinguish progress from idle, and use equals-form forwarding for leading hyphens. Reviewer read the corrected Codex and quiet bytes and the process-write base; coordinator verifies the small leading-hyphen regression and final combined gate. The separately observed b10x no-wire interrupt no-op has a generic fallback and measured ESS regression. No credential or paid provider execution was performed by the reviewer.

Codex pin changes preserve the historical0.144 capture; generated warnings name0.153.4. Registration/turn, replacement and hosted semantics remain unverified. Process-write specification operations inspect real argv/admission, not kernel mount containment; native host withholding remains explicit.
