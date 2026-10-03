# Independent documentation

Operator decision, 2026-10-03: Metaharness owns an independent documentation site,
with Mantle's visual style and all existing documentation retained.

The canonical landing is `https://beyond10x.github.io/metaharness/`. Reference
pages retain `/metaharness/docs/` and their existing slugs and heading fragments.
The 13 authored pages remain repository-owned Markdown. A Rust/clap static builder
renders them with a common HTML/CSS shell, validates local links and emits a
deterministic site. No browser JavaScript or Node toolchain is required.

Builds run without publication credentials. A successful push build on `main`
uploads `b10x-project-site`, including `.well-known/b10x-site.json` with the
`b10x-project-site/v1` schema, repository, exact source commit and `/metaharness/`
base URL. A separate bot-only caller uses the immutable Website project-site
publisher at `fb4024ef7846729e5456591b9070db3d48c87e64`, the publisher used by
Mantle. It validates the source identity and exact successful build, and emits
delivery provenance. Source code never executes in the publication job.

Atlas relinquishes unified documentation ownership for Metaharness. The shared
source manifest, bundle producer, source check and redirect facade are retired.
Website retains compatibility redirects from `/docs/metaharness/` to the matching
independent reference pages. The independent site is published and verified first;
shared-site compatibility changes follow so the old content stays available during
cutover. Future Metaharness documentation changes need no Atlas or root-site release.

This is documentation tooling and routing, not a product runtime entity or a new
software release. Existing ESS product contracts and the 0.9.1 tag are unchanged.
