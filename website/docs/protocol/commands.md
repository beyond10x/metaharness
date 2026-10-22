---
id: commands
title: Commands (in)
sidebar_position: 2
---

# Commands

One JSON object per line on stdin, each tagged `metaharness.command/1`. A decision is an object
tagged by its own `decision` field, so an allow and a deny read:

```json
{"format":"metaharness.command/1","id":"c1","command":"tool.decide","call_id":"…","decision":{"decision":"allow"}}
{"format":"metaharness.command/1","id":"c2","command":"tool.decide","call_id":"…","decision":{"decision":"deny","reason":"this step only reads; propose the change instead"}}
```

## Silence is not a legal outcome

**Every command carries an `id` and produces exactly one `command.result`.** A command that can be
silently ignored is a control surface that cannot be tested.

That includes a line that does not parse. It is refused `MALFORMED`, with the decoder's own words
as the reason, under the line's `id` when the line is a JSON object whose `id` is a string, and
under `-` otherwise, so `-` is not an id to send. A line that is not UTF-8 is answered under `-`,
and its reason gives the byte offset of the first invalid sequence and quotes none of the line.
A blank line is no command and gets no answer. `stream.closed` is the last line of a run, so a line
read after it has gone out gets no answer. The first line above with its decision written flat,
`"decision":"allow"`, is answered:

```json
{"format":"metaharness.event/1","seq":4,"run":"…","event":"command.result","id":"c1","outcome":{"result":"refused","refused":{"code":"MALFORMED","reason":"malformed tool.decide: invalid type: string \"allow\", expected internally tagged enum Decision"}}}
```

A control this adapter cannot honour is refused **by name, at run start** rather than at the call
— so a run that will fail on control fails *before it spends money*.

## The seven commands

| Command | Needs | Notes |
|---|---|---|
| `tool.decide` | `call` tier | Decide one pending tool call. |
| `frame.set` | `call` tier | Put a new [frame](../frames.md) in force at the next boundary. |
| `message.inject` | `turn` tier | Add text to the conversation between turns. |
| `steer` | a mid-turn tier | Steer a running turn. **Claude Code headless has no such tier** — on that adapter this is always refused by name. |
| `permission.set` | `run` tier | Change the permission posture mid-run, in the vendor's own vocabulary. |
| `interrupt` | — | Stop the running turn. **Every adapter must deliver this.** |
| `halt` | — | Stop the run. **Every adapter must deliver this**: a control surface with no way out is not a control surface. |

### `frame.set` is not partially deliverable

An adapter that could inject the frame's *text* but not enforce it would tell the model *"strictly
only these operations"* and make that false.

So a run whose configuration needs `frame.set` without call-level enforcement is **refused at
start**. An embedder that genuinely wants advisory text uses `message.inject`, which claims
nothing.

`frame.set` takes effect at the **next** turn or step boundary, and its result says which — a
frame that took effect mid-turn would mean a call adjudicated against a frame the model was never
shown.

## Four decisions

| Decision | What it does |
|---|---|
| `allow` | Let the call run. |
| `deny` | Refuse the call, **and tell the model why**. |
| `replace` | Run the call with this input instead. |
| `abstain` | Claim nothing; let the vendor's own permission pipeline decide. |

### `allow` *grants* — a departure worth naming

The harness honours a hook `allow` and **bypasses the rest of its permission pipeline**. So an
`allow` from metaharness overrides a stricter rule elsewhere in the vendor's settings.

A run that also relies on such a rule must use **deny-only policy, and say so.**

### Which is why `abstain` exists

Without `abstain`, the only way to let a call through is `allow` — which grants. A run with no
frame, deciding `allow` because it had nothing to narrow with, would silently be **a run with the
vendor's permission system switched off**.

Abstaining says the true thing instead: *metaharness adjudicated nothing here.*

It is the convention proven on Claude Code, where the reference hook passes a call through by
exiting 0 and emitting **no `permissionDecision` at all** — because saying `allow` there would
claim an authority the layer does not have.

`abstain` is the default policy's answer when no frame is in force.

### `deny` requires a non-empty reason

Both vendors' hook wires require it, and the reason is the only part the model can act on: **the
difference between a wall and an instruction.**

### `replace` is never silently downgraded

Both vendors' hook wires carry an updated input, and hiding that would push embedders into
deny-and-re-prompt — a whole turn spent expressing something the wire already supports. A
`replace` an adapter cannot deliver is **refused by name**; it never quietly becomes an `allow`.

## Five refusal codes

| Code | Meaning |
|---|---|
| `UNSUPPORTED_CONTROL` | This adapter cannot honour this command at all. Emitted **at run start** for every command the run's configuration will need. |
| `UNKNOWN_CALL` | The `call_id` does not correlate to an open request. |
| `TOO_LATE` | The window closed — the decision deadline expired, or the turn ended. |
| `MALFORMED` | The command did not parse, or a required field is missing. |
| `SHADOWED` | The vendor would accept this and **another layer would silently override it**. |

`SHADOWED` is refused rather than delivered on a simple principle: **a control that appears to
work and does not is worse than one that is absent.**
