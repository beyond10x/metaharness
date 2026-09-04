# Two runs, read side by side — Design v0.1

> **Repository:** `metaharness/metaharness`
> **Status:** **binding.** Written before the code, on `AGENTS.md` invariant 8. It amends
> `docs/design/metaharness-protocol-v0.1.md` § 4.4 (amendment **a15**) and § 8.1 H1a
> (amendment **a16**); where the two disagree, the protocol document's own amendment record is the
> pointer and this page is the detail.
> **Amended 2026-09-03 by protocol amendment a17**, which adds a twentieth event: § 1.2's table
> gains `stream.closed` as row 20 and it is the one kind with no IR family that is **not** `unk`.
> The counts below are corrected at the point of change; the rest of P2 is unchanged.
> **Decides:** the three things `epic:runs-side-by-side` could not start without — the
> event → IR mapping `metaharness project` writes, the alignment rule the viewer aligns two runs
> by, and what `--plugin` does and what the attestation says about it.
> **Verification date:** 2026-09-03, against `claude` **2.1.258** (read, never spent) and `aep`
> **0.42.0**.
> **Audience:** whoever builds the reading surface `beyond10x/bench` embeds, and whoever later
> disagrees with one of these three decisions.

---

## 0. What is decided here, in one table

| # | decision | § |
|---|---|---|
| **P1** | `project` writes a `trace-ir/1`-shaped document, byte-stable, no clock, no network | 1.1 |
| **P2** | every one of the nineteen event kinds maps to an IR family or to `unk`, and `unk` carries the kind | 1.2 |
| **P3** | the document's `transcript_digest` is over the **event stream's own bytes**, and says so | 1.3 |
| **P4** | `aep observe trace check` is a consumer of the **event stream**, not of this document; the document's consumer is the viewer | 1.4 |
| **V1** | align by workflow state entry when both runs are driven, else by tool-call index | 2.2 |
| **V2** | a step in one run and not the other renders as a **gap row**, never skipped | 2.3 |
| **V3** | one file, no server, no network, JS inline and minimal, deterministic bytes | 2.4 |
| **G1** | `--plugin <marketplace-repo>@<name>@<pin>`; an unpinned spelling is refused by name | 3.2 |
| **G2** | the plugin is resolved from an **already-fetched local marketplace cache**; a run reaches no network | 3.3 |
| **G3** | placement is into the scratch **config home**, in the layout read from a real one, and it is labelled unverified | 3.4 |
| **G4** | the attestation lists every installed plugin and says `plugins: none` when there are none | 3.5 |

---

## 1. `metaharness project` — the event → IR mapping

### 1.1 P1 — the document form

`metaharness project <events.jsonl>` reads a `metaharness.event/1` stream and writes one JSON
document tagged `trace-ir/1` on stdout, or to `--out <file>`.

**Byte-stable by construction, and the construction is the claim.** Three properties, each of
which a test asserts rather than a sentence promising it:

* **No clock.** Nothing in the writer reads one. Every timestamp in the output is the vendor's,
  passed through from `EventLine::at` — the same rule `metaharness-protocol` already carries
  (design § 4.1, D2) and the same rule the IR carries on the other side (`trace-domain::ir`,
  *"No clock, anywhere"*).
* **No network.** The writer opens one file and writes one file.
* **One serialization order.** Object keys are emitted in a fixed order, per node, decided by the
  Rust struct's field order and not by a map's iteration; the only maps in the document are
  `BTreeMap`s, which have one order.

So `project` over the same input twice is the same bytes, on any machine, on any day — which is
what lets the projected document be committed, diffed, and cited by a bench result.

### 1.2 P2 — every event kind, and `unk` is a node

The protocol emits **twenty** event kinds (`metaharness_protocol::EVENT_NAMES`) — nineteen when
this page was written, and `stream.closed` since amendment a17. `trace-ir/1` has **ten** families.
The nine that do not land in one are the control-plane events, and the decision this page makes is
that **they are still nodes**:

> An event with no IR family is written as a node of family `unk` carrying its metaharness event
> name. It is never dropped, and it is never quietly folded into `opaque`.

