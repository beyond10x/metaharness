# Changelog

What changed. The design document carries *why*; where code and design disagreed, the design
was amended and the amendment is named here.

## [Unreleased]

### Added

- **Four payload fields at the seam, so four expectation kinds stop being undecidable about a
  driven run (design amendment a9).** The motivation is a consumer's, not ours:
  `engineering-protocols` reads `metaharness.event/1` as a transcript and its gap register
  recorded *"Four expectation kinds cannot be decided about a driven run, because the seam's wire
  does not carry what they read … not this repository's to close: it is four fields at the seam."*
  They are now carried. `tool.result` gains **`tool_use_result`** — the vendor's own per-tool
  result record, verbatim, which is where Claude Code's `Skill` writes `commandName` and `success`
  and its `Bash` writes `stdout`, `stderr` and `interrupted`; `usage` gains **`thinking_tokens`**
  (Claude Code's `output_tokens_details.thinking_tokens`, codex's `reasoning_output_tokens` — the
  billed figure, never `thinking.estimate`'s guess), **`iterations`** (the *length* of the vendor's
  own per-iteration list, never a counter of ours), **`speed`** and **`cost_usd`** (Claude Code's
  `modelUsage[…].costUSD`, so a cost scoped to one model is answerable; the aggregate carries none
  because the vendor prices no aggregate and multiplying tokens out would be a number nobody
  billed). All additive and all optional: an absent field is an explicit `null` as every other
  payload field is, so a stream from a build that predates the amendment parses identically.
  What codex honestly has is one of the four, and its reader's own documentation carries the table
  of what it does not: no per-iteration list, no speed tier, no cost anywhere, and no per-tool
  result record beside a tool's output — each an `unk` in a verdict and never filled from a
  neighbouring field. The golden expected streams are regenerated from the **committed** recorded
  wire, which is where the new values come from: the recorded Claude run really did carry a
  `tool_use_result`, `iterations: 1`, `speed: "standard"` and two priced models. Vector counts are
  unchanged (20 claude / 10 codex); `cargo test -p metaharness-claude --lib regenerate --
  --ignored` now also regenerates the three synthesised C2 expectations, because a protocol
  amendment moves every expectation at once.

