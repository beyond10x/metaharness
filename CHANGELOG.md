# Changelog

What changed. The design document carries *why*; where code and design disagreed, the design
was amended and the amendment is named here.

## [0.4.0] — 2026-08-31

### Added

- **Sandbox inversion now has its first executable contract layer.** A harness-neutral sealed
  process-envelope request names runtime roots, writable paths, staged executable digests,
  constructed environment, credential channel, network reach and bounds. An injected
  `ProcessEnvelope` port returns measured child-boundary facts; exact comparisons preserve
  withheld evidence as unknown, and strict mode kills and refuses a child whose measurement is
  absent or wider than requested. Scripted vectors prove stable sealing, matching evidence,
  named write-surface mismatch and absence without spawning a vendor process.
- **The engineering-protocols eval preflight now adversarially exercises the protected-store
  rule without a model.** It requires whole-file replacement inside the planning store to be
  refused, targeted edit there to be admitted, and whole-file replacement outside it to remain
  admitted under the ordered catch-all. Paid arms may still abstain after reading the boundary;
  the free probe guarantees the refusal mechanism itself was exercised first.
- **The b10x contract now checks seven free vectors.** Provider-emulated budget exhaustion and
  operator cancellation join the launch, replay, version, unpublished-tool and approval-denial
  checks. Both are selected, unedited lines from deterministic local `0.9.1 --json` captures.

### Fixed

- **Direct-provider H6 provenance is no longer reported as unavailable.** H6 now names exact
  credential carriage: vendor adapters keep the one-file-per-spawn rule, while b10x attests that
  no operator login is copied and its only source is caller-named or none. The row remains
  advisory because neither mechanism is visible in a provider record.
- **The public b10x documentation now names the released 0.9.1 pin and all seven contract checks.**
  It no longer reports the original 0.8.0/three-vector launch state after the adapter was re-pinned.

## [0.3.0] — 2026-08-31

### Changed

- **The b10x adapter is pinned to released harness 0.9.1 at revision `b626120`.** Its golden loop
  was recaptured from that exact binary against the deterministic local Responses endpoint. The
  opening record now carries the loop's credential class (`api-key:environment` in the capture)
  instead of the adapter literal `named`, and the launch supplies the exact binary's version,
  model and cwd to the observer rather than leaving those launch facts stale.
- **The b10x contract now checks five free vectors.** Two `provider_emulated` enforcement excerpts
  pin the released loop's unpublished-tool refusal and approval denial, including the failed
  outcome returned to the model. They are selected unedited lines from real `0.9.1 --json`
  captures; timestamps and unrelated context digests are deliberately not retained.
- **Pi 0.80.3 and OpenCode 1.4.7 now have contract-first adapter crates without being advertised as
  runnable kinds.** Each pins the installed version, records a byte-exact scratch-home launch and
  fills the launch/version obligations. Their model-backed JSON wire and blocking per-call seam
  remain explicit contract gaps; runtime dispatch, doctor and capabilities therefore do not name
  them yet.
- **Sandbox inversion has a boundary design.** Metaharness owns a sealed process-envelope policy
  and consumes measured facts through an injected port; substrate remains the execution mechanism
  and is not imported. Credential bytes stay outside the envelope, network reaches only a declared
  model proxy, and strict hermetic claims require measured request/result agreement.

### Fixed

- **Eval repository and harness paths are resolved before a child changes directory.** Relative
  `--ep-repo`, `--harness-repo` and `--b10x-binary` values now become canonical paths during
  preflight, so a later scratch-workspace cwd cannot reinterpret an operator-supplied checkout.
- **`metaharness run b10x --hermetic strict --audit` can now pass on evidence the direct-provider
  launch actually supplies.** The builder queries the resolved executable before launch and makes
  `--strict-version` a real refusal; records the scratch config home, constructed environment,
  explicit-or-absent hooks and clean scratch ancestry as imposed controls; and names operator cwd
  and the absence of an operator login as unavailable controls rather than pretending they were
  imposed. The constructed toolchain environment carries `RUSTUP_HOME` and `CARGO_HOME`, never
  `HOME`, so project-memory discovery is not reopened to make Rust available.
- **The hermetic audit no longer treats inapplicable vendor settings as missing evidence on a
  direct-provider loop.** With no declared plugin directory, the absence of an ambient plugin
  registry satisfies H1a; a loop with no output-style setting satisfies H1b. Vendor adapters keep
  their existing evidence requirements.

## [0.2.2] — 2026-08-31

### Added

- **`conformance b10x` is now a real, free adapter contract.** It records the executable, argv and
  complete base child environment (C1), replays a captured `b10x-harness 0.8.0 --json` loop record
  byte for byte (C2), and reconciles the capture's version banner with the adapter pin (CT-3). The
  capture is `provider_emulated` evidence from harness's deterministic local Responses endpoint;
  no live provider, credential or model cost is involved. Its `contract_result` is pinned as bytes
  with `checked: 3`, and b10x now participates in the same symmetry tests as Claude and Codex. The
  hook-input obligation stays a reasoned N/A because this adapter observes and has no metaharness
  decision seam.

### Fixed

- **A b10x run no longer inherits the operator's default permission profile.** Harness 0.8.0 added
  `$XDG_CONFIG_HOME/b10x/harness.toml`; toolchain launches still passed `HOME`, so the supposedly
  constructed environment could silently load `[default]`. The child now gets a scratch
  `XDG_CONFIG_HOME`, pinned by the C1 vector. Its opening hermetic attestation also states
  `decisions: observe` instead of inheriting the generic `frame` default.
- **The delivered b10x observation mode is launchable without inventing a decision channel.** The
  generic capability check no longer requires `tool.decide` when an adapter explicitly delivers
  observe with no command seam. b10x now refuses launches in the misleading `frame` default or
  `ask` mode by name; callers must say `--decisions observe`, matching every emitted call's
  `decision_required: false` and `seam: none`.

## [0.2.1] — 2026-08-31

### Changed

- **The b10x adapter and owned tool surface are pinned to harness 0.8.0 at released commit
  `45fdccb`.** The adapter's version claim moves with the git dependencies. The
  engineering-protocols eval now requires both that exact checkout revision and the installed
  `b10x-harness 0.8.0` banner; it no longer treats filesystem modification time as binary
  provenance.

