---
format: aep.planning-md/3
id: approval-record:d209-codex-live-follow-ups
kind: approval-record
status: draft
title: 'D209: the Codex catalogue route checked live at both pins'
owner: metaharness
relations:
- decides: story:codex-catalogue-live-follow-ups
revision: 1
---
## D209

Approved 2026-10-29 by `b10x-chef-orchestrator` under the delegated grant (class C). Quote: "I accept that
story (approval-record citing D209). Check each point live on the ChatGPT/Codex subscription login only
(Timo 2026-10-20: subscription quota is not spending; a pay-per-token API key is a stop-and-report). Also
run one live probe that tries to make `exec` act outside the catalogue (e.g. a file write or network call
the catalogue does not offer) and record what happens; if it can act outside, that is a security finding:
stop the route (refuse --catalogue on Codex until fixed) in this wave and tell me at once. Record results
as evidence; fix what fails (failing test first)." And: "If only evidence/store changes: commit them on a
branch plan/codex-live-20261029 and report (rides next PR)."

## Outcome

- Six live runs of the released 0.24.0 binary on the subscription login (`auth_mode: chatgpt`, no API key;
  `OPENAI_API_KEY` and `CODEX_API_KEY` removed from the environment), three at codex-cli 0.153.4 and three
  at 0.160.0: `docs/research/2026-10-29-codex-catalogue-live-follow-ups.md`, amendment a67.
- Items 1-3 hold on a login. The `exec` probe found no effect outside the catalogue at either pin, so the
  route stays admitted and no stop was needed.
- Item 4: nothing failed. Two checks recorded as unchanged with their reasons; the duplicate-name check
  was already refused on every kind, as `catalogue-malformed`. One test pins it; the public pages that
  named `catalogue-name-duplicate` for it are corrected.
- No shipped behaviour changed, so no release. The evidence, the test and the page corrections are on
  branch `plan/codex-live-20261029` and ride the next PR.