- **A run can be pointed at a model gateway: `--model-endpoint <root>` and `--effort <level>`
  (the model-adapter design's endpoint slice; MA-V1–V4 verified).** Each harness reaches its own
  dialect under the declared root — Claude Code speaks Anthropic messages at `{root}/v1/messages`
  (`ANTHROPIC_BASE_URL` plus a placeholder `x-api-key`, never a credential), codex the Responses
  wire at `{root}/v1/responses` (a `model_providers.metaharness_endpoint` entry with no
  `env_key`, and therefore no auth header at all). The composition with a real credential source
  is refused by name on both adapters: a child pointed at a foreign endpoint holds no operator
  credential, so `--credentials none` is required — H4's attestation row says the placeholder is
  what the child carries. A **declared** endpoint is the difference from the ambient
  `ANTHROPIC_BASE_URL`/`OPENAI_BASE_URL` H3 scrubs, which stay refused. `--effort` exists because
  an endpoint may hold a different vocabulary than the vendor's service: the gateway this was
  verified against accepted `xhigh|medium|low` and refused Claude Code's default `high`. Proven
  live end to end against a vLLM-class gateway serving `qwen3.8-27b`: both
  `metaharness run <kind> --model-endpoint …` chains exit 0 with the model's answer in the event
  stream (claude 25,546 in / 89 out; codex 10,508 in / 2 out).

- **`doctor` now answers about the binary the run will execute, and the contract names a version
  pair that disagrees (CT-3; Q18 closed as amendment a8).** Q18's cause was two binaries, not one
  binary lying: the operator's shell resolves pacman codex 0.145.0 at `/usr/bin` while the launch
  plan's constructed child `PATH` resolves npm codex 0.144.0 at `~/.local/bin` first — so the
  pre-flight blessed a binary the spawn never executed, and every driven a7 claim was in fact
  driven through 0.144.0. `doctor <kind>` now resolves the vendor binary on **the child's
  `PATH`** (`child_path()`, exported by both adapters) and prints the resolved absolute path, so
  on a machine with two installs it reports the one that will spend the money — here,
  `/home/timo/.local/bin/codex 0.144.0, OFF the pin`, exit `1`, where it previously said on-pin.
  A `golden-version-pair` vector per adapter reads the recorded golden sample's own version claim
  against the pin: agreement passes silently, disagreement is a **named warning** — rendered as
  `warn` in the vector listing and on stderr beside the `--contract` record, never a silent pass
  and never a failure, because the recorded fact is known and reddening the contract over it
  teaches operators to ignore red. Both adapters warn today (codex 0.144.0 vs 0.145.0, claude
  2.1.240 vs 2.1.239). `conformance` now runs 20 claude / 10 codex vectors. What remains is the
  machine's: two codex installs, one to be removed or the pin re-verified against 0.144.0 — the
  operator's call.

- **An adapter's conformance run is a `contract_result` (CT-1, design
  `adapter-contract-v0.1.md`).** `metaharness conformance <kind> --contract` emits the record
  `engineering-protocols`' `contract-testing` principle reads — `{checked, failed,
  breaking_changes, provider, consumer}` — so a consumer reads the `metaharness ⇄ vendor` mapping
  as a contract without a crate dependency crossing the boundary: the vocabulary is shared, the
  code is not. `provider` carries the pin (`codex 0.145.0`), `consumer` is `metaharness.event/1`,
  and `breaking_changes` counts only the vendor-facing tiers (C1/C2) — a C3 failure is
  metaharness's own control machinery regressing, red in `failed` but not the vendor's fault.
  The design is written against the three drifts CX-M2's live run surfaced (the `Bash`/`exec`
  vocabulary split, the 0.144/0.145 version mismatch, the un-joinable ids); CT-3–4 (the version
  reconciliation, cross-adapter symmetry) stay proposed.

- **Each adapter's contract now holds recorded real wire, not only synthesized shapes (CT-2).**
  `metaharness run <kind> --retain-dir <dir>` is the capture surface: when the run ends, its raw
  vendor wire — the retained transcript or rollout, the thin codex `--json` stdout, and every raw
  `PreToolUse` stdin — is copied out of the scratch root before the scratch is deleted, named
  file by file and never the scratch home, so a copied credential cannot travel; wire the
  operator asked for that is not there is a `RETAIN_FAILED` warning, never silence. One hermetic
  capture run per adapter promoted both faces to `fixtures/golden/` in each adapter crate:
  `golden-transcript`/`golden-rollout` replay the recorded record byte-exact against a committed
  expected stream, and `golden-hook-input` pins every field the seam reads off the recorded hook
  stdin **and** that the rendering table agrees with the wire (`operation.shell` → the recorded
  `tool_name`). A mutation test per sample proves a flipped byte fails its vector, and a
  `#[ignore]`d `regenerate` test per crate makes re-capture at a new pin a reviewed diff rather
  than a rewrite. The recorded bytes earned their keep on arrival: codex's real call is a
  `custom_tool_call` where every synthesized vector used `function_call`, and its `session_meta`
  claims `cli_version` 0.144.0 out of the 0.145.0 binary — Q18 as a committed byte, warned as
  `version_outside_pin` in the golden stream. `conformance` now runs 19 claude / 9 codex vectors;
  the contract records read `checked: 19` / `checked: 9`, 0 failed, 0 breaking.

- **`metaharness run codex` drives a real Codex session (CX-M2).** A scratch `CODEX_HOME`, a
  constructed child environment, the operator's `~/.codex/auth.json` copied in immediately before
  every spawn, and a blocking `PreToolUse` hook metaharness answers per call. Events come from the
  **session rollout**, discovered under the scratch home and tailed as it is written — the record
  that carries timestamps, durations and per-turn usage where `codex exec --json` stdout carries
  none — and every line is retained as the transcript for the auditor (O8), with the thin `--json`
  stream retained *beside* it rather than as it. `--tool-surface owned`, `--max-turns` and
  `--plugin-dir` are refused **by name** on this adapter rather than silently dropped: an option
  that was set and ignored is a run that is not the one that was asked for.