- **The Claude golden event preserves the captured per-model cost's exact decimal spelling.**
  Harness 0.8.0's wire dependency enables `serde_json` arbitrary-precision numbers in the unified
  binary, so the flattened event retains `0.0010919999999999999` instead of shortening the same
  `f64` to `0.001092`. The repository-owned golden generator re-pinned that one wire-visible
  number; no vendor record was recaptured and its numeric value is unchanged.

### Fixed

- **The native eval stops where its map says a person takes over.** Walk 11's apparent
  budget/turn stop was downstream damage: the old harness treated `decompose`'s `operator` step
  as another model session, ran past the intended terminus and spent in `establish_verifiers`.
  Harness 0.8.0 makes the operator handoff an exit-0 `flow-paused` boundary. The eval's free
  confined preflight now repeats that proof against a closed endpoint and requires exactly one
  terminal pause with no tool, approval, hook or completion event, so the closure needs no paid
  rerun.

## [0.2.0] — 2026-08-31

### Added

- **The engineering-protocols comparison has one Rust runner and a mandatory free preflight.**
  `engineering-protocols-eval` owns the shared fixture for the native, b10x-driven and
  Claude-driven shapes, holds paid runs to one at a time, and crosses the paid boundary only when
  `--spend`, `METAHARNESS_LIVE=1` and exact USD caps agree. The copied protocol tree now lives at
  `ws_project/.engineering/protocols`, inside substrate's mounted workspace. Before any model can
  start, a command-only confined workflow uses the staged driver to create a
  `decision-blocker`; the runner requires its initial state to be `open`, so the permissive
  fallback to `draft` that walk 10 exposed is a free, deterministic failure. The two executable
  Bash runners are retired. The Claude fixture also carries the source-built driver inside the
  workspace and its derived prompts name that path, while b10x receives the same build through its
  staged-driver mount; neither arm can silently resolve an older ambient `protocol` install.

- **harness is pinned by git revision.** The four dependencies on `b10x-harness-tools`,
  `b10x-harness-wire` and `b10x-harness-loop` were `path = "../../../harness/crates/*"` — the
  sibling checkout, whatever it held; `--locked` locked nothing of it, and every green gate since
  the adapter was written was green against an unnamed tree. They are now
  `git = "https://github.com/beyond10x/harness"`, `rev = 3467bf0` (harness `main` on 2026-08-29:
  the loop that reads skills and agents, which this adapter's `--plugin-dir` and the
  `skills`/`agents` fields on `session.started` need). `.cargo/config.toml` sets
  `net.git-fetch-with-cli` so the private repository is fetched with the system git's credential.
  Invariant 10 holds it. Gate at the pin: 26 suites, 502 passed, exit 0.

- **Skills and named agents reach the native arm, and both arms take the same plugin.** The b10x
  adapter carries `RunSpec.plugin_dir` through to `--plugin-dir`, and the eval passes it to both
  arms. It was withheld from the native one by name, on the grounds that a plugin is a vendor
  mechanism and that loop had none — true until the loop learned to read the skills and agents
  halves of the vendor's on-disk format (harness `a405f46`). Withholding it after that would be
  handing one arm its instructions at session start and making the other discover them, which is a
  difference between the columns that has nothing to do with the harnesses.

  The seam reads `skills` and `agents` from the record rather than asserting them. `skills` was a
  hardcoded `Some(vec![])`, correct while the harness had no skills mechanism and wrong the moment
  it had one: it would have claimed *none were offered* about a run that was offered several.
  `mcp_servers` stays `[]`, because that one is still a standing fact about a harness with no MCP
  client rather than a guess about a run.

- **The native arm's four enforcement tiers reach the loop, and its refusals are readable.** The
  b10x column measured one tier of four: publication was live, and the ceiling, the approver and
  the content hook were declared nowhere the loop could see. A driven run measured with three
  tiers off is not a comparison.
  - `RunSpec.hooks` → `--hooks` carries the operator's content rule. Named and never discovered: a
    hook found in a workspace would be a program the repository runs on this machine.
  - `RunSpec.driver` → `--driver` carries a program the confined run must be able to start. A
    different question from `allow_program`, and the difference is the whole point — the
    allow-list says what a `run` may *name*, this says what the sandbox *contains*. A path on the
    allow-list the sandbox does not hold is admitted and then dies at `ENOENT`, which is what left
    a driven run hand-writing a planning store because the CLI it was told to use was not there.
  - `--yes` is replaced by `--approve-up-to high`. `--yes` approves the destructive class and, by
    the loop's own rule, does not combine with a ceiling — so the ceiling was unreachable through
    metaharness and the arm approved more than the comparison asked for.

- **`hook-ran` and `approval-resolved` are readable instead of opaque.** A hook's block becomes
  `warning{code: "hook-refused"}` and a failure `hook-failed`; an approver's denial becomes
  `approval-denied`. Both were silent or indistinguishable before: a hook refusal crossed as
  `Opaque`, and a denied call arrived as `tool.result{is_error: true}` with `content: null` —
  identical to a tool that ran and broke, which is the opposite finding.

  **Warnings and not `tool.decided`**, for invariant 9's reason: this adapter runs in observe mode
  and decides nothing. Every `DecidedBy` the protocol has — `Embedder`, `Frame`, `Deadline`,
  `Adapter`, observe — names a metaharness-side decider, and neither the loop's hook programs nor
  its approver is one of them. `ApprovalResolved` carries no reason on the wire, so the warning
  names the `call_id` a reader joins to the `tool.requested` rather than inventing a cause.

- **`mcp_servers` and `skills` say `[]` where the comment already said they had none.** That block
  read "Absent because the loop has none of these, not because nobody looked" and then wrote the
  value that means *nobody looked*. `b10x-harness` has no MCP client at all — its README states
  the refusal and the reason — so that is knowable without observing anything, the same class of
  standing fact `credential_source: named` already states. `agents`, `plugins` and
  `slash_commands` stay `null` deliberately: this adapter has not established those, and asserting
  `[]` on an unchecked belief is the defect being fixed, not a smaller version of it.

