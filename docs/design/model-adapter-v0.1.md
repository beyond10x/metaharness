# Model adapters — v0.1 (proposed; MA-0 verified, the endpoint slice built)

Status: **proposed**; **MA-0 is done and the endpoint slice of MA-1 is built** (2026-08-23,
operator-directed: "how can we configure so metaharness → codex|claude → the gateway").
MA-V1–V4 all verified — first against a local recording stub, then live against a real
vLLM-class gateway (`llm.dev.former organization.com`, one model `qwen3.8-27b`, both dialect doors, no
auth) — and `--model-endpoint <root>` + `--effort <level>` are `RunSpec` options: claude gets
`ANTHROPIC_BASE_URL` plus a **placeholder** key (never a credential), codex a
`model_providers.metaharness_endpoint` entry (`{root}/v1`, `wire_api = "responses"`, no
`env_key`), both **requiring `credentials: none`** and refusing an operator credential by name.
Proven end to end: `metaharness run <kind> --model-endpoint … --model qwen3.8-27b` exits 0 with
the model's answer in the event stream, both vendors. What stays proposed of MA-1/MA-2: the
`ModelAdapter` type with alias tables, `models()`, and the `model_endpoint`/`model_list` events
of § 6. Verified facts worth keeping: Claude Code authenticates to a base URL with **`x-api-key`**
(not a bearer); codex sends **no auth header** for a provider without `env_key`; the gateway's
effort vocabulary was `xhigh|medium|low` and refused Claude Code's default `high` (HTTP 500) —
which is why `--effort` is a run option; codex's streaming SSE off `{root}/v1/responses` works.

Companion to `metaharness-protocol-v0.1.md`; nothing here changes
the event/command wire except the two events named in § 6.

## The requirement, in the owner's words

A metaharness instance can say `with_model("opus")` and get the **default adapter**: an alias map
per harness kind (`opus` → `claude-opus-5`, and the vendor's own routing and billing do the rest).
Or it can inject another one — `with_model_adapter(ModelAdapter::generic("https://llmgw.example"))`
— and then **Claude picks the `v1/messages` endpoint and Codex the `responses` endpoint** of that
gateway (a vLLM-class server exposing both dialects), with `mh.models()` fetching and showing the
adapter's model ids. former organization has already attempted this adaptation and **will be a consumer of
metaharness later**, which makes its lessons requirements rather than trivia.

## What an adapter is

One value answering three questions the harness adapter otherwise answers from vendor defaults:

| question | default adapter | generic adapter |
|---|---|---|
| where is the model API | the vendor's own service | `base_url` given at construction |
| what does a model name mean | per-kind alias table (`opus` → `claude-opus-5`), pass-through otherwise | the gateway's ids, verbatim; `models()` lists them |
| how is it paid and authed | the harness's own credential custody | a named credential reference (env var or file), never an inline secret |

The adapter decides none of the dialects. **The harness picks its native dialect from the
adapter's base**: Claude Code speaks Anthropic messages (`{base}/v1/messages`), Codex speaks the
OpenAI family (`{base}/responses`, or chat where responses is absent). One gateway, two doors,
each harness through its own.

## Realization per harness — and what must be verified first

| harness | mechanism | status |
|---|---|---|
| Claude Code | `ANTHROPIC_BASE_URL` (+ auth env) into the launch environment; `--model <id>` verbatim. The hermetic env scrub must **whitelist** the adapter's auth variable — hermeticity currently deletes exactly the class of variable this feature requires | documented; **to verify against pinned 2.1.239** (MA-V1) |
| Codex | a `model_providers` entry injected into the run's config (`base_url`, `wire_api`, `env_key`), profile-selected | documented; **to verify against pinned 0.145.0** (MA-V2), incl. whether `wire_api = "responses"` works against a vLLM-class gateway or only `"chat"` (MA-V3) |
| `models()` | `GET {base}/v1/models`, the one endpoint both dialect families share | to verify per gateway (MA-V4) |

## former organization's lessons, adopted as constraints

1. **No silent fallback between adapter classes.** A harness that cannot honour the injected
   adapter (flag combination, wire dialect, version) refuses by name — the same posture as the
   control tiers in the protocol design.
2. **A budget the adapter cannot observe is refused, not ignored.** A gateway relays bytes and
   reports no price; `with_max_spend(...)` under a generic adapter is an error naming the reason,
   not a ceiling that silently never fires.
3. **Reasoning/opaque items are wire-tagged.** former organization carries provider blobs verbatim, tagged
   by the wire that produced them, so one provider's chain-of-thought is never replayed into
   another's. The same rule binds if metaharness ever replays context across adapters.

## Protocol additions (two events, both audit-bearing)

- `model_endpoint`: emitted at session start — adapter kind (default | generic), base host (never
  credentials), dialect chosen, model id as resolved. Makes "which brain answered" a checkable
  fact; the hermetic contract gains a row asserting the endpoint the run *used* is the one the
  embedder *named*.
- `model_list`: the result of `models()` when the embedder asked — ids as the gateway stated them.

## Milestones

| # | content | acceptance |
|---|---|---|
| MA-0 | study former organization's attempt (their responses wire, their provider routing) + verify MA-V1..V4 against pinned versions | each V-row flips to verified or the feature row that needs it is marked refused |
| MA-1 | `ModelAdapter` in the `metaharness` crate: default (alias tables) + `generic(base_url)`, builder verbs `with_model` / `with_model_adapter`, `models()` | model-free tests over command/env/config construction for both harnesses |
| MA-2 | live smoke against a real gateway — operator-run, network-dependent | one turn through each door of one gateway; `model_endpoint` event matches the injection |

## Open questions

- **Codex dialect default against vLLM-class gateways** (responses vs chat). Decides: owner.
  Default if nobody answers: **configurable on the adapter with `responses` preferred and `chat`
  the named fallback** — former organization implemented the responses wire against its gateway, so
  preferring it follows the working precedent; the fallback is named because MA-V3 may fail.
- **Alias table ownership.** Default: built into each harness adapter, overridable per
  `ModelAdapter` — an alias is a harness fact (`opus` means something to Claude, nothing to a
  gateway), so it lives with the harness and yields to the adapter.
