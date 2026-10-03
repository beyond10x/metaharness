# Codex observation provenance and limits

Issues #18–20 are implemented against official Codex **rust-v0.153.4**, commit
`042fb41b7c813ac7999105e886b2b7aa715b5081`. These are source-backed mappings with
synthetic parser regressions. Subsequent [native fixture observations](2026-10-03-native-codex-fixture.md)
cover the selected mappings on 0.153.4; they do not complete the wider compatibility
matrix or advance the pin.
Protocol amendment a22 is the binding decision.

| Observation | Source and normalized meaning |
|---|---|
| Final answer | `TurnCompleteEvent.last_agent_message`, accepted only with successful terminal evidence. Intermediate assistant text and message phase never choose it. Missing, blank, failed or unfinished terminal evidence supplies no answer. |
| Model | `TurnContextItem.model` is the recorded turn selection. Preserve turn number, optional turn ID and selection changes. It does not prove a routed provider's actual serving model; cumulative usage is not assigned to the last selection. |
| Command outcome | A retained `ItemCompleted` containing `CommandExecution` identifies the call and records status and optional exit code. A completed command requires measured exit0 to claim success. |
| Patch outcome | A retained `FileChange`, or legacy `PatchApplyEnd`, identifies the call and records status. Patch success supplies no numeric exit code. |

The upstream [protocol definitions](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/protocol/src/protocol.rs)
declare terminal, context and legacy patch records. The
[turn implementation](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/core/src/session/turn.rs)
selects the last assistant message when sampling no longer needs follow-up.
The [item definitions](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/protocol/src/items.rs)
define command and patch statuses; [tool event construction](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/core/src/tools/events.rs)
assigns each item ID from its call ID. Correlation requires that identifier,
the observed tool family and compatible turn evidence. Contradictory completions
or reused identifiers invalidate the public outcome and emit a warning.

## Retention is a coverage boundary

The [rollout retention policy](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/rollout/src/policy.rs)
retains command completion items in paginated history, but does not retain
`ExecCommandEnd` in legacy history. Legacy history retains patch completions.
The [function-call output serializer](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/protocol/src/models.rs)
serializes its body without its internal success flag. A string such as
`Process exited with code 0` is therefore content, not normalized exit evidence.

The adapter neither enables another history mode nor parses that string.
Legacy commands without structured completion remain unverified. Native qualification
is still required, but the enum default does not establish the default exec path.
The official [exec entry point](https://github.com/openai/codex/blob/042fb41b7c813ac7999105e886b2b7aa715b5081/codex-rs/exec/src/lib.rs)
requests Paginated history in `thread_start_params_from_config` for persistent
threads. Metaharness refuses --ephemeral. `start_thread` falls back to the default
only for the specific server error about required paginated listing support.
This corrects the earlier source inference that a new exec thread normally uses
legacy history. Existing structured completion support may therefore cover normal
new threads without a new flag or transport; native probes must establish that.
Resumed or fallback legacy threads still require unknown outcomes where metadata
is absent. Another transport needs verified correlation only if actual observations
show it is needed.
Authorization is exclusively `tool.decided`; a tool outcome never authorizes it.

Multiple `tool.result` records can describe one call: later structured evidence
can enrich earlier content, and a contradiction can invalidate earlier success.
Consumers must read complete framing and reconcile the last result and warnings.
Content-only updates preserve already observed evidence. Final answers and model
observations are optional fields on `session.ended`; old serialized records read
them as absent. No frame/1 bytes or event tags change.

## Evidence and outstanding acceptance

Ten offline regression tests exercise terminal authority, missing evidence,
model changes, structured success/failure/denial, patch status, malformed codes,
wrong family/turn and reused identifiers. The initial tests failed before the
implementation. An adversarial test then exposed an internal-only invalidation:
the reader discarded its cached success but left the last public result as
success. It failed before the explicit unknown result was added and passed after.
This was coordinator review; worker quota prevented independent review.

The ESS suite adds nine generated outcome scenarios through the real Codex
reader. Synthetic raw inputs remain inside the adapter. AEP's conformance target
reads only normalized protocol fields. Actual final text, current native model
selection, command/patch outcomes and fixture usage now have bounded native
observations against an owned credential-free provider; see the linked report.
Hook denial preserves an unknown tool outcome because its native record omits a
command completion. The broader qualification and hosted spending decision remain
open. No native transcript is committed and no vendor pin advances yet.