### Fixed

- **The driven scorer gates on the protected outcome, not on whether the model forced a refusal.**
  Both live arms read their declared scope and abstained from the forbidden surface call. The store
  stayed valid and unchanged, which is the shared outcome; the mechanism census remains visible as
  an advisory zero. Requiring a refusal would make a better-informed run fail for obeying its
  boundary, the same defect the native step correction below already removed.

- **The native workflow eval scores its protected-store outcome rather than requiring a forbidden
  call.** Walk 9 showed the per-step scope working and disclosed to the model: Opus declined the
  forbidden edit on all four attempts, the store remained valid with no forged revision, and the
  workflow nevertheless failed because its prompt demanded a runtime refusal. The scope remains
  declared and enforced. The shared step now passes when neither forbidden effect happened,
  whether the record contains a refusal or an informed abstention; the following command step still
  validates the store independently. The advisory refusal rows continue to distinguish the two.
  A dated Opus 5 list-price card now makes the eval's existing $5 ceiling enforceable and leaves
  the source and cache-rate choice in the run record; without a card the runner still refuses to
  pretend it knows a cost.

- **The native arm's eval map declares where a step may write, its rows read the native vocabulary,
  and its census reads a program refusal.** Three ways the b10x column was reporting something
  other than what happened.

  `driven.steps.yaml` — both `llm` steps now carry `scope:`: `.engineering/**` `denied` (the store,
  the run's own records, and the project and task documents, none of which are a step's to write)
  with a `**` `allowed` catch-all. That is the subject's design § 6 **O2**, and it turns the
  store-integrity column from *not observable* into *the write was refused by the tool, on the
  path, before it ran*.

  **The catch-all is `allowed` because a paid run proved `denied` starves the honest step.**
  2026-08-29: five scratch-file writes refused, so the model fell through to
  `artifact body --from -` with nothing on stdin, the store took an empty body at revision 2, the
  validator step exited 1 on every attempt, and the run spent its whole budget in `receive` — so
  `specify`, the state the denial column is *scored* on, never ran at all. A scratch project is a
  place where a step may write a scratch file; the scope's job is the store.

  The driven eval's surface-denial census counts `warning{code: program-refused}` beside
  `unpublished-tool`. A program outside the declared set is refused inside the `run` tool and
  reached the wire only as a failed result with `content: null`, so that column read 0 whatever
  happened. The store walk now covers the whole store rather than one expected path, and
  `artifact validate --format json` must report `pre_provider` 0 — a well-formed hand-written
  document is invisible to a drift check and visible to that count.

  The expectation twins — every tool row unions `tools:` with `operations:` so it decides on both
  arms rather than on the vendor's spelling alone; `shell` joins those unions, because that is what
  the native arm writes in a `run` call and a union whose operation half matches nothing is one
  witness wearing two names; `the-frontmatter-edit-came-back-refused` names `file.edit` beside
  `file.write`, so it is decidable whichever the model reached for; and
  `the-creating-call-succeeded` reads the *outcome* of the creating call
  (`tool.error_rate` ≤ 0.99), because the row beside it counts the call and says nothing about
  whether it worked — which is how that row went green over a run that created nothing.
  `expectations.trace.yaml` stays byte-identical to engineering-protocols'
  `conformance/trace/expectations.trace.yaml`, which is the point of it.

  Gate at the time: 26 suites, 502 passed.

- **`conformance <kind>` no longer blames a missing crate for a missing vector suite.** The
  refusal read "the adapter crate is a later milestone", which was the only way to reach it when
  it was written. It is now also what an adapter that *exists* and has no free vectors yet raises
  — `b10x`, whose crate drives the driven eval — so the message asserted an absent crate about one
  that is present, and a reader chasing it looked for the wrong thing. It names what is absent and
  points at `contract`, which reports the obligations row by row.

- **`session.started` says what the run asked for and the machine would not admit** — `withheld`,
  a list of `{tool, reason}`, and design amendment **a12**. Two fields already answered two
  questions: `offered_tools` is what the model was *offered*, `available_operations` is what the
  run could *do*. Both describe a set that is **present**, so a tool a publication gate refused to
  admit is missing from each of them in exactly the way a tool nobody wanted is — and the two runs
  produce an identical record. On 2026-08-29 that cost weeks: a driven session whose only legal
  route was running a program was published a six-entry catalogue instead of seven, with no error,
  no warning and no fact anywhere in the record; it hand-wrote the files instead and the failure was
  read as a **model** failure. It was the machine's. What was missing was never a refusal — putting
  the tool back in front of the model is the thing publication exists to prevent — it was the
  **fact**, with the predicate that decided, in the machine's own words.
  `WithheldTool { tool, reason }` is this crate's own type and not the harness's, on invariant 1:
  `metaharness-protocol` imports nothing but clap, serde, `serde_json` and sha2, so the wire is the
  contract between the two repositories rather than a shared Rust type.
  **`null` is *the harness did not say*, and never *nothing was withheld*** (invariant 3, a4's
  rule): a producer that writes the field states `[]` for a run that got everything it asked for,
  and one that has never heard of it states nothing at all. It serializes as an explicit `null`
  rather than being skipped, on § 2.1's rule — a missing key is precisely the silence the field
  exists to end. Every vendor adapter states `None` with the reason in one line: the vendor does not
  say. **The b10x adapter reads silence as silence**, because the observed version cannot decide
  which silence it is — the field is under `b10x-harness`'s `[Unreleased]` and that binary answered
  `0.1.0` both before and after it landed, which is the failure `emitted_flags` exists for. The
  harness's own converter answers `[]` instead, and that is not a disagreement: it stamps
  `harness_version` with its *own* `CARGO_PKG_VERSION`, so it has already claimed the record as
  that build's.