`opaque` already means something else and must keep meaning it: *the vendor said something the
adapter could not read*. `unk` means *metaharness read this perfectly well and `trace-ir/1` has no
family for it*. A reader that folded the two together would report a protocol-vocabulary gap as a
vendor-format gap, and would send the wrong person looking.

The mapping, complete:

| # | event | `trace-ir/1` family | note |
|---|---|---|---|
| 1 | `session.started` | `session_start` | the opening record; `withheld`, `available_operations` and `hermetic` are metaharness fields the IR has no home for and travel under `metaharness` (§ 1.5) |
| 2 | `session.ended` | `run_outcome` | the terminal record |
| 3 | `step.entered` | **`unk`** | the embedder's unit of work; the IR has no workflow vocabulary |
| 4 | `step.left` | **`unk`** | with its `StepOutcome`, which is what the viewer's gap rows are built from |
| 5 | `turn.started` | **`unk`** | the vendor's unit of work |
| 6 | `turn.ended` | **`unk`** | |
| 7 | `text` | `assistant_text` | |
| 8 | `thinking` | `assistant_thinking` | kept apart from `assistant_text` on the IR's own rule |
| 9 | `thinking.estimate` | `thinking_estimate` | a mid-stream guess, never the invoice |
| 10 | `injection` | `synthetic_injection` | |
| 11 | `tool.requested` | `tool_call` | |
| 12 | `tool.decided` | **`unk`** | metaharness's own per-call denial audit; **it contributes to nothing else** (design D6, finding F10) |
| 13 | `tool.result` | `tool_result` | |
| 14 | `usage` | `run_outcome` | folds into the terminal record's usage rather than standing alone (design § 4.3) |
| 15 | `rate_limit` | `rate_limit` | |
| 16 | `command.result` | **`unk`** | the answer to a steering command |
| 17 | `warning` | **`unk`** | metaharness has something to say |
| 18 | `opaque` | `opaque` | the vendor said something the adapter could not read |
| 19 | `auth.expired` | **`unk`** | a control-plane fact about the credential (amendment a1, Q13) |
| 20 | `stream.closed` | **`stream_closed`** | the completeness record, and the one kind with no IR family that is **not** `unk` (amendment a17). `unk` means *the IR has no family for this*; writing the marker under it would file the one node a completeness check reads among the protocol-vocabulary gaps. The node carries `events`, `reason` and `run_id`, and the `metaharness` block carries the verified `stream_complete` beside them |

Ten `unk`-bearing kinds would be a mapping nobody had thought about. **Nine is the whole
control-plane list and it is closed**: `metaharness_protocol::CONTROL_PLANE_EVENTS` already
enumerates eight of them and the ninth, `usage`, is not control-plane at all — it folds. The
writer's match is exhaustive with no wildcard arm, so a twentieth event cannot be added without
this table being answered for it.

**A twentieth event was added, and this is that answer.** `stream.closed` (amendment a17) joins
`CONTROL_PLANE_EVENTS` — it is metaharness's own record and the IR has no family for it — and it is
the one member of that list the writer does **not** render as `unk`. The `unk`-bearing set is
therefore still the eight kinds above; the ninth member of the list has a node of its own.

**`unk` in this document is not the `unk` of a verdict.** A verdict's `unk` means *nobody found
out*; this one means *the IR has no family*. They are different claims and the document says which
it is carrying: every `unk` node states `reason: "no trace-ir/1 family"`.

### 1.3 P3 — what `transcript_digest` is over

`trace-ir/1`'s `transcript_digest` is *"the digest of the raw transcript bytes, so a report can
name exactly which run it judged"*. metaharness is projecting an **event stream**, not a vendor
transcript, so writing the vendor transcript's digest there would name a file this document was
not made from.

