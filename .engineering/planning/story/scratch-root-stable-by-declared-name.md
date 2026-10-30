---
format: aep.planning-md/3
id: story:scratch-root-stable-by-declared-name
kind: story
status: active
title: A caller can name the run's scratch root, so the child's paths are the same across the sessions of one case
summary: '--scratch-name: one plain component, 0700 root, leftover replaced, link refused, held name refused, removed at end'
owner: metaharness
tags:
- prompt-cache
relations:
- decomposes: epic:loom-agent-work-through-metaharness
scope:
- confidence: cited
  path: crates/metaharness-protocol/src/spec.rs
- confidence: cited
  path: crates/metaharness/src/builder.rs
- confidence: cited
  path: crates/metaharness/src/run.rs
- confidence: cited
  path: crates/metaharness/tests/scratch_name.rs
revision: 8
transitions:
- {from: "draft", to: "proposed", at: "2026-10-30T17:48:56Z", actor: "human:timo", revision: 6}
- {from: "proposed", to: "active", at: "2026-10-30T17:48:56Z", actor: "human:timo", revision: 7}
---
# Story: a caller can name the run's scratch root, so the child's paths are the same across the sessions of one case

## Scheduling

Filed 2026-10-30 for trial finding C5, the metaharness part that Loom's
`story:session-executor-stable-copy-path` names ("metaharness's own per-session paths for the
child (its scratch root, `TMPDIR`, `CLAUDE_CONFIG_DIR`)"). One wave with the other C5 stories,
integration branch `wave/2026-10-30-c5`.

## Outcome

A caller can give a run a stable scratch name (`RunSpec::scratch_name`, `--scratch-name <NAME>`).
Every run under that name works in the same scratch root, `<temp dir>/metaharness-u<EUID>/<NAME>`,
under a private 0700 parent of the effective user's. So every path metaharness derives from the
root is the same from one session of a case to the next: the child's `TMPDIR`,
`CLAUDE_CONFIG_DIR` and `CODEX_HOME`, the hook channel, and the copies of the declared context
files that the b10x kind hands the loop's own `--context`. Without a name, the root is a new
temporary directory, as it is today.

- **The name.** It is 1 to 64 bytes of ASCII letters, digits, `.`, `_` and `-`. It is not `.` or
  `..`, and it does not end in `.lock`. Any other name is refused before anything is created.
- **The root and leftovers.**
  - The root is created with mode 0700.
  - A directory an earlier run of the same user left at the root is removed first, read-only
    directories included. It must be a directory, not a link, owned by the effective user.
  - A link or any other entry there refuses the run, naming the path, and is left untouched.
- **A held name.** A second run under a name whose run is still going is refused by name. The
  first run holds exclusive locks on the name and on the root for its lifetime.
- **The end of a run.**
  - The root is removed when the run ends, as the temporary directory is today.
  - A named run's vendor child runs in its own process group, which is killed when the run is
    dropped.

Amended 2026-10-30 after the unit B reviews (amendment a72, review corrections passes 1 and 2):
the per-user parent, the charset, the `.lock` suffix, the root lock and the process group.

## Why

The b10x kind hands each declared context file to the loop's `--context` as a copy at
`<scratch>/context/<path>` (`crates/metaharness/src/context.rs:323`, `write_copies`), and the loop
renders a provided-context layer with `source="<that path>"` into its system prompt
(`harness-loop` `context.rs`, `ContextPackage::render`, at harness `origin/main` 90422fc). The layer
sits right after `harness.instructions`, so a per-run path there breaks the provider's prefix cache
for everything after the first 647 bytes. Loom passes declared context through
`SessionOverrides::context` (`loom-session-executor/src/lib.rs:1133`). On the Claude kind the same
root holds `CLAUDE_CONFIG_DIR`, whose memory path reaches the system prompt (see
`story:claude-child-runs-without-auto-memory`, which removes that one line for every caller). The
name is supplied by the host, as Loom's copy name is, because a host builds one run per step.

## Acceptance

`crates/metaharness/tests/scratch_name.rs` (new), against the scripted runner, no vendor binary:

- `two_runs_under_one_scratch_name_use_one_scratch_root`: two runs in turn under one name report
  the same scratch root, and on the b10x kind hand the loop the same `--context` copy paths.
- `without_a_scratch_name_each_run_gets_its_own_root`: today's behaviour, two different roots.
- `a_scratch_name_that_is_not_one_plain_component_is_refused`: `a/b`, `..`, `.`, empty, a control
  character and 65 bytes are each refused before a directory is created.
- `a_leftover_root_of_the_same_user_is_replaced`: a directory with a file in it at the root is
  removed and the run starts.
- `a_link_or_foreign_entry_at_the_root_is_refused`: a symbolic link to a directory outside the temp
  dir, and a regular file, at the root each refuse the run; the link's target is untouched.
- `a_second_run_under_a_held_name_is_refused`: while one run holds the name, a second is refused
  by name and the first run's root is untouched.
- `the_root_is_removed_when_the_run_ends`.
- The CLI's `metaharness run --help` names `--scratch-name`; `crates/metaharness-cli` has a test
  that the flag reaches `RunSpec::scratch_name`.

## Out of scope

- Turning Claude's auto-memory off: `story:claude-child-runs-without-auto-memory`.
- The b10x loop's own context order (environment layers ahead of project instructions): the
  harness repository's.
- Loom passing the name: Loom's, after this releases.

## Scope

- cited: `crates/metaharness-protocol/src/spec.rs`: `struct RunSpec`, one new field
  `scratch_name` beside `scratch_cwd` (about line 448), and its `check_spec` refusal
- cited: `crates/metaharness/src/builder.rs`: the three `tempfile::TempDir::new()` calls in
  `start_b10x` (line 563), `start_claude` (line 934) and `start_codex` (line 1251), through one
  new helper beside `guard_scratch_path` (line 1882)
- cited: `crates/metaharness/src/run.rs`: `scratch: Option<tempfile::TempDir>` (lines 280, 329)
  and `fn scratch_root` (line 467)
- cited: `crates/metaharness/tests/scratch_name.rs` (new)
- inferred: `crates/metaharness-cli/src/lib.rs` (the flag, if not derived from `RunSpec`)
- inferred: `docs/design/metaharness-protocol-v0.1.md` (one amendment)
- Shares `spec.rs` and the CLI with `story:claude-prompt-cache-ttl-declared`: each adds one field
  and its refusal, nothing else in those files.
- Touches the scratch root that holds a copied credential: a security-reviewer pass runs beside
  the adversary.
