# Metaharness documentation

This directory owns the independent static site at `/metaharness/`. The landing
page is authored HTML, the 13 documentation pages are Markdown, and presentation
is local CSS. There is no JavaScript runtime, package manager or network font.

The Rust/clap builder lives in `crates/metaharness-docs`:

```console
cargo run --locked -p metaharness-docs -- check
cargo run --locked -p metaharness-docs -- build --out /path/to/empty/site --commit <full-git-revision>
```

The output directory is the site root mounted at `/metaharness/`; it contains
`index.html`, `docs/**/index.html`, local assets and `.well-known/b10x-site.json`.
The accompanying `.well-known/b10x-routes.json` lists each public page and its
sorted rendered anchor IDs, stamped with the same source commit.
The build refuses invalid provenance, duplicate routes and broken internal links
or heading fragments. Output may be a new or empty directory, or a complete
previous Metaharness build with matching project provenance and the exact generated
file inventory. A rebuild stages the whole site before replacing that output;
foreign or incomplete output, extra files/directories and symlinks are refused
without modifying the previous site. Build output is never committed. Publication is owned by the repository workflow.

Keep the existing page paths and headings stable: they are public deep links.
Use relative `.md` links between documentation pages. Tables, fenced code,
Docusaurus-style `:::note` / `:::info` / `:::warning` / `:::danger` admonitions and
explicit heading IDs are supported. The full page allowlist and reading order
live in `crates/metaharness-docs/src/content.rs`; b10x is a first-class navigation
entry. Design and research documents outside this directory remain unpublished.

The asset contract is deliberately small: local HTML `href`, `src`, `poster` and `background` references
are checked against the generated files. `srcset`/`imagesrcset`, inline styles,
CSS imports and CSS asset-loading functions are refused as unsupported. The local
stylesheet uses none of them; its tokens are checked with a Rust CSS parser,
including escaped spellings and nested blocks, rather than a text search.
