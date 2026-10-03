# Installed adapter compatibility inventory, 2026-10-03

Issue #15 requires observations of each current installed binary before its pin advances.
This inventory records the remaining work, not a compatibility certificate.

Read-only `--version` commands in the integration checkout reported:

| Adapter | Installed banner | Existing pin | Disposition |
|---|---|---|---|
| Claude | 2.1.288 (Claude Code) | 2.1.259 | Pin unchanged; current release unqualified. |
| Codex | codex-cli 0.153.4 | 0.145.0 | Pin unchanged; current release unqualified. |
| b10x | b10x-harness 0.13.3 | 0.12.1 | Pin unchanged; current release unqualified. |

The declarations are `PINNED_VERSIONS` in each adapter's `src/lib.rs`. A banner proves
discovery, not behavior. The existing Codex evidence also distinguishes historical parent
and child PATH versions; every new probe must record the actual child version.

| Required surface | Claude 2.1.288 | Codex 0.153.4 | b10x 0.13.3 |
|---|---|---|---|
| compatibility-success | Unverified | Unverified | Unverified |
| compatibility-terminal-failure | Unverified | Synthetic regression green; native unsupported-model probe pending | Unverified |
| compatibility-tool-decision | Unverified | Production mapping and AEP authorization tested offline; native hook pending | Observe-only contract remains; native current-version observation pending |
| compatibility-cancellation | Unverified | Offline interruption maps to NoVerdict; native cancellation pending | Unverified |
| compatibility-model-usage | Unverified | Unknown cost remains absent in synthetic records; native usage pending | Unverified |
| compatibility-declared-controls | Unverified | Unsupported effectful operations denied offline; native controls pending | No-model embedded tools discovery passed in a managed checkout; wider controls unverified |

The b10x observation was `tools --workspace . --substrate-embedded` in a real managed
checkout: file.write and file.edit were offered, and directory and Git identity remained
unchanged. This does not prove a model run, every filesystem operation, process confinement,
or a governed b10x execution. The pinned owned-tools conformance target separately denies an
outside symlink; it does not qualify the installed binary.

Offline evidence is the integration `task check`: 761 passed, 13 existing live tests ignored;
36 ESS scenarios passed with no skips or refusals. Synthetic tests prove the tested parser,
launch and authorization paths, not how these installed vendor binaries behave. No live
transcript was captured in this continuation. Existing historical evidence is not relabelled
as evidence of the current installed releases.

## Remaining qualification

The operator's live-probe budget choice is pending. No successful-model request is authorized
by this inventory. Once that choice is explicit, run short probes sequentially in private
scratch, with a total spending limit, a per-probe timeout, and a harmless task. Record actual
child version, command, exit, normalized outcome, observed usage/cost (including absence),
the exercised hook/control and denied side-effect evidence. An unexercised surface stays
unverified. Unknown pricing is not a zero-cost observation.

For Codex #11, retain a bounded unsupported-model failure, check normalized failure and
nonzero metaharness exit, and verify preliminary text cannot override it. For #15, include
success, failure, a supported tool decision, cancellation, usage and declared controls for
each release. Add only sanitized synthetic regression vectors to source; private transcripts
stay outside Git. Advance a pin only after the relevant matrix is complete. An empty pin
change does not satisfy the issue.

## Added observation issues

Source-backed Codex #18–20 mappings and their limits are recorded in
[Codex observation provenance](2026-10-03-codex-observations.md). Final-answer,
turn model and tool outcome regressions are green offline. Native acceptance is
still open. Legacy history omits command completion status. Source inspection now establishes
that persistent Codex exec 0.153.4 requests Paginated history, which can retain the
structured completions already supported here. The earlier assumption that the
enum default described exec was incomplete; native probes must establish the actual
path, including any fallback, before an additional transport is justified.
The final 36-scenario suite kills 18 of 18 guard-negation mutants and catches a
planted production final-answer defect. These counts qualify this bounded model,
not installed vendor behavior.

## Credential-free native Codex observations

[The native fixture report](2026-10-03-native-codex-fixture.md) records actual
Codex 0.153.4 success, provider refusal, command exits0/7, hook denial, patch write,
observed model selection and synthetic-provider usage through the production
adapter. Retained native metadata confirms paginated history. A denied command
has no structured completion and stays unknown; its deny response and absence
of the marker effect are separately verified. These observations refine the
earlier unverified Codex rows for the explicit fixture-provider scope. Two further
cases verify success and refusal through the built Metaharness binary, including
its final closure, native process status and CLI verdict. Cancellation,
the AEP path, all declared controls, hosted providers and the other
two current vendor versions remain outside this result. No pin advances yet.
