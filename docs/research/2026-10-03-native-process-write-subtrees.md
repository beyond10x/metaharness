# Explicit native process writes, 2026-10-03

Issue #21 exposed a forwarding gap: the native harness accepts repeated
`--process-write-subtree DIR`, but Metaharness's public declaration and native
argv omitted it. RunSpec, clap, the builder and B10xLaunch now carry the same
explicit list. Empty emits no flag. `write_scope` remains independent. The
adapter's observe-only decision seam is unchanged.

Source inspection covered Harness 0.13.3 revision
798325f03cf5a18df8fadb346d31b314826136ec and the existing Cargo revision
90f10a4314c1c630691c85e812bd8d5d23d73fcc (0.12.1). Both contain the native flag,
`process_workspace_access`, and the ReadOnly default. This corrects the initial
assumption that the flag was necessarily introduced after the pin. No dependency
or compatibility pin was changed. The installed native 0.13.3 and the source
revision are separate observations; source inspection does not qualify an older
installed binary.

Native normalization sorts and deduplicates declarations before the Substrate
0.7.8 validator at 05695970b069f79e6678f2f02cbd78bbe5fa2a56 checks them. The adapter
preserves explicit argv order/duplicates while validating the canonical set:
at most 64 directories, depth at most 64, no absolute/root/traversal/empty
components, NUL or backslash, and no ancestor/descendant overlap. Metaharness's
public declaration additionally rejects `*?[]{}` to exclude glob spelling;
Substrate's path syntax could treat those characters literally. No widening or
glob expansion occurs. The native host still requires existing nonsymlink
directories and enforces its pinned mount admission; this syntax check is not
containment evidence.

The first adapter regression failed with missing public method and validator
errors. After implementation, two adapter tests and two public-builder tests
passed. They cover exact forwarding on both daemon and embedded declarations,
read-only omission, file-scope independence, invalid paths, component/declaration
bounds, duplicate normalization and overlap, plus named unsupported-adapter and
absent-confinement refusals before any injected process spawn. The complete core
and b10x package tests passed. A RunSpec structural test was updated for the new
empty default. CLI parse tests are authored for the combined workspace build.

The existing ESS workspace domain now names four production-target outcomes:
`process-read-only-default`, `process-explicit-subtrees`,
`process-invalid-directory`, and `file-scope-is-not-process-scope`. The target
runs the public builder against an injected process and inspects the native argv;
it does not reimplement the rule as a specification interpreter. Synthesis over
this unit's base produced 40 scenarios, zero refusals. The existing session
budget-overlap candidate-search note remains visible. The coordinator regenerates
and executes the combined suite with the independent quiet-stream amendment;
this file does not claim that yet-unrun combination passed.

Real containment remains a separate native observation. The issue reporter saw
process capability withheld for missing cgroup I/O facts, rather than a confined
Cargo write failure. A staged Rust probe will distinguish declared writes from
undeclared sibling/outside writes where the host admits execution; withholding
must remain explicit, with no unconfined fallback. Toolchain/dependency closure
is separate from this write declaration. No hosted or paid model call is needed.
