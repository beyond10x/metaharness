//! The claude adapter.
//!
//! Everything claude-specific lives here — how the binary is launched, how its transcript maps
//! onto [`metaharness_protocol::Event`], how a decision reaches it, and what hermetic means for
//! it. Nothing outside this crate may know any of that.
//!
//! Pinned to **2.1.304** and **2.1.305** ([`PINNED_VERSIONS`]). Every claim this crate makes about
//! those binaries is either read from `docs/design/metaharness-protocol-v0.1.md` § 2.7 — where each
//! row carries its own method — or labelled unverified at the point of use with the `Q` row that
//! would close it.
//! A string in a binary is weaker than a driven call, and this crate says which it has.
//!
//! **Where a claim below names 2.1.239, that is the binary the observation was read from, not the
//! pin** (design amendment a11). The pin moved on 2026-08-23, when the installed 2.1.240 ran a
//! whole session through this adapter; the § 2.7 rows were read on 2026-08-22 from
//! 2.1.239 and are left naming it, because rewriting the version on a dated observation would
//! invent evidence rather than move a pin. What is recorded from 2.1.240 says so at its own point
//! of use — the golden fixtures under `fixtures/golden/` are 2.1.240's own bytes.
//!
//! # The pinned binaries read files a prompt mentions, and nothing records it
//!
//! Claude Code 2.1.304 and 2.1.305 expand an `@path` in prompt text — the `-p` argument, stdin,
//! a stream-json user message — into a synthesised Read result in the request they send: no tool
//! call, no `PreToolUse` hook, no `tool.requested`, no line in the transcript. Paths outside the
//! working tree, through symbolic links, untracked files and directories expand alike
//! ([`MENTION_CANARIES`], design amendment a35). So the `--context` block never enters the user
//! turn: it goes to `--append-system-prompt`, which neither binary scans ([`CONTEXT_CHANNEL`]).
//! The task text is the user turn and stays exposed; no route the canary measured carries it
//! unexpanded.
//!
//! # Three things this crate deliberately does not do
//!
//! * **It spawns nothing.** [`plan_launch`] returns the command line, the child environment, the
//!   settings document, the hook definition and the copy list as *values*, because
//!   `AEP` asserts its own argv rather than trusting it *"because every one of
//!   the failures would be silent"* (design § 8.4 O7). [`hook_program`] is the same rule applied
//!   to the seam's executable: the program is a value this crate renders and a caller places.
//! * **It reads no clock.** Every event's timestamp is the one the vendor recorded, passed
//!   through, or absent — [`metaharness_protocol::Emission::at`] and
//!   [`metaharness_protocol::Emission::untimed`] are the only two ways to make one (design D2).
//! * **It drops nothing.** A vendor record this crate cannot map becomes
//!   [`metaharness_protocol::Event::Opaque`], carrying the vendor's own `type`, `subtype`, a
//!   digest of the raw line and its 1-based source line (design D4). An unrecognised *field* is
//!   ignored in silence; an unrecognised *record* is preserved.

mod bridge;
mod hook;
mod launch;
mod marketplace;
mod seam;
mod transcript;
mod vectors;

pub use bridge::{ClaudeSeam, ClaudeSeams};
pub use hook::{
    HOOK_WAIT_SECONDS, HookChannelPaths, PathNotUtf8, hook_program, shell_quote, shell_quote_path,
};
pub use launch::{
    CredentialCopy, ENDPOINT_KEY_POSTURE, HOOK_COMMAND_PLUGIN_VARIABLES, HOOK_TIMEOUT_SECONDS,
    LaunchContext, LaunchPlan, LaunchRefusal, LoopbackParams, SCRATCH_PATH_NAMES_PLUGIN_VARIABLE,
    ScratchFile, child_path, hook_program_path, mcp_config_path, plan_launch, settings_path,
};
pub use marketplace::{
    INSTALLED_PLUGINS, KNOWN_MARKETPLACES, MARKETPLACES_HOME, MarketplaceMatch, MarketplaceRefusal,
    PLUGIN_CACHE_HOME, PLUGIN_REGISTRY_HOME, ScratchEntry, resolve_marketplace, scratch_registry,
};
pub use seam::{HookInput, capabilities, parse_hook_input, render_hook_response, render_operation};
pub use transcript::TranscriptReader;
pub use vectors::{CONTRACT_OBLIGATIONS, conformance_vectors};

