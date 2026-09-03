# metaharness — a unified interface to agent harnesses — Design v0.1

> **Repository:** `metaharness/metaharness`
> **Status:** **proposed.** Nothing here is built. The placeholder types in
> `crates/metaharness-protocol` are aligned to the names this document decides and carry no
> behaviour.
> **Review:** one adversarial review, 2026-08-22 — 4 blocker, 12 major, 2 minor, **all folded in**.
> The verdicts and what each one changed are § 13. Corrections are marked at the point of change,
> so the first draft's claims stay visible.
> **Amendment a1, 2026-08-22**, during the M1 build: the credential row H6 gained a token
> lifetime, a nineteenth event (`auth.expired`) exists, and **Q13** carries what is still unknown.
> **Amendment a2, 2026-08-22**, from building the Claude adapter: three verified rows the design
> needed and did not have (V19–V21) and one new question (**Q14**).
> **Amendment a3, 2026-08-22**, from building the run loop: a **fourth decision value**,
> `abstain`, and four choices the document did not decide (§ 6, § 8.5, § 9.4).
> **Amendment a4, 2026-08-22**, from the real spawner and the **first live runs**: three driven
> rows (**V22–V24**) that close **Q16** and answer **Q14**; two corrections to § 8.1's floor
> (**H4** and **H10**) that a live opening record found and no free tier could have; and one new
> question (**Q17**). Marked at each point of change, on the same rule as the review's
> corrections.
> **Amendment a5, 2026-08-22**, for the embedder integration: **the on-disk frame format the
> review left owed now exists** — `metaharness.frame/1`, § 5.5 — resolved by the library so D11
> holds, sealed-digest-required so an edited document is refused, and a launch-time frame now
> requires the decision channel (`tool.decide`) rather than the still-undriven mid-session
> `frame.set` (§ 6, F9's "both halves or neither" met by per-call enforcement).
> **Amendment a6, 2026-08-22**, same integration: **`--cwd <dir>`, the operator-named working
> directory** — the declaration that trades H7 and H11 for real work in a real tree. Both rows
> move from imposed to attested-unavailable with the trade named, so `--hermetic strict` refuses
> such a run and `--hermetic` reports it; the two cwd refusals (outside-scratch, memory
> ancestors) apply only to the scratch case they were written for. `--add-dir` stays denied.
> All marked at each point of change, on the same rule as the review's corrections.
> **Amendment a6.1, 2026-08-23**, from a paid run (codex-1982431) that the amendment as written
> could not deliver: **giving up H7 and H11 buys nothing unless the child can write to the named
> tree**, and on Codex it could not. The scratch `config.toml` set `sandbox_mode = "read-only"`
> for every run, so a `--cwd` child got a real repository it could only read — the vendor's own
> stream: *"the workspace is read-only, so the planning-store patch was rejected"*. So a6 is
> corrected at its point of definition: **an operator-named `--cwd` grants the vendor sandbox over
> that tree** (`sandbox_mode = "workspace-write"` on codex — the vendor's own value, *"permits
> reading files, and editing files in `cwd` and `writable_roots`"*), and the H7 attestation row
> **states the grant in words**, so a reader sees "this run could write to the operator's tree"
> without diffing a scratch config that no longer exists. Scratch-cwd runs are unchanged and still
> write nothing. Nothing else moves: `--add-dir` stays denied, no `writable_roots` widens the grant
> past the declared tree, no network claim is made or changed, and `--hermetic strict` still
> refuses — the grant changes what the child may **do**, never what the attestation **claims**.
> A trade whose consideration is undeliverable is not a trade, which is why this is an amendment
> and not a bug fix.
> **Amendment a7, 2026-08-22**, from the **first driven Codex run** (CX-M2): the § 7.4 matrix's
> `PreToolUse` row moves from documented to **driven** — a real `codex exec` was refused one shell
> call at metaharness's own hook and the vendor's record shows the command blocked with empty
> output — and four things the matrix had wrong or did not have are corrected at the point of
> change: the hook is declared in **`config.toml`**, not in a `hooks.json` (which is a plugin
> manifest's file this binary never reads for a user hook); the hook's **tool vocabulary is Claude
> Code's** (`tool_name: "Bash"`), not the rollout's (`exec`) and not the binary's model-facing list
> (`shell`); the hook's `tool_use_id` and the rollout's `call_id` for one call are **different
> strings**, so the two records of a call are not joinable by id; and `codex --version` and
> `session_meta.cli_version` **disagree on the same machine** (**Q18**). Marked at each point of
> change, on the same rule as the review's corrections.
> **Amendment a8, 2026-08-23**, from CT-3's investigation: **Q18 is closed, and the cause was
> not the row's guess.** Two binaries, resolved by two `PATH`s — a pacman codex 0.145.0 at
> `/usr/bin`, an npm codex 0.144.0 at `~/.local/bin`, the operator's shell resolving the first
> and the launch plan's constructed child `PATH` resolving the second — so `doctor` blessed a
> binary the spawn never executed, **and every driven a7 claim was in fact driven through the
> 0.144.0 npm binary**, which is what its rollout honestly recorded. The a7 rows stand as
> behavioural claims about that binary; where they say "0.145.0" they name the machine's
> reported pin, not the binary that ran. `doctor` now resolves on the child's `PATH` and prints
> the resolved absolute path, and the adapter contract carries a `golden-version-pair` vector
> whose off-pin answer is a named warning (Q18's row carries the record).
> **Amendment a9, 2026-08-23**, from a consumer's gap register rather than from a build: **four
> payload fields, all additive and all optional.** `AEP` reads this wire as a
> transcript, and its register records *"Four expectation kinds cannot be decided about a driven
> run, because the seam's wire does not carry what they read"* — `skill.completed`, which reads the
> vendor's per-tool result record; `tokens.thinking`, `iterations` and `speed`, which had no key in
> `usage`; and a `cost.total` scoped to one model, which had none in the per-model record. That row
> closes with *"not this repository's to close: it is four fields at the seam"*, and this is the
> amendment that adds them: `tool.result` gains `tool_use_result`, and `Usage` gains
> `thinking_tokens`, `iterations`, `speed` and `cost_usd`. Every one is read from what the vendor
> wrote and none is computed — § 4.1's rule is unchanged and is the reason the aggregate `usage`
> carries no cost. Marked at each point of change, on the same rule as the review's corrections.
> **Amendment a10, 2026-08-23**, for the three-arm evaluation program (R2.5, R2.6): **a third
> decision mode and one more thing a launch may install.** The program measures how well each
> harness follows this repository's workflows under three treatments — raw instructions, the
> shipped plugin, and metaharness enforcing — and *"the instrument is constant across arms; only
> the treatment varies"*. Two things were missing for that. **`DecisionMode::Observe`** (§ 4.2) is
> the capture mode: it allows every call and records every call, through the same seam and the
> same `tool.decided` every other mode writes, so an unsteered run and a steered one produce the
> same shape of transcript and are comparable. It is not a bypass — the hook fires and answers
> `allow` — and because `allow` *grants* on this wire (§ 6, **F8**), an observe run is reported as
> what it is: a run whose seam permits everything, not a run with no seam. It is refused beside a
> frame, on **F9**'s rule. **Plugin injection** (§ 8.1 H1a, § 8.3) makes `--plugin-dir` a copy into
> the run's own scratch tree with the plugin's digest on the launch plan and in the attestation, so
> the treated arm's plugin is a pinned artifact rather than a directory that can change under the
> run. Where a plugin has to sit for a vendor to load it is a **vendor fact**: verified on Claude
> Code (its own `--plugin-dir` flag names the path), and on Codex — where there is no such flag —
> chosen from strings in the binary and then **driven once** by a directed probe (**Q19**,
> 2026-08-23): the vendor surfaced an injected plugin's skills catalog into the model's context from
> `$CODEX_HOME/plugins/<name>`, with zero tool calls. Two limits travel with that observation and
> are carried in the record itself — the child was 0.144.0 against a 0.145.0 pin, and the vendor's
> opening record still lists no plugins, so **H1a still reads `unk`** there. Marked at each point of
> change, on the same rule as the review's corrections.
> **Amendment a11, 2026-08-23**, from the machine rather than from a build: **the claude adapter
> is re-pinned 2.1.239 → 2.1.240.** The installed binary had moved and the pin had not, so every
> run reported a version disagreement it could do nothing about — `doctor claude` read *"OFF the
> adapter's pin"*, the hermetic floor's **H9** row came back `Gap`, and the contract carried a
> standing `golden-version-pair` warning. What closes it is evidence, not bookkeeping: a live run
> on 2026-08-23 drove 2.1.240 end to end through this adapter — the session opened, streamed and
> ended in the stream dialect § 4 reads, and its opening record reported
> `claude_code_version` **2.1.240** — and the
> recorded wire this contract already replays byte for byte in the free tier
> (`crates/metaharness-claude/fixtures/golden/`) is **that same binary's**, captured off-pin on
> 2026-08-23 and never re-labelled. So the pin moved to the bytes; the bytes did not move to the
> pin. **§ 2.7's rows are deliberately left naming 2.1.239**, on a8's rule for the codex a7 rows:
> a dated observation keeps the binary it was read from, and what has not been re-read on 2.1.240
> is unverified there rather than silently inherited. The version pair is now reconciled on
> claude's side — `pass C2 golden-version-pair` — while codex still warns 0.144.0-vs-0.145.0,
> which stays the operator's install to resolve. **And the vendor moved again the same afternoon**:
> 2.1.241 installed at 14:02, so `doctor claude` reads *"OFF the adapter's pin"* about a machine
> that is once more ahead of the evidence. **The pin does not follow it.** A pin is the version the
> adapter holds bytes for, and chasing the installed binary with a search-and-replace is the one
> move this row forbids — the next move costs a capture, which is a decision with a price on it
> rather than an edit.
> **Amendment a12, 2026-08-29**, from a defect that cost weeks rather than from a review: **one
> more payload field on `session.started` — `withheld`, what the run asked for and the machine
> would not admit.** `offered_tools` says what the model was offered and `available_operations`
> says what the run could do; both describe something *present*, so a tool a publication gate
> dropped is missing from each of them exactly as a tool nobody wanted is. A driven session whose
> only legal route was running a program was published a six-entry catalogue instead of seven — no
> error, no warning, nothing anywhere in the record — hand-wrote files instead, and the failure was
> read as a **model** failure for weeks. It was the machine's: the capability facts the gate needs
> were absent. What was missing was never a refusal (putting the tool back in front of the model is
> the thing publication exists to avoid); it was the **fact**, with the predicate that decided.
> Additive and optional on § 4.1's rules, and **[`None`] is *the harness did not say* and never
> *nothing was withheld*** — the invariant 3 reading, the same one a4 made for the hermetic rows.
> The b10x adapter reads it from `b10x-harness --json` and states `None` where the line is silent,
> because the field is under that repository's `[Unreleased]` and its version string has not moved,
> so the observed version cannot decide which silence this is. Marked at each point of change, on
> the same rule as the review's corrections.
> **Amendment a13, 2026-08-31**, from making the direct-provider arm pass the same strict audit it
> emits: **launch facts are observations, and a vendor-only setting is not evidence owed by a loop
> that has no such setting.** The b10x builder now queries the exact resolved executable before any
> model request, refuses `--strict-version` when its banner is absent or off-pin, and supplies that
> version together with the requested model and actual cwd to the observer. The loop's own opening
> record supplies the credential class; the adapter no longer replaces it with `named`. H2, H3,
> H8 and scratch H11 are imposed from the constructed launch, while an operator cwd and the fact
> that no operator login was copied are stated unavailable. H1a and H1b are satisfied for the
> direct-provider class when no plugin directory was declared and the loop exposes no output-style
> setting; vendor adapters still owe their record of those ambient vendor surfaces. Two unedited
> `provider_emulated` excerpts pin the enforcement outcomes this distinction exists to preserve:
> an unpublished call and an approval denial both become readable refusals and failed tool outcomes.
> **Amendment a14, 2026-08-31**, from auditing that direct-provider launch: **H6 names credential
> carriage, not the vendor-specific act of copying one operator-login file.** Vendor adapters keep
> the one-file, per-spawn rule. A direct-provider adapter instead imposes zero operator-login
> copies and one caller-named source or none. The former wording made a fully known, stronger
> posture permanently `unavailable`; the row now records either adapter-class-specific
> imposition and remains advisory because neither is visible in the provider record.
> **Amendment a15, 2026-09-03**, from building the reading surface `beyond10x/bench` embeds:
> **the projection has a document form, and the events that map to no IR family are still nodes.**
> `metaharness project <events.jsonl>` writes a `trace-ir/1`-tagged JSON document — byte-stable, no
> clock, no network — and every one of the nineteen event kinds lands in an IR family or in a node
> of family **`unk` carrying its metaharness event name**, which is a metaharness extension over
> the IR's ten families and is deliberately *not* `opaque`: `opaque` means the vendor said
> something the adapter could not read, and this means metaharness read it perfectly well and
> `trace-ir/1` has no family for it. Folding the two together would report a protocol-vocabulary
> gap as a vendor-format gap. Two further corrections at the point of change in § 4.4:
> `transcript_digest` is over the **event stream's own bytes** and says so, because a document
> projected from an event stream that carried a vendor transcript's digest would name a file it was
> not made from; and **`aep trace check` is a consumer of the event stream, not of this document** —
> `aep` 0.42.0 dispatches on the first line's `format` tag and has exactly two readers,
> `metaharness.event/1` and `claude-code/stream-json`, so Q9 is **half closed** rather than closed.
> The mapping table, the alignment rule for the two-column viewer, and the `--plugin` semantics are
> `docs/design/runs-side-by-side-v0.1.md`.
> **Amendment a16, 2026-09-03**, same build: **H1a's declared set may be added to on purpose.**
> `--plugin <marketplace-repo>@<name>@<version-or-commit>` places a named third-party plugin into
> the scratch config home before launch, resolved from a marketplace the operator has **already
> fetched** so the run itself reaches no network, pinned — an unpinned spelling is refused by name,
> not warned about — and digested before the copy. H1a is unchanged and is the reason this is
> allowed: it says *plugins are exactly the declared set*, never *no plugins*. The placement layout
> is **read from a real config home and not driven**, so `InstalledPlugin::loaded_by` says which of
> the two mechanisms carried the install and how strong that claim is, and the hermetic report
> prints `plugins: none` rather than nothing when the list is empty.
> **Amendment a17, 2026-09-03**, from a consumer's undecided verdicts rather than from a build:
> **a twentieth event, `stream.closed`, and it is the last line of every stream this driver
> writes.** On 2026-09-03 eight `aep trace check` reports ended `undecided` because every
> *negative* row — `nothing-was-moved`, `no-store-command-was-run`, `nothing-was-written-to-tmp` —
> came back `unk`. The bound those rows need is *this run did X zero times*, and a reader of a file
> cannot assert it: a stream with no `Bash` call and a stream that was **cut off before the first
> one** are the same bytes, so the only honest answer was `unk`, forever, for a question the run
> itself had already answered. That is D4's failure — *"a checker reporting the tool was never
> called when what happened is that it stopped being able to see tool calls"* — one level up, about
> the file instead of about a record inside it. `opaque` closed it for a line; this closes it for
> the stream. **The driver owns the stream and knows when it ended, so the driver says so**, in a
> line carrying `events` (how many lines preceded it), `reason` (`completed` · `budget` · `killed` ·
> `error` · `steer-halt`) and `run_id`. Three consequences are decided at the point of change and
> not left to the writer: the marker is **not** `unk` in the projection but a terminal node of its
> own (§ 4.4), because `unk` means *the IR has no family for this* and reporting the one node a
> completeness check reads as a vocabulary gap would send the wrong person looking; **completeness
> is verified, never restated** — a marker whose count disagrees with the lines before it, or that
> is not the last line, is `inconsistent` and never `complete`; and a stream with **no** marker is
> named `truncated` by the audit rather than treated as a stream that did nothing, which is
> invariant 3 applied to the file. What it does **not** do is decide a row: turning a closed stream
> into a verdict about `nothing-was-moved` is the consumer's change, in the repository that owns the
> checker.
> **Audience:** whoever reviews this for acceptance, and whoever builds it afterwards.
> **Sources studied:** `beyond10x/aep` (public, read-only), and a private
> agent runtime whose patterns are described here generically and whose names, records and
> postures are not reproduced.
> **Verification date:** 2026-08-22, against `claude` **2.1.239** and `codex-cli` **0.145.0**.

---

## 1. What this is

An outward-facing protocol around an agent harness. A run **emits events** and **accepts
commands**. Everything that points inward — how a vendor binary is launched, how its records are
read, how a decision reaches it — lives in that vendor's adapter crate and is named nowhere else.

Three promises, and each one is a section below because each one is a claim that can be false:

| promise | what makes it real | § |
|---|---|---|
| one interface to many harnesses | one event vocabulary, one command vocabulary, one workflow frame; adapters render, never re-decide | 4, 5, 6 |
| completely hermetic runs | a testable list of imposed controls, each asserted from the run's own record | 8 |
| in control at every step of which tools the harness can call | a per-call decision seam, delivered at a named tier per adapter, and refused by name where the tier does not exist | 7 |

### 1.1 Two faces, one protocol

* **Binary.** `metaharness run <claude|codex> --hermetic …` — protocol events as JSON lines on
  stdout, commands as JSON lines on stdin.
* **Library.** `Metaharness::new(Kind::Codex).with_…().start(prompt)` — the same events and the
  same commands, as Rust values.

The two faces are the same protocol because they are the same **type**. § 9.3 states the
anti-drift rule mechanically: the CLI holds no protocol logic and its flag set is a `derive`
on the builder's own options struct, so a flag the library does not have cannot be added and an
option the CLI cannot express cannot be introduced.

### 1.2 What v0.1 is not

metaharness does **not own the model loop**. In v0.1 every adapter is a *harness adapter*: the
vendor keeps its loop, its sessions, its tools, its authentication and its credential custody, and
metaharness drives its documented outside surface. § 11 names the future adapter class and the
rule that keeps the two apart.

---

## 2. What the sources proved

Every row of this section is a fact read out of a file or a command, cited. Nothing here is
inferred from an adjacent fact.

### 2.1 The hermetic contract, as lived practice

`integrations/claude-code/eval/run.sh` and `eval/run-driven.sh` are a hermetic contract that has
been run rather than written down. What they impose, and why each line exists:

| control | how | the failure it exists for, in the script's own words |
|---|---|---|
| scratch config home | `CLAUDE_CONFIG_DIR=$WORK/claude-home` | *"observed before this existed: 5 foreign plugins and the user's output style, visible in the init event"* |
| credentials, and only credentials | one `cp` of `~/.claude/.credentials.json` into that home | *"auth is the one thing the run must share with the operator, and the only thing it does"* |
| environment key unset | `unset ANTHROPIC_API_KEY` unless `EVAL_USE_API_KEY=1` | an exported key *"takes precedence over the claude.ai login and may point at an account with no credits"* |
| MCP exclusion | `--strict-mcp-config`, always | *"account-level MCP servers arrive with the login, over the network, and no directory the runner controls excludes them — observed in governed run W4-1/1, three servers in a scratch home with no `mcpServers` key"* |
| scratch project, copied not referenced | the document tree is `cp -R`'d per run | *"a checkout that changes mid-run cannot change what this run was judged against"* |
| never `/tmp` | `$TMPDIR`, defaulting to `~/.cache/claude-tmp` | the machine's tmpfs drops writes under pressure |
| the record, not the configuration | `protocol trace check` over the transcript | a scratch directory is a control; the opening record is the evidence |

The last row is the doctrine, and `crates/trace-domain/src/spec.rs` states it where the bound is
defined: an MCP-server count is `unk` when the harness records no list, *"because absence of
evidence is not hermeticity, and a bound that read a missing field as zero would report its
blindest case as its best one."*

`crates/protocol-cli/src/drive.rs` asserts three properties over the constructed argv rather than
leaving them as notes, *"because every one of the failures would be silent"*: it never contains
`--bare` (which skips hooks and would delete the enforcement arm), it always carries `--settings`,
and it always carries `--strict-mcp-config`.

### 2.2 The control seam that is proven

`integrations/claude-code/hooks/` is a `PreToolUse` hook pair. Four properties were proven there
and all four are load-bearing here:

1. **Deny with a reason, not a wall.** `aep_deny` emits
   `hookSpecificOutput.permissionDecision = "deny"` with a `permissionDecisionReason`, and the
   comment states why the JSON form is used rather than exit 2: *"both deny deterministically, and
   only this one carries a reason the model is told, which is the difference between a wall and an
   instruction."*
2. **That plugin's hooks deny and never grant — by its own choice, not because the harness
   forbids it.** `aep_allow` emits no `permissionDecision` at all, and the comment gives the
   reason: *"saying `allow` here would claim an authority the layer does not have and would
   override a stricter rule elsewhere."* **The harness does honour a hook `allow`** — 2.1.239
   carries the log lines `Hook approved tool use for ${name}, bypassing permission prompt` and
   `Hook approved tool use for ${name}, but canUseTool is required`. So this is a convention worth
   copying deliberately, not a property of the mechanism, and § 6 states plainly where metaharness
   departs from it (review finding **F8**).
3. **Fail closed.** With neither `jq` nor `python3` present the hooks deny: *"a guard that silently
   stops guarding is the defect this repository writes registers about."*
4. **1:1 denial parity, measured.** `docs/plan/gap-register.md` and
   `docs/plan/harness-wave-4-governed-dogfood.md` record the answer to F13 on Claude Code 2.1.238:
   **11 hook denies, 11 `permission_denials` entries**, each carrying the tool's name, across four
   sessions of a governed run. The row is nonetheless kept **advisory** in the specification,
   because it asserts a model behaviour (that something forbidden was attempted at all) on top of
   an undocumented harness detail; the gating evidence stays in the hook-decision log and in
   `aep artifact validate`.

And two limits, stated by the same code:

* **A hook is a separate process and cannot mutate the embedder's state.** `hooks/lib.sh`:
  a hook *"cannot call `Engine::authorize`, which takes `&mut Execution` — an in-memory value
  inside the driver's process, whose mutation is the point."* So it writes
  `hook-decisions.jsonl` and the driver folds each line in **after the step's process exits**.
  *"Decisions land a moment late and they land in the real trail."* § 10.1 is the removal of that
  lateness, and it is the single largest concrete gain from adopting metaharness.
* **A whole-run denial count cannot distinguish enforcement from inactivity.** `run-driven.sh`:
  `permission.denied` is a whole-run count and *"`0` cannot distinguish enforcement holding from
  nothing being attempted, so a run in which nothing forbidden was tried audits nothing."* Getting
  the deliberate-denial case right took two attempts: the first asked for a hand-edited `status:`
  and the model legally used `aep artifact move` instead, so the guard was never exercised.

### 2.3 The embedder metaharness must serve

`crates/aep-driver/src/run.rs` and `src/executor.rs` are a deterministic engine that launches one
model session per workflow step with a tool set derived from that step's state. Four of its shapes
are constraints on this design:

| shape | where | what it constrains here |
|---|---|---|
| `StepContext` — state, index, attempt, tool config, run directory, requirements, unmet outgoing requirements, preceding step | `executor.rs` | the workflow frame's field set (§ 5) must be a superset of this, or the driver loses information by adopting metaharness |
| per-state tool set, changing at every transition | `run.rs`, `tool.rs` | the frame is **per step**, not per session |
| `tool_config` is a pure function, deliberately not a trait | `tool.rs` | *"Making this point a trait method would let a second harness quietly re-decide that `repository.write` admits a shell."* metaharness adapters **render** an operation set; they never re-decide it |
| `StepOutcome::NoVerdict` — nothing was observed | `executor.rs` | a crashed harness is not a failing run. metaharness must make "nobody found out" a first-class outcome (§ 9.4) |

### 2.4 The IR this protocol projects into, and does not rival

`crates/trace-domain/src/ir.rs` defines `trace-ir/1`: ten event families, every field an `Option`
down to the leaves, an `Opaque` family that preserves what the reader did not understand, and
indices assigned centrally so a verdict cites one thing. `crates/trace-domain/src/spec.rs` defines
`trace-spec/1`: **51 expectation kinds**, three verdicts (`ok` / `gap` / `unk`), a `Severity`
that makes an advisory expectation *evaluated and reported but not gating*, and a stated bar for
admitting a kind — *"can a transcript decide it, and can the report say what it saw."*

**Decision D1 — metaharness invents no second IR.** The event stream is the transport; `trace-ir/1`
is the judged form; the projection between them is a total function, declared in a table (§ 4.4)
and exercised by a verb (`metaharness project`, § 9.2). The reason is the one `trace-spec` gives
for its own reuse of `infra-spec`'s shape: a second vocabulary for the same claims is a second
thing that goes stale, and an author who has met one would meet a new idea in the other for no
gain.

### 2.5 Codex, as verified by the same repository

`integrations/codex/README.md` is a table of claims each labelled *verified* (run against
codex-cli 0.145.0), *vendor doc*, or *unverified*. The rows that matter here:

* Codex has a `PreToolUse` hook, **stable and enabled by default** (`codex features list`).
* Its hook **output** wire is Claude Code's shape:
  `hookSpecificOutput.{hookEventName, permissionDecision, permissionDecisionReason, updatedInput}`.
  A `deny` must carry a non-empty reason; `permissionDecision: "ask"` is refused for `PreToolUse`.
* Its hook **input** carries `tool_name` and an **unconstrained** `tool_input`.
* Codex writes files through `apply_patch` and runs shells through the exec family. There is no
  native `Write`, `Edit` or `NotebookEdit`. **This is the fact that stops the hooks porting**: every
  rule in the Claude hooks is a rule about `tool_input.file_path`, `old_string`, `new_string` or
  `command`, and `apply_patch`'s input is a patch envelope with none of those keys. A naive port
  *"would therefore look at every store write, find no `file_path`, and pass it through — a guard
  that has silently stopped guarding."*
* The transcript worth adapting is the session rollout JSONL under `$CODEX_HOME/sessions/`, not
  `codex exec --json` stdout, which carries no timestamps, no durations and no cost. The rollout
  format has **no documented stability guarantee** and drift is already observable across one
  install's history, which is why an adapter must version-gate on the recorded CLI version and
  treat an unknown shape as opaque.
* A scratch `CODEX_HOME` isolates a run the way `CLAUDE_CONFIG_DIR` does; Codex's own `.system`
  skills still appear.

### 2.6 The private prior art, described generically

Five patterns, taken as patterns:

1. **Two adapter classes, and no silent fallback.** A *harness adapter* drives a documented
   agent-product surface while that product keeps its loop, sessions, tools, approvals and
   credential custody. A *direct-provider adapter* calls a model API while the embedder owns the
   loop. Neither class silently falls back to the other; a run declares the class and the capability
   subset it requires, and an adapter that does not satisfy them refuses before it starts rather
   than degrading during.
2. **An approval is a blocking call, and the effect cannot precede it.** The dispatcher thread is
   suspended on the decision path itself with exactly two powers — publish one event, await exactly
   one correlated decision or cancellation. The holder cannot read the stream, resume the run,
   dispatch another call, or decide on its own behalf. Blocking is the mechanism, not a workaround.
3. **The approval identity is the digest of the exact request.** A different operation, input, call
   or run is a different identity, so a decision cannot be replayed against a different input, and a
   request mutated after the person saw it is a named refusal rather than a silent approval.
4. **Ordering: the tool response is written before any control is applied.** Cancelling first would
   clear the active call and leave the child waiting on a correlation that no longer exists.
5. **Adapter conformance is a tested claim, not a paragraph.** Portable lifecycle vectors — one
   stimulus and its complete observable expectation, including the typed refusal — are run against
   every adapter class, with no model and no credential.

And one honest cost carried over verbatim in shape: **the run clock is not suspended while a
decision is pending.** A decision that outlasts the remaining budget is still honoured and the call
still executes; the run then dies at the next read. § 7.6.

### 2.7 What was verified for this design

Read on 2026-08-22 from `claude` 2.1.239 (`~/.local/share/claude/versions/2.1.239`) and
`codex-cli` 0.145.0 (`/usr/bin/codex`). Method is stated per row because the strength differs.

**The claude adapter has since been re-pinned to 2.1.240** (amendment a10, 2026-08-23) and these
rows are deliberately left naming 2.1.239: each is a dated reading of a named binary, and editing
the version on one would claim a reading nobody took. What 2.1.240 has driven is recorded where it
was driven — the golden fixtures in `crates/metaharness-claude/fixtures/golden/` are that binary's
own wire, replayed byte for byte in the free tier.

**Where a row reports a count, the count is matching *lines* of `strings -n 6 <binary>`**, stated
once here rather than left to the reader to guess. A count of occurrences is a different and
larger number, and mixing the two silently is how a figure becomes unreproducible (review finding
**F17**).

| # | claim | how known |
|---|---|---|
| V1 | `--input-format stream-json` carries a **bidirectional control protocol**: `control_request` / `control_response` / `control_cancel_request` | binary strings: **104 / 83 / 42 matching lines** |
| V2 | one control-request subtype is **`can_use_tool`**, carrying `tool_name`, `input`, `permission_suggestions`, `blocked_path`, and awaited by the harness | binary strings, including the client-side dispatcher `if (e.request.subtype === "can_use_tool") { … await this.canUseTool(e.request.tool_name, e.request.input, {…}) }` and the error `Ignoring can_use_tool control_response for request_id=` |
| V3 | other subtypes present: `interrupt`, `set_permission_mode`, `set_model`, `hook_callback`, `mcp_message`, `initialize` | binary strings |
| V4 | **`can_use_tool` is shadowed and the vendor says so**: *"canUseTool will not be invoked: permissionMode 'bypassPermissions' auto-approves every tool call … To gate every tool call, use a PreToolUse hook instead"* and *"Bare allowedTools entries auto-approve the whole tool before the callback is consulted … Allow rules from settings files can also shadow the callback but are not visible here"* | binary strings, verbatim |
| V5 | `canUseTool` and `--permission-prompt-tool` are **mutually exclusive**: *"canUseTool callback cannot be used with permissionPromptToolName. Please use one or the other."* | binary string, verbatim |
| V6 | `--permission-prompt-tool` **exists** on 2.1.239, takes an **MCP tool name**, and is a *"tool_name+input wire"*; a non-MCP tool is refused | binary strings, including its own refusal messages. **It is absent from `claude --help`** — verified by enumerating all 63 documented options |
| V7 | **an SDK hook-*callback* timeout fails closed**: *"PreToolUse hook did not respond before its timeout (host client may be unreachable). **The tool call was not executed**; other configured hooks may not have completed."* | binary string, verbatim. **It is the SDK/`hook_callback` control-request path, not the on-disk `type: command` hook** — its own wording ("host client may be unreachable") and its adjacent telemetry (`tengu_sdk_hook_callback_timeout`) say so. The command hook's timeout behaviour is **not** established by this row (review finding **F7**; see Q10) |
| V7b | a `PreToolUse` hook may declare itself **non-blocking**: `async` — *"If true, hook runs in background without blocking"* — and `asyncRewake`, which *"Implies async"* | binary strings, from the hook schema. A hook that is `async` **cannot** be a control seam, which is why § 8.4 O7 asserts the emitted hook definition as a value |
| V8 | a hook **matcher may be empty, and empty matches all tools**: *"The matcher is a string: a tool name (\"Bash\"), pipe-separated list (\"Edit\|Write\"), or empty to match all."* | binary string, verbatim (the CLI's own hook documentation) |
| V9 | `permissionDecision` accepts `"allow"`, `"deny"`, `"ask"`; a further value **`defer` exists and is print-mode only** — *"returned permissionDecision=defer in interactive mode; ignoring (defer is print-mode only)"* — with a parked/auto-resumed deferred-tool mechanism (`hook_deferred_tool`, `[print.ts] Auto-resuming deferred tool:`) | binary strings. **Semantics undriven here**; see Q3 |
| V10 | `--include-hook-events` exists: *"Include all hook lifecycle events in the output stream"*, and the stream carries `{type:"system", subtype:"hook_response", hook_id, hook_name, hook_event, output, stdout, stderr, exit_code, outcome: "success"\|"error"\|"cancelled", uuid, session_id}` | `claude --help`, plus the record's own schema in binary strings. **It carries no tool-call id**, so it is a hook-lifecycle log and *not* a per-call decision audit — which is why § 4.1's `tool.decided` exists (review finding **F17**) |
| V11 | `--tools <tools…>` exists: *"Specify the list of available tools from the built-in set. Use \"\" to disable all tools, \"default\" to use all tools"* | `claude --help` |
| V12 | `--setting-sources <user,project,local>` exists and disabling a source is observable in the CLI's own messages | `claude --help`, plus *"userSettings source is disabled (--setting-sources)"* |
| V13 | `notifications/tools/list_changed` is present in the Claude Code binary (20 occurrences) | binary strings. **Whether the client re-lists a server's tools mid-session, and whether the model's offered set changes as a result, is unverified.** See Q1 |
| V14 | Codex app-server exposes **`turn/steer`** and **`thread/inject`** beside `turn/start`, `turn/interrupt`, `thread/start`, `thread/resume`, `thread/fork`, `thread/compact`, `thread/rollback` | binary strings, from the method table |
| V15 | Codex app-server issues **blocking server→client requests**: `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/permissions/requestApproval`, `item/tool/requestUserInput`, `item/tool/call`, `mcpServer/elicitation/request` | binary strings, from the request table |
| V16 | Codex `DynamicToolSpec { namespace, description, inputSchema, deferLoading }` is registered at `thread/start` and requires `initialize.params.capabilities.experimentalApi = true` | binary strings for the type; the `experimentalApi` requirement is the private prior art's recorded finding against 0.145.0 |
| V17 | Codex carries process-level sandbox knobs on its surface: `sandboxPolicy`, `permissionProfile`, `networkAccess`, `workspaceWrite`, `writableRoots`, `excludeTmpdirEnvVar`, `excludeSlashTmp` | binary strings. **Claude Code's CLI has no equivalent**, verified by the same option enumeration as V6 |
| V18 | Codex emits `hook/started` and `hook/completed` turn notifications | binary strings |
| V19 | **`--output-format stream-json` under `--print` requires `--verbose`.** Without it the CLI exits before the session starts: *"Error: When using --print, --output-format=stream-json requires --verbose"* | binary strings, **including the guard itself**: `if(c.outputFormat==="stream-json"&&!c.verbose){process.stderr.write(…)}`. Found by building the adapter: § 9.2's argv would not have run (amendment a2) |
| V20 | **`--setting-sources ""` is the vendor's own defined no-sources case**, not an accident of parsing: the parser returns the empty list for the empty string | binary strings: `function R8u(e){if(e==="")return[];…}`, and its caller is the flag's own handler — `function Sn0(e){try{let t=R8u(e);…}catch(t){…"Invalid --setting-sources flag"…}}`. This upgrades H2/V12 from *the flag exists* to *the empty value is defined* (amendment a2) |
| V21 | **2.1.239 offers no directory-listing tool.** `Glob` and `Grep` are present; `LS` is not | matching lines of `strings -n 3` (a shorter run than V1's `-n 6`, because `LS` is two characters — the shorter run is stated here rather than left to the reader): `LS` **0**, `Glob` 20, `Grep` 16, `Read` 35, `Write` 24, `Edit` 21. So the neutral operation `dir.list` renders to `Glob` (amendment a2) |

**Amendment a4 — three rows that are driven rather than read.** The first live runs produced
these, and they are the strongest evidence in this section: each was observed in a session that
was actually paid for.

| # | claim | how known |
|---|---|---|
| V22 | **The `PreToolUse` hook input carries `tool_use_id`, and it is the same string the transcript's `tool_use` block calls `id`.** So a decision correlates to exactly the hook process holding that call | **driven.** a live 2.1.239 run's hook received a `tool_use_id` equal, byte for byte, to the `id` on the `tool_use` block of the assistant record the same session had already written to stdout, and the binary builds the payload as `{…, hook_event_name:"PreToolUse", tool_name:e, tool_input:r, tool_use_id:t}` with the same `t` as its `toolUseID`. **This closes Q16** |
| V23 | **The assistant record carrying a `tool_use` block reaches stdout *before* the hook runs** | **driven.** A hook that recorded how many bytes of stdout had already been flushed when it fired reported **5504**, and byte 5504 is exactly the end of the assistant record carrying that call. The seam does not depend on the ordering — a decision may be parked before its request arrives — but it is why the common path never waits |
| V24 | **A `--settings` file *outside* `CLAUDE_CONFIG_DIR` still loads its hooks under `--setting-sources ""`** | **driven.** The hook fired in a live run configured exactly that way. **This answers Q14** in the direction the adapter had already chosen, so nothing moves |

A string in a binary is weaker than a driven call. Every row above is labelled with its method, and
§ 12 lists what would upgrade the weak ones.

---

## 3. Framing and versioning

**Decision D2 — one JSON object per line, in both directions, each carrying its own format tag.**

```
{"format":"metaharness.event/1","seq":7,"run":"…","at":"…","event":"tool.requested", …}
{"format":"metaharness.command/1","id":"c-3","command":"tool.decide", …}
```

* `seq` is a monotone per-run counter assigned in one place, for the reason `trace-ir` assigns
  indices centrally: a verdict cites one thing, and a producer that numbered its own events would
  be a second place that decides what is cited.
* `at` is the timestamp **the vendor recorded**, passed through, or absent. metaharness derives
  durations by subtracting two recorded times and yields nothing where either is missing. It never
  measures. This is `trace-ir`'s invariant and it is what lets a run's numbers be committed and
  diffed.
* The format tag is on **every line**, not on a handshake, so a truncated capture is still
  self-describing.
* Unknown **fields** on a known event are ignored in silence; an unknown **event or command name**
  is a named refusal. That asymmetry is `trace-spec`'s adapter rule applied to our own wire, and it
  is right for the same reason: our wire is an authored schema, so a misspelling here is a mistake
  the author wants to be told about.

**Decision D3 — the version is in the tag and the tag is checked.** `metaharness.event/1` and
`metaharness.command/1`. A consumer that reads a tag it does not know refuses the line and says so;
it does not guess. A field added to an existing event is additive and does not move the number; a
field removed, retyped or given new meaning does.

---

## 4. Events

### 4.1 The vocabulary

`metaharness.event/1`. Eighteen events in five groups — **nineteen since amendment a1** and
**twenty since amendment a17**, whose rows are stated below the five groups rather than folded into
them, so the first draft's count stays visible. The right-hand column is § 4.4's projection.

**Session lifecycle**

| event | payload | why it exists | → `trace-ir/1` |
|---|---|---|---|
| `session.started` | **every field of `trace-ir/1`'s `SessionStart`** — resolved model, permission mode, credential source, harness version, **output style**, cwd, offered tools, **slash commands**, skills, agents, plugins, MCP servers (list, not count) — plus adapter id and class, the raw-transcript reference (§ 8.4 O8), the `hermetic` attestation block (§ 8.3) and, **since amendment a12, `withheld`: the tools the run declared that the machine would not admit, each with the predicate that decided** | the opening record is where a class of defect is visible before a turn is spent, and it is the only place that can distinguish *offered* from *called*. The field set is the IR's rather than a shorter one of our own, because a field metaharness omits is an expectation kind that becomes undecidable (review finding **F11**) | `session_start` |
| `session.ended` | **every field of `trace-ir/1`'s `RunOutcome`** — `is_error`, subtype, stop reason, terminal reason, API error status, `num_turns`, **`duration_ms`, `duration_api_ms`, `ttft_ms`, `time_to_request_ms`**, `total_cost_usd`, `permission_denials`, **`subagents_spawned`**, `usage`, `model_usage` — plus metaharness's own decision census | the terminal record is the source of every resource fact. Same reason as above: omitting the four duration fields and `subagents_spawned` would silently kill `duration.total`, `duration.api`, `ttft`, `time_to_request` and `subagent.spawned` (**F11**) | `run_outcome` |

**Boundaries**

| event | payload | why it exists | → `trace-ir/1` |
|---|---|---|---|
| `step.entered` | `StepRef { workflow, state, index, attempt }`, `frame_digest` | the *embedder's* unit of work. The driver's per-state tool set changes here, and a run that cannot name the step cannot attribute a denial to one | — (control plane) |
| `step.left` | `StepRef`, outcome | closes the bracket | — |
| `turn.started` | `turn`, `frame_digest` in force | the *vendor's* unit of work. Distinct from a step because one step may hold several turns and, on the relaunch strategy, one turn is one session | — |
| `turn.ended` | `turn`, stop reason | | — |

A step and a turn are two different things and conflating them is how a per-step guarantee quietly
becomes a per-session one.

**Content**

| event | payload | why | → |
|---|---|---|---|
| `text` | text, request id | what the model said to the operator | `assistant_text` |
| `thinking` | text | kept separate from `text` because an assertion about what the model *said* must not match its reasoning | `assistant_thinking` |
| `thinking.estimate` | estimate, delta | the harness's live estimate, never the billed figure — one is a mid-stream guess, the other is an invoice | `thinking_estimate` |
| `injection` | text, origin | text the *harness* put in the conversation (a loaded skill body, an injected frame). Recorded, and deliberately given no expectation kind: a matcher over injected text is a wording assertion in a structural costume | `synthetic_injection` |

**Tool calls, in three events rather than two**

| event | payload | why | → |
|---|---|---|---|
| `tool.requested` | `call_id`, name, input, `decision_required: bool`, `deadline_ms`, `seam` | emitted **before** the decision and before any effect. When `decision_required` is true the run is blocked here | `tool_call` |
| `tool.decided` | `call_id`, `decision` (`allow` / `deny{reason}` / `replace{input}`), `decided_by` (`embedder` / `frame` / `deadline` / `adapter`), `seam`, `latency_ms` | **the denial record.** It is a first-class event and not a log file, which is the correction of the one gap § 2.2 names | — (control plane) |
| `tool.result` | `call_id`, `is_error`, content, byte counts, and **`tool_use_result` — the vendor's own per-tool result record, verbatim (amendment a9)** | the per-tool fields this row always promised, now carried as the vendor's JSON rather than enumerated here: the shape belongs to the *tool*, and a fixed field set would mean a tool nobody has heard of yet reports into a record with no room for it | `tool_result` |

Three events rather than two because a denial has no result and a decision is not a result. A
protocol that folded the decision into the result could not express *"this was refused and nothing
ran"* without inventing a fake result, which is fabricating an observation.

**Accounting, diagnostics and the escape hatch**

| event | payload | why | → |
|---|---|---|---|
| `usage` | per-request or per-turn tokens, cache reads and creations, per-model split, and since amendment a9 **`thinking_tokens`, `iterations`, `speed` and `cost_usd`** | costs are read from what the vendor reported, never computed | folded into `run_outcome.usage` / `requests` |
| `rate_limit` | status, window, utilization, overage flag | a billing guard: *this run must not have been paid for out of overage* is a fact about money nothing else carries | `rate_limit` |
| `command.result` | `id`, `ok` or `refused { code, reason }` | § 5.4 | — |
| `warning` | code, message | metaharness has something to say. Distinct from `opaque`, which means *the vendor said something we could not read* | — |
| `opaque` | vendor `type`, vendor `subtype`, digest of the raw record, source line | **required.** An adapter that cannot map a vendor record emits this and never drops it | `opaque` |

**Decision D4 — `opaque` is mandatory and unconditional.** The failure it prevents is the one
`trace-ir` names: *a checker reporting "the tool was never called" when what happened is that it
stopped being able to see tool calls.* An adapter that recognised a record's envelope and read
nothing out of it emits `opaque` too, because an event that produced nothing has vanished whatever
the intention was.

**Amendment a1 — a nineteenth event: `auth.expired`.** Added from a live observation rather than
from a review. A governed run on 2026-08-22 died an hour in with the vendor reporting *"Failed to
authenticate: OAuth session expired and could not be refreshed"*. The credential such a run is
launched with is a **copy** (§ 8.1 H6), and a copy cannot refresh itself.

| event | payload | why it exists | → `trace-ir/1` |
|---|---|---|---|
| `auth.expired` | `credential_source`, the vendor's own words passed through, `source_line` | the run's credential aged out mid-flight. Distinct from `session.ended` carrying an error, because *the token expired* and *the model failed* ask the operator for two different things, and an embedder forced to match on vendor prose to tell them apart is reading a string the vendor is free to change | — (control plane) |

**It is not the `error` channel § 4.3 refuses.** It does not end the run, it is not a second
terminal record, and the run still ends with `session.ended`. What it buys is a deterministic
refresh-and-retry: the one condition where re-running the identical spec is the correct response.
Its detection is the weak half — it reads the vendor's own error text, which no row of § 2.7
verifies — so it is emitted beside the record it was read from and never as the only evidence a
run's outcome rests on. **Q13.**

**Amendment a9 — four payload fields, because four expectation kinds were undecidable without
them.** No new event: the vocabulary stays at nineteen. What changed is that `tool.result` and
`Usage` were each missing a key a reader needed, and a reader with no key does not get a wrong
answer — it gets `unk`, forever, for a question the vendor had already answered in its own record.
The motivation is a consumer's, recorded in `AEP`' gap register: *"Four
expectation kinds cannot be decided about a driven run, because the seam's wire does not carry what
they read … not this repository's to close: it is four fields at the seam."*

| field | rides on | type | present when |
|---|---|---|---|
| `tool_use_result` | `tool.result` | the vendor's own JSON, verbatim | the vendor writes a per-tool result record beside the content. Claude Code does — `Skill` records `commandName` and `success`, `Bash` its `stdout`, `stderr` and `interrupted` — and a Codex rollout does not |
| `usage.thinking_tokens` | `usage`, `session.ended.usage`, each `model_usage` entry | `u64` | the vendor breaks billed thinking out of the output figure: Claude Code's `output_tokens_details.thinking_tokens`, Codex's `reasoning_output_tokens` |
| `usage.iterations` | the same three | `u64` | the vendor keeps a per-iteration usage list. **A length read off that list**, never a counter metaharness kept |
| `usage.speed` | the same three | `String` | the vendor names a speed tier, beside and distinct from `service_tier` |
| `usage.cost_usd` | the same three | `f64` | the vendor **prices** that slice. Claude Code prices its per-model split (`modelUsage[…].costUSD`) and nothing else, so this is filled under `model_usage` and absent in the aggregate and in every per-request `usage` |

**Additive and optional, and the two rules that already governed this wire decide the rest.** An
absent field is an explicit `null` and never a missing key (§ 2.1), so a build that predates this
amendment and a vendor that reports nothing stay distinguishable. And **nothing here is computed**:
the aggregate `usage` carries no cost not because a total was hard to reach but because multiplying
tokens by a rate card would produce a second figure that can disagree with the invoice — the run's
own number is `session.ended.total_cost_usd` and it stays the vendor's. An adapter that filled
`thinking_tokens` from `thinking.estimate`, or `tool_use_result` from the result content, would be
answering a question with a different question's evidence; both are refused by name in the adapters
that could have done it.

**Amendment a12 — a third question the opening record has to answer, because the first two cannot.**
No new event; the vocabulary stays at nineteen. `session.started` gains **`withheld`**, a list of
`{tool, reason}`.

| field | rides on | type | present when |
|---|---|---|---|
| `withheld` | `session.started` | a list of `{tool, reason}`, or `null` | the harness states what a run **declared** and the machine would not admit. `b10x-harness` does, from its own capability facts; no vendor harness in this workspace does, and each says `null` rather than `[]` |

**Three fields, three questions, and the third is the one that was silent.** `offered_tools` is
what the model was **offered**. `available_operations` is what the run could **do**. Both describe
a set that is *present*, so a tool a publication gate refused to admit is absent from both of them
in exactly the way a tool nobody wanted is — and the two runs produce an identical record. That is
not a gap in a report; it is a gap in the only evidence anybody has after the run.

**What it cost.** On 2026-08-29 a driven session whose only legal route was running a program was
published six catalogue entries instead of seven. No error, no warning, no fact in the record. It
hand-wrote files instead, and for weeks the failure was read as a model failure. The cause was the
machine's: the capability facts the gate reads were absent, which is the gate working exactly as
designed. The missing thing was never a **refusal** — a refusal would put the tool back in front of
the model, which is what publication exists to prevent — it was the **fact**, plus the predicate
that decided, in the machine's own words.

**`null` is *the harness did not say*, and never *nothing was withheld*.** Invariant 3 and
amendment a4's rule, one level down: a producer that writes this field states `[]` for a run that
got everything it asked for, and a producer that has never heard of the field states nothing at
all. An adapter that read silence as `[]` would be asserting *this machine admitted everything*
about a machine it never probed. Which is why the field serializes as an explicit `null` rather
than being skipped — § 2.1's rule, restated by a9: a missing key is precisely the silence this
field exists to end.

**The b10x adapter reads silence as silence, and the reason is a version that cannot decide.**
`b10x-harness` skips its own `withheld` when empty, so an absent key on its wire is either *nothing
was withheld* or *a build from before the field*. The observed version cannot tell them apart: the
field is under that repository's `[Unreleased]` and the binary answered `0.1.0` before and after it
landed — the same failure `emitted_flags` was written for, where `--substrate-embedded` changed
shape under an unmoved version string. So the adapter states `null`. **The harness's own converter
answers `[]`**, and that is not a contradiction: it stamps `harness_version` with its *own*
`CARGO_PKG_VERSION`, so it has already claimed the record as that build's, and that build writes
the field whenever the loop reports one.

**Where it stops: `trace-ir/1` has no `withheld` field.** The fact crosses this wire and is not
projectable — § 4.4's structural check lists the IR's fields per family, and `SessionStart`'s do
not include this one, so the projection carries it no further than the event stream. That is
stated rather than papered over: an entry added to that list would claim a family was filled from
a field it cannot receive. Until the repository that owns the IR carries it, the reader of the
fact is § 9.4's audit report, which prints it beside the census, and any consumer reading the
event stream directly.

**Amendment a17 — a twentieth event: `stream.closed`, and it is the last line.** Added from a
consumer's undecided verdicts rather than from a review. Eight `aep trace check` reports on
2026-09-03 ended `undecided` because every *negative* row was `unk`: a bound of the shape *this run
did X zero times* cannot be asserted from a file, because **a stream with none of them and a stream
that was cut off before the first one are the same bytes.** This driver owns the stream and knows
when it ended, so it says so in a line of its own.

| event | payload | why it exists | → `trace-ir/1` |
|---|---|---|---|
| `stream.closed` | `events` — how many lines preceded this one; `reason` — `completed` \| `budget` \| `killed` \| `error` \| `steer-halt`; `run_id` | **the completeness record.** Without it an absence and a truncation are indistinguishable, so every negative expectation about a run is undecidable whatever the run did | **`stream_closed`** — a terminal node, and deliberately *not* `unk` (§ 4.4) |

**Five rules, each of which a test asserts rather than a sentence promising it.**

1. **It is the last line, on every exit path and for every harness kind.** Normal end, a budget
   stop, a kill, an error, a `halt` steering command: the stream ends here or it was truncated.
   There is no sixth path that ends a stream silently, and a run that broke so badly that the loop
   never wound up writes no marker — which is the honest account, because that stream *was* cut off.
2. **`events` is the count of preceding lines, and it is checked rather than believed.** A marker
   whose count disagrees with the lines before it, or that is not the last line, is **inconsistent**
   and never *complete*. A field a reader has to take on trust adds nothing a reader did not already
   have.
3. **`run_id` is on the payload as well as on the line.** D2 already puts `run` on every line, and
   the duplication is deliberate: the marker's whole purpose is to be readable **on its own**, by
   something that seeks to the end of a file. Both are rendered from the same `EventStream`, so they
   cannot disagree. It is spelled `run_id` and not `run` because the line's own key is `run` and
   this payload is flattened into the same object.
4. **`reason` is read from the run's own record, never guessed.** `budget` is written when the
   terminal record's own word says a budget stopped it — in this workspace that word is
   `budget-exhausted`, which the b10x loop writes and this repository has read out of a record. **A
   vendor's word for the same thing that nobody here has read is not guessed at**: such a run closes
   `completed` or `error` on the terminal record's own `is_error`, and reporting it as a budget stop
   would be inventing an observation (invariant 3). `error` also covers *no terminal record at all*
   — the stream is complete and the run is not, and those are two different facts in two different
   fields.
5. **It ends no run and decides no row.** It is not the `error` channel § 4.3 refuses and it is not
   a second terminal record: `session.ended` is still the terminal record, still carries every
   resource fact, and still comes first. What this line adds is the one thing that record cannot
   carry — *and then the file stopped, on purpose*. Turning that into a verdict about
   `nothing-was-moved` belongs to the consumer that owns the checker.

**Additive on D3's rule, and the vocabulary moves from nineteen to twenty.** A reader of an older
stream finds no marker and is told `truncated`, which is the correct answer about a file whose
producer never promised to close it — and is why the absence is *named* rather than defaulted to
complete.

### 4.2 Decision modes

**Decision D5 — the embedder chooses, per run and overridable per operation, between two modes.**
**Amendment a10 adds a third, and it is not a shortcut for either of them.**

| mode | who decides a call | cost | when |
|---|---|---|---|
| `frame` | the adapter, from the frame's allowed set. `tool.requested` is emitted with `decision_required: false`, followed immediately by `tool.decided { decided_by: "frame" }` | no round trip | the common case: the frame already says yes or no |
| `ask` | the embedder. `decision_required: true`, and the run blocks | one round trip per call | argument-level judgement, or an embedder whose state must move at decision time (§ 10.1) |
| `observe` **(a10)** | **nobody.** Every call is allowed and recorded: `decision_required: false`, then `tool.decided { decision: "allow", decided_by: "observe" }` | no round trip | **measuring a harness that is not being steered, with the instrument that measures one that is** |

Two modes rather than one because a round trip per call costs latency, and an embedder that answers
"yes" to everything the frame already admits has bought nothing. A per-operation override exists so
`shell` can be `ask` while `read` is `frame`.

Every mode emits `tool.decided`. The census in `session.ended` counts them all. A run in `frame`
mode is still fully audited; it is not less controlled, it is controlled by a policy stated in
advance rather than by a callback.

**Why `observe` is a decision mode and not the absence of one.** A run with no seam installed and a
run whose seam allows everything are different runs, and only the second can be *compared* to a
governed one: it produces the same events, in the same order, with the same correlation keys, so
one set of expectations scores both. That is the whole requirement the evaluation program puts on
this wire — the treatment varies, the instrument does not. `decided_by: "observe"` rather than
`"adapter"` because the two say different things to anybody counting: an adapter allow is a
judgement about one call, and this is a run-wide posture that judged none.

**What it costs, said once and repeated at every point of use.** `allow` **grants** on this wire
(§ 6, finding **F8**) — the harness honours it and bypasses the rest of its own permission
pipeline. So an observe run is *more* permissive than a run with no hook at all, and it is reported
as such: the launch attestation carries the mode, and an `ambient_inputs` line states the grant.
Two consequences follow and both are enforced rather than documented:

* **A frame beside `observe` is refused by name.** Finding **F9**: a frame whose text reaches the
  model while nothing enforces it tells the model *"strictly only these operations"* and makes it
  false. Observe enforces nothing by construction.
* **`observe` is reached by asking for it and by nothing else.** The default is and stays `frame`,
  and an adapter that has not driven the `allow` half of its own decision wire declares the mode
  `unverified` in its capability descriptor and **refuses it at plan time** (§ 8.4 O4) — which is
  where Codex stands today, since only its `deny` half has been driven (a7).

### 4.3 What is deliberately not an event

* **No `approval.required` / `approval.resolved` pair.** A blocking tool decision *is* the approval.
  Two vocabularies for one blocking question is two places for a race, and the prior art's own
  race table is a table about exactly one of them.
* **No `error` event.** A vendor error is `session.ended` with a reason, or a `tool.result` with
  `is_error`, or a `warning`. An error channel beside the outcome channel is a second place a run
  can end. Amendment a1's `auth.expired` is **not** that channel and the test is mechanical: it
  ends no run, and every run that emits it still ends with `session.ended`.
* **No aggregate `metrics` event.** `trace-ir`'s census is derived from the events; a computed
  summary on the wire is a second copy of the numbers that can disagree with the first.

### 4.4 The projection into `trace-ir/1`

**Decision D6 — projection is a total function, `Vec<Event> -> TraceIr`, and it is tested against
the existing reader.**

* Every event maps to exactly one `trace-ir` family or to none. The events mapping to none are the
  control-plane ones — `step.*`, `turn.*`, `tool.decided`, `command.result`, `warning`, and
  `auth.expired` (amendment a1) — and they are listed exhaustively in the table above so "none" is
  a decision rather than an omission.
* **`tool.decided` maps to nothing, and contributes to nothing.** `run_outcome.permission_denials`
  is passed through from `session.ended` — which is the vendor's own terminal record — and
  metaharness never adds to it. An earlier draft said the projection "contributes to" that count;
  that both contradicted the bullet above and made metaharness compute an aggregate from its own
  events, which § 4.3 forbids and which would guarantee a disagreement in the cross-check below
  every time a frame-mode deny was one the vendor did not count (review finding **F10**). The
  per-call denial audit lives in the metaharness stream. Widening `trace-spec` to a per-call denial
  kind is a proposal to make **there**.

**Decision D6a — the adapter retains the raw vendor transcript, and the projection is exempt from
three fields.** Three of the IR's fields are properties of a *file*, not of an event stream:
`transcript_digest` is the digest of the **raw transcript bytes**, `source_line` is a 1-based line
of that file, and `adapter` names the reader. An event stream alone cannot fill any of them — and
one transcript line can produce several IR events, so a `Vec<Event>` cannot reconstruct the
mapping either (review finding **F4**). Two consequences, both taken:

1. **§ 8.4 O8 requires the adapter to retain the raw vendor bytes and their digest.** This is not
   only for the projection: § 9.4's auditor contract runs over that transcript, and without it
   there is nothing for `protocol trace check` to read.
2. **`adapter` is exempt and is expected to differ**, because the whole point of the cross-check is
   that two different readers agreed.

**The cross-check, stated so it can pass.** For a recorded Claude Code session, the IR metaharness
projects and the IR `trace-spec`'s `claude-code/stream-json` adapter reads from the same bytes must
agree on **every event family, every index, every `source_line`, and `transcript_digest`** —
`adapter` excepted. Disagreement is a defect in the metaharness adapter. It is a C2 conformance
vector (§ 8.5) and costs nothing to run.

**Q9 gates the document form.** `trace-ir/1` is today a **`Serialize`-only** Rust type: none of the
eighteen derives in `crates/trace-domain/src/ir.rs` carries `Deserialize`, its identity fields are
`&'static str`, and `schemas/generated/` publishes `trace-spec.schema.json` and no trace-ir schema
(review finding **F1**). So in v0.1 the projection produces an **in-process Rust value**, not a
document any third party can read back. `metaharness project --to trace-ir` writes JSON for a human
and for diffing, and says so; a machine-readable trace-ir document is Q9's prerequisite, named in
§ 12.

> **Amendment a15, 2026-09-03.** The document form now exists, and three things about it are
> decided here rather than left to the writer:
>
> 1. **Every event kind is a node.** The nine control-plane kinds — `step.entered`, `step.left`,
>    `turn.started`, `turn.ended`, `tool.decided`, `command.result`, `warning`, `auth.expired`, and
>    nothing else — are written as nodes of family **`unk`** carrying the metaharness event name and
>    `reason: "no trace-ir/1 family"`. They are not dropped, and they are not folded into `opaque`,
>    which means the opposite thing (*the vendor said something the adapter could not read*). The
>    tenth kind D6 could have listed, `usage`, is not control-plane: it folds into `run_outcome`.
> 2. **`transcript_digest` is over the event stream's own bytes.** D6a made this field exempt
>    because it is a property of a file; the file this document is made from is the event stream,
>    so that is the file it names. The vendor transcript's own reference, where `session.started`
>    carried one, travels beside it as `metaharness.vendor_transcript` — two digests meaning two
>    things, neither pretending to be the other.
> 3. **`aep trace check` reads the event stream, not this document.** Established by reading
>    `aep/crates/trace-spec/src/reader.rs` at `e27c84b`: the reader dispatches on the first
>    non-blank line's `format` tag and has exactly two adapters, `metaharness.event/1` and
>    `claude-code/stream-json`. So **Q9 is half closed** — the document is written, tagged and
>    byte-stable, and nothing outside this repository can read it back into an IR. The § 4.4
>    cross-check is therefore asserted by comparing **censuses**, per family, rather than two
>    deserialized values. The full decision, with the complete nineteen-row mapping table, is
>    `docs/design/runs-side-by-side-v0.1.md` § 1.

> **Amendment a17, 2026-09-03, corrects a15's first point at its point of definition.** The
> mapping table is twenty rows, and the twentieth does **not** land in `unk`. `stream.closed` has
> no `trace-ir/1` family either — the IR has no vocabulary for *the file ends here* — but writing
> it as `unk` would say *metaharness read this and the IR has no family for it* about the one node
> a completeness check is supposed to decide on, and a reader counting `unk_kinds` to find
> protocol-vocabulary gaps would find the marker sitting among them. So it is written as its own
> terminal node, family **`stream_closed`**, carrying `events`, `reason` and `run_id`; the `unk`
> set is unchanged at the eight kinds a15 lists, and the document additionally carries the fact in
> its `metaharness` block (§ 1.5's rule: a metaharness fact the IR has no field for goes in one
> namespaced sibling, never scattered into the IR's nodes). **`stream_complete` there is verified,
> not copied**: it is true only when a marker is present, is the last node, and counts exactly the
> nodes before it.

---

## 5. The workflow frame

**Owner requirement, binding.** *On every turn the embedder presents the workflow state — prior
evidence, current node, next step(s), required handoff per node — and for that step strictly only a
certain set of tool calls is allowed. The frame is a typed protocol structure the embedder composes
and the adapter injects per turn (not just per session), and the per-step tool set is part of it.*

### 5.1 The type

```rust
pub struct Frame {
    pub workflow: WorkflowRef,          // id and pinned version
    pub node: NodeRef,                  // the state the run is in
    pub step: StepRef,                  // which step of that node, which attempt
    pub prior: Vec<EvidenceLine>,       // what has already been established, one line each
    pub obligations: Vec<Line>,         // what must hold while here
    pub reaching: Vec<Line>,            // what does not hold yet on a way out, prefixed with where it goes
    pub next: Vec<NodeRef>,             // the nodes reachable from here
    pub handoff: Handoff,               // what this step must produce before it may end
    pub operations: OperationSet,       // strictly the operations admitted here
    pub entities: Option<EntityList>,   // the enumerated set a routing step chooses from (§ 10.4)
    pub digest: Digest,                 // sha256 over the canonical form of everything above
}
```

Field-by-field, the source of each obligation:

* `prior`, `obligations`, `reaching`, `next` are the driver's `StepContext` (§ 2.3), and
  `reaching` is there because of a recorded failure: a run in which *"the model was never told that
  `implement` wanted a red suite and an approved specification, so it wrote neither, and the guard
  refused work that had already been paid for."*
* `obligations` are carried **verbatim, one line per requirement, each naming the document that
  asked** — never summarised. A driver that summarised here would be the only place the summary
  existed.
* `handoff` states what the step owes: a named artifact, a structured answer against a schema, or
  nothing. A step that owes nothing says so; a step whose handoff is unstated is a step nobody can
  fail.
* `digest` exists so an event can cite the exact frame in force without repeating it, and so a
  frame mutated after the model saw it is detectable rather than silent. This is the prior art's
  digest-pinning pattern (§ 2.6 item 3) applied to the frame rather than to a single approval.

### 5.2 Operations, not tool names

**Decision D7 — the frame names harness-neutral operations; the adapter renders them into vendor
tool names, and never re-decides which operations an admission implies.**

This is `aep-driver`'s adapter point 2, kept: *"Making this point a trait method would let a second
harness quietly re-decide that `repository.write` admits a shell, and the protocol would have no way
to notice."* The v0.1 operation vocabulary is deliberately small and closed:

`file.read`, `file.write`, `file.edit`, `dir.list`, `search`, `shell`, `web.read`, `skill.load`,
`subagent.spawn`, `task.todo`, `mcp.call{server,tool}`.

The rendering is the adapter's whole per-harness contribution here, and it is a value the adapter
must expose (`metaharness capabilities <kind> --render`) so an embedder can assert on it without a
run. Claude Code renders `file.edit` to `Edit`; Codex renders it to `apply_patch`; both render
`shell` to their own shell tool. **`dir.list` has no directory-listing tool to render to on Claude
Code 2.1.239** — there is no `LS` — so it renders to `Glob` (V21, amendment a2), and `mcp.call`
renders to nothing publishable because its vendor name is parameterised by server and tool rather
than absent. `subagent.spawn` defaults to **not admitted** on every adapter,
because a subagent's tool set is derived by nothing in these decisions and would be a route around
the per-step admission — the position `protocol-cli` already takes on `Task`.

### 5.3 Injection: per turn, and what that costs per adapter

The frame reaches the model in two independent ways, and both are used:

1. **As instruction text**, injected at the start of the step or turn — rendered from the typed
   frame by a function that lives in `metaharness-protocol` and is shared by every adapter, so two
   harnesses cannot describe the same frame differently.
2. **As enforcement**, through the control seam (§ 7). The text tells the model what the step is;
   the seam is what makes it true.

**Decision D8 — the offered set is fixed at launch to the union over the workflow's steps; the
admitted set is per call.** The reason is a limit, stated plainly: on Claude Code the offered tool
list is a launch flag and cannot change within a session (V11 changes what may be offered, not when).
Narrowing per step therefore happens at the decision seam, not at the offer. The honest cost is that
the model **sees tools it may not use in this step and will attempt them**, and each attempt costs a
turn and a denial. That is survivable because a denial carries a reason the model is told — which is
the difference between a wall and an instruction — but it is not free, and § 7.5 lists the two ways
out (relaunch per step; an owned tool surface) with what each costs.

### 5.4 Setting a frame is a command with a result

`frame.set` takes effect at the **next** turn or step boundary, never inside a running turn, and its
`command.result` states which boundary it will apply at. A frame that could take effect mid-turn
would mean a tool call adjudicated against a frame the model was never shown.

### 5.5 The on-disk frame document — `metaharness.frame/1` (amendment a5)

> **Amendment a5, 2026-08-22.** § 9.3 correction 3 left this format owed; the embedder
> integration is what finally needs it — a driver in another repository, which cannot link this
> workspace, hands a frame across a process boundary as a file.

One JSON object per file:

- a `format` field carrying `"metaharness.frame/1"` — the D2 per-line rule applied to a file: a
  copied or truncated document is still self-describing, and an unknown tag is refused rather
  than guessed at;
- every § 5.1 field, `digest` included, spelled exactly as the wire spells them.

**The digest is required to describe the contents.** It is SHA-256, hex, over the compact JSON
serialization of the frame object with the `digest` and `format` fields absent, object keys
sorted lexicographically at every level (which is `serde_json`'s default map order), and the
`operations` list sorted by its `op` name — then by `server` and `tool` for `mcp.call`. The
ordering rule is part of the format because the first cross-repository document failed on
exactly this: the enum's derived order was the canonical one, and no producer outside this
workspace could have known it. An external producer needs no library of ours, only these two
rules. A document whose digest is absent, stale or
wrong is **refused, never resealed**: an unsealed frame cited by digest downstream would pin
nothing, and a resealing consumer would repair exactly the mutation the digest exists to catch.

**Resolution is the library's job, on both faces.** The CLI's `--frame <file>` and the builder's
`.with_frame_file(path)` set the same spec field; `start` reads, parses and digest-verifies the
document before any I/O toward a spawn, so every failure is a free refusal by name — unreadable,
untagged, misshapen, digest-broken — and giving a document *and* an in-memory frame at once is
refused rather than resolved by precedence. D11 is intact: the binary still carries only a path.

What the document deliberately does **not** do: reach the model as text. A launch-time frame is
the enforcement half — per-call decisions from its admitted set — and the prompt stays the
embedder's, who renders § 5.1's instruction text into it if the step wants the model told. The
run therefore requires the decision channel (`tool.decide`), not the mid-session `frame.set`
command, which remains undriven and refused (§ 7.3).

**Both sides of this format are now pinned to one artifact (2026-08-23).** Until then the seam was
held together by two readings of the paragraphs above: `AEP` mints these
documents, cannot link this workspace, and tests its minter against a **transcription** of
`frame.rs` — so a drift in either the minter or the reader was invisible until a driven run died at
its first step with the session already paid for. The document its driver mints for one
deterministic step is now committed here byte-identically
(`crates/metaharness-protocol/fixtures/golden/metaharness-frame-canonical.json`, sha256
`ef897a58…`, sealed digest `43a6f845…`) and replayed through the real `Frame::parse_document`
(`crates/metaharness-protocol/tests/frame_golden.rs`), which re-derives that digest rather than
trusting the one the file states. Two things the replay found that neither side had claimed: the
two implementations agree **byte for byte on the file**, not merely on the digest — `to_document`
reproduces the minted bytes, tag and trailing newline included — and the ordering rule this section
spells out is the only part an outside producer could not have guessed, which is why it is asserted
about the bytes rather than left to a `sort`. When that fixture fails it is asking which side moved;
re-sealing it answers by deleting the evidence.

---

## 6. Commands

`metaharness.command/1`. Every command carries an `id` and produces exactly one `command.result`
event. **Decision D9 — a command that can be silently ignored is a control surface that cannot be
tested**, so silence is not a legal outcome; refusal is.

| command | payload | tier it needs | refusal when unavailable |
|---|---|---|---|
| `tool.decide` | `call_id`, `allow` / `deny{reason}` / `replace{input}` | call-level | `UNSUPPORTED_CONTROL` at run start, not at the call |
| `frame.set` | `Frame` | turn-level (text) **and** call-level (enforcement) — both, or neither | `UNSUPPORTED_CONTROL` **at run start**, like every other row |
| `message.inject` | text | turn-level | `UNSUPPORTED_CONTROL` |
| `steer` | text | mid-turn | `UNSUPPORTED_CONTROL` — and on Claude Code headless this is **always** the answer (§ 7.4) |
| `permission.set` | posture | run-level | `UNSUPPORTED_CONTROL` |
| `interrupt` | reason | kill | every adapter must deliver this |
| `halt` | reason | kill | every adapter must deliver this |

**`frame.set` is not partially deliverable, and an earlier draft said it was.** A frame whose text
reaches the model while nothing enforces it tells the model *"strictly only these operations"* and
makes it false — which is exactly the silent weakening § 7.1 forbids, and `command.result`'s two
values (`ok`, `refused`) cannot express it anyway (review finding **F9**). So a run whose adapter
cannot deliver call-level enforcement is refused at start when its configuration will need
`frame.set`; it is never allowed to run with an advisory frame. An embedder that genuinely wants
advisory-only text says so with `message.inject`, which claims nothing. **Amendment a5: a
*launch-time* frame (`--frame <file>`, § 5.5) requires `tool.decide`, not this command** — its
enforcement is per-call from the moment the session starts, and `frame.set` stays what it always
was: the mid-session change, still undriven and still refused by the Claude adapter (§ 7.3).

**`allow` grants, and that is a departure worth naming.** § 2.2 records a plugin convention of
denying and never granting, on the reasoning that an `allow` claims authority the layer does not
have. metaharness's `allow` **does** claim it: the harness honours a hook `allow` and bypasses the
remaining permission pipeline. The authority is taken deliberately, because metaharness is not one
guard among several — it is the embedder's decision point, and a seam that can only ever say no
cannot express a workflow's *"in this step, this is exactly what is permitted"* (review finding
**F8**). The consequence is stated so nobody discovers it: an `allow` from metaharness overrides a
stricter rule elsewhere in the vendor's settings, so a run that also relies on such a rule must use
`deny`-only policy and say so.

`deny` **must** carry a non-empty reason. Both vendors' hook wires require it, and the reason is the
only part the model can act on.

**Amendment a3 — there is a fourth decision value, `abstain`, and its absence was a hole in the
default.** The three values above are `allow`, `deny` and `replace`, and `allow` *grants*: the
paragraph above says so, and the consequence is that it overrides a stricter rule elsewhere in the
vendor's settings. Building the run loop turned that into a concrete default: `--decisions frame`
with **no frame in force** — which is what `metaharness run claude -p "…"` is — had no value
meaning *metaharness adjudicated nothing here*. Answering `allow` because there was nothing to
narrow with would have shipped a default invocation that switches the vendor's own permission
pipeline off; denying every call would have shipped one that does nothing and bills for it.

`abstain` is neither. It renders as the shape § 2.2 already records as proven — the reference hook
passes a call through by exiting 0 and emitting **no `permissionDecision` at all**, *"because
saying `allow` here would claim an authority the layer does not have"* — and the census counts it
in its own column, because *we let it through* and *we claimed nothing* are different facts about
who was in control.

`replace` exists because both vendors' hook wires carry `updatedInput` and refusing to expose it
would push embedders into deny-and-re-prompt, which costs a turn to express something the wire
already supports. A `replace` that the adapter cannot deliver is refused by name; it never silently
becomes an `allow`.

### 6.1 Refusal codes

| code | means |
|---|---|
| `UNSUPPORTED_CONTROL` | this adapter cannot honour this command **at all**. Emitted at run start for every command the run's configuration will need, so a run that will fail on control fails before it spends money |
| `UNKNOWN_CALL` | the `call_id` does not correlate to an open request |
| `TOO_LATE` | the window closed — the decision deadline expired, or the turn ended |
| `MALFORMED` | the command did not parse, or a required field is missing |
| `SHADOWED` | the command would be accepted by the vendor and silently overridden by another layer (§ 7.3 row `can_use_tool`). Refused rather than delivered, because a control that appears to work and does not is worse than one that is absent |

**These five are *command* refusal codes, and a launch refusal is not one of them** (amendment a2).
H11's ancestor walk finding a `CLAUDE.md` above the scratch root, and H8's denylist finding
`--safe-mode`, both refuse the run before it starts and neither is a command. They carry their own
name and no code, because inventing one would put a wrong word on a right refusal. `SHADOWED` is
the one code that is also reachable at launch, and on the v0.1 `RunSpec` surface it is reachable
**only** under `--tool-surface owned` — no other field can put a bare `--allowedTools` entry in the
argv. The adapter's guard is nonetheless written over the constructed argv rather than over that
flag, so it stays correct when a field that can is added.

---

## 7. The control seam

### 7.1 Four tiers, named

**Owner requirement, binding.** *The control-seam section must state, per adapter, which tier is
delivered — registration-level, call-level (blocking), turn-level (injection), or kill-only —
refusing to pretend parity.*

| tier | what it means | what it cannot do |
|---|---|---|
| **registration** | the set of tools the model is offered is decided before the session starts | cannot see an argument; cannot change within a session |
| **call (blocking)** | every call is presented for a decision **before it executes**, and the harness waits | costs a round trip; only as universal as the seam's coverage |
| **turn (injection)** | text can be added to the conversation between turns | cannot stop a call; only advises |
| **kill** | a running turn can be stopped | loses the turn; cannot be selective |

A control the adapter cannot honour is **refused by name**. It is never silently weakened into a
lower tier, and the refusal happens at run start.

### 7.2 The race: can an effect land before a deny?

This is the question the whole seam exists to answer, so it is answered case by case rather than in
general.

| case | can the effect precede the decision? | why |
|---|---|---|
| Claude Code, `PreToolUse` hook, `type: command`, **not** `async` | **No.** The harness runs the hook and waits for its exit before dispatching the tool | measured, not inferred: 11 hook denies produced 11 `permission_denials` and the forbidden write did not land (§ 2.2). **A hook that declares `async` does not block** (V7b), so the blocking property is a property of *this* hook definition and is asserted as one (§ 8.4 O7) — review finding **F6** |
| …with matcher `""` (all tools) | **No** for any call the hook sees, and matcher `""` is documented to see all | V8 is the vendor's own hook documentation. **The measured runs used narrow matchers, not `""`** — see § 7.3 and Q11 |
| …on hook **timeout** | **Unknown for a `type: command` hook.** metaharness closes it from its own side regardless (§ 7.7 rule 2) | V7's string is the SDK hook-*callback* path, not the command hook (**F7**). Q10 |
| …on hook **crash or non-JSON output** | **To verify (Q4).** metaharness closes it from its own side regardless: § 7.7 | the vendor's behaviour on a malformed hook response is not stated in any string read |
| …when metaharness's own decision is slow | **No**, by construction: the injected hook's deadline is strictly less than the configured hook timeout, and on its own deadline the hook emits `deny` | § 7.7 |
| Claude Code, `can_use_tool` | **No when it fires** — the harness awaits the `control_response`. **But it does not always fire** | V2, V4 |
| Claude Code, `--allowedTools` only | Not a race: a tool that was never offered cannot run. But arguments are invisible | `--allowedTools` governs which tools are offered; only the hook sees what one of them is allowed to *say* |
| Codex, `item/*/requestApproval` | **No** for command execution, file change and permissions: these are blocking server→client requests | V15 |
| Codex, `item/tool/call` dynamic tools | **No, and there is no race to have**: metaharness executes the tool itself, so the model never reaches an implementation | V16 |
| Codex, `PreToolUse` hook over `apply_patch` | **No** for the *call*, but the rule cannot read the *arguments*: the patch envelope carries no `file_path`, `old_string` or `new_string` | § 2.5 |
| a **second call in the same assistant message**, while the first is blocked | **No for that call's own effect**, because each call gets its own hook and each hook blocks its own call. **But the calls are not serialised with each other**: a parallel call B may execute while A's decision is pending, so an embedder that wants A-before-B must deny B and say so | `isConcurrencySafe` occurs 58× in the 2.1.239 binary; the CLI's own instruction text tells the model to batch tool-use blocks for parallelism. § 7.7 rule 5 |
| any post-hoc denial record | **Yes — the effect already landed.** A record written after the call is an audit, not a control | stated so nobody mistakes the two |

### 7.3 Claude Code — the realization matrix

Pinned to **2.1.240** (amendment a10, 2026-08-23). The rows below name 2.1.239 where that is the
binary the row was read from on 2026-08-22 — a dated observation keeps the version it was made
against, and the pin names the binary the adapter is tested against today.

| mechanism | tier | status | what it delivers |
|---|---|---|---|
| `--allowedTools` / `--tools` at launch | registration | **proven** (in daily use in `AEP`) | the offered set, fixed for the session. `--tools ""` disables the entire built-in set (V11) |
| **`PreToolUse` hook, matcher `""`, calling back into metaharness** | **call (blocking)** | **mechanism and 1:1 parity: measured — under narrow matchers.** The wire, the deny-with-reason and the 11-for-11 `permission_denials` were observed with matchers `Edit\|Write\|NotebookEdit` and `Bash` (`integrations/claude-code/hooks/hooks.json`), never with `""`. **Matcher `""` itself is a vendor doc string (V8) and undriven**, and it changes the regime: a child process per `Read`, `Glob`, `Grep`, `WebFetch`, `TodoWrite` and every MCP call, with the latency and timeout budget that implies. **Timeout behaviour is Q10, not V7** | **the default seam**, and the one row in this table whose status was overstated in the first draft (review findings **F13**, **F7**) |
| `can_use_tool` control request over `--input-format stream-json` | call (blocking) | **verified present** (V1, V2); **shadowed** by bare `--allowedTools` entries, by settings allow rules and by `bypassPermissions` (V4, the vendor's own strings); **mutually exclusive** with `--permission-prompt-tool` (V5) | offered only under a posture where nothing shadows it; otherwise `SHADOWED` (§ 6.1). Its advantage over the hook is that it needs no child process and no on-disk plugin |
| `--permission-prompt-tool <mcp tool>` | call (blocking) | **exists** on 2.1.239 and is **absent from `--help`** (V6); undriven here | not used in v0.1. An undocumented flag is not a foundation, and it excludes `can_use_tool` |
| `--input-format stream-json` multi-turn + hook `additionalContext` | turn (injection) | flags verified; the composition is **undriven here** | how a frame reaches a running session without relaunch |
| `set_permission_mode` control request | run-level | verified present (V3) | posture change mid-run |
| `interrupt` control request | kill | verified present (V3) | stop the turn |
| **mid-turn steer** | — | **does not exist headless.** A running turn can only be killed | `steer` is refused by name on this adapter |
| metaharness-owned MCP server as the whole tool surface (`--tools ""` + `--mcp-config` + `--strict-mcp-config`) | registration, per step | flags verified (V11); **the composition is undriven** and per-step re-listing is **unverified** (V13) | § 7.5, opt-in |

### 7.4 Codex — the realization matrix

Pinned to **0.145.0**.

| mechanism | tier | status | what it delivers |
|---|---|---|---|
| launch-level allowlist / profile | registration | verified | the coarse surface |
| `dynamicTools` at `thread/start` under `experimentalApi` | registration, **per thread** | verified type and requirement (V16) | metaharness-defined tools, whose list is the step's operations. **Per thread, not per turn** — a per-step change means a new thread |
| `item/tool/call` (server→client) | call (blocking), for dynamic tools | verified (V15) | metaharness executes the tool; the model cannot reach the implementation or its credentials |
| `item/commandExecution/requestApproval`, `item/fileChange/requestApproval`, `item/permissions/requestApproval` | **call (blocking)**, for exec, patch and permissions | verified (V15) | this is where Codex's effects are, so this covers most of what matters — but it is **not** every tool |
| **`PreToolUse` hook, no matcher, calling back into metaharness** | **call (blocking)** | **driven, and the deny is measured (a7).** One live 0.145.0 run: the hook process received the call, metaharness answered `deny` with a reason, and **the vendor's own session record carries `Command blocked by PreToolUse hook: <the reason>` with an empty `Output:`** — the effect did not land. The model's closing message was *"The command was blocked and did not run."* The **`allow` half is undriven** and stays so | **the default seam on this adapter.** It admits or refuses by tool name and by whatever the envelope carries. **`apply_patch`'s input has no `file_path`, `old_string` or `new_string`**, so a Claude-style content rule does not port and must not be claimed to |
| …where the hook is **declared** | — | **a7 correction.** `[hooks]` in **`config.toml`**, `PascalCase` event keys. `$CODEX_HOME/hooks.json` is **not a source this binary reads** — a `hooks.json` is a *plugin manifest's* file — and an unrecognised key under `[hooks]` is **dropped without failing the config load** | so a misconfigured seam is indistinguishable from a run in which nothing was attempted, and § 7.8's rule applies here with no discount: the seam is asserted from a hook request that arrived |
| …what makes it **fire** in a scratch home | — | **a7, driven.** `--dangerously-bypass-hook-trust` on `exec`: a hook in a fresh `CODEX_HOME` is not *managed* and has no persisted trust, so without the flag it never runs. `codex features list` reports `hooks stable true` | the flag's name says the opposite of what it does here: the danger it warns of is running *somebody else's* hook unvetted, and the only hook in the scratch home is the one metaharness wrote into it |
| …the hook's **tool vocabulary** | — | **a7, driven, and it is a third vocabulary.** The hook receives `tool_name: "Bash"` for a shell call — Claude Code's word, on this vendor's wire — while the *rollout* records the same call as `custom_tool_call{name:"exec"}` and the binary's model-facing list calls it `shell` | a rendering table built from the record would deny every shell call in frame mode and report it as a frame decision. The adapter renders to the **hook's** word |
| …**correlating** the two records of one call | — | **a7: not possible by id.** The hook's `tool_use_id` (`exec-96257928-…`) and the rollout's `call_id` (`call_u3Igtq…`) are different namespaces for one call | the adapter emits both and joins neither; `turn_id` is carried on both sides for a reader that wants the join |
| `thread/inject` | turn (injection) | method verified present (V14); undriven | frame text between turns |
| `turn/steer` | mid-turn | method verified present (V14); **undriven** | claimed, not proven. Declared as `unverified` in the adapter's capability set, which means an embedder that requires it gets a refusal, not a silent no-op |
| `turn/interrupt` | kill | verified (V14) | |
| `sandboxPolicy` / `networkAccess` / `writableRoots` / `excludeSlashTmp` | process-level confinement | verified present (V17) | **Codex can constrain the process; Claude Code's CLI cannot.** This asymmetry is real and is not levelled down |
| `approval_policy` / `sandbox_mode` in the scratch config | process-level confinement | **a7, driven and read back.** `codex doctor` against the scratch home reports `restricted fs + restricted network · approval Never` | **Codex can constrain the process; Claude Code's CLI cannot.** This asymmetry is real and is not levelled down. `approval_policy = "never"` is what makes the seam the *only* thing that can refuse a call: `codex exec` on 0.145.0 has **no `--ask-for-approval` flag**, and the operator's own default (`on-request`) would let a prompt nobody is there to answer turn a call away before the hook saw it |

### 7.5 The three ways to make "strictly only these operations" true, and what each costs

| strategy | how the narrowing happens | cost | v0.1 |
|---|---|---|---|
| **A — narrow at the decision seam** | offered set is the union over steps; the hook denies anything outside the current frame | the model sees and attempts tools it may not use; each attempt is a turn and a denial | **the default.** It works on both adapters today with no unverified behaviour |
| **B — relaunch per step** | one session per step, offered set exactly the step's operations | loses conversation continuity and prompt cache; this is what `AEP`' driver does today, and it is why a step's input is *"a function of persisted state"* | supported, and the only strategy that makes the *offered* set per-step on Claude Code today |
| **C — own the tool surface** | `--tools ""` plus a metaharness MCP server whose tool list **is** the step's operations; on Codex, `dynamicTools` | metaharness must implement read, write, edit, shell. That is owning half the harness, and it changes what "the vendor keeps its loop" means | **opt-in, behind `--tool-surface owned`, and not the v0.1 default.** Per-step re-listing on Claude Code depends on unverified `list_changed` behaviour (Q1) |

Strategy C's payoff is stated because it is large: under it the race window of § 7.2 does not exist
at all, since metaharness runs the tool. Its cost is stated because it is also large.

### 7.6 What blocking does not stop

**The run clock keeps elapsing while a decision is pending.** metaharness does not suspend the
vendor's timeouts, its rate-limit windows or its own run deadline during a block. A decision that
outlasts the remaining budget is still honoured and the call still executes; the run then fails at
the next read. This is stated rather than fixed because fixing it would mean reaching into a
vendor's clock, and a control that claims to pause something it cannot pause is worse than one that
does not claim it. `tool.requested` carries `deadline_ms` so an embedder knows the budget it is
spending.

### 7.7 Ordering and deadline invariants

Five rules, each with the failure it prevents:

1. **The decision is written before any control is applied.** If an embedder answers `deny` and
   then `interrupt`, the deny is delivered to the vendor first. Cancelling first clears the active
   call and leaves the child waiting on a correlation that no longer exists.
2. **metaharness's own deadline is strictly less than the vendor's hook or callback timeout**, and
   on expiry metaharness itself emits `deny` with `decided_by: "deadline"`. This converts a
   vendor-owned ambiguity into a metaharness-owned refusal. On Claude Code the vendor's own
   behaviour is also fail-closed (V7), so the two agree; the rule exists so the guarantee does not
   depend on that agreement.
3. **A decision correlates to one request and cannot be replayed.** `tool.decide` is refused
   `UNKNOWN_CALL` for an unopened call and `TOO_LATE` for a closed one. The correlation key is the
   `call_id` **plus** the digest of the request as presented, so a decision cannot be applied to a
   different input under the same id.
4. **`interrupt` is a legal answer to a pending decision.** An embedder that does not want to decide
   must be able to stop, rather than being forced to allow or to deny in order to unblock.
5. **Several decisions may be pending at once, and they may be answered out of order.** One
   assistant message can carry several tool-use blocks and the harness runs concurrency-safe tools
   in parallel — `isConcurrencySafe` occurs 58 times in the Claude Code 2.1.239 binary, and the
   CLI's own instruction text tells the model to *"send a single message with multiple tool use
   content blocks"* to get parallelism. So the protocol carries **no single-pending-decision
   assumption**: every pending `tool.requested` is tracked by `call_id`, an answer to one does not
   release another, and an embedder that serialises its own policy does so in its own code. This
   is the one place where the studied prior art's guarantee does **not** carry over — there a
   second approval while one is pending was unreachable because the dispatcher was single-threaded
   and blocked; here it is reachable, and a design that assumed otherwise would deadlock or, worse,
   release the wrong call.

   **And the deadline follows from it.** Each pending call carries its own vendor timeout and its
   own metaharness deadline, and § 7.6 says the clock never stops — so a single-threaded embedder
   deciding call A burns call B's budget, and the adapter would emit `decided_by: "deadline"`
   denies the embedder never chose (review finding **F15**). Two rules close it: **`next_event`
   delivers every currently-pending `tool.requested` before the embedder is obliged to answer any
   of them**, and **`deadline_ms` is armed at delivery, not at the vendor's request**, so an
   embedder that answers in the order it was handed cannot be timed out by its own queue. What
   remains is a real budget, and it is the embedder's to spend.

### 7.8 Coverage is asserted, not assumed

At `session.started` the adapter compares the **offered** tool list from the vendor's own opening
record against the set its seam covers. A tool that is offered and not covered is a
launch-time refusal naming the tool. This closes by construction the failure class
`AEP` hit once and fixed by hand: a matcher that *looked* exhaustive while a
second file-writing tool walked past it.

**Coverage is not the only thing that can be nominal.** A hook that matches every tool and does not
block is a guard that has already stopped guarding, so the same launch-time assertion covers the
hook *definition* as a value: `type: command`, and neither `async` nor `asyncRewake` set (V7b,
review finding **F6**). § 8.4 O7 is where that assertion lives, beside the argv one, for the same
reason both exist: the failure would otherwise be silent.

---

## 8. The hermetic contract

### 8.1 The list

Twelve rows. Each is imposed; each is asserted **either** from the vendor's own record **or** from
a value metaharness asserts before spawning — and the two are not the same strength, so the table
says which.

**Gating is per row, not global.** An earlier draft said any `unk` fails `--hermetic strict`, while
two rows (H2, H6) declared themselves unobservable *unconditionally* — so every strict run would
have exited `3` forever (review finding **F3**). `trace-spec` already has the right shape for this
and it is borrowed here: a row is **gating** or **advisory**, an advisory row is evaluated,
reported and printed like any other, and it does not move the exit code. A row is advisory only
when its unobservability is a property of the mechanism rather than of the run.

| # | control | imposed by | asserted how | unknown means | gating? |
|---|---|---|---|---|---|
| H1a | config home is scratch — **plugins** | `CLAUDE_CONFIG_DIR` / `CODEX_HOME` to a fresh directory. **a10: a declared `--plugin-dir` is copied into the run's own scratch tree and digested before the copy, so the declared set is a pinned artifact and not a directory that can change under the run** | record: loaded plugins are **exactly** the declared set | no plugin list ⇒ `unk` | **gating** |
| H1b | …and **output style** | same | record: output style is the default | no output style ⇒ `unk`. Split from H1a because they fail independently and one unknown must not mask the other (**F11**) | **gating** |
| H2 | settings sources excluded | `--setting-sources` with user/project/local omitted — the empty value, which V20 shows is the vendor's own defined no-sources case | launch: the flag is in the argv. The *absence of allow rules that would shadow the seam* is **not separately observable in any record** | — | **advisory.** The mechanism, not the run, is what cannot be observed; § 7.3 already refuses `can_use_tool` rather than trusting this row |
| H3 | the environment is constructed, not inherited | an explicit allowlist; everything else dropped — including `ANTHROPIC_BASE_URL`, `ANTHROPIC_MODEL`, `HTTP(S)_PROXY`, `CLAUDE_CODE_*`, `DISABLE_*`, `SSH_AUTH_SOCK`, `GIT_*`, and `PATH` reduced to a stated set | **launch:** the constructed child environment, as a value, before spawning (§ 8.4 O7) | n/a — launch assertion | **gating** |
| H4 | no API key unless declared | `ANTHROPIC_API_KEY` is not in the allowlist unless the run declares `credentials: api_key` | record: credential source in the opening record | absent ⇒ `unk` | **gating** |
| H5 | MCP surface is exactly what the launch gave | `--strict-mcp-config`, always | record: the MCP server **list** — length and names | list absent ⇒ `unk`, **never zero** | **gating** |
| H6 | credential carriage is exactly declared | vendor adapters: one file into the scratch home, nothing else, **re-copied immediately before every spawn** (amendment a1); direct-provider adapters: no operator login is copied and the only source is the caller-named source or none (**amendment a14**) | **not directly assertable in any record.** The evidence is the effect: H1a, H4, H5 | — | **advisory**, and § 8.3 says why an attestation is not evidence |
| H7 | the working directory is ours | a directory metaharness created; `--add-dir` never passed. **a6: under an operator-named `--cwd` this row is attested unavailable, never claimed** — and **a6.1: its reason states that the vendor sandbox was widened to that tree** (codex: `sandbox_mode = "workspace-write"`), because a run that may change the operator's repository must say so in its own record and not only in a scratch config file | record: `cwd` in the opening record | absent ⇒ `unk` | **gating** |
| H8 | hooks and customizations are not skipped | an argv **denylist**: neither `--bare` nor **`--safe-mode`**, and neither `CLAUDE_CODE_SAFE_MODE` nor `CLAUDE_CODE_SIMPLE` in the child environment | launch: the argv and environment as values | n/a — launch assertion | **gating** |
| H9 | the vendor version is the pinned one | `doctor` before the run | record: the harness version in the opening record | absent ⇒ `unk` | **gating** |
| H10 | governing documents cannot move under the run | inputs are copied into the run directory, not referenced | record: the digest of the copied tree, in `session.started` | n/a | **gating** |
| H11 | **no memory file outside the copied tree is discoverable** | the scratch root has no `CLAUDE.md` / `AGENTS.md` ancestor above it, or an explicit `--system-prompt` replaces discovery. **a6: under an operator-named `--cwd` the walk's findings go into the attested-unavailable reason instead of refusing — the operator declared the tree, memory files and all** | launch: the ancestor walk from the scratch cwd, as a value | n/a — launch assertion | **gating** |

Three rows deserve their reasons in full.

**H8 gained a second name, and the first draft's one-name assertion was a spelling check rather
than a guard** (review finding **F5**). `--safe-mode` disables *"CLAUDE.md, skills, plugins,
**hooks**, MCP servers, custom commands and agents, output styles …"* — the same deletion of the
control seam that `--bare` performs — and it sets `CLAUDE_CODE_SAFE_MODE=1`, so the environment
half of the denylist is not decorative.

**H11 did not exist and should have** (review finding **F14**). `CLAUDE.md` auto-discovery is on in
every run that is not `--bare`, and H8 forbids `--bare` — so a `CLAUDE.md` in any ancestor of the
scratch working directory enters the context of a run this design calls hermetic. H10 pins the
*copied* documents and was silent about the uncopied ones. A second ambient input is named by the
same source and is **not** closed here: `--exclude-dynamic-system-prompt-sections` describes moving
*"cwd, env info, memory paths, **git status**"* out of the system prompt, which means git status is
in it. metaharness reports git status as an ambient input in the attestation and does not claim to
have removed it.

**H3 previously claimed an assertion it did not have** (review finding **F12**): its evidence cell
read "credential source is `none`", which answers H4 and says nothing about a proxy variable, a
model override or an ssh agent socket reaching the child. It is now a launch assertion, with the
same honest `n/a` H8 always carried.

Two more rows deserve their reasons in full, because both were bought with a real failure:

* **H5** is not a count of names but a list, because a server the session cannot authenticate to
  still exists, is still named, and is still a reach outside the sandbox. Such a server exposes no
  tool, so a tool inventory is identical with and without it, and one re-authentication between two
  runs turns it into a reachable network surface with nothing standing in the way.
* **H8** exists because `--bare` reads like a hermeticity flag and is the opposite: it *"skips
  hooks"* — which on this design is the control seam — and it also switches authentication to
  API-key-only, which silently breaks H4.

**Amendment a4 — H4 and H10 were both wrong in a way only a live run could show.** The first
hermetic run against the real binary failed its own floor, and neither failure was the run's:

* **H4 compared the wrong thing.** The row asks *"no API key unless declared"*, and the evidence
  is the opening record's `apiKeySource`. The build read that field looking for the word the spec
  used — `login` for an operator login — and 2.1.239 writes **`"none"`**, because under an
  operator login there is no API *key*: the session authenticates from the copied credential
  file. So every hermetic operator-login run would have reported a gap on the one row it most
  clearly satisfied. The row now turns on **whether the record names a key at all**, which is
  what H4 actually asserts and is robust to the vendor's choice of word.
* **H10 made `--hermetic strict` unpassable for any run that pins nothing.** A run that copied no
  input tree has **no governing document that could move under it**, so the row is *satisfied*,
  not unknown. Reading it as `unk` failed `metaharness run claude --hermetic strict -p "…"` —
  this document's own § 9.2 example — for having nothing to pin. This is finding **F3**'s shape a
  second time, and it is corrected the same way: absence of evidence is not a property, but the
  copied tree is *metaharness's own launch input*, so whether there is one is something
  metaharness knows for certain rather than something it failed to observe.

Both are recorded here rather than fixed silently because they are the argument for the C4 tier
existing at all: **there is no real opening record below it**, so no free vector could have
caught either one.

**H6 gained a lifetime, and the first draft treated a credential as a file rather than as a token**
(amendment a1). A governed run on 2026-08-22 died an hour in: *"Failed to authenticate: OAuth
session expired and could not be refreshed"*. The copy in the scratch home was valid when the run
started and was not when the harness went to refresh it, and a snapshot has nothing to refresh
against. Three ways out were considered and the choice is stated with what it does not fix:

| option | what it does | why not chosen |
|---|---|---|
| **(a) re-copy immediately before every spawn** | the freshest token the operator has, per session | **taken.** It shortens the window; it does not close it. A token that expires mid-session still kills that session |
| (b) share the live file into the scratch home by hardlink or bind, so the harness's own refresh writes back | closes the window entirely, isolation kept for everything else | **not taken in v0.1, and open as Q13.** It makes the run a writer to the operator's own credential file — the one thing § 1.2 says the vendor keeps custody of — and whether the harness's refresh is atomic against a concurrent operator session is unverified |
| **(c) surface the expiry as `auth.expired`** | the embedder can refresh and retry deterministically instead of reading the failure as a model failure | **taken**, together with (a) |

(a) and (b) are alternatives; (a)+(c) is what v0.1 does. What remains unfixed is stated rather
than hidden: a session longer than the remaining token lifetime still dies, and metaharness turns
that from an unexplained failure into a named event.

**H6 gained a direct-provider specialization** (amendment a14, 2026-08-31). The original row
described only vendor harnesses, whose operator login is copied into a scratch home. A loop that
talks to the provider directly has no operator login to copy: it accepts one caller-named source
at call time or accepts none. Reporting that stronger, fully known launch posture as
`unavailable` produced a permanent advisory gap even though metaharness knew exactly what
crossed the boundary. The control is therefore credential carriage being exactly declared; the
vendor one-file rule and the direct-provider zero-copy rule are its two adapter-class-specific
impositions. It remains advisory because neither mechanism proves carriage in the provider's own
record.

### 8.2 What hermetic does not mean

Named because a reader will otherwise assume it.

* **Not deterministic.** The model is not deterministic. What is fixed is the *inputs*: the same
  frame, the same offered set, the same copied documents, the same pinned vendor version.
* **Not network-isolated on Claude Code.** Its CLI carries no sandbox knob (V17's counterpart).
  Codex does. metaharness reports the difference in `session.started.hermetic` and does not level
  it down by omitting the field on the harness that has it.
* **Not a claim about the model's training, routing or provider.**

### 8.3 The attestation is not the evidence

`session.started` carries a `hermetic` block listing every control metaharness imposed and every
one it could not. **That block is metaharness's own claim about its own actions, and it is not
independent evidence.** The independent evidence is the vendor's opening record: the plugin list,
the MCP list, the credential source, the cwd, the version. The block exists so a reader can see the
intent beside the outcome and notice when they disagree — which is exactly the case H6 cannot cover
any other way.

**Amendment a10 gives it two more fields, and both are claims of exactly that kind.**

| field | what it says | why it is a field and not prose in `imposed` |
|---|---|---|
| `decisions` | which mode decided every call in this run (§ 4.2) | a run whose model called no tool emits no `tool.decided` at all, and *"the model never called a tool"* and *"metaharness would have allowed anything it called"* are not the same fact. A reader must be able to tell them apart from the opening record alone |
| `installed_plugins` | one entry per injected plugin: its name, the directory it came from, where it was put, its **digest**, and `loaded_by` — how this vendor is told it is there **and how strong that claim is** | the eval matrix's arm-b column is a plugin identity, so it has to be a value a consumer reads rather than a sentence somebody parses. `loaded_by` carries the observation **and its limits**, which is why it is prose and not a boolean: on Codex it now records a driven sighting (Q19) *and* that the run's plugin list was still empty and the binary was off the pin. `loaded_by` exists because the two adapters do not know it equally well (§ 8.4 O1's rule applied to placement): Claude Code's own `--plugin-dir` names the directory, and on Codex nothing names it at all — **Q19** |

`installed_plugins` is **always present and empty when there is none**. A key that vanished on a
plugin-less run would make *"this run installed nothing"* and *"this build does not report
installations"* the same bytes — the reading § 8.1 refuses everywhere else. There is no `unk` case:
this is metaharness's claim about its own copying, and it always knows.

**Whether the vendor then loaded the plugin is not in this block and must not be.** That is H1a,
read from the vendor's own plugin list — and a record that carries no plugin list leaves H1a `unk`
however strong the evidence elsewhere. Codex is the live case and it is worth keeping the two
apart: Q19's probe watched an injected plugin's **skills reach the model's context**, and that same
run's `session.started.plugins` was `null`. The treatment demonstrably arrived; the vendor still
did not enumerate it. `unk` is the honest verdict for the second question, and the first is
answered in `loaded_by`, where it belongs.

---

### 8.4 Adapter obligations

Eight, and an adapter that cannot meet one says so in its descriptor rather than meeting it
approximately.

| # | obligation | why |
|---|---|---|
| O1 | **Pin the vendor version.** The adapter declares the versions it was written against, and every `session.started` carries the version actually observed. A version outside the pin is a `warning`; under `--strict-version` it is a refusal before the run | the vendor formats are not stable public schemas. `trace-ir` versions its adapters for exactly this reason: *a verdict that changed because the reader changed is visible as such rather than as a change in the agent's behaviour* |
| O2 | **Total projection.** Every vendor record becomes exactly one metaharness event, or an `opaque` carrying its declared type, subtype and digest. Nothing is dropped, and a record whose envelope was recognised but whose body was not is `opaque` too | D4 |
| O3 | **Unknown fields tolerated, unknown records preserved.** An unrecognised *field* on a recognised record is ignored in silence; an unrecognised *record* is `opaque` | a reader that refused a transcript for carrying a new key is a reader that stops working on the next patch release, and it fails in the worst available way |
| O4 | **Declare the capability set honestly.** The adapter publishes which tiers it delivers (§ 7.1), which commands it can honour, which it refuses, and — **a10** — which **decision modes** (§ 4.2) it delivers. A tier or mode it has not driven is declared `unverified`, and an embedder that *requires* an unverified one gets a refusal rather than a silent no-op | § 7.3's `turn/steer` row is the live case. The mode table's live case is `observe` on Codex: the mode is the `allow` half of that vendor's decision wire and only the `deny` half has been driven (a7), so it is `unverified` in the descriptor **and** refused at plan time — one decision, read from one place, so the published capability and the behaviour cannot drift apart |
| O5 | **No cross-class fallback, and no mid-run degradation.** A harness adapter never becomes a direct API call. An adapter that cannot honour a declared requirement refuses at start | § 11 |
| O6 | **Publish the operation rendering as a value.** `capabilities <kind> --render` prints the neutral-operation → vendor-tool table without running anything | § 5.2. A rendering that only exists inside a run cannot be asserted on before one |
| O7 | **Assert the argv, the environment and the hook definition before spawning.** The constructed command line, the constructed child environment, the ancestor walk for memory files (H11) and the emitted hook definition — `type: command`, neither `async` nor `asyncRewake` — are all values the adapter's tests read | `AEP` does exactly this for three flags *"because every one of the failures would be silent"*. The hook-definition clause is review finding **F6**: a hook that matches everything and does not block is a guard that has already stopped guarding |
| O8 | **Retain the raw vendor transcript and its digest.** The adapter keeps the bytes it read and the digest of them, and `session.started` references both | three things depend on it and none of them works without it: `transcript_digest` and `source_line` in the projection (D6a), the § 4.4 cross-check, and § 9.4's auditor, which reads a **transcript** (review findings **F1**, **F2**, **F4**) |

### 8.5 Conformance, and what it costs

**Decision D13 — four tiers, three of which need no model, no network and no credential.**

| tier | what it runs | cost | what it proves |
|---|---|---|---|
| **C1 — launch vectors** | the argv and the child environment the adapter would construct for a given `RunSpec`, compared to a recorded expectation | free | H3, H5, H8, and the whole launch half of § 8.1 |
| **C2 — replay vectors** | recorded vendor transcripts in, expected metaharness event stream out, byte-exact JSONL | free | O2, O3, and the transcript→event mapping |
| **C3 — control vectors** | a scripted fake vendor process speaking the vendor's own wire — for Claude Code, `stream-json` plus `control_request`; for Codex, the app-server JSON-RPC — driven through allow, deny, `replace`, deadline expiry, cancel-instead-of-decide, a decision for an unknown call, and a decision that arrives after the window closed. Each step carries **one stimulus and its complete observable expectation, including the typed refusal** | free | § 7.7's four invariants, and § 6.1's refusal codes |
| **C4 — one live run** | a real session against the real binary, with a **deliberate denial** in it | costs money and network; **never part of the default gate** | the rows nothing else can reach: the vendor really does wait for the hook, the deny really does stop the effect, the record really does say what the record-asserted rows (H1a, H1b, H4, H5, H7, H9, H10) read |

**Amendment a3 — what C3 covers in M1, and what it does not.** The vectors run against a neutral
scripted seam, not against `stream-json` plus `control_request` as the row above says. The reason
is attribution rather than convenience: with the vendor's wire inside the C3 harness, a change to
that wire reports a **C2** defect under a **C3** name, and the two tiers exist to be told apart.
The vendor wire is covered by C2 against the real transcript reader. What is therefore **not**
proven at C3 in M1: that the vendor's own control channel carries a decision. That needs the real
spawner, and it is named here rather than implied by a green tier.

**Amendment a4 — C3 gained a second kind of vector, and C4 exists now.** Three **spawn vectors**
join the seven control vectors. They drive the real [`SpawnRunner`] and the **real hook program**
against a *fake vendor* — a shell script that prints stream-json and then runs the hook the launch
installed — and they cover what a scripted process structurally cannot: that the installed program
blocks and a decision reaches it through a second process, that the credential is copied at
**every** spawn and not once per run (a1), and that the raw bytes are retained as they are read
(O8). They remain free: `/bin/sh`, no model, no network, no credential.

**C4 is written and it is gated twice.** `crates/metaharness/tests/live.rs` carries the two runs
this tier is for — a hermetic run judged against its own floor, and a deliberate denial the model
cannot route around. Both are `#[ignore]`d **and** behind `METAHARNESS_LIVE=1`, because a paid
tier that a default gate could reach is a paid tier that bills an account by accident. That is not
hypothetical: it happened once during the M2 build, when a CLI test that asserted `run` exited `2`
kept running after `run` learned to spawn, and two sessions were billed before anybody noticed.
The interlock that stops it recurring is a test over the test file's own source.

C3 is the tier that carries the safety argument, and it is free. The pattern is the prior art's
portable lifecycle vectors (§ 2.6 item 5), and the reason to copy it is that it makes the adapter's
promises a **tested claim** rather than a paragraph in this document.

The **projection cross-check** (§ 4.4) is a C2 vector: for a recorded Claude Code session, the IR
metaharness projects and the IR `trace-spec`'s own adapter reads must agree. That is how "losslessly
projectable" stops being an adjective.

C4 must contain a denial that the model cannot legally route around. `AEP` learned
this the expensive way: its first deliberate-denial case asked for a hand-edited status field, the
model correctly used the CLI verb instead, and the guard was never exercised — a green run that
audited nothing.

---

## 9. The two faces

### 9.1 Library

```rust
let mut run = Metaharness::new(Kind::Codex)
    .with_hermetic(Hermetic::Strict)
    .with_credentials(Credentials::OperatorLogin)
    .with_model("sonnet")
    .with_decisions(DecisionMode::Ask)
    .with_tool_surface(ToolSurface::Native)
    .with_frame(frame)
    .with_max_turns(30)
    .start(Input::Prompt(prompt))?;

while let Some(event) = run.next_event()? {
    if let Event::ToolRequested { call_id, name, input, decision_required: true, .. } = &event {
        run.send(Command::ToolDecide { call_id: call_id.clone(), decision: policy(name, input) })?;
    }
}
```

**The builder is a face on one value, not a second configuration surface.** Every `with_…` sets
one field of a `RunSpec`; `start` consumes it. That is what makes § 9.3 possible: the fluent form
and the CLI's flags are two spellings of the same struct, and neither can grow a knob the other
cannot express. Where a caller has a whole `RunSpec` already — the driven case, § 10.1 —
`Metaharness::from_spec(spec)` skips the builder entirely.

**Decision D10 — the v0.1 library surface is synchronous and blocking.** The embedder it exists to
serve is synchronous; the studied approval mechanism is a blocking call on a worker thread, and the
prior art states plainly that *blocking is the established mechanism on this loop, not a
workaround*. An async surface would be a second concurrency model for the same seam, and the seam is
the thing that must not have two shapes.

**Synchronous does not mean one decision at a time**, and the loop above only works because of
§ 7.7 rule 5's two guarantees: `next_event` hands over every currently-pending `tool.requested`
before an answer to any of them is due, and each one's `deadline_ms` is armed at delivery. Without
those, a single-threaded `policy(…)` deciding call A would burn call B's budget and the adapter
would emit denies the embedder never chose (review finding **F15**). An embedder whose policy is
slow enough to matter drains the batch first and answers second; the type makes that the natural
shape rather than a thing to remember.

### 9.2 CLI

```
metaharness run <claude|codex> [--hermetic|--hermetic strict] [-p <prompt>] [--frame <file>]
                               [--decisions frame|ask|observe] [--tool-surface native|owned]
                               [--credentials operator-login|api-key|none]
                               [--model <m>] [--max-turns <n>] [--plugin-dir <d>]…
                               [--cwd <dir>] [--strict-version]
                               [--audit] [--spec <expectations>] [--auditor <prefix>]
                               [-- <auditor pass-through args>…]
metaharness capabilities <kind> [--render]     # declared tiers, pinned versions, operation rendering
metaharness conformance <kind>                 # the free vectors (§ 8.5) — no model, no credential
metaharness project <events.jsonl> --out <f>   # the projection, as a verb (a15: --events became a positional)
metaharness project --html <f> <a.jsonl> <b.jsonl>  # two runs, aligned, as one static page (a15)
metaharness audit --transcript <f> [--events <f>] [--spec <s>] [--auditor <p>]  # judge offline
metaharness doctor <kind>                      # installed vendor version vs the adapter's pin
```

`clap` derive throughout. `capabilities` exists so an embedder can refuse early rather than
discovering mid-run that a tier is absent; `doctor` exists because H9 needs an answer before money
is spent.

### 9.3 The anti-drift rule

**Decision D11 — there is one options type, and the `run` verb is a `derive` on it.**

`RunSpec` lives in `metaharness-protocol` and carries `#[derive(clap::Args)]` behind a feature.
`metaharness-cli` parses into `RunSpec` and passes it to `Metaharness::from_spec` unchanged. A flag
the library cannot express cannot be added, and an option the CLI cannot express cannot be
introduced.

**Three corrections the review forced, because the first statement of this rule was decorative**
(review finding **F16**):

1. **The test is scoped to `run`.** `project` and `audit` carry `--events`, `--transcript` and
   `--to`, which are not `RunSpec` fields and never will be. A test that claimed to cover "the CLI"
   could not have meant those verbs, so it is stated as what it is: the `run` subcommand's
   long-flag set equals the derived set, exactly.
2. **The document's own two surfaces already disagreed, and the flags are now added.** § 9.1 sets
   `credentials` and O1 names `--strict-version`; neither appeared in § 9.2. Both do now. That the
   drift appeared *within one document* is the argument for the mechanical test rather than against
   it.
3. **`RunSpec.frame` is a `PathBuf`, not a `Frame`.** The builder's `.with_frame(frame)` takes a
   value and sets an in-memory override; the *spec* field is a path, and resolving it is the
   library's job. Otherwise the CLI would have to parse a frame document — protocol logic in the
   binary, which this rule exists to forbid — in a serialization format § 5 does not define. **The
   on-disk frame format is therefore owed and is not in v0.1**: `--frame <file>` is refused until
   it is specified, rather than shipped against an undefined format.
   **Amendment a5, 2026-08-22: the format is now specified (§ 5.5) and the flag resolves.** The
   division this correction drew is unchanged — the library reads and parses, the binary carries
   a path — and what was refused for being undefined is now refused only for being unreadable,
   untagged, misshapen or digest-broken.

### 9.4 `--audit`: one invocation that runs and judges

**Owner requirement, binding.** *`metaharness run <kind> -p "…" --audit` does what
`AEP`' eval process does today: hermetic launch → transcript → expectation check →
`ok`/`gap`/`unk` report with distinct exit codes.*

**Decision D12 — `--audit` has a built-in floor and a pluggable ceiling. metaharness embeds no
expectation language.**

| layer | who owns it | always runs? |
|---|---|---|
| **the hermetic verdict** — the twelve rows of § 8.1, each `ok` / `gap` / `unk`, with the two advisory rows reported and not gating, plus the decision census (allowed, denied, by seam) and, **since amendment a12, what the opening record said this machine would not admit** | metaharness, built in, no spec file | yes, whenever `--audit` is given |
| **the expectation check** — arbitrary claims about what the run did | an external auditor over `trace-ir/1` | only when `--spec` is given |

The reasons, in order of weight:

1. **A rival specification language is the same mistake as a rival IR, one layer up.** D1 refuses
   the second IR; embedding a checker would reintroduce it as a second vocabulary for the same
   claims. `trace-spec/1` already carries 51 kinds, three verdicts, a severity model and a stated
   bar for admitting a kind. Re-implementing that is a second definition that goes stale.
2. **The judge should be replaceable and the projection should not be.** metaharness's job is to
   make a run *judgeable*. `trace-ir/1` is the contract; who reads it is the embedder's choice.
3. **But the hermetic rows cannot be delegated**, because they are claims about metaharness's own
   imposition and must fail even where no auditor is installed. A hermeticity that only holds when
   somebody remembered to pass a spec file is not a promise.

**The auditor contract, rewritten to fit the one auditor it names** (review finding **F2**). The
first draft invoked `<auditor> --spec <spec> --ir <path>` and it would not have run: the existing
auditor is a **two-word subcommand** that takes **`--transcript`**, not `--ir`, and it carries
options this design has no way to pass — `eval/run.sh` already passes `--advisory
billed-to-the-session`. Three corrections:

1. **`--auditor` is an argv prefix, and extra arguments pass through.**
   `--auditor 'protocol trace check' -- --advisory billed-to-the-session`. A single-word program
   name is a degenerate prefix; a subcommand is not a special case.
2. **The subject is the raw vendor transcript, not a trace-ir document.** metaharness has the bytes
   because § 8.4 O8 requires it to keep them, and the existing auditor reads exactly that. The
   trace-ir document form is Q9's, not v0.1's (D6a). The full invocation is
   `<prefix…> --spec <spec> --transcript <path> [pass-through…]`.
3. **Exit `1` from this auditor is ambiguous and must not be trusted alone.** Everything
   `protocol trace check` rejects about *itself* — an unreadable specification, an unknown
   `--advisory` id — also leaves as `1`
   (`crates/protocol-cli/src/trace.rs`: *"Everything this module rejects itself … leaves through the
   binary's top-level error handler as `1`"*). So metaharness applies the guard `run.sh` already
   applies: **an audit that produced no verdict rows is a setup failure, not a contradiction**, and
   is reported as exit `2`. A verdict table with no rows in it would otherwise go green — or red —
   while checking nothing.

For `AEP` the auditor is `protocol trace check`. Nothing in metaharness names it.

**No discovery.** The auditor is named explicitly (`--auditor`, or the field in `RunSpec`). A
`--spec` with no auditor is a **refusal**, not a skip: a specification nobody checked reads exactly
like a specification that passed.

**Exit codes of `metaharness run --audit`:**

| code | meaning |
|---|---|
| `0` | the session ran and every gating verdict is `ok` |
| `1` | a gating verdict is `gap` — a hermetic row failed, or the auditor exited `1` |
| `2` | metaharness itself could not do its job: the adapter refused, the vendor binary is off its pin, the auditor is missing or not invokable, the spec is unreadable, **or the auditor produced no verdict rows** — a table with nothing in it is a setup failure, never a verdict |
| `3` | **nobody found out** — a gating hermetic row is `unk`, or the auditor exited `3`, or the harness died without producing a record |

**Without `--audit`, `metaharness run` exits `0` when the session ran to a terminal record, `2`
when metaharness could not do its job, and `3` when the harness died without producing one. It
never exits `1`, because without an audit there is no verdict to contradict.** Stated because two
exit-code tables for one verb is how a caller comes to treat `0` as "it was fine".

**Amendment a3 — `--hermetic strict` implies the floor, so `1` and `3` are reachable without
`--audit`.** The sentence above and `strict`'s own definition (§ 8.1: a gating row that is not
`ok` fails the run) contradicted each other, and the build had to pick one. It picked `strict`:
a hermeticity that reports a gap through exit `0` unless someone also remembered `--audit` is the
same defect as one that only holds when someone remembered a spec file, which § 9.4 already
refuses two paragraphs above. `--hermetic` and `--hermetic off` are unaffected.

`3` is not a softer `1`. It is `aep-driver`'s `NoVerdict`, and it exists for the same reason: a
crashed suite is not a failing suite, and submitting a failing verdict for something that never ran
fabricates an observation. A caller that wants a run nobody could judge to be red says so in CI, as
`AEP`' own eval already does.

**The census is always printed.** A report that hides "0 denials" reads as clean when it may mean
nothing was ever attempted — § 2.2's ambiguity, in a report rather than in a counter.

**And `withheld` is printed beside it (amendment a12), because the census cannot answer what it
answers.** A tool this machine would not admit was never put in front of the model, so nothing was
ever refused for it and every denial count stays zero — the same `0` a run that was offered
everything prints. It is missing from `offered_tools` and `available_operations` too, in exactly
the way a tool nobody wanted is. One line after the census, in three shapes that are three
different facts: `withheld: <tool> (<reason>); …` when the record named any, `withheld: none
declared` when the harness stated it withheld nothing, and `withheld: not stated by the harness`
when it said nothing at all — including when there is no opening record to have said it. Rendering
the third as the second would assert that a machine nobody asked admitted everything (invariant 3,
a4's rule), which is the confusion this field exists to end.

---

## 10. Adopting it in `AEP`

### 10.1 The driver, through the library — and the gap it closes

Today `CliExecutors::run_llm` builds a `claude` argv and spawns it (`crates/protocol-cli/src/drive.rs`).
The swap is: build a `Frame` from `StepContext`, build a `RunSpec`, `start`, and answer
`tool.requested` events.

The concrete gain, and it is not ergonomic. `hooks/lib.sh` states the current limitation exactly:
a hook *"cannot call `Engine::authorize`, which takes `&mut Execution` — an in-memory value inside
the driver's process"*, so it writes `hook-decisions.jsonl` and the driver folds each line in
**after the step's process exits**. The signature is
`fn authorize(&self, execution: &mut Execution, request: &ActionRequest) -> Decision`
(`crates/aep-engine/src/engine.rs:167`), and the `&mut` is the point: the call mutates the
execution. With `DecisionMode::Ask` the decision callback runs **inside the driver's process**, so
`Engine::authorize` is called at decision time, its events land in the real execution, and the
audit trail stops being a side channel that arrives late.

Second gain: the per-state surface rules that today live in a shell script — *this state does not
admit `command.execute`*, *one simple invocation, no pipes* — become embedder code with the
engine's types in scope, and the plugin's hooks stop being a second, weaker copy of the driver's
policy.

What does **not** change, deliberately: `tool_config` stays a pure function in `aep-driver`. Frames
carry operations; metaharness renders them. The protocol still decides what a capability admits.

### 10.2 The eval, through the binary

`eval/run.sh`'s sections 1 and 2 — the scratch home, the credential copy, the `unset`, the flags —
become `metaharness run claude --hermetic strict`. Section 3.4's `protocol trace check` invocation
becomes `--audit --spec eval/expectations.trace.yaml --auditor protocol` (§ 9.4), and the trace
expectations file is **unchanged**, because the IR it is checked against is unchanged.

`run-driven.sh`'s hook-decision log stops existing as a file: the denials are `tool.decided` events
in the run's event stream, and the "allow decisions ≥ 1 and deny decisions ≥ 1" assertion — *"a
guard that denied everything is as broken as one that denied nothing"* — reads the census instead
of a JSONL file.

What the eval keeps and must keep: the **deliberate-denial case**. A run in which nothing forbidden
was attempted audits nothing, and it took two attempts to write one that the model could not legally
route around.

### 10.3 What a driven eval over the real `workflows/` and `drivers/` requires of this interface

The eval must source `workflows/development/default.yaml`, `workflows/incidents/standard.yaml`,
`workflows/migrations/forward-only.yaml`, `workflows/releases/progressive.yaml` and the step maps
in `drivers/development/` — not bespoke fixtures — so that every run improves both the bridge and
the workflow documents. Five requirements follow, and each is a constraint this design has already
taken:

| requirement | where it is met |
|---|---|
| the frame must be constructible from a workflow state plus a step-map step, losing nothing | § 5.1's field set is a superset of `StepContext` |
| the workflow document is the source; no fixture format | the frame is data the embedder composes; metaharness parses no workflow file |
| the tool set must change per step without the embedder re-implementing a naming table | § 5.2: neutral operations, adapter rendering, exposed as a value |
| the workflow is pinned for the life of the run | H10: inputs copied, digest recorded |
| a run must be judgeable without a paid call for everything except the model's own behaviour | § 8.5's free tiers; the paid tier is one live run |

**The bespoke map, and the real reason it exists.** `run-driven.sh` drives
`--map eval/driven.steps.yaml`, a map written for the eval. Its own header states why, and the
reason is cost rather than convenience: the shipped `drivers/development/default.yaml` is *"seven
states, four model sessions and three `cargo` invocations"* and would *"cost several dollars an
attempt and would spend most of that proving things `task check` already proves."* That objection
is correct and this design does not wave it away.

What metaharness changes is **which parts of the real map need a model at all**. Frame
construction from each state, the operation rendering, the per-step admitted set, the denial
policy and the projection are all exercisable at tiers C1–C3 (§ 8.5) — **free, no model, no
credential** — against the *real* `workflows/` and `drivers/` documents, driven by a scripted fake
vendor. Only the model's own behaviour needs C4. So the split is: the real documents are sourced
at every tier, and the bespoke short map survives only as the *paid* tier's script, if a short
paid tier is still wanted. That is the concrete requirement this interface places on itself — a
fixture can only fail in ways somebody wrote into it, and the cost argument stops being a reason
to use one.

**Two purposes, and how a red run is attributed to one of them.** A run over real documents is
evidence about the bridge *and* about the document, and a report that cannot say which is a report
that gets both ignored. The attribution rule is the one the sources already use:

| what went red | attributed to | because |
|---|---|---|
| a hermetic row (§ 8.1), a refusal code, an `opaque` event, a projection disagreement | **the bridge** | these are claims about metaharness's own imposition and reading; the workflow document has no way to affect them |
| a `tool.decided { deny }` for an operation the state genuinely needs, or a state whose obligations cannot be met with the operations it admits | **the workflow document** | the guard did its job and the document asked for something it did not grant. `drivers/development/default.yaml` already carries one instance of exactly this, below |
| the model failed to satisfy an obligation it was given, with the operations it needed | **neither** — it is a result about the model | and it is the only one of the three that may legitimately vary between runs |

A sixth, which is a **finding rather than a requirement**: `drivers/development/default.yaml` states
that no development profile grants `command.execute`, so a driven `llm` step holds no shell — and
the planning skill's entire surface is `aep artifact …`, every verb of which is a shell
command. `AEP` resolved that with a capability grant plus a hook constraint. Under
metaharness the same resolution is expressible without a second mechanism: the frame admits `shell`
and the embedder's `ask` policy holds it to one program and two verbs, in Rust, with the reason fed
back to the model. That is a real simplification and it should be reported to that repository rather
than assumed here.

### 10.4 The worked example: routing

**The pattern.** A user request arrives; it is *classified*; the classification is *mapped onto a
list of available entities*; the run then does the one thing that entity admits. It is general —
which workflow governs this task, which artifact kind this request becomes, which runbook this alert
matches, which handler this intent routes to — and `AEP` already contains an
instance of it: the planning skill's *"Discover, do not memorise"* rule, where the entity list comes
from `aep artifact kinds` and `aep artifact lifecycle <kind>` **at use time**, because
*"a prose copy of a validated document is a copy that goes stale."*

Three steps, three frames.

**Step 1 — `enumerate`.** `operations: {shell}` held to the enumeration command; `handoff: none`.
metaharness records the entity list in the frame it builds for step 2. The list is *data in the
frame*, not something the model remembers, because an entity list the model recalled is an entity
list the model can hallucinate having read. The model is still permitted to run the enumeration
itself — the check is on the *choice*, not on the reading.

**Step 2 — `classify`.** `operations: {}` — no tools at all. `entities: [the enumerated set]`.
`handoff: StructuredAnswer { schema }` naming exactly one member. A step whose only job is a
judgement holds no tool, so nothing it does can have an effect, and the whole step is decidable from
its handoff.

**Step 3 — `route`.** `operations:` exactly what the chosen entity admits — **a function of step 2's
answer**, which is why the frame is per step and could not be per session.

Four claims, each mechanically checkable, and this is the point of the example:

| claim | how it is checked |
|---|---|
| the classification names a member of the enumerated set | a non-member is a `tool.decided { deny }` whose reason lists the legal set — the planning skill's guardrail 4 (*"a refusal is the answer, not an obstacle"*) as a mechanism rather than as instruction text |
| the routed step's surface differs from the classify step's | two `step.entered` events with different `operations`, and `env.tool_available` / `tool.absent` in the trace specification |
| nothing outside the routed set was called | `tool.absent` over the projected IR |
| the refusal path was actually exercised | the run includes a request that classifies to an excluded entity. **Without it the denial census is `0` and audits nothing** |

The last row is the F13 lesson applied to a new workflow, and it is the row people will want to drop.

---

## 11. Out of scope, named

| out of scope | why, and where it goes |
|---|---|
| **owning the model loop** | a *direct-provider adapter* is a **different adapter class**: the embedder holds the conversation, calls a model API, and publishes tools through a port so that an in-process call and an over-the-wire callback are indistinguishable to the loop. It is a future `Kind`, and the rule that comes with it is the prior art's: **neither class silently falls back to the other.** A run declares the class and the tiers it requires; an adapter that cannot satisfy them refuses at start |
| network isolation on Claude Code | no vendor mechanism exists at the CLI (§ 8.2). Codex's knobs are reported, not emulated |
| judging a run | `trace-spec/1` and whatever else an embedder points `--auditor` at (§ 9.4) |
| a workflow engine | metaharness consumes a `Frame`; it does not decide what the next node is |
| credential minting, rotation or custody | the harness keeps its own. metaharness copies at most what the operator already has and says which file |
| multi-agent orchestration | `subagent.spawn` is not admitted by default (§ 5.2) and fan-out is the embedder's |
| a second transcript IR | D1 |
| a second expectation language | D12 |

---

## 12. Open questions and the to-verify register

Each row names the command that closes it, because a row whose closing command is not written down
is a row nobody intends to close.

| id | question | closing command | if the answer is no |
|---|---|---|---|
| Q1 | Does Claude Code's MCP client act on `notifications/tools/list_changed` mid-session, and does the model's offered set change? (V13: the string is present) | one session with a metaharness MCP server that changes its tool list between turns, reading the offered set from the stream | strategy C (§ 7.5) is per-session only; per-step narrowing stays with strategy A or B |
| Q2 | Does `can_use_tool` fire for **every** call when no bare `--allowedTools` entry, no settings allow rule and no bypass posture is present? (V4 says what shadows it; not that nothing else does) | one session in `ask` posture with a deliberately forbidden call, comparing the control requests to the tool calls | the hook stays the only universal seam, which is already the default |
| Q3 | What are `permissionDecision: "defer"`'s print-mode semantics, and does a deferred call resume through a control request? (V9) | one hook returning `defer` under `-p`, with `--include-hook-events` | nothing is lost; `defer` is simply not used |
| Q4 | What does Claude Code do when a `PreToolUse` hook exits non-zero with non-JSON output? (Timeout is answered by V7; malformed output is not) | one hook printing garbage, one exiting 1 | § 7.7 rule 2 already fails closed from our side; the answer only tells us whether the vendor agrees |
| Q5 | Does the Codex `PreToolUse` hook's `tool_input` for `apply_patch` carry any path-bearing field? (§ 2.5 says none of Claude's keys) — **and, since a7, is the hook's name for a patch call even `apply_patch`?** The one driven call showed this vendor speaks Claude Code's tool vocabulary at the hook (`Bash`, not `exec`), so the adapter's `apply_patch` rendering is the vendor's documented string and not a driven one | one recorded `apply_patch` hook invocation, reading both the `tool_name` and the `tool_input` keys | Codex's hook tier stays tool-name-level and the design already says so. A patch call whose hook name is not `apply_patch` is admitted by no frame and denied by name — fail-closed, but for the wrong reason, which is why the rendering is labelled unverified rather than assumed |
| Q6 | Does `turn/steer` deliver a mid-turn steer on 0.145.0, and what does the model see? (V14: method present) | one app-server session, steer during a long turn | `steer` is refused by name on Codex too, and the matrix says kill-only |
| Q7 | Does a Codex thread accept a `dynamicTools` change without a new thread? (V16: registered at `thread/start`) | `thread/start`, then attempt a re-registration | per-step tool sets on Codex mean a new thread per step, which is strategy B |
| Q8 | Is the Codex rollout JSONL adaptable to the same IR with no loss? (§ 2.5: no stability guarantee, drift observed) | project a corpus of rollout files and diff the census against `codex exec --json` | the Codex adapter's projection is partial and says which families it cannot fill |
| Q9 | **Can a `trace-ir/1` document be read back by anything?** It is `Serialize`-only, its identity fields are `&'static str`, and no schema is published | a change **in `AEP`**: `Deserialize` on `trace-domain`'s IR types plus a generated `trace-ir.schema.json` | D6a stands as written — the projection is an in-process value and the auditor reads the raw transcript. Nothing in v0.1 depends on the document form |
| Q10 | **What does Claude Code do when a `type: command` `PreToolUse` hook exceeds its timeout?** V7's fail-closed string is the SDK hook-*callback* path | one `claude -p` run with an on-disk hook that sleeps past its declared timeout, reading the transcript for whether the tool ran | § 7.7 rule 2 already fails closed from metaharness's side; the answer only says whether the vendor agrees |
| Q11 | **Does matcher `""` behave as documented, and what does a child process per tool call cost?** The measured parity runs used two narrow matchers. **Partly answered by amendment a4, and deliberately not called closed:** live runs with matcher `""` fired the hook for `Bash` and the deny was honoured — but a single tool is not "all tools", and neither the per-call child-process cost nor the behaviour over `Read`, `Glob`, `Grep`, `WebFetch` and `TodoWrite` was measured | one `claude -p` run with matcher `""` over a prompt that calls several **different** tools, counting hook invocations against tool calls and recording added latency | the seam enumerates the offered set instead, and § 7.8's coverage assertion becomes the guard that the enumeration is complete |
| Q12 | **Is a hook `allow` honoured for a tool a settings allow-rule would have denied, and in which direction does the conflict resolve?** § 6 takes the grant authority; the resolution order is stated by two log strings and undriven | one run with a hook `allow` against a `deny` rule in `--settings` | metaharness's policy becomes `deny`-only and § 6's grant claim is withdrawn by name |
| Q13 | **Can the operator's live credential file be shared into a scratch config home — hardlink, bind mount — so the harness's own refresh writes back, without handing the run write access to the operator's credential custody?** And, separately, which record does Claude Code write an expired-OAuth failure into, so `auth.expired` can be read from a field rather than from prose? (amendment a1) | two runs: one with a hardlinked credential file, reading whether a refresh during the run updates the operator's own file and whether a concurrent operator session survives it; one against a deliberately expired token, reading the transcript for the record that carries the failure | option (a) stands alone — the copy is re-taken per spawn, the window is short and not closed, and a session that outlives its token dies with `auth.expired` recorded before `session.ended` |
| Q14 | **Does a `--settings` file placed inside `CLAUDE_CONFIG_DIR` count as the *user* source, which `--setting-sources ""` has just switched off?** If it does, the hook the seam depends on loads from nowhere and the guard silently stops guarding — the exact failure class § 7.8 exists for. Raised while building the adapter; the vendor string *"userSettings source is disabled (--setting-sources)"* says user settings can be switched off, and says nothing about where an explicit `--settings` path sits in that order (amendment a2) | **CLOSED by amendment a4 (V24).** A live run with `--setting-sources ""` and the settings file **outside** the config home fired the hook. The placement the adapter had already chosen is the one that works, so nothing moves. What is still unasked is whether a settings file *inside* the config home would load — and the adapter has no reason to find out | — |
| Q15 | **Does the vendor's own control channel actually carry a decision?** (amendment a3). **The half that matters is now driven and the row is narrowed rather than closed:** the seam this adapter actually uses — the on-disk `PreToolUse` hook — carried a `deny` to the real 2.1.239 in a live run, the call did not run, and the vendor's own terminal record listed `Bash` in `permission_denials` (amendment a4). What remains unexercised is the *other* channel, `stream-json` + `control_request`, which this adapter does not use and refuses `SHADOWED` rather than trusting | one scripted fake vendor speaking `stream-json` with a `control_request`, driven through the same seven C3 stimuli — needed only if `can_use_tool` is ever adopted as a seam | C3's safety argument stands for the metaharness half — correlation, ordering, deadlines, refusal codes — and the hook half is now a driven C4 claim rather than a pending one |
| Q16 | **Where does the decision *envelope* belong — the thing that correlates one decision to one `call_id` on the way back to the child?** (amendment a3) | **CLOSED by amendment a4 (V22).** The spawner and the hook program are written, and the answer was read off the vendor: the hook input carries **`tool_use_id`**, which *is* the transcript's `tool_use` block `id` and therefore `Event::ToolRequested`'s `call_id`. The correlation is exact and needs no digest, no ordering assumption and no per-process bookkeeping. **M1's provisional envelope — `{"call_id":…, "response":…}` — turns out to be exactly right, so nothing about it changes.** The one thing that did change is where the rendezvous *name* comes from: the hook process picks its own, publishes its stdin under it, and metaharness matches `tool_use_id` to a call — so the shell parses no JSON | — |
| Q17 | **Can `session.started` carry the transcript's digest, when the transcript is a file the run is still writing?** § 8.4 O8 says the opening record references the retained bytes **and their digest**, and the opening record is emitted at line 1 of a file whose last line does not exist yet. M2 retains the bytes and the path, and leaves `digest` and `bytes` absent there (amendment a4) | decide which of two shapes the IR wants: a digest emitted at `session.ended`, when the file is complete, or a `transcript.sealed` event carrying it — then check whether `trace-spec`'s `transcript_digest` expectation can read it from either | O8 is met in substance — the bytes are retained and the auditor reads them by path — and the § 4.4 cross-check stays unbuilt, which it already is |
| Q18 | **Which version is the Codex adapter actually pinned to?** `codex --version` reports `codex-cli 0.145.0`, and the `session_meta.cli_version` written by the run that binary starts reports **`0.144.0`** — on the same machine, in the same run (amendment a7). `codex doctor` reports two npm installs whose package roots differ, which is the likely cause but is **not verified as the cause**. It matters because the two are read by different rows: `doctor codex` compares the *former* against the pin before money is spent, and H9's floor compares the *latter* after — so a run can pass the pre-flight and report off-pin from its own record, which is exactly what CX-M2's live run did | **CLOSED by amendment a8 (CT-3, 2026-08-23), and the cause was not this row's guess.** Not one binary reporting two strings and not two npm roots: **two binaries, resolved by two `PATH`s.** `/usr/bin/codex` is pacman `openai-codex 0.145.0-1`; `~/.local/bin/codex` is npm `@openai/codex` and reports 0.144.0; the operator's shell puts `/usr/bin` first, the launch plan's constructed child `PATH` puts `~/.local/bin` first — so the pre-flight and the run answered about different binaries, verified end to end: the golden rollout's `cli_version` 0.144.0 equals `~/.local/bin/codex --version` exactly. Closed by two mechanisms: `doctor` resolves on **the child's `PATH`** (`child_path`, exported by both adapters) and reports the resolved absolute path — on this machine it now honestly reads `~/.local/bin/codex 0.144.0, OFF the pin (0.145.0)` — and the contract carries a `golden-version-pair` vector per adapter, whose off-pin answer is a **named warning** on every conformance surface (stderr beside the `--contract` record), never a silent pass and never a failure | what stays is the machine's, not the protocol's: two installs remain, and resolving them to one — or repinning to 0.144.0 and re-verifying the a7 claims against it, since a8 shows the driven evidence was that binary's — is the operator's call. The reader's gate is unchanged: a version outside the pin is a `warning` and never a mid-read refusal. Nothing is silently widened |

| Q19 | **Does codex load a plugin from a directory placed in its `CODEX_HOME`, with no marketplace manifest and no `codex plugin add` behind it?** (amendment a10.) `codex exec` has no `--plugin-dir` — `codex plugin` installs from *marketplace snapshots* — so unlike Claude Code there is no flag with which to name a directory, and the adapter must pick a location. It picks `$CODEX_HOME/plugins/<name>`, from strings in the binary: 0.145.0 resolves `plugins/cache` and `plugins/data` under the Codex home, and a marketplace's own plugin entries are `./plugins/<plugin-name>` relative to a marketplace root | **ANSWERED YES, once, by a directed live probe (2026-08-23, run `codex-2139643`).** `metaharness run codex --hermetic --decisions observe --plugin-dir integrations/codex` copied the plugin to that placement (digest `154857db…`) and asked the model to answer **from its runtime context only, using no tools**. It answered *"Available skills catalog — `## Skills`"* — and the run made **zero tool calls**: the census read `0/0/0/0` and no `tool.requested` was emitted at all, so the catalog could not have been read off disk. **The vendor surfaced the injected plugin's skills into the model's context from this path**, with no `[marketplaces]` table and no `codex plugin add`. The launch still writes no marketplace table, deliberately — an unrecognised key under a table this binary reads is dropped without failing the config load (§ 7.4), a malformed one can fail it outright, and the probe shows the copy alone is enough | **the row is answered, not closed, and two limits travel with it in the record itself.** (1) The child was **codex 0.144.0**, the binary this machine's constructed `PATH` resolves, against a pin of 0.145.0 (Q18/a8) — a driven fact about that binary and an inference about the pin. (2) `session.started.plugins` was still `null`: **the vendor's opening record enumerates no plugins, so H1a still reads `unk` on this vendor.** What was observed is the plugin's *content* reaching the model, which is what an evaluation's treated arm needs; it is not the vendor stating what it loaded, which is what H1a asks for. Nothing is claimed about how *well* the surfaced skill is used, and nothing about 0.145.0 |

---

## 13. Adversarial review

One adversarial review, 2026-08-22, briefed to break the control seam's race windows, the hermetic
list, the IR projectability claim, the two-faces drift risk and the adapter-refusal honesty.
**18 findings: 4 blocker, 12 major, 2 minor. All 18 are folded into the text above.**

No finding was resolved by argument. Where the review was right the document changed; where a fix
was impossible in v0.1 the claim was withdrawn and a to-verify row took its place. Every correction
carries its finding number at the point of change, so a reader can see what the first draft
asserted.

### 13.1 Verdicts

| # | finding | verdict | where |
|---|---|---|---|
| F1 | `trace-ir/1` is `Serialize`-only with no published schema, so a written trace-ir document has no reader | **NEEDS-CHANGE applied.** The projection is an in-process value in v0.1; the document form is gated on **Q9** | D6a, § 12 Q9 |
| F2 | the auditor contract does not fit `protocol trace check` (`--transcript` not `--ir`; two-word subcommand; no pass-through; exit `1` ambiguous) | **NEEDS-CHANGE applied.** `--auditor` is an argv prefix with pass-through; the subject is the raw transcript; an audit with no verdict rows is exit `2` | § 9.4 |
| F3 | `--hermetic strict` could never pass — H2 and H6 were unconditionally `unk` | **NEEDS-CHANGE applied.** Per-row gating, borrowed from `trace-spec`'s severity model; H2 and H6 are advisory | § 8.1 |
| F4 | the § 4.4 cross-check cannot pass: `transcript_digest` and `source_line` are unfillable from an event stream | **NEEDS-CHANGE applied.** New obligation **O8** — the adapter retains the raw bytes and their digest; `adapter` is a named exemption | D6a, O8 |
| F5 | `--safe-mode` disables hooks exactly as `--bare` does, and H8 named only `--bare` | **CONFIRMED, applied.** H8 is a denylist over argv *and* environment | § 8.1 H8 |
| F6 | a hook may declare `async` and then does not block; nothing asserted metaharness's hook is not | **CONFIRMED, applied.** New row **V7b**; O7 asserts the hook definition as a value | V7b, § 7.8, O7 |
| F7 | V7's fail-closed string is the SDK hook-*callback* path, presented as the command-hook contract | **CONFIRMED, applied.** V7 relabelled; the command hook's timeout is **Q10** | V7, § 7.2, § 7.3 |
| F8 | "a hook can deny and never grant" is one plugin's convention, not a harness property — and § 6 ships `allow` | **CONFIRMED, applied.** § 2.2 corrected; § 6 states the grant and its consequence; **Q12** added | § 2.2, § 6 |
| F9 | `frame.set`'s "partial" outcome is unrepresentable and is the weakening § 7.1 forbids | **CONFIRMED, applied.** `frame.set` is refused at run start when enforcement is absent | § 6 |
| F10 | D6 contradicted itself on `tool.decided`, and the fix made `permission_denials` a computed number | **CONFIRMED, applied.** The "contributes to" clause is deleted; the count is passed through | § 4.4 |
| F11 | the event payloads could not fill six `trace-spec` kinds, and H1's output-style half was unassertable | **CONFIRMED, applied.** Both lifecycle payloads take the IR's full field sets; H1 split into H1a/H1b | § 4.1, § 8.1 |
| F12 | H3 claimed an assertion it did not have (credential source answers H4, not H3) | **CONFIRMED, applied.** H3 is a launch assertion over the constructed environment | § 8.1 H3 |
| F13 | matcher `""` has never been run; "proven parity" was measured with two narrow matchers | **CONFIRMED, applied.** The status cell is split; matcher `""` is **Q11** | § 7.3, § 12 Q11 |
| F14 | `CLAUDE.md` / `AGENTS.md` auto-discovery is an ambient input the list missed entirely | **CONFIRMED, applied.** New row **H11**; git status named as a second input and explicitly not closed | § 8.1 H11 |
| F15 | D10's synchronous surface collides with § 7.7 rule 5 and manufactures deadline denies | **CONFIRMED, applied.** `next_event` delivers the whole pending batch; `deadline_ms` is armed at delivery | § 7.7 rule 5, D10 |
| F16 | the anti-drift test was decorative; the document's own two surfaces already disagreed | **CONFIRMED, applied.** Test scoped to `run`; `--credentials` and `--strict-version` added; `RunSpec.frame` is a path and the on-disk frame format is owed, not shipped | § 9.2, § 9.3 |
| F17 | V1's counts reproduce under no method; V10 misnamed the field it read | **CONFIRMED, applied.** One counting method stated once; V1 corrected to 104/83/42 matching lines; V10 corrected and marked *not* a decision audit | § 2.7 |
| F18 | "four groups" over five, "Four rules" over five, § 13.1 referenced and absent | **CONFIRMED, applied.** | § 4.1, § 7.7, here |

**Nothing was found INFEASIBLE.** Four findings were resolved by *withdrawing a claim* rather than
by meeting it — F1 (the trace-ir document form), F13 (matcher `""` as proven), F16 (`--frame` as
shipped), F7 (command-hook timeout as verified) — and each left a named to-verify row behind, which
is the outcome this document prefers to a claim it cannot support.

The review also checked the private prior art for leaks and found none: no product name, internal
identifier, ADR reference or credentials posture appears in § 2.6 or anywhere else.

---

## Appendix A — every claim's method

| method | rows |
|---|---|
| read from a file in `AEP` at the path cited | § 2.1–2.4, § 10 |
| labelled *verified* in `AEP`' own Codex table, against codex-cli 0.145.0 | § 2.5 |
| pattern from a private runtime, described generically, no names or records reproduced | § 2.6 |
| `claude --help` / `codex --version` on 2.1.239 / 0.145.0, 2026-08-22 | V6, V10, V11, V12, V17 (absence side), H8's `--safe-mode` and H11's `--bare` clauses |
| strings in the shipped vendor binary, quoted verbatim where the string is the evidence. **Where a count is given it is matching lines of `strings -n 6`** | V1–V5, V7, V7b, V8, V9, V13–V18 |
| **not verified**, and labelled as such in place | Q1–Q16 |
| **observed in a live governed run and reported by the operator**, not reproduced here | amendment a1's failure: the expired OAuth session |
| **a claim the first draft made and the review removed**, each with a Q row in its place | the trace-ir document form (Q9), the command-hook timeout (Q10), matcher `""` as proven (Q11), the hook `allow` conflict order (Q12), and `--frame`'s on-disk format (§ 9.3) |
