---
format: aep.planning-md/1
id: story:explicit-context-capacity
kind: story
status: draft
title: Expose supported complete-input capacity and offline launch admission
relations:
- informed_by: story:current-claude-and-explicit-cache-compatibility
revision: 1
---
## Consumer contract
The brain consumer prepares immutable, digest-bound model context containing exact held identities, revisions and scalar comparison evidence. The compatible adapter implementation at b4998a6d611de5e03eedcdae5df606d31fb74413, crates/metaharness/src/builder.rs::preload_claude_context, fixes the declared-context input bound at 2,000,000 bytes. The consumer duplicates this ceiling and estimates the driver wrapper before reserving a native session. Once mandatory comparison context approaches that ceiling, reducing incoming signal count cannot make a launch fit. Loss of comparison evidence or bypassing adapter admission is not an acceptable consumer fallback.

## Requested outcome
Publish a supported way to declare and discover complete-input capacity, with an offline admission result that includes the complete effective prompt, declared-context framing and adapter/driver additions. Preserve current default behavior when omitted. A caller requesting more authority must either receive a validated supported bound or an explicit refusal before a native session or charge can start. Document byte bounds separately from provider token capacity, and provide the supported consumer invocation and release provenance. If a supported release already supplies this, identify that release and contract instead of implementing another capability.

## Acceptance and ownership
Verify exact-bound and one-byte-over-bound cases, UTF-8 byte accounting, multiple context files and all framing, unreadable or growing context, bounded reading, and rejection without native spawn. Preserve hermetic credentials, tool-free output enforcement, native completion checks, actual and unknown cost evidence, explicit effort/cache controls and retained old runs. Coordinate the driver's offline projection with AEP rather than making the consumer reverse-engineer vendor instructions. Metaharness agents own implementation and release; brain agents consume a published compatible result and validate their own matching admission bound. This is a scoped request only, with no upstream implementation, deployment or release performed by the consumer session. Public evidence is code-derived; no adopter data, paths, source material or native transcripts are included.
