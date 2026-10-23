# metaharness-tools serves the tools crates' code (2026-10-23)

This was checked at `main` `f39e773` (Release 0.15.0) against the criterion in atlas ADR 0097:38-40. The criterion: the tool code metaharness serves must be the tools crates' code, and it does not have to be a direct dependency.

## Verdict

All five tools come from `beyond10x/tools` tag `0.4.0` (`6017fccc`): fs, code-editor, toolchain, search and process.

- **Cargo tree.** For each tool, `cargo tree -i` reaches `metaharness-tools` through `b10x-harness-catalogue` 0.18.0 (harness `330e2cde`), and through no other path.
- **No harness implementation.** No crate in the tree carries its own implementation of any of the five.
- **How the catalogue calls them.** `harness-catalogue/src/local.rs:42-45` at `330e2cde` imports `tool_fs::Fs`, `tool_code_editor::Editor`, `tool_process::Process` and `tool_search::Search`. Each `Operations` method at `local.rs:187-330` calls its tools operation directly.
- **Toolchain.** It goes through `tool_toolchain`'s `plan`, `executor` and `builtins` (`toolchain.rs`).
- **Direct dependency.** metaharness-tools takes only `b10x-tool-contract` 0.4.0 directly (`crates/metaharness-tools/Cargo.toml:16`).

## Output

### b10x-tool-fs

```console
$ cargo tree --locked --offline -p metaharness-tools -i b10x-tool-fs -e normal
b10x-tool-fs v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
├── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde)
│   └── metaharness-tools v0.15.0 (./crates/metaharness-tools)
└── b10x-tool-search v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
    └── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde) (*)
```

### b10x-tool-code-editor

```console
$ cargo tree --locked --offline -p metaharness-tools -i b10x-tool-code-editor -e normal
b10x-tool-code-editor v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
└── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde)
    └── metaharness-tools v0.15.0 (./crates/metaharness-tools)
```

### b10x-tool-toolchain

```console
$ cargo tree --locked --offline -p metaharness-tools -i b10x-tool-toolchain -e normal
b10x-tool-toolchain v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
└── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde)
    └── metaharness-tools v0.15.0 (./crates/metaharness-tools)
```

### b10x-tool-search

```console
$ cargo tree --locked --offline -p metaharness-tools -i b10x-tool-search -e normal
b10x-tool-search v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
└── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde)
    └── metaharness-tools v0.15.0 (./crates/metaharness-tools)
```

### b10x-tool-process

```console
$ cargo tree --locked --offline -p metaharness-tools -i b10x-tool-process -e normal
b10x-tool-process v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
├── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde)
│   └── metaharness-tools v0.15.0 (./crates/metaharness-tools)
└── b10x-tool-toolchain v0.4.0 (https://github.com/beyond10x/tools?tag=0.4.0#6017fccc)
    └── b10x-harness-catalogue v0.18.0 (https://github.com/beyond10x/harness?rev=330e2cde6299e4e55b05664001e73903959a076f#330e2cde) (*)
```