/// This adapter's id, as it appears in `session.started` and on the command line.
pub const ADAPTER_ID: &str = "claude";

/// The channel `--context` files reach Claude Code through: the appended system prompt
/// (`--append-system-prompt`), recorded on each `session.started.context_files` entry (design
/// amendment a35, correction round 1).
///
/// Not the user turn, because the pinned binaries expand an `@path` anywhere in it — `-p`, stdin
/// and stream-json input alike — into a synthesised file read that passes no tool seam and leaves
/// no record ([`MENTION_CANARIES`]). The cost is stated where it is paid: the files' text carries
/// system-prompt authority, which the block's preamble answers by calling it reference data.
pub const CONTEXT_CHANNEL: metaharness_protocol::ContextChannel =
    metaharness_protocol::ContextChannel::SystemPrompt;

/// The vendor versions this adapter was written against (design § 8.4 O1).
///
/// Each entry is a binary this adapter was driven against, because the vendor's transcript and
/// hook shapes are not stable public schemas: a verdict that changed because the reader changed
/// must be visible as such rather than as a change in the agent's behaviour. A version outside
/// this pin is a `warning` (H9 `gap`), or a refusal before the spawn under `--strict-version`,
/// which reads the resolved binary's `--version` without a model call.
///
/// Two entries since 2026-10-21 (design amendment a33): the vendor ships faster than a pin moves,
/// and the installed `claude` moved 2.1.304 → 2.1.305 within a day of the 2.1.304 qualification.
/// Each entry is qualified on its own — the native fixture runs against each binary — and each
/// declares its own built-in list ([`BUILTIN_PLUGINS`]); a version is added here only with both.
///
/// Moved 2.1.239 → **2.1.240** on 2026-08-23 (design amendment a11). Two pieces of evidence, and
/// they are separate: a live run of the installed 2.1.240 through this adapter spoke the stream
/// dialect this crate reads — the session opened, streamed and ended, and its own opening record
/// reported `claude_code_version` 2.1.240 — and the golden fixtures in `fixtures/golden/` are that
/// same binary's wire, including a `PreToolUse` stdin it really published into the hook channel,
/// replayed here byte for byte in the free tier. A pin is what the adapter is *tested against*, so it names the binary
/// whose bytes are committed here, not the one the older § 2.7 rows were read from.
///
/// Moved 2.1.240 → **2.1.241** on 2026-08-24, and this move is the cheap kind. The earlier note
/// here said moving the pin again was "a **capture**, not a search-and-replace", which read the
/// rule backwards: `fixtures/golden/README.md` § *"Capture version and pin are two different
/// facts"* is the governing text, and it says the opposite for a good reason. The golden's
/// `captured`/`binary` rows are a fact about **those bytes** and are never edited; the pin is what
/// the adapter is *run* against; and `golden-version-pair` is the single place the two are
/// compared, where a disagreement is a **named warning that stands until somebody pays for a
/// re-capture**. So a re-pin is free and honest, and only a re-capture costs money. Holding the pin
/// behind the installed binary bought nothing and cost every `--hermetic strict` run an H9 gap.
///
/// What was read on 2.1.241 before moving it, and what was not:
///
/// * **Verified, live** — two runs through this adapter on 2026-08-24 opened, streamed a tool
///   call, and ended; each opening record reported `claude_code_version` 2.1.241 and was read by
///   this crate's own transcript reader without a dialect complaint.
/// * **Verified, from the binary's own help** — every flag the launch builds still exists, and
///   two of them still say what the design quotes them as saying: `--tools` — *"Use \"\" to
///   disable all tools"* (V11) — and `--strict-mcp-config` — *"Only use MCP servers from
///   --mcp-config, ignoring all other MCP configurations"* (H5).
/// * **Carried over unverified** — the `R8u(e)` parser reading behind V12, read from the 2.1.239
///   bundle; V4's bare-`--allowedTools` auto-approval, never driven on any version; and Q11's
///   `""` hook matcher regime. None of these were re-read, and none is claimed to have been.
///
/// Moved 2.1.241 → **2.1.259** on 2026-09-03, the cheap kind again, and with one thing read that the
/// earlier moves had no occasion to: the **wire grew**. A paid run of the installed 2.1.259 through
/// this adapter (the golden-path eval case, 1,515 events) opened, streamed and ended, its opening
/// record reported `claude_code_version` 2.1.259 — and 183 of its lines were shapes this reader had
/// never seen: `system/task_started`, `system/task_progress`, `system/task_notification`,
/// `system/task_updated` and a top-level `tool_progress`. Each became `opaque`, which is what D4
/// requires, and every `tool.absent` row in the checker that read the stream came back `unk` over
/// them. `transcript::control_plane` names those five as the vendor's own bookkeeping and drops
/// them; anything else unnamed still goes `opaque`. `metaharness doctor claude` on the same day
/// confirmed every flag the launch builds is declared by the 2.1.259 binary, including the
/// `--max-budget-usd` this release starts sending.
///
/// **Two more shapes, and the lesson of how they were met (0.6.2 → 0.6.4).** Later runs of the
/// same case carried `system/background_tasks_changed` and `system/api_retry`. Both were first
/// handled from the record as it appeared in a stream rather than from the binary's own schema,
/// and both were wrong for it:
///
/// * `background_tasks_changed` was dropped as bookkeeping. Its schema says the payload is *"every
///   live background task after the change"* with REPLACE semantics, emitted on completion and
///   kill as well as start — and only the start is on the wire elsewhere. It is read now.
/// * `api_retry` was read for `delayMs` and a string `error`. Its schema is `attempt`,
///   `max_retries`, `retry_delay_ms`, `error_status` and an `error` **object**, so every real
///   retry reported no reason and no backoff.
///
/// Both are now read from the schema the installed binary carries, which is the thing this
/// constant exists to make somebody go and look at. A shape met from one observed record is a
/// guess with a sample size of one.
///
/// The golden fixtures stay labelled 2.1.240, because that is the binary whose bytes they are.
/// `golden-version-pair` therefore warns, by design, and the warning is the outstanding invoice.
///
/// Moved 2.1.259 → **2.1.304 and 2.1.305** on 2026-10-21 (design amendment a33), credential-free.
/// It stayed at 2.1.259 through 2.1.288 because the built-in plugins every opening record carries
/// left H1a unqualified (amendment a25); [`BUILTIN_PLUGINS`] now declares them per version and H1a
/// admits each one on its name and its `@builtin` source. What was read on each binary, and how
/// (`docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md`):
///
/// * **Verified, driven credential-free, on each** — the native fixture (`tests/native_fixture.rs`)
///   ran the production driver against the 2.1.304 binary and against the 2.1.305 binary under
///   `credentials: none` and `credentials: loopback`, each against an owned `/v1/messages` stub in
///   a loopback-only namespace: the opening record reported that binary's `claude_code_version`,
///   the stream opened, streamed, decided a real `Bash` call both ways and ended, the floor read
///   H9 and H1a `ok`, and `--strict-version` let the run through; the same flag refused 2.1.303
///   before the spawn.
/// * **Verified, from each binary's own help** — every flag the launch builds is declared, except
///   `--max-turns`, which both register with `.hideHelp()`: present in the binary, absent from
///   `--help`, and honoured (the fixture's turn-ceiling case stops on `error_max_turns`).
/// * **Read from the binary, not driven** — the twelve-name built-in list and the
///   `projectSettings` gate on memory discovery (H11; the native fixture's canary case is the
///   driven half).
/// * **Not verified** — an operator-login or API-key session on either (no paid run); the fourth
///   built-in of an operator login, `cc-plugin-sec-default`, is relayed from another session's
///   2.1.304 record.
pub const PINNED_VERSIONS: [&str; 2] = ["2.1.304", "2.1.305"];