- **The seam holds on a second vendor, and it was proven with a paid run** (design amendment a7).
  A policy admitting no shell met a prompt asking for one. The hook process received the call, the
  embedder answered `deny` with a reason, and **the vendor's own session record** reads
  `Command blocked by PreToolUse hook: this step admits no shell, so the command did not run` with
  an **empty** `Output:` — the deny reached the child before the effect. `tool.decide` is now
  `Honoured` and the **call tier is `Delivered`**; the `allow` half of that wire is deliberately
  **not** claimed, because only the deny path has been driven.
- **Three Codex facts that are each a silent failure, found the expensive way.** (1) A user hook is
  declared in **`config.toml`** under `[hooks]`, not in a `hooks.json` — that is a plugin manifest's
  file — and an unrecognised key there is dropped *without failing the config load*. (2) A hook in a
  fresh `CODEX_HOME` **never fires** without `--dangerously-bypass-hook-trust`, because a scratch
  home cannot hold persisted trust; the flag's warning is about running somebody else's hook
  unvetted, not the one metaharness just wrote. (3) The hook speaks **Claude Code's** tool
  vocabulary — `tool_name` is `Bash`, where the rollout calls the same call `exec` and the binary's
  own tool list calls it `shell`, so the operation rendering targets the hook's word and a table
  built from the record would have denied every shell call as a frame decision.
- **`approval_policy = "never"` and `sandbox_mode = "read-only"` in the scratch config.** `codex
  exec` on 0.145.0 has no `--ask-for-approval` flag, and the operator's own default (`on-request`)
  would let a prompt nobody is there to answer turn a call away before the seam saw it. `never`
  makes metaharness's hook the one thing that can refuse a call, so a denial is attributable.
  `read-only` is this vendor's process-level floor, which Claude Code's CLI has no counterpart for
  and which the attestation therefore gets to claim. Both read back from `codex doctor` against the
  scratch home — `restricted fs + restricted network · approval Never` — for free.
- **The builder dispatches by kind.** `Metaharness::start` and the start path now `match spec.kind`
  into `start_claude` / `start_codex`, each with its own launch plan, runner and seam factory. A
  `match` rather than a trait, deliberately: the two plans are different types with different
  fields, and a third adapter is when the abstraction earns its keep. The Claude path is unchanged.
- **Three C3 spawn vectors for the codex path**, mirroring the Claude ones: a fake vendor that
  writes a real session file under a scratch `CODEX_HOME` and blocks on the real hook program, so
  the seam round trip, the rollout tail-and-retain and the per-spawn credential copy are all checked
  with no model, no network and no credential. `conformance codex` runs **7** vectors.
- `metaharness/tests/live_codex.rs` — the C4 tier for this adapter: one live run behind `#[ignore]`
  and `METAHARNESS_LIVE=1`, asserting the three facts nothing cheaper can reach, each from the run's
  own record.
- **Correction: the codex plugin went back to engineering-protocols.** The evals migration
  briefly carried `integrations/codex/` here as `evals/codex/`; the operator's call is the right
  boundary — a plugin (instruction surface, skill) is the subject repository's product, like the
  claude plugin's skills and agents that never left. This repository keeps the harness machinery:
  the `metaharness-codex` adapter and its research record.
- **`metaharness-codex`, CX-M1: the adapter's input is built and its claims are labelled.** The
  rollout reader maps `$CODEX_HOME/sessions/…/rollout-*.jsonl` — session_meta, paired
  function/custom tool calls, token_count (usage and rate limits), task_started/complete — onto
  the protocol's events, with a terminal `session.ended` built at finish from the vendor's own
  duration and usage and **no invented cost** (the vendor never emits one). The format has no
  documented stability guarantee, so the reader version-gates on `cli_version` (a warning, never
  a mid-read refusal) and preserves every unmapped shape as `opaque` — the April-era
  `exec_command_begin` drift is a conformance vector, not a failure. `capabilities codex`
  declares every tier `Unverified` and keeps `tool.decide` refused until a driven run proves the
  vendor's documented hook contract; `doctor codex` checks the installed binary against the
  0.145.0 pin (and the version-token picker learned that `codex-cli 0.145.0` leads with a name,
  not a number); `conformance codex` runs 4 replay vectors. At CX-M1 `run codex` was refused by
  name and every tier was `Unverified`; CX-M2 above is the driven spawn that changed both, and it
  changed them only as far as one live run reached. Evidence base:
  `docs/research/2026-08-21-codex-harness-research.md`, migrated here with the adapter.
