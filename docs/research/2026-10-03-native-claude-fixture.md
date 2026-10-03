# Native Claude observations with an owned Messages fixture

The installed Claude Code **2.1.288** was exercised on 2026-10-03 through the
production `metaharness run claude` binary and its existing adapter. The Rust
fixture is `crates/metaharness-claude/tests/native_fixture.rs`. Actual vendor
opening records carry the version; no vendor pin is advanced by this report.

The provider and vendor run together in a Linux user/network namespace with
only loopback enabled. The fixture checks `/proc/net/dev` before spawning any
vendor. External networks are unreachable under these test conditions. This is
fixture confinement, not a claim that the product's launch flags isolate the
network. In particular, the `credentials:none` endpoint path does not suppress
all vendor analytics traffic by itself.

Each invocation has fresh HOME/config/cwd/tmp directories. The existing
`--credentials none --model-endpoint` path supplies the synthetic
`metaharness-model-endpoint` placeholder, never operator credentials. The owned
provider admits only this exact key on Messages calls and no Authorization
header. The observed preliminary `HEAD /api/hello` request carries no key and
receives an empty response. Retained request metadata records credential
predicates, never unexpected credential values.

| Native observation | Result and boundary |
|---|---|
| Success | Exact fixture answer appears as normalized text, native exit 0, terminal `is_error:false`, one final `stream.closed` with the correct event count, CLI verdict exit 0. |
| Provider refusal | Local HTTP 400 produces terminal `is_error:true`, native exit 1, closure `error`, CLI verdict exit 3. Native subtype is `success` despite the error flag; failure is not inferred from that subtype. |
| Model/usage | Opening model is the requested model. Assistant usage carries input 3/output 0; terminal usage and per-model totals carry input 3/output 4. The fixture intentionally supplies these numbers. |
| Accounting | Native terminal cost 0.000069 for the successful one-response fixture is a vendor calculation over synthetic usage, not money spent or hosted usage evidence. The adapter preserves it. |
| Allowed Bash | The production ask seam receives and allows the correlated call. The real command creates the marker; normalized tool result reports `is_error:false`. |
| Denied Bash | The same seam receives deny. The marker is absent and the correlated native tool result reports `is_error:true`. The subsequent fixture answer succeeds; denial is distinct from terminal failure. |
| Turn ceiling | With `--max-turns 1`, a provider that requests another tool cannot continue indefinitely: native terminal subtype `error_max_turns` and `is_error:true` are preserved. |
| Extension inventory | MCP list is empty. Three vendor-built-in plugins remain loaded, listed below; an empty plugin set is **not** established. |

The built-in plugins are `cc-plugin-agents-md`, `cc-plugin-telemetry`, and
`cc-plugin-plugin-authoring`, each with a `@builtin` source. Fresh config and no
injected plugins do not make H1a's empty-plugin qualification pass on this
version. The test records the observed inventory instead of suppressing it.
The wider compatibility matrix remains incomplete, including general plugin
isolation, hosted routing, paid accounting, arbitrary tools, and AEP governance.

The initial probes exposed two incorrect fixture assumptions: all requests were
assumed to have JSON bodies, whereas the vendor first sends a credential-free
HEAD request; and all extensions were assumed absent, whereas the native opening
record lists built-ins. The original red logs remain in private scratch. The
coordinator authorized correcting the inventory expectation while retaining the
H1a qualification gap. No production behavior changed for these observations.

## Cancellation and execution bounds

Interrupt/halt tests wait until the native process reaches a deliberately quiet
provider, send the existing CLI steering command, and require process closure
within two seconds. The first such probe failed after that bound: the CLI did
not service interrupt while the provider was quiet. The private
`native-cancel-red.log` captures the failure; the guard terminated the owned
processes. Cancellation qualification depends on the separate CLI steering
liveness repair and a green rerun. The non-cancellation lane passed three tests
exercising five processes.
Every native fixture invocation has an outer 45-second deadline and an RAII
guard that kills the owned process group and reaps the direct child during
unwind. HTTP headers and bodies have a shared three-second read deadline;
writes are bounded, and the quiet provider checks a stop flag. The default,
non-native test exercises cleanup during panic.

## Reproduction

Build the production driver from the candidate tree, and compile the ignored
fixture tests. Execute the test binary in an isolated namespace, not Cargo's
dependency fetching/build steps. The vendor and driver paths must be absolute.

```console
cargo build --locked --bin metaharness
cargo test --locked -p metaharness-claude --test native_fixture --no-run
unshare --user --map-root-user --net /bin/sh -c \
  'ip link set lo up && exec "$@"' fixture \
  env METAHARNESS_NATIVE_NETNS=loopback-only \
  METAHARNESS_NATIVE_CLAUDE="$(command -v claude)" \
  METAHARNESS_NATIVE_DRIVER="$PWD/target/debug/metaharness" \
  METAHARNESS_NATIVE_EVIDENCE="$HOME/.cache/metaharness-native-claude-fixture" \
  target/debug/deps/native_fixture-<hash> --ignored --test-threads=1
```

Use the executable path printed by Cargo in place of `<hash>`. These native
cases remain ignored in `task check`. Private evidence for this work is under
`~/.cache/metaharness-issue-repair/native-claude-fixture`; no actual transcript or
provider request is committed. Native executions are implementor evidence and
do not constitute independent review.