/// The twelve plugins 2.1.304 and 2.1.305 compile in, in the order each registers them.
///
/// Read separately from each binary's registration function (`builtinPluginsInitialized`, with
/// `rg -a`); the two lists are equal, so one constant serves both rows of [`BUILTIN_PLUGINS`].
const COMPILED_IN_2_1_304_AND_2_1_305: [&str; 12] = [
    "cc-plugin-sec-default",
    "cc-plugin-agents-md",
    "cc-plugin-telemetry",
    "cc-plugin-plugin-authoring",
    "cc-plugin-mods-guide",
    "cc-plugin-tips",
    "cc-plugin-mermaid",
    "cc-plugin-responsive-mode",
    "cc-plugin-diff",
    "cc-plugin-you-should-know",
    "cc-plugin-easel",
    "cc-plugin-claude-test",
];

/// Every plugin each pinned binary compiles in, keyed by the version (design amendment a33).
///
/// One row per entry of [`PINNED_VERSIONS`], in the same order, and a test holds the two to each
/// other: a version cannot be pinned without its list, so a pin move that did not re-read the new
/// binary fails before it lands. **Read from the binary, not driven.** A built-in is compiled in,
/// so a scratch config home cannot remove one: the launch can only switch off what the binary lets
/// it switch off, and the floor names the rest.
///
/// | built-in | seen in an opening record (2.1.304 and 2.1.305) | what the launch does about it |
/// |---|---|---|
/// | `cc-plugin-agents-md` | `none`, `loopback` (native fixture); operator login (relayed, 2.1.304) | nothing it can; admitted by name and `@builtin` source. Its ancestor walk for `AGENTS.md` is off without the `projectSettings` source (read from the binary); with the project and local sources on and no `CLAUDE.md`-family file in or above the cwd it sent both `AGENTS.md` shapes, and under the launch none (native fixture) |
/// | `cc-plugin-plugin-authoring` | `none`, `loopback` (native fixture); operator login (relayed, 2.1.304) | nothing it can; admitted by name and `@builtin` source |
/// | `cc-plugin-telemetry` | operator login (relayed, 2.1.304); resolved by `claude plugin details` in a plain scratch home of either binary, **not** with the traffic switch; absent from every native-fixture record | `CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1` under `credentials: none` and `loopback`; admitted by name and `@builtin` source under an operator login or an API key, where the switch is not set |
/// | `cc-plugin-sec-default` | operator login (relayed, 2.1.304, Team/Enterprise policy) | nothing it can: seated by organization policy; admitted by name and `@builtin` source |
/// | the other eight | never, in any probe here | none needed; each sits behind an entrypoint check or a further gate and is admitted by name and `@builtin` source if a record ever carries it |
///
/// **What this list moves, and what it does not.** H1a reads `ok` over a record plugin only when it
/// is a declared installed plugin or a name listed here **for the version the record reports**,
/// carrying exactly [`builtin_plugin_source`]'s source; the row names each built-in it saw. A name
/// alone matches nothing, so an ambient plugin that borrows a built-in's name stays a `gap`, and a
/// record on an unpinned version has no list at all. The list never makes a row `ok` by itself
/// (invariant 3): the record still has to carry the plugin. H9 is moved by [`PINNED_VERSIONS`],
/// not by this list.
pub const BUILTIN_PLUGINS: [(&str, &[&str]); 2] = [
    ("2.1.304", &COMPILED_IN_2_1_304_AND_2_1_305),
    ("2.1.305", &COMPILED_IN_2_1_304_AND_2_1_305),
];