- **`--audit` prints what the machine would not admit, beside the census that cannot say it** —
  one line after `decision census:`, in three shapes because they are three different facts:
  `withheld: <tool> (<reason>); …` when the opening record named any, `withheld: none declared`
  when the harness stated it withheld nothing, and `withheld: not stated by the harness` when it
  said nothing at all. The census is no help here and neither are the tool lists: a tool that was
  never admitted was never put in front of the model, so no call was refused for it and
  `denied=0` is the same `0` a run that got everything prints — and it is missing from
  `offered_tools` and `available_operations` exactly as a tool nobody wanted is. So a reader of an
  audit report could not tell *nothing was refused* from *the tool was never there*, which is the
  reading that cost weeks on 2026-08-29. `AuditReport` gains the field and `withheld_tools` reads
  it off the run's own opening record; a stream with no opening record is *did not say* rather
  than an empty list, on the same rule (invariant 3, a4). Design § 9.4 amended.
  **It is not in `required_ir_fields`, and the reason is in `projection.rs`:** `trace-ir/1`'s
  `SessionStart` has no `withheld` field, so the fact crosses this wire and stops at the IR
  boundary. Listing it would claim a family was filled from a field it cannot receive, and the
  check could never fail anyway — the key is always serialized. Projecting it is a change the
  repository that owns the IR makes first.

## [0.1.0] — 2026-08-24

First tagged release. The entries below cover everything since the crate was established; the
commit history and `docs/design/` carry the full reasoning per change.

### Verified

- **Q19 is answered: Codex surfaces an injected plugin's skills from `$CODEX_HOME/plugins/<name>`
  — observed once, on 0.144.0.** The placement crossing #4 picks on Codex was chosen from strings
  in the binary and driven by nobody, which left the treated arm of an evaluation unclaimable on
  that vendor. A directed live probe on 2026-08-23 (run `codex-2139643`) settled it:
  `metaharness run codex --hermetic --decisions observe --plugin-dir integrations/codex` copied the
  plugin to the placement (digest `154857db…`) and asked the model to answer **from its runtime
  context only, using no tools**. It answered *"Available skills catalog — `## Skills`"*, and the
  run made **zero tool calls** — the census read `0/0/0/0` and no `tool.requested` was emitted at
  all — so the catalog could not have been read off disk. The vendor put the injected plugin's
  skills into the model's context, with **no `[marketplaces]` table and no `codex plugin add`**
  behind it; the copy alone was enough, which is what the launch had bet on.
  **Two limits travel with the claim, and they are carried in the record rather than in a footnote
  here.** The child was codex **0.144.0** — the binary this machine's constructed `PATH` resolves —
  against a pin of 0.145.0, the same caveat R2.4's allow-half proof carries (Q18/a8): a driven fact
  about that binary, an inference about the pin. And `session.started.plugins` was still `null`:
  **the vendor's opening record enumerates no plugins, so H1a still reads `unk` on Codex.** What was
  observed is the plugin's *content* reaching the model — which is exactly what an evaluation's
  treated arm needs — and not the vendor stating what it loaded, which is what H1a asks for. The two
  are different claims and the attestation's `loaded_by` now says which one it is making.
  **Nothing more is claimed:** not that the surfaced skill is *used* well, and nothing about
  0.145.0. Every label moved together — `PLUGIN_HOME`'s doc, the per-install `loaded_by`, H1a's
  `how` text, design § 8.3 and the Q19 register row, `status.mdx` (which left the open-questions
  table the way Q18 did, with the mechanism and the caveat below it) and `harnesses/codex.mdx` —
  and the codex launch test now requires the record to carry the observation **and both limits**,
  so a row that quietly upgraded to a bare *"it loads"* reddens.

### Fixed

- **`session.started` on the b10x arm says what a run can do.** The loop's opening record carries
  the published toolset, and a line the reader could parse is no longer reported as one it could
  not — control plane is not opacity.
- **The b10x child is told where its toolchain is.** Under a constructed environment
  (`env_clear`), a declared toolchain was unreachable until `RUSTUP_HOME` and `HOME` were passed
  by name — and only when a toolchain was declared, so a run that asked for none still inherits
  nothing.
- **The loop's own turn count is read** from the b10x terminal record, so an advisory bound can
  decide a completed run instead of reading `null`.
- **`--cwd` on Codex now gives the child a tree it can actually write to (design amendment a6.1).**
  Amendment a6 is the declaration that trades H7 and H11 **for real work in a real tree** — and on
  this vendor it bought nothing. A paid subscription run on 2026-08-23 (`codex-1982431`,
  `run codex --hermetic --decisions observe --cwd <a real clone>`) spawned, worked, and could not
  write one file: the child reported *"this workspace is mounted read-only"* and the vendor's own
  stream said *"the workspace is read-only, so the planning-store patch was rejected."* The cause
  was that the scratch `config.toml` wrote `sandbox_mode = "read-only"` for **every** run, so the
  vendor's sandbox applied to the operator's own repository exactly as it applied to a scratch
  directory. A trade whose consideration cannot be delivered is not a trade, which is why this is an
  amendment to a6 rather than a bug fix under it.
  **What changed:** `sandbox_mode` is now decided by the cwd declaration —
  `sandbox_mode = "workspace-write"` when the run carries an operator-named `--cwd`, and
  `"read-only"` otherwise, unchanged, for every scratch run. The value is the vendor's own spelling,
  not a guess: `SandboxMode` deserialises `read-only`, `workspace-write` and `danger-full-access`
  (kebab-case, read from the pinned 0.145.0 binary's serde variant list, where the snake-case trio
  beside it belongs to a different type), and the binary's own description of it is *"The sandbox
  permits reading files, and editing files in `cwd` and `writable_roots`. Editing files in other
  directories requires approval."* The child is spawned **in** the named tree, so `cwd` is that tree
  and **no `writable_roots` entry is written** — the grant stops where the declaration stopped.
  Nothing wider goes with it: `--add-dir` stays denied, `danger-full-access` is never written, and
  no `[sandbox_workspace_write]` table is emitted, so this vendor's `network_access` default is
  neither changed nor claimed (it is undriven here, and so is stated nowhere).
  **The grant is visible without diffing a config that no longer exists.** H7's attested-unavailable
  reason now says it in words — *"THE CHILD COULD WRITE TO THAT TREE: the vendor sandbox was widened
  to it for this run — sandbox_mode = "workspace-write" …"* — and the scratch case's imposed row says
  the opposite just as plainly. **`--hermetic strict` still refuses a named-cwd run**: the grant
  changes what the child may *do*, never what the attestation *claims*, so H7 and H11 stay
  unavailable and the strict floor reads them exactly as before.
  Five unit tests on the plan, no new vector: the config carries the grant iff the cwd is
  operator-named, the scratch posture is unmoved, the attestation states the grant, a named-cwd run
  is still not hermetically clean, and a **mutation** — the grant stripped back to `read-only` —
  fails the same check that passes on the real plan, so the check can go red. The six recorded C1
  expectations were regenerated because the config document's comment changed; **no value in them
  moved** (none uses `--cwd`, so all six still record `sandbox_mode = "read-only"`), **no vector was
  added, and `checked` stays 17** — the `contract_result` bytes the other repository reads are
  untouched.