**Decision:** `transcript_digest` is the SHA-256 of the event stream's own bytes, and the document
carries `metaharness.source: "metaharness.event/1"` beside it so the field is never read as a
vendor-transcript digest. Where the stream's `session.started` carried a `TranscriptRef`, that
reference travels too, under `metaharness.vendor_transcript` — the vendor's digest, named as the
vendor's, in a field of its own. Two digests that mean two things, and neither pretending to be
the other (invariant 3, and § 4.4's D6a which made these three fields exempt in the first place).

### 1.4 P4 — who reads what, and the acceptance line this corrects

<!-- recorded-under-this-name: the sentence below quotes `story:trace-ir-reader`'s acceptance
     verbatim, and that artifact is the planning store's, mutated only through the CLI. -->
The story's acceptance says *"`aep trace check` over the projected document"*. **That is not a
thing `aep` 0.42.0 can do, and this page says so rather than shipping a document nobody reads.**

Established by reading `~/beyond10x/aep/crates/trace-spec/src/reader.rs` at `e27c84b`: `aep observe
trace check --transcript <file>` dispatches on the first non-blank line's `format` tag and has exactly
two readers — `metaharness.event/1` and, as the fallback, `claude-code/stream-json`. There is no
`trace-ir/1` reader, and there cannot easily be one: `trace_domain::ir::TraceIr` is `Serialize`
only, its identity fields are `&'static str`, and no trace-ir schema is published. That is design
§ 4.4's **Q9**, still open.

So the two consumers are split, and the split is the decision:

| consumer | reads | why |
|---|---|---|
| `aep observe trace check` / `aep observe trace inspect` | the **`metaharness.event/1` stream** | it already has that reader, and it builds its own IR from it |
| the viewer (§ 2), and `beyond10x/bench` | the **projected `trace-ir/1` document** | it needs an aligned, positional, digest-named form and must not re-implement an adapter |

The acceptance is therefore met in the form the consumer supports: `aep observe trace check` is run over
the two recorded runs' event streams — the same bytes `project` reads — and must report **no `unk`
verdict row** for the kinds the driver emits. A row that is `unk` there means *aep's own
event-stream adapter did not understand something metaharness wrote*, which is exactly the failure
this acceptance exists to catch, and it is caught whichever of the two forms is handed over.

**The remaining half of Q9 is named and not hidden:** until `trace-domain` publishes a reader,
nothing outside this repository can read the projected document back into an IR, and the
cross-check of § 4.4 is asserted by comparing **censuses** (family counts, per-family) rather than
by comparing two deserialized values.

### 1.5 Where a metaharness fact goes that the IR has no field for

Under one top-level `metaharness` object, never scattered into IR nodes. `session.started` carries
`withheld`, `available_operations` and the whole `hermetic` attestation; the IR's `SessionStart`
has no room for any of them, and design § 4.1's note on `withheld` is explicit that *"projecting it
is a change the repository that owns the IR makes first."* Putting them in a namespaced sibling
keeps that true — the IR half of the document stays exactly the IR's shape, and a reader that only
knows `trace-ir/1` reads it unchanged.

---

## 2. The viewer — how two runs are aligned

### 2.1 What the viewer is for

A person compares two runs of one task. The question is never *which is better* — `aep drive
eval matrix` counts and nobody here scores (epic, § Out of Scope) — it is **where did these two stop
doing the same thing**. Everything below serves that one question.

### 2.2 V1 — the alignment rule

> **Align by workflow state entry when both runs are driven; otherwise align by tool-call index.**

This is the epic's declared default, and it is adopted rather than replaced. What it means
mechanically:

* A run is **driven** when its stream contains at least one `step.entered`. That is the only
  test: a driven run is one an embedder was stepping, and `step.entered` is how an embedder says
  so. Nothing infers it from an adapter name, a flag, or a directory (invariant 3).
* **Both driven** → the alignment key is the ordered sequence of `StepRef`s taken from
  `step.entered`. Two rows align when their step keys are equal. Everything a run did *inside* a
  step is nested under that step's row, in stream order, and is not itself aligned — because two
  runs of one state are two different pieces of reasoning, and pretending call 3 of one is call 3
  of the other is the false precision this rule exists to avoid.
* **Otherwise** → the alignment key is the **tool-call index**: the 0-based position of the
  `tool.requested` event within its own run. Call *n* aligns with call *n*. Text, thinking and
  injections attach to the call that follows them, which is the same attribution
  `aep observe trace inspect`'s `gen` split already uses, so two tools do not disagree about which turn
  produced a call.

**Not by time, and the reason is not taste.** metaharness reads no clock: a timestamp exists only
when the vendor recorded one, and the first events of a real run carry none at all
(`trace-domain::ir`, `TraceEvent::timestamp`). An alignment keyed on time would be undefined for
the opening of every run it was asked about, and would silently reorder when two runs were served
at different speeds — which is the one difference a reader must be able to *see* rather than have
folded away.

**Not by content similarity.** A fuzzy match makes the viewer's output depend on a threshold
nobody can cite, and two people reading the same page would be reading two different alignments.

### 2.3 V2 — a gap is a row

A step or a call present in one run and absent in the other is rendered as a **gap row**: the
column that has it shows it, the column that does not shows an explicit gap marker, and the row
counts as a **divergence**. It is never skipped and the two columns are never independently
scrolled past each other.

The first divergence in the page is marked as such, because *where they stopped agreeing* is the
answer to the question in § 2.1 and burying it in row 40 of a table makes the reader find it by
eye.

### 2.4 V3 — what the page is

One file. No server, no network fetch, no external stylesheet, no framework, no build step. The
IR documents are embedded in the page as JSON at generation time, so the file works from a
`file://` URL and from inside whatever the bench site embeds it in.

**JS minimal and inline**, and it does exactly two things: expand/collapse a row, and jump to the
next divergence. Alignment is computed at **generation** time in Rust, not in the browser — a
viewer that computed its own alignment would be a second implementation of § 2.2, and the two
would drift.

**The bytes are deterministic.** No timestamp, no random id, no "generated at" footer. The page is
a fixture the gate can compare against, which is what makes "the two recorded runs render" a check
rather than a screenshot somebody looked at once.

### 2.5 What each column shows, per row

State entries; tool calls with the decision that was taken on them (`tool.decided`'s decision,
decider and seam) and the refusals among them; the per-step duration **derived from recorded
timestamps and left absent where either end has none**; and the cumulative cost, taken from
`session.ended.total_cost_usd` and from `usage`, never multiplied out here (design § 4.1, D4 — a
cost metaharness computed is a number nobody billed).

---

## 3. `--plugin` — a marketplace plugin in the scratch home

### 3.1 The tension, stated

The hermetic floor is *"no ambient plugins"* (§ 8.1 H1a). The bench needs the opposite **on
purpose**: a named third-party plugin, present, so two stacks can be compared through one
instrument. The resolution is the one H1a was always written for — H1a does not say *no plugins*,
it says *plugins are exactly the declared set*. `--plugin` adds to the declared set. It is opt-in,
it is named, it is pinned, and the attestation lists it.

### 3.2 G1 — the spelling, and the refusal

```
--plugin <marketplace-repo>@<name>@<version-or-commit>
```

Three segments, split on the **last two** `@`, because a GitHub repo spelling never contains one
and a name never does. Examples: `bdfinst/agentic-dev-team@dev-team@1.4.0`,
`beyond10x/agentplugins@aep-planning@21147b7667dfaefcfa45a094e9542891b1783541`.

**Two segments is a refusal, by name, before anything is spawned.** `--plugin repo@name` names a
plugin whose contents can change between two runs that both claim to have used it, which makes the
bench's two arms incomparable and the run unreproducible. The refusal says which segment is
missing and what a pin looks like. It is not a warning: a control that reports and proceeds has
already stopped controlling (design § 7.1).

### 3.3 G2 — resolution is local, and a run reaches no network

**A run installs nothing over the network.** `claude plugin marketplace add` and `claude plugin
install` both fetch, and a launch that fetched would make the run depend on a remote repository's
state at launch time — unreproducible, unpinnable through that CLI (established in
`docs/research/2026-09-03-claude-plugin-headless-install.md`: neither verb takes a ref, a tag or a
commit), and a network reach inside the one boundary § 8 exists to draw.

Instead, `--plugin` **resolves against a marketplace the operator has already fetched**, and copies
from it:

1. the operator's real config home (`CLAUDE_CONFIG_DIR`, else `~/.claude`) is read for
   `plugins/known_marketplaces.json`, and the marketplace whose `source.repo` equals the given
   repo is found — this is the only thing the repo spelling is used for;
2. `plugins/installed_plugins.json` is read for `<name>@<marketplace>`, and the entry whose
   `version` **or** `gitCommitSha` equals the given pin is found;
3. that entry's `installPath` is the source tree; it is walked, digested with
   `metaharness_protocol::tree_digest`, and copied.

Every failure along that path is a refusal that names the `claude plugin marketplace add …` and
`claude plugin install …` the operator has to run **once, deliberately, outside a run** to make the
plugin resolvable. That is where the network reach belongs: in an operator's hands, not in a
governed run's launch.

The digest is computed over the operator's tree **before** the copy, exactly as `--plugin-dir`
does, because that is what the attestation is a claim about.

### 3.4 G3 — placement, and how strong the claim is

The plugin is placed **inside the scratch config home**, in the layout a real one uses:

```
<config home>/plugins/known_marketplaces.json     the marketplace, with installLocation in scratch
<config home>/plugins/installed_plugins.json      {"version":2,"plugins":{"<name>@<mkt>":[…]}}
<config home>/plugins/marketplaces/<mkt>/         the marketplace manifest, minimal
<config home>/plugins/cache/<mkt>/<name>/<ver>/   the plugin tree itself
```

**Read, not driven, and labelled as such** (invariant 4). The layout is what the operator's own
`~/.claude` contains under Claude Code 2.1.258, recorded verbatim in the research note. What has
**not** been established is that a session launched with `CLAUDE_CONFIG_DIR` pointing at a home
metaharness assembled loads that plugin and reports it in the opening record — that needs a paid
run, and it is the note's open probe.

So `InstalledPlugin::loaded_by` says exactly that, per install, and the two mechanisms in this
repository are now distinguishable by reading one field:

* `--plugin-dir` → *"`--plugin-dir <path>` in the argv: the vendor's own flag… (verified)"*
* `--plugin` → *"placed in the scratch config home's plugin registry… (read from a real config
  home at 2.1.258; **not driven** — whether the session loads it is asserted from the opening
  record's plugin list, H1a, never from this row)"*

**`--plugin-dir` is not also passed for a `--plugin` install.** Two mechanisms loading one plugin
would report it twice, under two different `source` strings, and H1a's *exactly the declared set*
would have to be widened to accommodate a duplicate metaharness created itself.

### 3.5 G4 — what the attestation says

`HermeticAttestation::installed_plugins` already exists and is already *"always present, empty
when there is none"*. `--plugin` entries join it, and nothing about that field changes.

What is added is the **rendering**: a run's hermetic report prints one `plugins:` line, and it
prints `plugins: none` when the list is empty. A report that printed nothing for a plugin-less run
would make *this run installed nothing* and *this build does not report installations* the same
bytes, which is the reading § 8.1 refuses everywhere else.

### 3.6 The conformance vector

One free C1 vector records the launch shape with a plugin present: the plan's config-home
placement list, the attested `loaded_by`, the pin, and the absence of `--plugin-dir` in the argv.
No model, no network, no credential — and no paid run, which is what makes it a gate.

---

## 4. What this page does **not** decide

* **Scoring.** The viewer aligns and shows. Nothing here ranks two runs (epic, § Out of Scope).
* **A `trace-ir/1` reader in `aep`.** That is Q9 and it is that repository's change.
* **Codex, Pi or OpenCode plugins.** `--plugin` is Claude-only and is refused by name on the other
  kinds, rather than accepted and ignored.
* **Whether the vendor loads a config-home-placed plugin.** Open probe, § 3.4.

---

## 5. Register

| id | question | state |
|---|---|---|
| **Q9** | a machine-readable `trace-ir/1` document a third party can read back | **half closed** — the document is written and tagged; no reader exists outside this repository, so the cross-check compares censuses |
| **Q19** | does a session launched against a metaharness-assembled config home load a marketplace plugin placed in it, and what `source` does the opening record report | **open** — needs one paid run; research note § *Open probes* |
| **Q20** | can a marketplace plugin be installed headlessly **at a pin** through `claude plugin install` | **open, and currently answered no** — neither `marketplace add` nor `install` takes a ref at 2.1.258 |
