---
format: aep.planning-md/1
id: story:declared-context-on-claude
kind: story
status: draft
title: Claude receives every declared context file without argv size failures
relations:
- decomposes: epic:runs-side-by-side
revision: 2
---
## Goal
Advance O3 by honoring the existing context declaration on Claude. Refuse missing or oversized inputs before launch and carry large prompts through finite stdin without weakening tool decisions or the hermetic floor.

## Design
See docs/design/declared-context-input.md. No event or frame wire shape changes. AEP forwards its existing context paths through this CLI capability; an older binary refuses them explicitly.

## Acceptance
- A declared context marker reaches the prompt, relative to the declared working directory.
- Missing and oversized context refuses before vendor spawn.
- Large input uses finite stdin and retains the strict MCP setting and hook decision seam.
- A bounded live fixture demonstrates the installed vendor reads the declared input; private transcripts remain outside this repository.
