---
id: control-seam
title: The control seam
sidebar_position: 4
---

# The control seam

**Which tools the harness may call is decided per call, by the embedder, through the protocol —
not once at launch.**

A launch-time allowlist answers the question "may this agent ever run a shell?". The seam answers
"may this agent run *this* shell command, right now, in this step?" — and it answers it while the
harness waits.

## Four tiers of control

Control is not one thing. An adapter declares what it delivers at each of four tiers, and an
embedder that needs a tier the adapter does not have is **refused early** rather than quietly
served something weaker.

| Tier | What it can do | What it costs / cannot do |
|---|---|---|
| `registration` | Decide the set of tools the model is offered, before the session starts. | Cannot see an argument. Cannot change within a session. |
| `call` | Present **every call for a decision before it executes**, and the harness waits. | Costs a round trip. Only as universal as the seam's coverage. |
| `turn` | Add text to the conversation between turns. | Cannot stop a call. Only advises. |
| `kill` | Stop a running turn. | Loses the turn. Cannot be selective. |

### Tier status is a first-class fact

| Status | Meaning |
|---|---|
| `delivered` | Driven, and asserted by a conformance vector. |
| `unverified` | The mechanism is present on the vendor's surface and metaharness **has not driven it**. An embedder that requires it is refused, not quietly served. |
| `absent` | The vendor has no such mechanism at this tier. |

Read them with `metaharness capabilities <kind>`.

## Who decides: `--decisions`

| Mode | Behaviour |
|---|---|
| `frame` *(default)* | The adapter decides from the [frame's](./frames.md) admitted set. No round trip. |
| `ask` | The embedder decides. The run blocks — one round trip per call. |
| `observe` | Nobody decides. Every call is allowed and recorded, through the same seam. |

An embedder that answers "yes" to everything the frame already admits has bought nothing. That is
why `frame` is the default and `ask` is opt-in.

**A run in `frame` mode is still fully audited.** Every mode emits `tool.decided`, and the decision
census counts them all.

### Which adapter delivers which mode

| Mode | `claude` | `codex` | `b10x` |
|---|---|---|---|
| `frame` | `delivered` | `delivered` | `delivered` |
| `ask` | `delivered` | `delivered` | `delivered` |
| `observe` | `delivered` | `delivered` | `delivered` |

Each cell is the adapter's own capability descriptor, `metaharness capabilities <kind>`, and the
same value decides at plan time: a mode declared `unverified` is refused before anything spawns.
`crates/metaharness-cli/tests/website_claims.rs` reads every cell against
`capabilities(kind).decision_mode(mode)`. On b10x, `frame` and `ask` launch the loop with
`--approve caller`: it puts each write and each `run` to metaharness and reads the answer on its
stdin. Its reads run unasked, so a frame that does not admit them is refused at start; see
[the b10x adapter](./harnesses/b10x.md).

### `observe` — measuring a harness nobody is steering

It is **not** "run without a seam". The hook is installed exactly as it always is, every call
arrives at it, and metaharness answers `allow` down the channel a `deny` would take. The point is
comparability: a run that is not being steered produces the same events, in the same order, with
the same correlation keys as one that is, so **one set of expectations can score both**.

`tool.decided` carries `decided_by: "observe"` — its own decider, because an adapter allow is a
judgement about one call and this is a run-wide posture that judged none.

Three things it says out loud rather than leaving you to find out:

- **An `allow` grants.** On Claude Code's hook wire it bypasses the rest of the vendor's permission
  pipeline. An observe run is therefore *more* permissive than a run with no hook at all, and the
  launch attestation says so in its `ambient_inputs`.
- **A `--frame` beside it is refused by name.** A frame whose text reaches the model while nothing
  enforces it tells the model "strictly only these operations" and makes that false.
- **You have to ask for it.** The default is `frame`, and an adapter that has not driven the
  `allow` half of its own decision wire declares the mode `unverified` and refuses it before the
  run. Codex declares it `delivered` on 0.153.4 and 0.160.0: in recorded runs against a fixture
  provider, every launch planned in `observe`, the real hook was answered `allow` and the real
  command wrote its marker (`docs/research/2026-10-03-native-codex-fixture.md`,
  `docs/research/2026-10-23-codex-0.160.0-record-surface.md`; design amendment a49). The probe
  wrote that `allow` through the production renderer; the record does not show metaharness's own
  observe loop answering the hook inside a `metaharness run` process (a49 § 4).

## Whose tools: `--tool-surface`

| Surface | Status |
|---|---|
| `native` *(default)* | The vendor's own tools, narrowed at the decision seam. |
| `owned` | metaharness owns the tool surface and runs the tools itself. **Refused** on `b10x`, and on `codex` without `--catalogue`, in this build — per-step re-listing depends on unverified vendor behaviour. On `claude` and `codex`, `--catalogue` implies it. |

## Three properties that are easy to break by accident

These are stated here because each was a real bug or a near miss, and each is invisible until a
live run.

1. **Every currently-pending `tool.requested` is handed over before an answer to any of them is
   due, and each one's deadline is armed at delivery.** Without both, a single-threaded policy
   deciding call A would burn call B's budget, and metaharness would emit `deadline` denials the
   embedder never chose.

2. **The decision reaches the child before any control does.** Cancelling first clears the active
   call and leaves the child waiting on a correlation that no longer exists.

3. **A record the adapter cannot map becomes `Event::Opaque` and is never dropped.** The failure
   that costs the most is a checker reporting *"the tool was never called"* when what actually
   happened is that it stopped being able to see tool calls.

## The census is metaharness's, not the vendor's

The decision census counts what **metaharness** decided. The vendor's record cannot see it, so the
census is *set* on the terminal record rather than computed by the transcript reader.

This matters for anything downstream that counts denials: read them from `tool.decided` events in
the run's own stream, not from the vendor's transcript.

## Proof, not configuration

The seam has been driven against a paid run on both vendors. On Codex, the evidence chain reads:

| Layer | What it said |
|---|---|
| The hook process | `"tool_name":"Bash"`, `"tool_use_id":"exec-96257928-…"` |
| metaharness | answered `deny`, with a reason |
| The vendor's session record | `Command blocked by PreToolUse hook: this step admits no shell, so the command did not run` — with an **empty** `Output:` |
| The model's closing message | *"The command was blocked and did not run."* |

So `tool.decide` is `Honoured` and the `call` tier is `Delivered`.

The **`allow` half of that wire** is read from a different record: on 0.153.4 and again on
0.160.0, against a fixture provider on loopback and with no model spend, the hook was answered
`allow` and the real command ran and wrote its marker on exit 0 and exit 7, while the otherwise
identical denied command wrote none (`docs/research/2026-10-03-native-codex-fixture.md`,
`docs/research/2026-10-23-codex-0.160.0-record-surface.md`; design amendments a25 and a49). It is
`delivered` on those records. A hosted model and a paid run are not what it was read from, and the
`allow` was written by the probe (`crates/metaharness-codex/tests/native_fixture.rs`,
`answer_hooks`), not by metaharness's observe loop inside a binary run (a49 § 4).
