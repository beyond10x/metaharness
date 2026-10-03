# Installed adapter compatibility, 2026-10-03

Issue #15 is qualified by observations of actual vendor binaries through an owned
loopback provider and the production Metaharness CLI. Native requests use fresh
private homes, no operator credentials, bounded processes and a Linux namespace
with only loopback. Provider replies and usage are synthetic; these tests do not
establish hosted authentication, model quality or paid spend.

| Adapter | Actual native version | Pin decision |
|---|---|---|
| Codex | 0.153.4 | Advance from0.145.0 after the final integrated-driver rerun. |
| Claude | 2.1.288 | Retain2.1.259: three built-in plugins leave H1a unqualified. |
| b10x | 0.13.3 | Advance to0.13.3 with source798325f and strict native prompted-role evidence. |

| Required surface | Codex0.153.4 | Claude2.1.288 | b10x0.13.3 |
|---|---|---|---|
| Success | Native final answer, exit0 and successful closure | Native final answer, exit0 and closure | Native terminal success and closure |
| Terminal failure | Provider refusal, sticky error, native1/CLI3 | is_error true preserved even with subtype success; native1/CLI3 | HTTP400 has no terminal record; unknown terminal, native1/CLI3 |
| Tools and decisions | Actual command exits0/7, patch effect/completion, hook allow/deny; full CLI ask deny prevents marker | Actual Bash allow writes marker; deny prevents it and emits error outcome | Actual file_read bytes and missing-file error; observe-only, no decision seam |
| Cancellation | Halt/interrupt terminate with signal9; command acknowledgement correlated | Halt/interrupt meet two-second command bound | Corrected halt/interrupt both meet two-second bound after no-wire fallback |
| Model and usage | Selected model and synthetic token usage; absent cost unknown | Native model and fixture usage; vendor price is synthetic, not paid spend | Native model and fixture usage; absent monetary cost unknown |
| Declared controls | Unsupported ceilings refused offline; frame/ask policy offline; native ask deny and stop controls | Native one-turn ceiling stops repeated tools; hermetic H1a gap retained | Native one-turn ceiling; strict-version current binary accepted; prompted read-only role and source pair observed |

The reports [Codex](2026-10-03-native-codex-fixture.md),
[Claude](2026-10-03-native-claude-fixture.md) and
[b10x](2026-10-03-native-b10x-fixture.md) record the cases, corrections and limits.
Private native transcripts stay outside Git; only Rust fixtures and sanitized
claims are committed. Reviewer checks were read-only, not independent native runs.

The first b10x cancellation fixture sent invalid command names; its timeouts were
fixture failures and the production-causal attribution was withdrawn. The corrected
fixture exposed a separate acknowledged-but-ineffective interrupt when no native
control wire exists. Both stop commands now pass against the repaired candidate.
Final release evidence must rerun these fixtures against the combined driver.

## Explicit remaining boundaries

Codex registration/turn tiers, replacement-input behavior, plugin/MCP enumeration,
hook matcher/timeout behavior and hosted/subscription semantics remain unverified.
Persistent exec metadata confirms Paginated history; missing or legacy completion
records still produce unknown outcomes. Advancing a pin does not widen these claims.

Claude's three built-in plugins violate the empty-plugin H1a assertion; no floor
verdict is softened. Its current-version observations remain useful without changing
the older pin. Native b10x strict-version launch and prompted read-only scoper are observed against
the selected source/version pair; this is not named-agent loading or a governed AEP
engine run. Native process-write execution is withheld on this host, with no fallback
or effects; successful kernel containment remains unverified. The ESS suite validates bounded production observations and does not
itself qualify vendor versions. No paid request has been made for this work.
