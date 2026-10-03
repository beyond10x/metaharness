---
id: frames
title: The workflow frame
sidebar_position: 5
---

# The workflow frame

**A frame is what the embedder says this step *is*, as a typed value.**

It carries what has already been established, what must hold here, what the step owes before it
may end, and — the enforcing part — **strictly the operations admitted here**.

## Presented every turn, not once per session

The embedder's per-state tool set changes at every transition. A frame that could only be set per
session would turn a per-step guarantee into a per-session one.

> A **step** is the *embedder's* unit of work. A **turn** is the *vendor's*. Conflating them is
> exactly how a per-step guarantee quietly becomes a per-session one.

## What a frame holds

| Field | What it is |
|---|---|
| `workflow` | Which workflow, at which **pinned** version. |
| `node` | The state the run is in. |
| `step` | Workflow, state, index, and which attempt — counting from 1. |
| `prior` | What has already been established, one line each, each with the document that established it. |
| `obligations` | What must hold while here. Verbatim, never summarised. |
| `reaching` | What does not hold yet on a way out, prefixed with where it goes. |
| `next` | The nodes reachable from here. |
| `handoff` | What this step must produce before it may end. |
| `operations` | Strictly the operations admitted here. |
| `entities` | The enumerated set a routing step chooses from, when there is one. |
| `digest` | SHA-256 over the canonical form of everything above. |

### Why `reaching` exists

A recorded failure: a run in which the model was never told what the *next* state wanted, wrote
neither, and was refused for work already paid for.

### Why lines are verbatim

A driver that summarised a requirement would be the only place that summary existed.

### Why `handoff` is explicit even when empty

`handoff: none` says a step owes nothing, deliberately. **A step whose handoff is unstated is a
step nobody can fail.**

## The operation vocabulary is small and closed

The frame names **neutral operations**. The adapter renders them into vendor tool names and
**never re-decides what an admission implies**.

```text
file.read     file.write    file.edit     dir.list      search
shell         web.read      skill.load    subagent.spawn
task.todo     mcp.call{server, tool}
```

That closure is load-bearing: a second harness that decided `file.write` admitted a shell would be
a weakening the protocol had no way to notice.

`subagent.spawn` is **not admitted by default on any adapter** — a subagent's tool set is derived
by nothing in these decisions, so it would be a route around the per-step admission.

## Sealing, and why the digest is on the document

```bash
metaharness run claude --hermetic --frame step.frame.json -p "…"
```

The on-disk form is one JSON object tagged `metaharness.frame/1`. Loading it checks, in order:

1. **The format tag** — a document is self-describing or it is refused.
2. **The shape.**
3. **The digest** — a document whose digest does not describe its contents is refused.

A frame is refused **by name** when it is unreadable, untagged, misshapen, or edited after
sealing. Writing a frame out does **not** reseal it: a producer that mutated a sealed frame should
be refused by every consumer rather than silently repaired.

The tag lives on the document itself, not on a file extension or a handshake, so a copied or
truncated file is still self-describing.

### Canonical ordering is a wire fact

Operations serialize sorted by their `op` name (then by MCP server and tool). The sealed digest is
over that serialization. This is deliberately a rule an external producer can follow **without
reading the Rust enum** — a digest that depended on variant declaration order would make the
canonical form depend on one file in one repository.

## Two independent ways a frame reaches the model

| | |
|---|---|
| **The text** | The frame is rendered as instruction text. It says what the step is. |
| **The control seam** | It is what makes the text *true*. |

Text alone is not enforcement. The rendering function lives in the protocol crate and is shared by
every adapter, so two harnesses cannot describe the same frame differently.

## This is the integration seam

An external driver writes the frame **as a file** and never links this workspace. That is the
whole interface: a sealed document on disk, verified on load, enforced per call from the first
turn.