/// What one pinned binary does with an `@path` mention, per route (design amendment a35,
/// correction round 1). `true` means the binary turned `@marker.txt` into a synthesised *"Result of
/// calling the Read tool"* block in the request it sent, holding the file's bytes.
///
/// **That read is invisible.** No tool call is made, so no `PreToolUse` hook fires and no
/// `tool.requested` reaches the stream; the transcript does not name the file. Measured by the
/// adversary of correction round 1 on both pins: any `@` token at the start of a line or after
/// whitespace expands, inside a code fence too, and `@../f`, `@/abs`, `@~/f`, directories,
/// untracked files and links out of the tree all reach the request, under every hermetic mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MentionCanary {
    /// The binary measured.
    pub version: &'static str,
    /// The text as the `-p` argument: the user turn.
    pub prompt_argument: Mention,
    /// The text on stdin, with `-p` and no argument: the user turn.
    pub stdin_text: Mention,
    /// The text in a user message on stdin under `--input-format stream-json`: the user turn.
    pub stdin_stream_json: Mention,
    /// The text as the value of `--append-system-prompt`, where the `--context` block goes.
    pub append_system_prompt: Mention,
    /// The text as the value of `--system-prompt`.
    pub system_prompt: Mention,
}

/// What a binary did with one `@path` mention on one route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mention {
    /// The mentioned file's bytes reached the request as a synthesised Read result.
    Expanded,
    /// The mention reached the request as the text it is.
    Text,
}