### Changed

- **The `allow` half of Codex's decision wire is driven, and `--decisions observe` on Codex plans
  now (R2.4's paid vector, spent 2026-08-23).** The hook held a real `Bash` call, metaharness
  answered `permissionDecision: allow`, the command ran, and the rollout's own
  `custom_tool_call_output` carried its output — census `allowed: 1, denied: 0`. The mode table
  moves `observe` to delivered, the plan-time refusal lifts with it (they read from one place, and
  the drift test held), and every prose site that said *built, undriven live* moves together:
  `README.md`, `status.mdx`, `harnesses/codex.mdx`, the adapter's module doc. The observation
  carries its own caveat wherever it is stated: the child's `PATH` resolved codex **0.144.0**
  while the pin is 0.145.0 — the two-install warning fired, as it must — so the grant is a driven
  fact about 0.144.0 and an inference about 0.145.0 until one machine holds one install.
- **The claude adapter is re-pinned 2.1.239 → 2.1.240 (protocol amendment a11).** The installed
  binary had moved and the pin had not, so every run reported a disagreement it could do nothing
  about: `doctor claude` read *"OFF the adapter's pin"*, the hermetic floor's **H9** row came back
  `Gap`, and the contract carried a standing `warn C2 golden-version-pair` on stderr beside its
  `--contract` record. The pin moves on evidence rather than on tidiness — a live run on
  2026-08-23 drove 2.1.240 end to end through this adapter's own seam (the session opened,
  streamed and ended, and its opening record reported `claude_code_version` **2.1.240**), and the
  recorded wire the free tier already replays byte for byte,
  `crates/metaharness-claude/fixtures/golden/`, is **that same binary's**. So the pin moved to the
  bytes; **no recorded fixture's bytes moved to the pin.** That is the whole distinction CT-3
  exists to hold: a golden carries its own capture version as a fact about the binary that wrote
  it, the pin is what the adapter is tested against, and `golden-version-pair` is the one place
  the two are compared — which is why claude's warning is gone (the capture was 2.1.240 all
  along and the pin came to it) while codex's stands, since there the two versions are two
  installs and no capture agrees with either pin. The version pair test now reads *"the committed
  golden sample now agrees with the pin and has nothing to warn about"*, off the committed capture
  and never off the machine's installed binary, so it says the same thing on a machine with no
  `claude` on it at all. One emitted byte changed with the pin and it is the one the golden
  record's own table predicts: `provider` `claude 2.1.239` → `claude 2.1.240` in
  `crates/metaharness/fixtures/golden/contract-result-claude.json`, regenerated deliberately
  through `regenerate_the_contract_records` — `checked: 20`, `failed: 0`, `breaking_changes: 0`,
  codex byte-identical. The consumer building against those bytes is reading about a different
  binary now, which is exactly what `provider` is for. **§ 2.7's verification rows are left naming
  2.1.239**, on a8's rule for codex's a7 rows: a dated observation keeps the binary it was read
  from, and what has not been re-read on 2.1.240 is unverified there rather than silently
  inherited. `tests/live.rs` stopped typing the pin out as a literal — a second pin that drifts
  out of step with the first, as this one had — and reads `PINNED_VERSIONS` instead. What is
  **not** done and costs money: a re-capture of the golden wire on 2.1.240 is unnecessary today
  because the committed capture already is 2.1.240; the next one is owed at the next pin move.
  **And the vendor moved again while this was being written** — `2.1.241`, installed
  2026-08-23T14:02, so `doctor claude` reads *"OFF the adapter's pin"* again. The pin deliberately
  does not follow: 2.1.240 is the version this workspace holds bytes for, a pin ahead of its
  evidence would make every row in the adapter a claim about a binary nobody read, and the next
  move is a **capture with a price on it** rather than an edit. That is the pin working, not the
  re-pin failing — `doctor` is supposed to say so, and `conformance claude` is green and silent
  because it compares the committed capture with the pin and never the machine.

### Added

- **A third kind: `b10x` — the adapter for a loop we own, which observes and decides nothing.**
  `metaharness run b10x` launches the b10x harness under `Seam::None`: there is no hook and no
  control request, because the published toolset *is* the policy. Confinement reaches the arm
  (`--substrate-embedded`, `--cgroup-root`, `--toolchain`, `--allow-program`), so it can build and
  test instead of only reading, and the adapter publishes its rendering table rather than making
  every consumer learn its tool names.
- **A b10x run carries its write scope and its preloaded context**: `--write-scope
  <glob>=<allowed|partial-only|denied>` (ordered, first match wins), `--context <file>`, and
  `--scope-announce stated|silent` on `RunSpec` and the CLI. On any other kind they are refused
  (`ScopeUnsupported`): a vendor arm's scope travels as `Frame.subjects`, sealed, and a flag would
  be a second unsealed copy of it.
- **A step says where its operations may act.** `Frame.subjects` (`SubjectScope`: ordered rules,
  first match wins, sealed into the frame digest) sits beside `Frame.operations`, and the seam
  refuses a call whose subject falls outside its scope — with a reason that names the path, the
  refused class, and the way in.
- **`tool.requested` says what a call touched, beside what it was**: neutral subjects
  (`file:<path>`, `proc:<program>`) resolved through the run's published rendering, on every arm.
