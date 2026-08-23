# website

The public documentation site for metaharness, built with [Docusaurus](https://docusaurus.io/)
and published to GitHub Pages at <https://beyond10x.github.io/metaharness/>.

## What lives here, and what does not

| | |
|---|---|
| `website/docs/` | The **public** documentation: what metaharness is, how to run one, the wire, the contracts, the adapters. Written for a reader who does not have this checkout. |
| `docs/design/` (repo root) | The binding design documents. Not published — they carry the full reasoning, the open questions and the amendment record. |
| `docs/research/` (repo root) | Vendor research records, with per-claim evidence labels. Not published. |

A published page states a conclusion and cites where the reasoning lives. When the two disagree,
the design document is right and this site is stale.

## Working on it

```bash
npm install
npm start          # dev server with hot reload
npm run build      # production build — a broken link is a failure, not a warning
npm run serve      # serve the built site locally
```

From the repository root, via `task`:

```bash
task docs          # dev server
task docs:build    # production build
```

## Publishing

`.github/workflows/docs.yml` builds on every pull request that touches `website/`, and publishes
from `main` through the GitHub Pages deployment API.

The built site is **never** committed to this repository — there is no `gh-pages` branch to keep
in sync, and `build/` is git-ignored.

Enable it once, in the repository's **Settings → Pages**, by setting the source to **GitHub
Actions**.

## Conventions

- `onBrokenLinks: 'throw'`. A documentation site that links to nothing in particular is worse than
  one page shorter.
- Facts go in tables, one per row. The docs are read by people looking something up, not reading
  an essay.
- Every claim about behaviour is one the repository can back: a pinned version, a conformance
  vector, or a recorded run. A claim that is not yet driven is **labelled unverified**, here as in
  the code.