/// The `@`-mention canary of every pinned binary, one row per entry of [`PINNED_VERSIONS`] in
/// the same order (design amendment a35, correction round 1).
///
/// Measured on 2026-10-21 against the native fixture's stub endpoint in a loopback-only namespace,
/// no model reached: on 2.1.304 and on 2.1.305 every user-turn route expands and neither system
/// prompt route does, and the text arrives byte-exact in the request's `system` field. A test holds
/// the rows to the pin, so a pin without a row fails — the coupling [`BUILTIN_PLUGINS`] has; it
/// cannot tell a measured row from a copied one. The native fixture's canary case re-measures the
/// binary under test against its row; it is ignored in `task check`, which does not run it, so it
/// is a step of every pin move. [`CONTEXT_CHANNEL`] rests on `append_system_prompt` being
/// [`Mention::Text`]; the task text, which is the user turn, has no route the rows show as text.
pub const MENTION_CANARIES: [MentionCanary; 2] = [
    MentionCanary {
        version: "2.1.304",
        prompt_argument: Mention::Expanded,
        stdin_text: Mention::Expanded,
        stdin_stream_json: Mention::Expanded,
        append_system_prompt: Mention::Text,
        system_prompt: Mention::Text,
    },
    MentionCanary {
        version: "2.1.305",
        prompt_argument: Mention::Expanded,
        stdin_text: Mention::Expanded,
        stdin_stream_json: Mention::Expanded,
        append_system_prompt: Mention::Text,
        system_prompt: Mention::Text,
    },
];

/// The `@`-mention canary of the pinned binary of `version`, or `None` for a version this adapter
/// does not pin.
#[must_use]
pub fn mention_canary(version: &str) -> Option<MentionCanary> {
    MENTION_CANARIES
        .iter()
        .find(|row| row.version == version)
        .copied()
}

/// The memory files the pinned binaries read from the cwd and every directory above it once the
/// project or local source is on, in the binary's own order (design amendments a33, a36).
///
/// Read from both binaries: `[["CLAUDE.md"],["CLAUDE.local.md"],["AGENTS.md"],[".claude",
/// "CLAUDE.md"],[".claude","AGENTS.md"],[".claude","rules"]]`, the same string in each. The last
/// entry is a directory: every `.md` file under it, at any depth. The ancestor walk looks for
/// exactly these, and what it finds is information in H11, never a refusal.
pub const MEMORY_SHAPES: [&str; 6] = [
    "CLAUDE.md",
    "CLAUDE.local.md",
    "AGENTS.md",
    ".claude/CLAUDE.md",
    ".claude/AGENTS.md",
    MEMORY_RULES_DIRECTORY,
];

/// The entry of [`MEMORY_SHAPES`] that names a directory rather than a file: the binary reads
/// every `.md` file under it, recursively.
pub const MEMORY_RULES_DIRECTORY: &str = ".claude/rules";

/// What one pinned binary did with a canary of every memory shape, planted in the cwd and in its
/// parent (design amendment a36). H11 rests on this row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoryCanary {
    /// The binary measured.
    pub version: &'static str,
    /// Canaries planted: each of [`MEMORY_SHAPES`] in the cwd and in its parent.
    pub planted: usize,
    /// How many the positive control sent, the vendor run directly with
    /// `--setting-sources project,local`. Fewer than planted means the probe could not see a
    /// shape, and the zero below says nothing about it.
    pub control_sent: usize,
    /// How many reached a request through the launch under `credentials: none`.
    pub launch_none: usize,
    /// How many reached a request through the launch under `credentials: loopback`.
    pub launch_loopback: usize,
}

