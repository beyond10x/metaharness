# Native b10x fixture qualification, 2026-10-03

The opt-in Rust integration target `metaharness-b10x/tests/native_fixture.rs` drives
an explicitly named, actual installed `b10x-harness` through the production
`metaharness run b10x` binary. Its owned Responses endpoint serves deterministic
fixture output. This qualifies the launch and observation path; fixture token
counts are synthetic and do not establish paid model consumption or hosted
provider compatibility.

The complete test process, fixture server and descendant processes run in a Linux
user/network namespace containing only loopback. Tests check `/proc/net/dev`
before spawning. They bind the provider to `127.0.0.1`, clear the driver's
environment, use a new private HOME and working directory, name `--credentials
none`, and reject any authorization or API-key header. No real credential or
placeholder is required. The provider never forwards a request. HTTP reads and
writes, the native process, and fixture shutdown are bounded. A child guard kills
and reaps the owned process group on panic. Private records stay outside Git.

The observations target installed binary **0.13.3**. Amendment a25 advances the
adapter pin and Cargo source pair to0.13.3 at
`798325f03cf5a18df8fadb346d31b314826136ec`, verified against remote main and the
peeled release tag. Native session metadata must name0.13.3. The fixture observes
behavior, while the source/version checks establish the declared release pair;
it does not prove binary reproducibility. Earlier pre-pin results below remain
historical observations.

## Reproduction

Build the production CLI and the test target with Cargo outside the namespace.
Use the exact test executable path printed by `cargo test --no-run`; do not select
a stale binary from an old toolchain. Set the three absolute paths explicitly:

```console
cargo build --locked --bin metaharness
cargo test --locked -p metaharness-b10x --test native_fixture --no-run
unshare --user --map-root-user --net /bin/sh -c 'ip link set lo up && exec "$@"' fixture \
  env METAHARNESS_NATIVE_NETNS=loopback-only \
  METAHARNESS_NATIVE_B10X=/absolute/path/to/b10x-harness \
  METAHARNESS_NATIVE_DRIVER=/absolute/path/to/metaharness \
  METAHARNESS_NATIVE_EVIDENCE=/absolute/private/evidence \
  /absolute/path/to/native_fixture-test --ignored --test-threads=1 --nocapture
```

The tests remain ignored in ordinary `task check`. Native executions must be
serialized with other native qualification in this workspace. Their stderr names
the private evidence directory; normalized events, vendor transcripts, provider
requests and errors remain there for inspection.

## Scope and limits

The suite asserts terminal success and provider refusal, selected model and
fixture usage, successful and failed `file_read` calls, the observe-only seam,
a declared one-turn ceiling, and cancellation of a quiet outstanding request.
The tool result replay verifies real fixture-file bytes from the native tool.
No Metaharness decision is sent: every tool request must have
`decision_required: false`, `seam: none`, and zero allow/deny census.

The final-answer and observed-model aggregation added to the Codex adapter is
not claimed for b10x. Cost remains unknown without a declared rate card. The
native usage event's model field is what that record states, not independent
identification of the serving model. This suite does not qualify MCP, skills,
plugins, native governance, AEP role execution, paid budgets, confined writes,
subscription authentication or real provider/model semantics. It does not
substitute for a governed native walk or authorize a pin upgrade.

## Probe corrections and observed failures

The first fixture assumed a `/v1/responses` route; the actual production b10x
launch uses its endpoint as a base URL and requested `/responses`. The fixture
now asserts the observed route. No production route was changed.

A second initial assertion assumed a provider 400 refusal would include a native
terminal record. Actual 0.13.3 writes no `finished` record on that error. The
normalized record preserves this absence: `NO_TERMINAL_RECORD`, no
`session.ended`, final closure reason `error`, native exit 1, and CLI exit 3.
The corrected test asserts those facts instead of synthesizing a terminal event.
The one-turn ceiling produces `max-turns`, native exit 2 and CLI exit 3.

Independent review found that the original quiet-request test sent unsupported
`run.cancel` and `session.interrupt` commands. Its timeouts establish a fixture
defect, not a production steering defect. Those private logs remain preserved;
the earlier causal attribution is withdrawn. The corrected test sends `halt`
and `interrupt`, waits for both an actual provider request and `session.started`,
and requires the correlated successful `command.result` and native termination.

## Recorded result

The non-cancellation lane executed four tests, covering six native launch cases:
text success, provider refusal, successful and failed file reads, the turn ceiling,
and strict-version refusal. It reported `4 passed; 0 failed; 1 filtered out`.
Strict-version refusal is a planning refusal (CLI exit 2), not a failed started
run (CLI exit 3); it names both versions and emits no event or provider request.
The initial probe expected exit 3 and was corrected to the planning-refusal
contract after observing the named refusal.

The corrected cancellation verifier requires closure within two seconds after
the command and a thirty-second overall startup ceiling. Against the quiet-stream
candidate, `halt` passed with native signal 9, CLI exit 3 and `steer-halt` closure.
`interrupt` received a correlated successful acknowledgement but did not stop
the native process within two seconds. The full lane reported `4 passed; 1 failed`
in 2.38 seconds. This valid-command observation exposed a separate no-control-wire
interrupt defect: the adapter advertises honoured interrupt, but supplies no
control line and the core did not stop the process. The no-wire fallback repairs that defect. Both halt and interrupt subsequently
passed against the repaired candidate, preserving measured signal9 and CLI3.

On Rust 1.99, the ordinary package lane retained its 31 passing unit tests and
two existing ignored fixture-regeneration tests, added five ignored native tests,
and completed its zero doctests. The default gate intentionally does not execute
these native tests; it cannot be cited as native qualification.

## Release candidate qualification

The full matrix now includes strict-version acceptance of the qualified0.13.3
binary, the prompted read-only scoper and the process-write declaration probes.
The older strict-mismatch refusal above was observed before the deliberate pin
advance; it is not the current expected result. Role observations and explicit
withheld-unverified containment limits are recorded in
2026-10-03-native-b10x-scoper-fixture.md and2026-10-03-native-process-write-subtrees.md.

The final integrated lane passed seven tests in 0.58 seconds against production
CLI SHA256484ea4fedfa25d2974982899991e84c9186a0b194ac00cc7d9f736cf6888227b.
The two containment cases explicitly report withheld-unverified, with a named run
withholding reason and no filesystem effects. They are not containment-success
observations. The other cases cover actual native launches and the strict pin.
