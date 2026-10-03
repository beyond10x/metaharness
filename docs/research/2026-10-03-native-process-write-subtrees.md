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

Independent source review identified one CLI representation edge: the native
path grammar permits a directory beginning with `-`, but a separate argv value
could be mistaken for an option. Such a value is now forwarded as
`--process-write-subtree=-out`; it remains the same literal directory. Other
values retain the ordinary repeated flag/value representation. A regression
pins that distinction, without narrowing the native directory syntax.

## Native containment fixture and host limitation

The opt-in b10x fixture now has
`actual_b10x_process_writes_are_contained_or_explicitly_withheld`. It stages the
Rust/clap `process_write_probe` example through the existing `--driver` contract.
On a capable host the actual native `run` tool invokes that immutable executable:
with no declaration, writes to existing target/generated/src directories must
return EROFS; with target and generated declared, those writes must succeed while
src remains EROFS. A host path outside the workspace must stay unwritable. The
probe checks its observed cwd, and the outer fixture checks actual host marker
files and the correlated native tool result. No shell checker or alternate
unconfined process is substituted.

Build the example with `cargo build -p metaharness-b10x --example
process_write_probe`. In addition to the native fixture's existing explicit
binary/evidence variables, name its absolute path in
`METAHARNESS_PROCESS_WRITE_PROBE` and an existing delegated cgroup root in
`METAHARNESS_NATIVE_CGROUP`. Follow the existing evaluation runner's
`systemd-run --user --scope` placement so the test process belongs beneath that
root, then run the complete fixture process in its loopback-only network
namespace. This qualification needs no operator credential or hosted model.

The installed 0.13.3 `tools` probe was executed in an owned user-manager scope
with `/sys/fs/cgroup/user.slice/user-1000.slice/user@1000.service` as its declared
root. It accepted both exact directories but published six tools without `run`.
Its record explicitly withheld process execution because `exec.resource-usage`
was absent: the required cgroup counters include block I/O. The root and its
parent expose only cpu/memory/pids controllers and no io.stat. Repeating in a
separate owned scope with `IOAccounting=yes` produced the same observation.
No global controller or system configuration was changed.

Consequently this host cannot establish native process write containment through
that tool. The production-CLI fixture records `withheld-unverified` when the
native catalogue refuses execution, requires the actual withholding record,
and verifies that no tool call or marker effect occurred. Its passing refusal
branch is **not** a containment-success claim; the staged probe's write assertions
remain unexecuted until a capable host publishes `run`. `containment-result.json`
is retained privately per run to distinguish the two outcomes. The combined CLI
must run this fixture before this new source is treated as native launch evidence.

The combined immutable CLI candidate (SHA256
`484ea4fedfa25d2974982899991e84c9186a0b194ac00cc7d9f736cf6888227b`) subsequently ran
both production-CLI fixture cases. The tightened test reported one passed, zero
failed: both empty/default and explicit target/generated declarations recorded
`withheld-unverified`, specifically naming `run` with a nonblank reason, and no
tool request or marker effect. In that isolated user/network namespace the native
probe withheld `exec.argv-only`; the host `tools` probe outside that namespace had
withheld `exec.resource-usage`. These are distinct observations, not one inferred
cause. The Rust write checker did not execute on either withheld branch, so this
record proves preservation of refusal and no fallback, not successful process
containment. A capable host is still required for the positive mount assertions.
