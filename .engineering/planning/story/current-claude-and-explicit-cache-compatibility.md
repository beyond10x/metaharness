---
format: aep.planning-md/1
id: story:current-claude-and-explicit-cache-compatibility
kind: story
status: draft
title: Validate current Claude compatibility and publish explicit cache control
relations:
- informed_by: story:pinned-plugin-loads-in-the-session
revision: 1
---
## Consumer compatibility evidence
Public metaharness 0.6.5 source declares Claude Code adapter pin 2.1.259 while the published vendor CLI is now 2.1.263. Doctor confirms that the invoked flags remain declared, but that does not validate version-specific transcript, interruption or cost behavior. Separately, the current public source lacks the explicit prompt_cache_ttl launch field needed by the brain consumer's locally retained build. The consumer must not drop this declared control or alter ambient/global settings merely to consume the same version string from a public release.

## Required outcome
Validate or explicitly refuse the current vendor version against the adapter's supported control, transcript, completion and accounting contracts. Publish a compatible release that accepts the consumer's explicit five-minute prompt cache duration, preserves credential isolation and medium reasoning effort, and leaves omitted controls backward compatible. Document the exact supported version, provenance and migration path. If existing published capabilities already satisfy these requirements, return the exact release and executable minimal consumer example; no implementation is presumed necessary. Native evidence belongs in private retained storage and public fixtures must be independently authored synthetic records.

## Ownership
This is a scoped consumer request. Metaharness agents own investigation, implementation and release if needed; the brain session will consume and validate a published compatible result. No customer, operator, credential or private run data belongs in this artifact.
