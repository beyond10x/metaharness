# The loopback provider — metaharness as the inner harness's API endpoint — v0.1 (proposed)

Status: **proposed** (operator, 2026-08-22), reviewed the same day; the review's corrections are
folded in below. Companion to `model-adapter-v0.1.md` (this is a third adapter kind beside
*default* and *generic*) and the answer this design owes to **Q13** in
`metaharness-protocol-v0.1.md`. Nothing here is built.

## The idea, in the operator's words

metaharness sets itself as the LLM/API provider for the inner harness (via environment
variables), runs plain HTTP between itself and the inner harness so requests are inspectable,
injects the subscription's OAuth2 token, and forwards upstream. Optional. Motivation: copying
credentials from the host into isolation directories — what the adapters do today, per spawn —
is dangerous: the copy can be expired, refreshing from inside the scratch home is dangerous, and
many simultaneous runs could invalidate each other.

## Review verdict: doable, and it is the right fix — with one correction

**The child should never hold the real token at all.** Injecting the OAuth2 token into the
child's environment would still put a live credential inside the isolation directory's process,
and it would still age there. The stronger shape, same mechanism:

1. The child is launched with `ANTHROPIC_BASE_URL=http://127.0.0.1:<per-run port>` and a
   **placeholder** bearer (`ANTHROPIC_AUTH_TOKEN=mh-run-<id>-<nonce>`). The scratch home contains
   **no credential file**. Loopback only, one port per run.
2. metaharness's proxy validates the placeholder (it names the run), strips it, attaches the real
   OAuth bearer from **one custody**, and forwards upstream over TLS. Streaming (SSE) is piped
   through verbatim; unknown paths under the base are reverse-proxied generically so a vendor
   endpoint nobody catalogued does not break the run.
3. **Refresh happens in exactly one place**, serialized (file lock around the credential store).
   A 401 upstream triggers refresh-and-retry inside the proxy; the child never sees it, so the
   child never attempts its own refresh.

### The danger claim, labelled

| claim | status |
|---|---|
| a copied credential can expire mid-run | **verified** — Q13 is a recorded incident: a governed run died an hour in on an OAuth session that could not be refreshed (protocol design, amendment a1) |
| refreshing from the scratch copy is dangerous | **verified by design** — § 8.4 considered sharing the live file (option b) and left it open as Q13 precisely because a child writing the operator's credential is the custody § 1.2 forbids |
| N simultaneous copies invalidate each other | **hypothesis, unverified** — true iff the vendor rotates refresh tokens on use. Consistent with common OAuth practice; nobody here has observed the race. V-LP5 below is the observation that would settle it. The loopback design removes the race whether or not it exists, which is the right order |

## What this buys beyond custody

- **Inspection**: the proxy sees every request and response — exact prompts on the wire, token
  counts and cost independent of vendor self-report, per-run request log (opt-in, stored with the
  run's transcript under the same custody; bodies carry the run's content and are never logged by
  default).
- **A new enforcement point**: a request naming a model outside the run's spec, or exceeding a
  size/count budget, can be refused at the wire — a control tier below the tool seam. Out of
  scope for v0.1; named so it is not invented ad hoc later.
- **Hermetic honesty improves**: H6 ("credentials are one file, copied") becomes "no credential
  in the child at all" — an imposition strictly stronger than today's advisory row, and
  attestable from the launch values (no credential copy in the plan, placeholder in the env).
- `model_endpoint` (model-adapter design § protocol additions) reports `adapter: loopback` and
  the upstream host, so which brain answered stays a checkable fact.

## What must be verified before building (the register)

| id | question | method | cost |
|---|---|---|---|
| V-LP1 | Claude Code 2.1.239 sends **all** API traffic to `ANTHROPIC_BASE_URL` and authenticates with `ANTHROPIC_AUTH_TOKEN` as a bearer — including under a subscription login absent from the config home. Capture the exact paths and headers | point the child at a recording stub; no upstream, no spend | free |
| V-LP2 | the upstream accepts the subscription OAuth bearer when replayed by the proxy with the captured headers | one minimal live session through the proxy | ~$0.05 |
| V-LP3 | SSE streaming survives the pass-through byte-for-byte (ttft within noise) | same run as V-LP2 | — |
| V-LP4 | the child never attempts its own OAuth refresh when it holds only a placeholder; proxy-side refresh-and-retry is invisible to it | forced 401 from the stub | free |
| V-LP5 | does the vendor rotate refresh tokens on use? (the mutual-invalidation hypothesis) | two custodies refreshing the same stored token, watch the second | risks one login's session; do this **last**, deliberately |
| V-LP6 | Codex 0.145.0: can subscription (ChatGPT-plan) traffic be routed through a `model_providers` entry at all, or only API-key providers? If not: which endpoint does subscription traffic pin to, and does `CODEX_HOME` isolation still allow a loopback base? | config injection against a recording stub, then one live turn | free + cents |

V-LP6 is the known hard one: the research record verified `model_providers` for API-key
providers; subscription-over-custom-provider is a **?**. If it refuses, the Codex door ships
API-key-only through the proxy and the subscription path keeps today's copy — stated, not
silently degraded (the model-adapter design's rule: no silent fallback between adapter classes).

## Build plan, if accepted

| milestone | content | acceptance |
|---|---|---|
| LP-0 | V-LP1–V-LP4 against the pins; this document's rows flip to verified or the affected milestone is narrowed | each row labelled, spend ≤ $0.10 |
| LP-1 | the proxy: std `TcpListener` + rustls-backed upstream client (one new dependency, chosen deliberately — the workspace is dependency-light), per-run port, placeholder auth, generic reverse-proxy, SSE pipe | C3-style vectors against a fake upstream: auth swap, stream pipe, 401-refresh-retry, unknown-path pass-through — free |
| LP-2 | custody: one credential store, file-locked serialized refresh, `auth.expired` emitted when refresh itself fails; the credential-copy path stays and remains the default | two concurrent scripted runs share one refresh |
| LP-3 | the switch: `--credentials loopback` (a fourth `CredentialSource`), H6 attestation row upgraded under it, `model_endpoint` emitted; flip the default only after a governed run has used it in anger | live driven run, then the default question goes to the operator |
| LP-4 | the Codex door, shaped by V-LP6 | per V-LP6's answer |

## Decisions taken in this review (defaults if nobody objects)

1. **Placeholder in the child, real token only proxy-side** — not "inject the token into the
   inner harness". This is the correction to the proposal as spoken, and it is what makes the
   custody claim attestable.
2. **Per-run proxy, shared custody** — one port and one placeholder per run (isolation, trivial
   attribution), one lock-serialized credential store per operator (kills the refresh race).
3. **Optional, default off** in v0.1: `--credentials loopback`. The copy path stays until LP-3's
   live proof, then the default flips by an operator decision, not silently.
4. Request-body logging is **opt-in** and lives with the run's transcript; the proxy log without
   bodies (method, path, model, tokens, timing, status) is always on — it is the inspection the
   proposal asked for and carries no content.