- **The owned tool surface resolves to operations** (`metaharness-tools`): `tool_search`,
  `tool_describe` and `tool_invoke` are read for the entry inside the call, so an owned-surface
  run's record names `file.write` rather than a verb — and a `native` run's invented `tool_invoke`
  resolves to nothing, because that run had no such tool.
- **A decision mode that steers nothing and records everything — `--decisions observe` (R2.5,
  design amendment a10).** The three-arm evaluation program's first design constant is *"the
  instrument is constant across arms; only the treatment varies"*: arms a and b measure a harness
  nobody is steering, and they have to be measured by the same instrument that measures arm c, or
  the comparison is between two tools rather than between three treatments. So observe mode is
  **not** "run without a seam". The `PreToolUse` hook is installed exactly as it always is, every
  call arrives at it, metaharness answers `allow` down the same channel a `deny` would go, and the
  call leaves a `tool.decided { decision: "allow", decided_by: "observe" }` — a decider of its own
  rather than `adapter`, because an adapter allow is a judgement about one call and this is a
  run-wide posture that judged none. A census that folded them together would report a capture run
  as a run whose policy happened to permit everything.
  The mode is **named in three places a consumer reads**: `capabilities <kind>` publishes a
  `decision_modes` table beside the tier table (§ 8.4 O6's rule — a posture that only exists inside
  a run cannot be asserted on before one), the launch attestation carries `decisions`, and that
  attestation is what reaches `session.started`. The attestation field is not redundant with the
  events: a run whose model called no tool emits no `tool.decided` at all, and *"the model never
  called a tool"* and *"metaharness would have allowed anything it called"* are not the same fact.
  **What it costs is carried in the record, not only in the design.** `allow` *grants* on this wire
  — the binary's own *"Hook approved tool use for ${name}, bypassing permission prompt"* (§ 6,
  finding F8) — so an observe run is **more** permissive than a run with no hook at all, and an
  `ambient_inputs` line says exactly that on every observe launch. Two things follow and both are
  enforced rather than documented: a frame beside `observe` is refused by name (`ObserveWithFrame`,
  finding F9 — a frame whose text reaches the model while nothing enforces it makes the model's
  instructions false), and **a run that did not ask for observe mode never gets it** — the default
  is and stays `frame`, and that polarity is asserted at both ends: the claude adapter's
  `c1-observe-mode` vector plans all three modes and checks the attestation names each one, and
  `only_a_run_that_asked_for_observe_mode_is_decided_by_it` drives the same scripted script under
  all three and requires `by=observe` in exactly one.
  **Codex refuses it by name.** Observe mode is the `allow` half of a vendor's decision wire and
  nothing else, and on Codex only the `deny` half has been driven (CX-M2, a7) while the 0.145.0
  binary carries a string suggesting some `permissionDecision` values are refused at `PreToolUse`.
  An allow the vendor discards would be a hook response that decided nothing — indistinguishable,
  on a capture run, from a capture that worked. So the descriptor declares the mode `unverified`
  and `plan_launch` refuses it, naming the milestone that would close it (R2.4); one test drives
  every mode through both adapters and requires the published capability and the plan-time answer
  to agree, so the two cannot drift apart.

- **`--plugin-dir` now installs a plugin and pins it — crossing #4 (R2.6).** The flag existed and
  passed a path through to the vendor. What it did not do is make the plugin a *fact about the
  run*: the eval matrix's arm-b column is a plugin identity, and a directory on the operator's disk
  can change between the launch and the transcript that is supposed to describe it. Now each
  declared directory is **read, digested, and copied into the run's own scratch tree** before the
  child starts, and the vendor is pointed at the **copy** — the same argument H10 makes for the
  copied input tree, and it is why the argv no longer names the operator's directory at all.
  **The plan is a value.** The copy list (`plugin_installs`: `from`, `to`, `digest`) and the digest
  are on the launch plan, readable before any process exists, exactly as the argv and the child
  environment are (§ 8.4 O7) — asserted by the new `c1-plugin-injection` vector. The digest rule is
  stated in full in `metaharness-protocol` so two processes can arrive at the same string: one line
  per file, `<relative path> <sha256 of its bytes>`, in byte order of the path, digested. Paths are
  in it because a digest over contents alone would not move when a file was renamed — and renaming
  is how a plugin's `SKILL.md` stops being loaded while every byte in the tree stays the same. The
  mutation clause runs against a **real directory**: edit one byte in one skill file and the tree
  digests differently.
  **The attestation row is never an omitted key.** `hermetic.installed_plugins` carries the name,
  the source, where it landed, the digest, and `loaded_by` — and it is present and `[]` on a run
  that injected nothing, because a key that vanished would make "this run installed nothing" and
  "this build does not report installations" the same bytes.
  **Where a plugin has to sit is a vendor fact, and the two adapters know it to different depths.**
  Claude Code: `--plugin-dir <path>` *"Load a plugin from a directory or .zip for this session
  only"* — verified from `claude --help` on the installed 2.1.240 — so the vendor is told the path
  and the placement is metaharness's to choose. It chooses `<scratch>/plugins/<name>`, deliberately
  **outside** `CLAUDE_CONFIG_DIR`, because the 2.1.240 bundle resolves a `plugins` directory of its
  own under the config home (beside `known_marketplaces.json` and a `marketplaces` cache) and H1a's
  *"exactly the declared set"* must not depend on the vendor's own bookkeeping not adding to it.
  Codex: **there is no such flag**, `codex plugin` installs from marketplace snapshots, and the
  placement `$CODEX_HOME/plugins/<name>` was read from strings in the binary rather than driven —
  so it is a named constant, and the open question is **Q19**, *answered by a live probe the same
  day; see the entry below*. The launch deliberately writes **no** `[marketplaces]` table to go
  with the copy: an unrecognised key under a table this binary reads is dropped in silence and a
  malformed one can fail the config load, which on this vendor is a run with no seam. The previous
  blanket refusal of `--plugin-dir` on codex is lifted — it was hiding a mechanism behind a fact
  about a flag — and what is unknown is now labelled where a reader meets it instead.
  **Refused by name, at plan time, exit 2**: a `--plugin-dir` that is not there, and one that is
  there and holds no file — which is what a typo looks like after somebody "fixed" it by creating
  the directory. Either would otherwise spawn, cost money, install nothing and report an injected
  plugin: the untreated arm wearing the treated arm's label.