impl MemoryCanary {
    /// Whether this row supports H11: the control saw every canary, and none got through the
    /// launch under either credential mode.
    #[must_use]
    pub fn kept_out(&self) -> bool {
        self.control_sent == self.planted && self.launch_none == 0 && self.launch_loopback == 0
    }
}

/// What no memory canary row covers, in the words an `ok` H11 ends with (design amendment a36 § 6).
/// The walk looks only above the cwd and the probe planted only there, so the policy directory's
/// memory file is outside both.
pub const MEMORY_NOT_PROBED: &str = "managed memory (/etc/claude-code/CLAUDE.md) is neither walked \
                                     nor probed, so this row says nothing about it";

/// The memory canary of every pinned binary, one row per entry of [`PINNED_VERSIONS`] in the same
/// order (design amendment a36).
///
/// Measured on 2026-10-21 by the native fixture against its stub endpoint in a loopback-only
/// namespace, no model reached (`docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md`,
/// § *H11: memory files*): on 2.1.304 and on 2.1.305 the positive control sent all twelve — the
/// eight `CLAUDE.md`-family canaries, and the four `AGENTS.md` ones where no `CLAUDE.md`-family
/// file sat in or above the cwd — and the production launch sent none under `none` or `loopback`.
/// User memory (a `CLAUDE.md` in the config home) and managed memory were not probed. A test holds
/// the rows to the pin, so a pin without a row fails, the coupling [`MENTION_CANARIES`] has; it
/// cannot tell a measured row from a copied one. The native fixture's canary case compares its
/// measurement with the row of the binary under test; it is ignored in `task check`, so re-running
/// it is a step of every pin move.
pub const MEMORY_CANARIES: [MemoryCanary; 2] = [
    MemoryCanary {
        version: "2.1.304",
        planted: 12,
        control_sent: 12,
        launch_none: 0,
        launch_loopback: 0,
    },
    MemoryCanary {
        version: "2.1.305",
        planted: 12,
        control_sent: 12,
        launch_none: 0,
        launch_loopback: 0,
    },
];

/// The memory canary of the pinned binary of `version`, or `None` for a version this adapter does
/// not pin.
#[must_use]
pub fn memory_canary(version: &str) -> Option<MemoryCanary> {
    MEMORY_CANARIES
        .iter()
        .find(|row| row.version == version)
        .copied()
}

/// The plugins the pinned binary of `version` compiles in, or `None` for a version this adapter
/// does not pin.
#[must_use]
pub fn builtin_plugins(version: &str) -> Option<&'static [&'static str]> {
    BUILTIN_PLUGINS
        .iter()
        .find(|(pinned, _)| *pinned == version)
        .map(|(_, names)| *names)
}

/// Every managed-settings source the pinned binaries read on Linux (design amendment a33).
///
/// Read from the 2.1.304 binary's strings: the policy directory is `/etc/claude-code` on Linux
/// (no override on the launch's path: `CLAUDE_CODE_MANAGED_SETTINGS_PATH` never reaches the
/// child, H3), holding `managed-settings.json` and a `managed-settings.d` drop-in directory; under
/// WSL the binary also reads the same pair from the Windows policy directory through `DrvFs`. Managed
/// settings are a source `--setting-sources ""` does not switch off, and their `env` or
/// `apiKeyHelper` can put a key in the child, so the caller looks for each of these before the
/// launch claims anything about the child's API key (H4). A directory counts by existing.
pub const MANAGED_SETTINGS_PATHS: [&str; 4] = [
    "/etc/claude-code/managed-settings.json",
    "/etc/claude-code/managed-settings.d",
    "/mnt/c/Program Files/ClaudeCode/managed-settings.json",
    "/mnt/c/Program Files/ClaudeCode/managed-settings.d",
];

/// The source a Claude Code opening record gives a built-in plugin: `<name>@builtin`.
///
/// The vendor's own sentinel — *"`name@builtin` for built-in plugins"*, in the 2.1.304 binary's
/// schema for the `plugins` field — and the shape the native fixture observed on every built-in.
#[must_use]
pub fn builtin_plugin_source(name: &str) -> String {
    format!("{name}@builtin")
}
