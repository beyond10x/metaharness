# Native Codex observations with a fixture provider

The actual installed **codex-cli 0.153.4** was exercised on 2026-10-03 using the
production adapter launch plan, an owned loopback Responses fixture, the existing
production hook program and response renderer, and the production rollout reader.
The native rollout itself reports version 0.153.4 and **paginated** history.
The opt-in Rust tests are `crates/metaharness-codex/tests/native_fixture.rs`.

| Native case | Observation |
|---|---|
| Final answer | A successful terminal contains the fixture's exact answer. The normalized final_answer carries it. |
| Model and usage | turn_context records the selected model; the normalized observation has turn-selection scope. Fixture input3/output4 token counts are retained, cumulative usage remains unassigned to a model, and money remains absent. |
| Provider refusal | A local HTTP400 model refusal produces a native error terminal and nonzero vendor exit. Normalization preserves failure and no final answer. |
| Full binary success/refusal | The built metaharness run codex path preserves both results, emits exactly one final stream.closed with the preceding event count, and records native exits0/1. Its own verdict exits are0/3. |
| Command exit0 | The real command writes the fixture marker and produces a correlated structured completion with exit0 and is_error=false. |
| Command exit7 | The real command writes the marker and produces a correlated structured completion with exit7 and is_error=true. The fixture then supplies a successful terminal answer; tool and session outcomes remain distinct. |
| Hook denial | The actual PreToolUse hook is answered deny. The otherwise identical command writes no marker. No CommandExecution completion is retained, so normalized tool status remains unknown; authorization stays separate. |
| Patch | The native apply_patch call creates the fixture file with the expected contents. A correlated patch completion reports success without inventing a numeric exit. |
| Full binary ask/deny | A real tool.requested is answered through CLI stdin, command.result reports acceptance, the denied census is1 and the marker is absent. The fixture then supplies a successful terminal answer. |
| Quiet-provider halt/interrupt | Both commands close the candidate CLI within5seconds while the provider deliberately keeps its response incomplete for10seconds. Native signal9 is retained; halt closes steer-halt and interrupt without a terminal closes error. CLI verdict3 preserves the lack of a successful run. |

The first tool probe incorrectly expected a failed outcome from the hook denial.
It failed: the observed normalized value was unknown. Inspection of the retained
native record found no command completion for that call. This was a probe
assumption contradicted by evidence, not a parser defect: the test now requires
unknown plus the independently retained deny response and absent side effect.
Both command cases and the patch case have actual structured outcomes.

The first full-binary probe expected Metaharness to return the native failure
exit1. Its actual exit3 matches the CLI's run-verdict contract; native exit1 was
already preserved in stream.closed.process. The corrected test checks both
values separately. Neither failed probe required a production change.

For these native calls, the hook's tool_use_id equals the rollout's call_id.
The Bash hook input contains only command, which is the shape the governed shell
mapping accepts. This differs from older historical observations of separate
identifier namespaces; the adapter still preserves observed identifiers and never
joins calls by text or timing. This probe does not exercise the full AEP driver.

## Bounds and reproducibility

Each run uses fresh HOME and CODEX_HOME directories, CredentialSource::None, no
credential or plugin copies, and an explicit loopback-only model endpoint. The
fixture rejects any Authorization header; all observed requests had none. The
child and its process group have a 30-second deadline and an unwind cleanup guard.
A gate test forces a servicing panic and checks that its owned child was reaped.
HTTP reads/writes have
three-second limits. Tests serialize native processes and retain evidence in an
explicit private directory outside source. The source gate only compiles these
tests; they remain ignored until explicitly selected. No paid model was called.

```console
cargo build --locked --bin metaharness
METAHARNESS_NATIVE_CODEX="$(command -v codex)" \
METAHARNESS_NATIVE_DRIVER="$PWD/target/debug/metaharness" \
METAHARNESS_NATIVE_EVIDENCE="$HOME/.cache/metaharness-native-fixture" \
cargo test --locked -p metaharness-codex --test native_fixture -- --ignored --test-threads=1
```

Both binaries must have absolute paths. The actual session version is
checked against 0.153.4; changing that expectation requires another qualification.
Private results for this run are under `~/.cache/metaharness-issue-repair/native-fixture`.
No actual transcript is committed. The deterministic provider responses are
synthetic inputs; the resulting vendor records, process behavior and file effects
are native observations. Fixture token figures are not evidence of model work or
monetary expenditure.

This is narrower than hosted-provider or subscription qualification. It does not
prove real model quality, hosted routing, AEP
governance, all hermetic controls, or either other vendor's current
release. Amendment a25 advances the pin to0.153.4 for the qualified surfaces; the
remaining unverified claims are retained.

All four opt-in tests passed, exercising eight native processes. The full binary
cases cover terminal success/refusal; command, denial and patch cases exercise
the adapter and native hook directly. Native executions are coordinator
verification. A separate read-only reviewer found an unbounded redundant version
probe and missing child cleanup on panic; the probe was removed and cleanup
added. That review did not independently execute the native suite.

## Quiet-stream regression and extended candidate

The native cancellation extension exposed a real production defect: the CLI did
not service its queued command while the provider stream was quiet. The original
timing test failed its five-second bound, even though the eventual closure
correctly recorded the command and native signal. The initial test's wire-field
spellings were first corrected against the protocol; the subsequent timing
failure is the actual defect, tracked by story:silent-stream-steering.

The polling repair candidate (binary SHA256
`0c03a7c4ed7002cc89dfee53111862c8d2f37048e255d8b0628ba62c7f3a2c63`)
passed all six native tests, including the original failing loop. This extended
run exercised eleven processes in96.23seconds. It ran inside an owned Linux
user/network namespace with only loopback enabled, using the same provider and
production launch paths. The namespace was a test condition, not a product
isolation claim. Private evidence is native-steering-candidate.log alongside the
original cancel-timing.log. The candidate still requires final integration and
review; this observation alone is not a release or pin qualification.

## Integrated 0.9.0 candidate

The final six-test native lane passed in56.27seconds against the immutable
production CLI SHA256484ea4fedfa25d2974982899991e84c9186a0b194ac00cc7d9f736cf6888227b.
It exercised11actual native processes in a loopback-only namespace with fresh
homes and no credentials. The corrected cancellation fixture waits for actual
session startup and provider activity before sending the correlated command.