- **The operator-named working directory — `--cwd <dir>` (amendment a6).** The driven case's
  declaration: the child runs in a real tree instead of a scratch one. H7 and H11 move from
  imposed to attested-unavailable with the trade named — `--hermetic strict` refuses such a run,
  `--hermetic` reports it — and the outside-scratch and memory-ancestor refusals apply only to
  the scratch case they were written for. The directory is used, never created: a typo is a
  refusal, not an empty run reporting success. `--add-dir` stays denied.
- **The on-disk frame document — `metaharness.frame/1` (amendment a5).** The format § 9.3
  correction 3 left owed now exists: one JSON object, a `format` tag on the D2 rule, every § 5.1
  field, and a digest that is **required to describe the contents** — SHA-256 over the compact,
  key-sorted serialization without `digest`/`format`, reproducible without linking this
  workspace. `--frame <file>` and `.with_frame_file(path)` resolve it in the library at start
  (D11 intact: the binary carries only a path), and every failure is a free pre-spawn refusal by
  name: `FrameUnreadable`, `FrameInvalid` (untagged, misshapen or digest-broken, parser text
  verbatim), and `FrameConflict` when an in-memory frame and a document compete. A launch-time
  frame now requires `tool.decide` rather than the undriven mid-session `frame.set`, and the
  Claude adapter's `FrameFormatUnowned` refusal is gone rather than left as a variant nothing
  produces.

- `metaharness-protocol`: the wire — 19 events, 7 commands, versioned JSONL framing with the tag
  on every line, sequence numbers assigned in one place, the workflow `Frame` and the one
  function that renders it for the model, the 12 hermetic rows, adapter capabilities, and the
  structural projection into `trace-ir/1`.
- `metaharness-claude`: hermetic launch construction against Claude Code 2.1.239 — scratch config
  home, environment scrub, `--strict-mcp-config`, the `--bare`/`--safe-mode` denylist, the memory
  ancestor walk, the non-`async` `PreToolUse` hook definition, and a `SHADOWED` refusal for a seam
  another layer would override — plus stream-json transcript reading in which nothing is dropped.
- `metaharness`: the run loop with per-call decisions, several pending at once and answerable out
  of order, deadlines armed at delivery, and `--audit`'s built-in hermetic floor with exit codes
  0/1/2/3.
- `metaharness-cli`: `run`, `capabilities [--render]`, `conformance`, and honest refusals for
  `project`, `audit` and `doctor`. 14 conformance vectors run with no model and no credential.
- Design amendments a1–a3 and questions Q13–Q16.

- **The real spawn (M2).** `metaharness run claude --hermetic -p "…"` starts Claude Code 2.1.239
  for real: a constructed environment, a scratch config home, the credential re-copied
  immediately before every spawn, stdout streamed through the transcript reader into protocol
  events, stderr retained whole, and the raw bytes kept on disk for the auditor (O8).
- **A control seam a separate process can answer over.** The adapter now renders the
  `PreToolUse` program its hook definition always named, and metaharness answers it over a
  request/response channel. The program parses no JSON and needs no interpreter — it publishes
  the vendor's input under a name only it holds and waits for a file — and it fails closed with a
  reason on a missing channel, an unwritable channel or an unanswered call.
- **The correlation key, read off the vendor rather than guessed (V22).** The hook input carries
  `tool_use_id`, which is the same string the transcript's `tool_use` block calls `id`. This
  **closes Q16**, and M1's provisional envelope turns out to have been right, so it did not move.
- Two further driven rows: the `tool_use` record reaches stdout **before** the hook runs (V23),
  and a `--settings` file outside the config home still loads its hooks under
  `--setting-sources ""` (V24, which **answers Q14**).