- **Counts moved deliberately, and the golden record with them.** `conformance claude` is **24**
  vectors (was 20): three new C1 launch vectors — `c1-observe-mode`, `c1-plugin-injection`,
  `c1-plugin-empty-refusal` — and one new C3 control vector,
  `c3/observe-allows-every-call-and-names-the-mode-that-did`, which asserts the three things any
  one of which would pass while the mode was broken: the decision **reached the child**, the record
  **names the mode**, and the census says **nothing was denied**. `conformance codex` is unchanged
  at **10** — its plugin and decision-mode behaviour is pinned by its own unit tests, because
  adding a C1 vector there would falsify the named `Obligation::Gap` that CT-4 exists to keep
  honest (*"no C1 vector … no `fixtures/c1/`"*), and closing that gap is R2.1's story, not this
  one. `fixtures/golden/contract-result-claude.json` moves `"checked":20` → `"checked":24` through
  the `#[ignore]`d regenerator, and **`engineering-protocols`' committed copy of that record needs
  the same refresh** — it is a consumer building against these bytes.
  The `hermetic` block gaining two keys moved every committed event expectation that carries a
  `session.started`: four fixtures regenerated through the two `#[ignore]`d regenerators, diff read
  line by line, and the only change in any of them is `"decisions":"frame"` and
  `"installed_plugins":[]` appearing inside `hermetic`. Nothing else in the stream moved.
- **Codex has a launch face now, and the record moved to say so: `checked: 10 → 17` (CT-4's named
  gap, closed).** The adapter-contract checklist found on its first run that codex tested no launch
  face at all — no `fixtures/c1/`, its argv and child environment pinned by unit tests and by
  nothing a consumer could read, while its `contract_result` said `checked: 10`, `failed: 0` and
  nothing whatsoever about the face it never tested. It was declared `Obligation::Gap(reason)`
  rather than left absent, precisely so closing it would be a deliberate act. This is that act.
  Six recorded expectations under `crates/metaharness-codex/fixtures/c1/`, declared
  `Obligation::Filled` and checked against the run that produces them: `c1-strict-hermetic`,
  `c1-api-key`, `c1-loopback`, `c1-loopback-subscription-refusal`,
  `c1-unsupported-option-refusal`, `c1-memory-ancestor-refusal`. **The observation is not claude's
  and could not be**: a codex launch vector records `program`, `args`, `env`, **the whole scratch
  `config.toml`** and the credential-copy list, because on this vendor the seam, the model provider
  and the sandbox posture are keys in a file rather than flags on a command line — a vector that
  recorded only the argv would pin nothing about the hook, and an unrecognised key under `[hooks]`
  is dropped *without failing the config load*. The copy list is in it because "how many
  credentials travel" is H6's claim and LP-4's upgrade both. A mutation test proves the fixtures
  can go red on a seam spelled wrong, and `regenerate_the_launch_expectations` (`#[ignore]`d) is
  the deliberate way to move them.
  **The count movement is the point, not a side effect.** `crates/metaharness/fixtures/golden/contract-result-codex.json`
  is regenerated to `{"breaking_changes":0,"checked":17,"consumer":"metaharness.event/1","failed":0,"kind":"contract_result","provider":"codex 0.145.0"}`
  — +6 launch and +1 the allow round trip below — through the `#[ignore]`d
  `regenerate_the_contract_records`, with the diff read: one line, one field, `checked: was 10, is
  now 17`. **Claude's record did not move in this change** (it moved twice the same day in sibling changes: `provider` to `claude 2.1.240` with the re-pin, `checked` to 24 with the observe/plugin vectors). The consumer building
  against these bytes is being told, which is the other half of the rule. The
  `contract_symmetry` test that pinned the gap is replaced by one that pins it **closed** — no
  adapter may answer the launch row with a gap now that both answer it with vectors — and the
  counts on the site and in `fixtures/golden/README.md` moved with it (the claude counts there were
  stale at 17 and are corrected to the 20 the binary really runs).

- **The codex loopback door, API-key half (LP-4) — the child holds no `auth.json` at all.**
  `credentials: loopback` on codex was a flat refusal by name; it is now a door for the login class
  the vendor's own shapes let metaharness route. A codex loopback run starts the same per-run proxy
  Claude Code's does, and the child is pointed at it by a `[model_providers.metaharness_loopback]`
  entry written into the scratch `CODEX_HOME` — `base_url = http://127.0.0.1:<port>/v1`,
  `wire_api = "responses"`, `env_key = "METAHARNESS_LOOPBACK_KEY"` — with the per-run placeholder in
  that variable and nothing else: no credential copy (H6 is attested as the **stronger** row, an
  imposition rather than an unavailable one), `OPENAI_API_KEY` and `CODEX_API_KEY` still scrubbed,
  and a declared `--model-endpoint` becoming the **proxy's upstream** one hop further out rather
  than the child's provider. Exactly one `model_provider` is ever written, so no vendor precedence
  rule decides which brain answered. Proven free and end to end: `builder.rs`'s codex vector starts
  the proxy over a fabricated custody, reads the provider base **out of the config file that was
  really written**, dials it, and the fake upstream sees the custody key with no trace of the
  placeholder; the port is closed by the run's wind-up. `CredentialCustody` is now kind-aware and
  reads codex's `auth.json` as well as Claude Code's `.credentials.json`, classifying the two logins
  the vendor's own `AuthDotJson` shape distinguishes.
  **What is not built, said out loud:** a **ChatGPT-plan** login is refused by name
  (`LoopbackSubscriptionUnverified`, `UNSUPPORTED_CONTROL`), because **V-LP6's subscription half is
  still unanswered** — the binary carries both `chatgpt.com/backend-api/codex` and the custom-provider
  machinery, and a string table cannot say whether a subscription session honours a custom
  `base_url`. Refused, never degraded to the credential-copy path the loopback provider exists to
  replace. And what no free tier can reach: that `codex` itself honours the entry. That is one paid
  turn, outstanding, and the design doc's LP-4 row now reads `built-free-half`.
  Vectors: +2 (`c1-loopback`, `c1-loopback-subscription-refusal`), counted in the 17 above.

