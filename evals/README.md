# evals — paid and recorded evaluations, by subject

Everything under here is **evaluation machinery, its fixtures and its results** — the things that
cost money to produce or exist to judge a paid run. Nothing here is part of `task check`.

| directory | subject | notes |
|---|---|---|
| [`engineering-protocols/`](engineering-protocols/) | the engineering-protocols driven loop, through this repository's seam | `run-driven.sh` is live; `run.sh` is retired with its subject (the shell hooks) |
| [`codex/`](codex/) | the Codex planning-plugin residue migrated from engineering-protocols | the adapter itself is `crates/metaharness-codex` |

The subject checkout is named by `EP_REPO` (default `~/projects/engineering-protocols`). These
evals were migrated here under that repository's `epic:metaharness-migration`: the eval logic,
recorded transcripts, contracts and result records belong with the harness seam they exercise,
and the subject repository keeps only its domain — workflows, drivers, principles, trace
specifications.