- `metaharness doctor <kind>` — the installed vendor version against the adapter's pin, for free.
- Three C3 **spawn vectors**: a real process and the real hook program against a fake vendor, so
  the seam's round trip, the per-spawn credential copy and the retained transcript are all
  checked with no model, no network and no credential. `conformance` now runs 17 vectors.
- A C4 tier that exists: two live runs in `tests/live.rs`, `#[ignore]`d and behind
  `METAHARNESS_LIVE=1`.

### Fixed
- **The loopback wind-up vector stopped failing on somebody else's socket.**
  `a_loopback_run_proxies_the_childs_request_with_custody_and_closes_the_port_after` asked
  `!port_accepts(port)` once, immediately after `drain()`, and failed 2 of 5 full-gate runs on it.
  The shutdown was never the problem and is not what changed: `LoopbackHandle::shutdown` joins the
  accept thread, that thread *owns* the `TcpListener`, so the listening socket is closed before
  `drain` returns — 27,000 shutdowns, in isolation and under load, never once left this proxy's own
  listener up. What the assertion actually asked about was a **port number**, which is machine-wide:
  the ephemeral number a run has just released is immediately bindable by any process on the box.
  Under a synthetic bind/close load the vector failed 3 of 25 runs, and every failing probe was
  answered by a socket that closed the connection at once (42µs–872µs, never this proxy's own 401)
  and had left `ss -ltn` by the next millisecond — a stranger holding the number, not a proxy
  outliving its run. The check now polls for up to 2s, which distinguishes the two: a stranger is
  transient, a proxy that really outlived its run accepts for the whole bound. 30/30 and 25/25 green
  under the load that produced the failures, and both `shutdown` and the poll now carry the
  measurement in their doc comments so nobody re-tightens it back into a flake.
- **The hermetic floor failed its own first live run, twice, and both were its fault**
  (design amendment a4). `H4` looked for the word the spec used in `apiKeySource`, a field that
  says `"none"` under an operator login — so every hermetic operator-login run reported a gap on
  the row it most clearly satisfied. `H10` read *"this run pinned no input tree"* as `unk`, which
  made `--hermetic strict` unpassable for every run that pins nothing, including the design's own
  example. Neither was reachable below C4: there is no real opening record before it.
- `metaharness run` no longer exits 2 on a well-formed spec, and `Refusal::NoSpawner` is gone
  rather than left as a variant nothing produces.

### Guarded
- **A paid run can no longer be reached from `task check`.** When `run` learned to spawn, two CLI
  tests that asserted it exited `2` kept passing their argv through and billed two real sessions.
  Those tests now use only pre-spawn refusals, and an interlock over the test file's own source
  refuses to let a prompt-carrying `run` argv back in.
- **The interlock's codex escape hatch is gone, and a second interlock covers the library tests.**
  A `codex` argv used to be free because `run codex` was refused by name; CX-M2 made it a paid
  session, which is the same shape of defect the interlock was written after — an argv that was
  free when it was written and stopped being free when the milestone under it landed. `run codex`
  argvs now have to earn their place the same way, and `metaharness/tests/run_loop.rs` gained an
  interlock of its own: a `Metaharness::start` whose result is not an expected refusal is a call
  that spawned, and the test file refuses to contain one.

### Not yet
- `metaharness project` is gated on Q9 (`trace-ir/1` is `Serialize`-only, so a document written
  there has no reader) and `metaharness audit` on the launch facts a foreign transcript cannot
  carry. Both refuse with exit 2, each naming what it waits for.
- `session.started` carries the transcript's path and not its digest: the opening record is line
  one of a file whose last line does not exist yet (**Q17**).
- On Codex: the `allow` half of the decision wire (only `deny` is driven), turn injection,
  registration-level narrowing, and the `apply_patch` operation rendering — the hook's word for a
  patch call is the vendor's documentation and not a driven observation (**Q5**).
- **Q18:** `codex --version` reports `0.145.0` and the `session_meta.cli_version` written by the
  run that binary starts reports `0.144.0`, on the same machine. `doctor codex` reads the first and
  the hermetic floor reads the second, so a run can pass the pre-flight and report off-pin from its
  own record — which is what the CX-M2 live run did. The pin is not widened to paper over it; the
  reader warns.