- **The allow half of Codex's decision wire: built, free-proven, and still labelled undriven.**
  The status page said *"only the deny path has been driven"* and left the grant half reading as
  unbuilt. It is built: `render_hook_response` emits the envelope the vendor's own
  `PreToolUseHookSpecificOutputWire` names, and a new C3 vector —
  `c3/codex-spawn-an-allow-reaches-the-hook-process-and-the-call-proceeds` — drives it through the
  **real** hook program to a second process holding a call, which then proceeds. Its stub honours
  the allow only when the hook really printed one, so the deny vector gained its **negative half**
  (a denied call leaves no output record) and a mutation test proves that assertion can go red —
  fail-closed polarity in both directions on one wire. Rendering is now pinned by unit tests
  against six literals read verbatim from the pinned 0.145.0 binary: a `deny` always carries a
  non-empty reason, `updatedInput` always travels **with** `permissionDecision: "allow"`, `ask` and
  the legacy `decision:approve` are never emitted, `continue`/`stopReason`/`suppressOutput` are
  never written, and an `abstain` is still no bytes at all.
  **The caveat is kept and sharpened rather than removed.** The same binary carries
  `PreToolUse hook returned unsupported permissionDecision:allow` beside
  `PreToolUse hook returned updatedInput without permissionDecision:allow` — one literal that would
  refuse an allow, one that requires it — and which code path emits which **cannot be told from a
  string table**. So no capability row moved on the strength of a string: the paid vector that
  settles it is written and gated (`an_allowed_shell_call_runs_and_the_codex_record_shows_its_output`,
  `METAHARNESS_LIVE=1`), and every label says *built, undriven live* until it is spent.
  Vectors: +1 C3, counted in the 17 above.

- **The contract record is a golden, and a new adapter's contract is now a checklist (CT-4; the
  adapter-contract milestone table closes).** `engineering-protocols` reads
  `metaharness conformance <kind> --contract` as evidence, and the two repositories share the
  `contract_result` vocabulary and no code — the same gap the frame document has, closed the same
  way. Each adapter's record is committed as the **exact stdout** of a live run
  (`crates/metaharness/fixtures/golden/contract-result-claude.json`, `…-codex.json`, recorded
  2026-08-23 from a CT-1..3 + a9 tree, both exit `0`, provenance in `fixtures/golden/README.md`),
  and `tests/contract_golden.rs` rebuilds it through the real
  `contract_result(kind, &conformance_vectors(kind))` and compares **byte for byte**. Key order is
  pinned with the values, because a consumer reads bytes and nothing in the code asks `serde_json`
  for sorted keys — `preserve_order` turned on anywhere in the workspace would re-order every
  record this binary prints. A failure names the field that moved (`checked: was 11, is now 10`)
  and says the golden is regenerated **deliberately**, through the `#[ignore]`d
  `regenerate_the_contract_records`, never to restore green.
  CT-4 is the other half: `ContractObligations` in `metaharness-protocol` is the one authoring
  shape every adapter fills — a launch vector, a recorded transcript/rollout vector, a recorded
  hook-input vector, a version pair, each answered `Filled(&[ids])` or `Gap(reason)` — with no
  `Default` and no optional field, so a declaration cannot be written without answering every row,
  and `contract_obligations(kind)` does not compile for a third adapter until it has one. Both
  adapters declare through it (`CONTRACT_OBLIGATIONS` per crate) and `tests/contract_symmetry.rs`
  checks each declaration against that adapter's own vectors and its own `provider` string: a named
  vector the run does not produce, produces in another tier or produces red is an unmet obligation,
  and so is a gap with no reason. It found what it exists to find on its first run — **the codex
  adapter has no launch vector at all**, no `fixtures/c1/`, its argv and child environment pinned
  by unit tests and by nothing a consumer can read, while its record said `checked: 10`,
  `failed: 0` and nothing about the face it never tested. That is now a named gap rather than an
  absence, on CT-3's rule that a known gap is never a silent pass. Nothing moved to make room for
  any of this: **no vector was added, no count changed, no emitted byte changed** — 20 claude /
  10 codex, `checked: 20` / `checked: 10`, 0 failed, 0 breaking, as the consumer is reading them
  today. The acceptance clause that named pi/opencode/**flux** is inherited by whichever adapter
  comes next; flux is struck (`docs/ROADMAP.md` § 3, operator: *"i dont want to embed any flux
  related"*).

- **The frame seam is now golden-pinned on both sides of it.** `engineering-protocols` mints the
  `metaharness.frame/1` documents this workspace reads, cannot depend on it (it is public, this is
  not), and therefore tests its minter against a **transcription** of `frame.rs` — a second
  implementation of the digest rule, written out by hand, whose own suite says the risk it cannot
  close is *"the transcription's continued agreement with `frame.rs` … only by the metaharness-side
  replay of these bytes."* That replay is here. The document that repository's driver minted is
  committed byte-identically at
  `crates/metaharness-protocol/fixtures/golden/metaharness-frame-canonical.json` (file sha256
  `ef897a58…`, recorded 2026-08-23, provenance and re-recording procedure in
  `fixtures/golden/README.md`), and `tests/frame_golden.rs` runs it through the real
  `Frame::parse_document`: the bytes are accepted, the step they describe is asserted field by
  field — workflow, node, step 2 attempt 1, the two verbatim requirement lines, the handoff, and
  the seven admitted operations in wire-name order — the digest the consumer **re-derives** is the
  literal `43a6f845…` both repositories pin, one flipped byte (`"index": 2` → `3`) comes back as
  `DigestMismatch` naming both digests, and `to_document` re-emits the minted file **byte for
  byte**, tag and trailing newline included. The last one is the surprise worth keeping: the two
  minters agree not merely on the digest but on the file. Nothing moved to make room for this —
  frame parsing is protocol-level, so these are plain tests in `metaharness-protocol` and no
  adapter conformance count changed (20 claude / 10 codex, as before).

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
  `~/.local/bin/codex 0.144.0, OFF the pin`, exit `1`, where it previously said on-pin.
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
