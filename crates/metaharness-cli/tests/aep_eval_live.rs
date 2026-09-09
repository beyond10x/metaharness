//! `metaharness aep drive eval run` — the gates before a cent is spent, and what the manifest is read out of.
//!
//! # The binary is a tool, and a machine without it is not a red gate
//!
//! The evaluation programme's fourth design constant says it plainly: `metaharness` on `PATH` is a
//! *tool* dependency of the eval runner, like `git`, and an absent binary is a **skip by name**.
//! Every test here that would need it either skips and says so, or proves the refusal instead — and
//! the refusal is what a machine without the binary sees, so the two directions cover each other.
//! Nothing in this file calls a model, and nothing in it can spend money: the one test that walks
//! the whole spawn path points `METAHARNESS_BIN` at a shell script this file writes.
//!
//! # What the streams are
//!
//! Two of the fixtures are the committed eval corpus's own transcripts, ingested unchanged. Three
//! are under `crates/edge/aep-cli/fixtures/eval-run/`, derived from them — the derivation and what
//! is and is not observed about them is in that directory's `README.md`.

mod support;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    support::aep_root()
}

/// Runs `protocol` with `args` and the environment given.
fn protocol_with(args: &[&str], env: &[(&str, &str)]) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_metaharness"));
    command.arg("aep").args(args).current_dir(root());
    // Removed rather than left alone: a developer who exported it for a paid sweep must not turn
    // this suite into one, and a test that asserts the *absence* of the flag has to control it.
    command.env_remove("METAHARNESS_LIVE");
    command.env_remove("METAHARNESS_BIN");
    // A scratch `HOME` unless the test names one: the live-spawn preflight resolves `aep` on the
    // child's `$HOME/.local/bin`, and a developer's own copy there — older, newer, absent — must not
    // decide what this suite reports.
    if !env.iter().any(|(key, _)| *key == "HOME") {
        command.env("HOME", scratch("aep-eval-run-home"));
    }
    for (key, value) in env {
        command.env(key, value);
    }
    command.output().expect("the protocol binary runs")
}

/// Standard output as a string.
fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// Standard error as a string.
fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The exit code, which is part of the contract with a calling harness.
fn code(output: &Output) -> i32 {
    output.status.code().expect("the process exited normally")
}

/// A path as an argument.
fn printable(path: &Path) -> &str {
    path.to_str().expect("a printable path")
}

/// An empty scratch directory to leave a run's products in.
fn scratch(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(name);
    std::fs::remove_dir_all(&directory).ok();
    std::fs::create_dir_all(&directory).expect("the temporary tree is writable");
    directory
}

/// The honest development case: a run that did the two things in the order the workflow requires.
const HONEST_CASE: &str = "conformance/eval/development-honest";

/// The violation case beside it, judged by the same document.
const VIOLATION_CASE: &str = "conformance/eval/development-tests-after-the-code";

/// The violation case's own transcript, whose instrument row is empty: nothing was injected.
const NO_PLUGIN_STREAM: &str = "conformance/eval/development-tests-after-the-code/transcript.jsonl";

/// A stream whose `session.started` attests the installed plugin with a source **and** a digest.
const ATTESTED_STREAM: &str = "crates/edge/aep-cli/fixtures/eval-run/claude-plugin-attested.jsonl";

/// The manifest one run left behind.
fn manifest(out: &Path, name: &str) -> String {
    std::fs::read_to_string(out.join(format!("{name}.manifest.yaml")))
        .unwrap_or_else(|error| panic!("{name} left a manifest: {error}"))
}

// --- crossing #4, on this side ---------------------------------------------------------------

// --- the gates before a spawn ------------------------------------------------------------------

/// The arguments of a spawn — no `--stream`, so every gate below is reached.
fn spawn_args<'a>(out: &'a Path, cwd: &'a Path, extra: &[&'a str]) -> Vec<&'a str> {
    let mut args = vec![
        "drive",
        "eval",
        "run",
        "--case",
        HONEST_CASE,
        "--harness",
        "claude",
        "--observed-at",
        "2026-08-23",
        "--cwd",
        printable(cwd),
        "--plugin-dir",
        "/plugins/aep-plan",
        "--out",
        printable(out),
        "--redact",
    ];
    args.extend_from_slice(extra);
    args
}

#[test]
fn without_the_binary_the_runner_refuses_by_name_and_exits_two() {
    // An explicitly selected missing host retains its named refusal independently of PATH.
    let out = scratch("aep-eval-run-no-binary");
    let cwd = scratch("aep-eval-run-no-binary-tree");
    let missing = out.join("missing-host");
    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw"]),
        &[("METAHARNESS_BIN", printable(&missing))],
    );
    assert_eq!(
        code(&refused),
        2,
        "the tool being absent has its own exit code: {}",
        stderr(&refused)
    );
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-001") && reason.contains("drives it as a tool"),
        "{reason}"
    );
    assert!(
        reason.contains("--stream"),
        "and the refusal names what needs no binary: {reason}"
    );
}

/// A stand-in for the tool, which records what it was called with and prints a canned stream.
///
/// This is what lets the whole spawn path — argv, capture, ingest, budget — be proven for nothing.
/// It is a `sh` script rather than a mock inside the binary on purpose: a mock would test a seam
/// this verb does not have, and the thing being asserted is that a **process** is started with
/// those arguments.
#[cfg(unix)]
fn stub(directory: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt as _;
    let path = directory.join("metaharness-stub");
    // Each argument is written whole and followed by a marker line, because one of them is a
    // multi-line prompt: a stub that separated arguments by newlines would make *the last argument*
    // unrecoverable, which is exactly the one this file asserts on.
    std::fs::write(
        &path,
        "#!/bin/sh\n: > \"$STUB_ARGV\"\nfor word in \"$@\"; do\n  printf '%s\\n%s\\n' \"$word\" \"$ARGV_MARKER\" >> \"$STUB_ARGV\"\ndone\ncat \"$STUB_STREAM\"\n",
    )
    .expect("the scratch tree is writable");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755))
        .expect("the stub can be made executable");
    path
}

#[cfg(unix)]
#[test]
fn a_spawn_without_the_live_flag_is_refused_by_name_and_nothing_is_started() {
    // The tool is present — the stub is right there — and the runner still refuses, which is the
    // point: *installed* is not *permitted to spend*.
    let out = scratch("aep-eval-run-not-live");
    let cwd = scratch("aep-eval-run-not-live-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-002"),
        "{}",
        stderr(&refused)
    );
    assert!(
        !argv.exists(),
        "and the tool was never started: it would have written its arguments here"
    );
}

#[cfg(unix)]
#[test]
fn a_stale_aep_on_the_childs_path_is_refused_before_anything_is_spent() {
    // The session runs on a PATH metaharness constructs — `$HOME/.local/bin` first — so the `aep` a
    // case's task executes can be a different binary from the one launching the run. It was, on
    // 2026-09-03: 0.40.1 there, 0.44.0 here, $10.96 spent before `aep doctor` failed.
    use std::os::unix::fs::PermissionsExt as _;
    let out = scratch("aep-eval-run-stale-child");
    let cwd = scratch("aep-eval-run-stale-child-tree");
    let home = scratch("aep-eval-run-stale-child-home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the scratch home is writable");
    let stale = bin.join("aep");
    std::fs::write(&stale, "#!/bin/sh\necho 'protocol 0.1.0'\n").expect("written");
    std::fs::set_permissions(&stale, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
            ("HOME", printable(&home)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-017")
            && reason.contains("0.1.0")
            && reason.contains(support::aep_package()["version"].as_str().unwrap())
            && reason.contains(printable(&stale)),
        "{reason}"
    );
    assert!(!argv.exists(), "and nothing was started");
}

/// A case whose subject names the `ess-specify` plugin, in a scratch directory of its own.
///
/// Written here because no case in the committed corpus declares a `subject` — the block is read by
/// the preflight and by nothing else, and a corpus case gaining one would be a change to the
/// programme rather than to this test.
#[cfg(unix)]
fn case_that_needs_ess(name: &str) -> PathBuf {
    let directory = scratch(name);
    std::fs::write(
        directory.join("expectations.trace.yaml"),
        "format: trace-spec/1\n\
         id: eval-case/needs-ess\n\
         expectations:\n\
        \x20 - id: nothing-shelled-out\n\
        \x20   expect:\n\
        \x20     tool.absent:\n\
        \x20       tool: Bash\n",
    )
    .expect("the scratch case is writable");
    std::fs::write(
        directory.join("case.yaml"),
        "format: eval-case/1\n\
         id: needs-ess\n\
         workflow: adp/default\n\
         task: draft the domain\n\
         expectations: expectations.trace.yaml\n\
         subject:\n\
        \x20 skills: [ess-specify:specify]\n",
    )
    .expect("the scratch case is writable");
    directory
}

/// Whether the fixed part of the child's `PATH` — the part no test can control — already has `ess`.
///
/// `EVAL-RUN-018` is *no `ess` anywhere the child will look*, and two of the three directories it
/// looks in belong to the machine. A developer who installed `ess` into one of them makes the
/// refusal unreachable, and the honest answer there is a skip by name, exactly as the missing
/// `metaharness` binary gets one at the top of this file.
#[cfg(unix)]
fn ess_is_installed_where_no_test_can_remove_it() -> bool {
    ["/usr/local/bin/ess", "/usr/bin/ess", "/bin/ess"]
        .iter()
        .any(|candidate| Path::new(candidate).exists())
}

#[cfg(unix)]
#[test]
fn a_preflight_with_two_faults_names_both_rather_than_the_first() {
    // Invariant 3: validation accumulates. The stale child `aep` (`EVAL-RUN-017`) and the missing
    // child `ess` (`EVAL-RUN-018`) are independent defects of one machine, and an operator who has
    // both should be told about both — the alternative is two live round trips to learn the second,
    // which is what returning the first refusal bought.
    use std::os::unix::fs::PermissionsExt as _;
    if ess_is_installed_where_no_test_can_remove_it() {
        eprintln!(
            "skipped by name: `ess` is installed in the fixed part of the child's PATH here, so              EVAL-RUN-018 cannot be reached. The gate is green either way — that is what this skip              is for."
        );
        return;
    }
    let out = scratch("aep-eval-run-two-faults");
    let cwd = scratch("aep-eval-run-two-faults-tree");
    let home = scratch("aep-eval-run-two-faults-home");
    let bin = home.join(".local/bin");
    std::fs::create_dir_all(&bin).expect("the scratch home is writable");
    let stale = bin.join("aep");
    std::fs::write(&stale, "#!/bin/sh\necho 'protocol 0.1.0'\n").expect("written");
    std::fs::set_permissions(&stale, std::fs::Permissions::from_mode(0o755)).expect("chmod");
    // …and no `ess` beside it, which is the second fault.
    let case = case_that_needs_ess("aep-eval-run-two-faults-case");
    let binary = stub(&out);
    let argv = out.join("argv");

    let mut invocation = spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]);
    // By the flag rather than by an index, for the reason the sibling below gives: `spawn_args`
    // gained a word when `eval` moved under `drive`.
    let case_flag = invocation
        .iter()
        .position(|word| *word == "--case")
        .expect("`spawn_args` names the case with `--case`");
    invocation[case_flag + 1] = printable(&case);
    let refused = protocol_with(
        &invocation,
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
            ("HOME", printable(&home)),
        ],
    );

    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("2 refusal(s)"),
        "one refusal carrying both, not the first of two: {reason}"
    );
    for code in ["EVAL-RUN-017", "EVAL-RUN-018"] {
        assert_eq!(
            reason.lines().filter(|line| line.contains(code)).count(),
            1,
            "`{code}` is named, on a line of its own: {reason}"
        );
    }
    assert!(
        reason.contains(printable(&stale)) && reason.contains("needs-ess"),
        "and each names what an operator has to go and fix: {reason}"
    );
    assert!(
        !argv.exists(),
        "and both were found before anything was started"
    );
}

#[cfg(unix)]
#[test]
fn a_spawn_with_no_cap_on_what_it_may_spend_is_refused_by_name() {
    let out = scratch("aep-eval-run-no-budget");
    let cwd = scratch("aep-eval-run-no-budget-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "raw"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-003"),
        "{}",
        stderr(&refused)
    );
    assert!(!argv.exists(), "and nothing was started");
}

#[cfg(unix)]
#[test]
fn arm_driven_is_not_launched_here_and_the_refusal_names_the_verb_that_does() {
    // A second way to launch a driven session would be a second policy to forget, which is the
    // mistake `epic:metaharness-migration` retired. The refusal is the design, not a gap.
    let out = scratch("aep-eval-run-driven-spawn");
    let cwd = scratch("aep-eval-run-driven-spawn-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "driven", "--budget-usd", "1.00"]),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-004") && reason.contains("protocol drive run"),
        "the refusal names the verb that does launch one: {reason}"
    );
    assert!(
        reason.contains("--arm driven --stream"),
        "and how a driven run reaches the matrix: {reason}"
    );
    assert!(!argv.exists(), "and nothing was started");
}

/// What the stub was called with, one whole argument per entry.
///
/// Split on the marker the stub writes, because one of the arguments is a multi-line prompt and
/// this file asserts on exactly that one.
#[cfg(unix)]
fn recorded_argv(path: &Path) -> Vec<String> {
    let recorded = std::fs::read_to_string(path).expect("the stub recorded its arguments");
    recorded
        .split(&format!("\n{ARGV_MARKER}\n"))
        .map(ToOwned::to_owned)
        .filter(|word| !word.is_empty())
        .collect()
}

/// The line the stub writes between two arguments.
#[cfg(unix)]
const ARGV_MARKER: &str = "<<<one argument ended>>>";

/// The environment a stubbed spawn runs in.
#[cfg(unix)]
fn stub_env(binary: &Path, argv: &Path, stream: &str) -> Vec<(String, String)> {
    vec![
        ("METAHARNESS_BIN".to_owned(), printable(binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        (
            "STUB_STREAM".to_owned(),
            root().join(stream).display().to_string(),
        ),
    ]
}

/// Borrows an owned environment for [`protocol_with`].
#[cfg(unix)]
fn as_pairs(owned: &[(String, String)]) -> Vec<(&str, &str)> {
    owned
        .iter()
        .map(|(key, value)| (key.as_str(), value.as_str()))
        .collect()
}

#[cfg(unix)]
#[test]
fn a_spawn_gives_arm_raw_the_committed_instructions_and_arm_plugin_the_plugin() {
    // The two treatments, asserted on the argv and the prompt a real process was started with.
    // They are deliberately different: arm a is *text and hope* — the workflow's rendered
    // instructions in front of the task — and arm b's treatment **is** the plugin, so giving it the
    // instructions too would measure both and attribute the result to b.
    let out = scratch("aep-eval-run-treatments");
    let cwd = scratch("aep-eval-run-treatments-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    // --- arm raw ---------------------------------------------------------------------------
    // The violation case, because its own transcript is the one that attests no plugin — and arm
    // `raw` over a stream attesting one is refused, which is the point of the sibling test.
    let owned = stub_env(&binary, &argv, NO_PLUGIN_STREAM);
    let mut raw_args = spawn_args(&out, &cwd, &["--arm", "raw", "--budget-usd", "1.00"]);
    // By the flag rather than by an index: `spawn_args` gained a word when `eval` moved under
    // `drive`, and an index into its argv silently overwrote `--case` instead of its value.
    let case = raw_args
        .iter()
        .position(|word| *word == "--case")
        .expect("`spawn_args` names the case with `--case`");
    raw_args[case + 1] = VIOLATION_CASE;
    let spawned = protocol_with(&raw_args, &as_pairs(&owned));
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let words = recorded_argv(&argv);
    assert_eq!(
        &words[..4],
        &["run", "claude", "--hermetic", "--cwd"],
        "the instrument is the same for every arm: {words:?}"
    );
    assert!(
        words
            .windows(2)
            .any(|pair| pair == ["--decisions", "observe"]),
        "arms a and b record everything and decide nothing: {words:?}"
    );
    assert!(
        !words.iter().any(|word| word == "--plugin-dir"),
        "and arm a is the arm with no plugin in it: {words:?}"
    );
    let prompt = words.last().expect("the prompt is the last argument");
    assert!(
        // The document, not the version. What this row asserts is *arm a is given the rendered
        // instructions and not the raw workflow*; spelling the version out made it a second,
        // unstated assertion that the workflow never changes, which `adp/default/2` then broke
        // without anything being wrong.
        prompt.starts_with("<!-- Rendered from `adp/default/")
            && prompt.contains("` by `protocol govern workflow instruct`"),
        "arm a's treatment is the committed instruction document, in front of the task: {prompt}"
    );
    assert!(
        prompt.contains("Work this through the development workflow"),
        "with the case's own task after it: {prompt}"
    );

    // --- arm plugin -------------------------------------------------------------------------
    let owned = stub_env(&binary, &argv, ATTESTED_STREAM);
    let spawned = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "1.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let words = recorded_argv(&argv);
    assert!(
        words
            .windows(2)
            .any(|pair| pair == ["--plugin-dir", "/plugins/aep-plan"]),
        "arm b's treatment is the explicitly named plugin: {words:?}"
    );
    let prompt = words.last().expect("the prompt is the last argument");
    assert!(
        !prompt.contains("Rendered from"),
        "and arm b gets the task alone, or the two arms would measure the same thing: {prompt}"
    );

    // And the run left the pair the matrix reads.
    assert!(
        out.join("claude-plugin-development-honest.manifest.yaml")
            .exists()
    );
    assert!(
        out.join("claude-plugin-development-honest.report.json")
            .exists()
    );
    assert!(
        out.join("claude-plugin-development-honest.events.jsonl")
            .exists(),
        "beside the stream it was judged over, which is what makes the pair re-derivable"
    );
}

/// The attested claude stream with its terminal cost replaced, written into a scratch file.
///
/// Whatever a fixture happens to cost is not what the ledger has to be tested against: the two
/// answers the ledger distinguishes are *the wire stated a cost* and *the wire stated none*, and a
/// test has to hand it each.
#[cfg(unix)]
fn stream_costing(directory: &Path, name: &str, cost: &str) -> PathBuf {
    let attested = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mut lines: Vec<String> = attested.lines().map(ToOwned::to_owned).collect();
    // The terminal record and not the last line: metaharness closes the stream with its own
    // `stream.closed` marker after it, and the cost is on the session's record.
    let last = lines
        .iter()
        .rposition(|line| {
            serde_json::from_str::<serde_json::Value>(line)
                .is_ok_and(|value| value["event"] == "session.ended")
        })
        .expect("the stream ends a session");
    let mut ended: serde_json::Value =
        serde_json::from_str(&lines[last]).expect("the stream ends a session");
    ended["total_cost_usd"] =
        serde_json::from_str(cost).expect("the cost is JSON — a number or `null`");
    lines[last] = serde_json::to_string(&ended).expect("it serialises");
    let path = directory.join(name);
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("the scratch tree is writable");
    path
}

#[cfg(unix)]
#[test]
fn a_run_whose_stream_states_a_cost_is_charged_that_cost_and_never_the_assumption() {
    // **The live defect, in the place it was visible.** A Claude run stated
    // `0.7977854999999999` — the shortest text that round-trips the `f64` sum of its per-turn
    // costs — and the ledger printed `$0.250000 spent`, because the cost reader refused seventeen
    // significant figures and `.ok()` turned the refusal into *this run stated no cost*. Eighty
    // cents were charged as twenty-five, and the manifest carried no cost at all, so the matrix
    // would have under-reported the sweep as well.
    //
    // Both halves are asserted here: what the ledger charged, and what the manifest kept.
    let out = scratch("aep-eval-run-ledger-states-a-cost");
    let cwd = scratch("aep-eval-run-ledger-states-a-cost-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "priced.jsonl", "0.7977854999999999");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let spawned = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "5.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let said = stdout(&spawned);
    assert!(
        said.contains("charged:  $0.797785 (stated)"),
        "the run is charged what its stream stated, and the line says which of the two numbers \
         the ledger used: {said}"
    );
    assert!(
        said.contains("1 run(s), $0.797785 spent against a cap of $5.000000"),
        "and the sweep's total is that cost, not the assumed rate: {said}"
    );
    assert!(
        !said.contains("$0.250000"),
        "the assumption is nowhere near a run that priced itself: {said}"
    );

    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        written.contains("cost_micro_usd: 797785"),
        "and the manifest keeps it, so the matrix reports what the sweep actually spent:\n{written}"
    );
}

#[cfg(unix)]
#[test]
fn the_assumed_rate_is_charged_only_where_the_stream_priced_nothing() {
    // The other side of the boundary, without which the test above would pass against a ledger
    // that had simply stopped consulting the assumption at all.
    let out = scratch("aep-eval-run-ledger-states-nothing");
    let cwd = scratch("aep-eval-run-ledger-states-nothing-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "unpriced.jsonl", "null");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let spawned = protocol_with(
        &spawn_args(
            &out,
            &cwd,
            &[
                "--arm",
                "plugin",
                "--budget-usd",
                "5.00",
                "--assume-usd-per-run",
                "0.40",
            ],
        ),
        &as_pairs(&owned),
    );
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let said = stdout(&spawned);
    assert!(
        said.contains("charged:  $0.400000 (assumed)"),
        "a wire that priced nothing is charged the assumed rate, and said to be: {said}"
    );
    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        !written.contains("cost_micro_usd"),
        "and the assumption never reaches the manifest, where it would become a measurement:\n\
         {written}"
    );
}

#[cfg(unix)]
#[test]
fn a_stated_cost_this_reader_cannot_convert_stops_the_run_instead_of_becoming_an_estimate() {
    // The failure mode that made the defect silent, refused where it enters. Charging an
    // unreadable cost at the assumed rate is how a sweep under-reports what it spent, and it
    // leaves nothing behind for anyone to notice.
    let out = scratch("aep-eval-run-ledger-unreadable");
    let cwd = scratch("aep-eval-run-ledger-unreadable-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = stream_costing(&out, "unreadable.jsonl", "\"0.80\"");

    let owned = vec![
        ("METAHARNESS_BIN".to_owned(), printable(&binary).to_owned()),
        ("METAHARNESS_LIVE".to_owned(), "1".to_owned()),
        ("STUB_ARGV".to_owned(), printable(&argv).to_owned()),
        ("ARGV_MARKER".to_owned(), ARGV_MARKER.to_owned()),
        ("STUB_STREAM".to_owned(), printable(&stream).to_owned()),
    ];
    let refused = protocol_with(
        &spawn_args(&out, &cwd, &["--arm", "plugin", "--budget-usd", "5.00"]),
        &as_pairs(&owned),
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-STREAM-011") && reason.contains("under-reports"),
        "{reason}"
    );
    assert!(
        !out.join("claude-plugin-development-honest.manifest.yaml")
            .exists(),
        "and no manifest claims a run whose cost nobody could read"
    );
}

#[cfg(unix)]
#[test]
fn the_cap_stops_the_sweep_before_the_run_that_would_pass_it() {
    // Checked before the spawn and against the assumed rate, because the only number available
    // before a run is the assumed one — a cap enforced afterwards is a receipt. The stub's stream
    // costs $0.5216, the cap is $0.60 and an unpriced run is counted at $0.25, so the first run
    // fits and the second cannot.
    let out = scratch("aep-eval-run-budget");
    let cwd = scratch("aep-eval-run-budget-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let stream = root().join(ATTESTED_STREAM).display().to_string();

    let stopped = protocol_with(
        &[
            "drive",
            "eval",
            "run",
            "--case",
            HONEST_CASE,
            "--case",
            VIOLATION_CASE,
            "--arm",
            "plugin",
            "--harness",
            "claude",
            "--observed-at",
            "2026-08-23",
            "--cwd",
            printable(&cwd),
            "--plugin-dir",
            "/plugins/aep-plan",
            "--out",
            printable(&out),
            "--budget-usd",
            "0.60",
            "--redact",
        ],
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
            ("ARGV_MARKER", ARGV_MARKER),
            ("STUB_STREAM", &stream),
        ],
    );
    assert_eq!(code(&stopped), 0, "{}", stderr(&stopped));
    let said = stdout(&stopped);
    assert!(
        said.contains("EVAL-RUN-006") && said.contains("1 run(s), with 1 not launched"),
        "the stop is reported by name, with what was and was not run: {said}"
    );
    assert!(
        said.contains("$0.521600") && said.contains("$0.600000"),
        "and with the numbers it decided on: {said}"
    );
    assert!(
        out.join("claude-plugin-development-honest.manifest.yaml")
            .exists(),
        "the run that fit happened"
    );
    assert!(
        !out.join("claude-plugin-development-tests-after-the-code.manifest.yaml")
            .exists(),
        "and the one that did not fit was never started"
    );
}

// --- a pinned marketplace plugin, forwarded --------------------------------------------------
//
// metaharness 0.5.0 gained `run claude --plugin <marketplace-repo>@<name>@<version-or-commit>`,
// which places a pinned third-party plugin into the scratch config home and attests it beside
// whatever `--plugin-dir` copied. This runner **forwards it verbatim and resolves nothing**: the
// marketplace's layout on disk is metaharness's business, and a second resolver here would be a
// second thing to get wrong. What this side owns is refusing a spelling nobody can reproduce
// before a run is paid for, and keeping the two treatments apart in the manifest.

/// A digest that is obviously synthetic and is the right shape: 64 lowercase hex characters.
fn synthetic_digest(pair: &str) -> String {
    pair.repeat(32)
}

/// The digest `ATTESTED_STREAM` already attests for the plugin `--plugin-dir` copied.
const DIRECTORY_DIGEST: &str = "7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332";

/// The two marketplace spellings these tests declare, in the order they are declared.
///
/// `aep-planning@0.4.0` keeps the old plugin name deliberately: it is a **released** pin, and
/// `beyond10x/agentplugins` really did publish `aep-planning` at `0.4.0` — the rename to `aep-plan`
/// landed at `agentplugins@a2077d2`, long after. Rewriting this to `aep-plan@0.4.0` would name a
/// release that was never cut, and the whole point of these tests is that a declared pin matches
/// what metaharness attests byte for byte. A pin an operator would type *today* is exercised in
/// `src/eval.rs`'s `a_pinned_plugin_reaches_the_argv_with_the_bytes_the_operator_wrote`.
const AEP_PLANNING: &str = "beyond10x/agentplugins@aep-planning@0.4.0";
const DEV_TEAM: &str = "example/review-tools@dev-team@1.4.0";

/// One `hermetic.installed_plugins` entry as metaharness writes it for a **marketplace** plugin.
///
/// `source` is the instrument's own spelling — the declared `<repo>@<name>@<pin>` followed by the
/// marketplace it resolved against — and that is what this reader matches a declaration to. It is
/// not the `loaded_by` prose, which is a sentence and not an identifier.
fn marketplace_entry(spelling: &str, marketplace: &str, digest: &str) -> serde_json::Value {
    let name = spelling
        .split('@')
        .nth(1)
        .expect("a three-segment spelling")
        .to_owned();
    serde_json::json!({
        "name": name,
        "source": format!("{spelling} (marketplace {marketplace})"),
        "installed_at": format!("/scratch/claude-home/plugins/cache/{marketplace}/{name}"),
        "loaded_by": "placed in the scratch config home's plugin registry",
        "digest": digest,
    })
}

/// `ATTESTED_STREAM` with its instrument row replaced, written into a scratch file.
///
/// The same shape as `stream_costing`: whatever the committed fixture happens to attest is not what
/// a rule about *two* attested plugins can be tested against, so the test hands the reader the row
/// it is about.
fn stream_attesting(directory: &Path, name: &str, entries: serde_json::Value) -> PathBuf {
    let attested = std::fs::read_to_string(root().join(ATTESTED_STREAM)).expect("readable");
    let mut lines: Vec<String> = attested.lines().map(ToOwned::to_owned).collect();
    let mut started: serde_json::Value =
        serde_json::from_str(&lines[0]).expect("the stream opens with a session");
    assert_eq!(
        started["event"], "session.started",
        "the instrument's row is on the opening event"
    );
    started["hermetic"]["installed_plugins"] = entries;
    lines[0] = serde_json::to_string(&started).expect("it serialises");
    let path = directory.join(name);
    std::fs::write(&path, format!("{}\n", lines.join("\n"))).expect("the scratch tree is writable");
    path
}

/// The entry `ATTESTED_STREAM` already carries for the directory plugin.
fn directory_entry() -> serde_json::Value {
    serde_json::json!({
        "name": "aep",
        "source": "/plugins/aep-plan",
        "installed_at": "/plugins/aep",
        "loaded_by": "--plugin-dir /plugins/aep",
        "digest": DIRECTORY_DIGEST,
    })
}

#[cfg(unix)]
#[test]
fn a_spawn_forwards_every_pinned_plugin_verbatim_and_the_manifest_keeps_them_apart() {
    // The story's two load-bearing lines in one run: `--plugin` reaches the `metaharness run
    // claude` argv **verbatim and repeatably**, beside `--plugin-dir` rather than instead of it;
    // and the manifest that comes back says which digest belonged to which mechanism, because a
    // matrix row that merged them could not say what was measured.
    let out = scratch("aep-eval-run-marketplace");
    let cwd = scratch("aep-eval-run-marketplace-tree");
    let binary = stub(&out);
    let argv = out.join("argv");
    let planning = synthetic_digest("a1");
    let dev_team = synthetic_digest("b2");
    let stream = stream_attesting(
        &out,
        "three-plugins.jsonl",
        serde_json::json!([
            directory_entry(),
            marketplace_entry(AEP_PLANNING, "beyond10x", &planning),
            marketplace_entry(DEV_TEAM, "bdfinst", &dev_team),
        ]),
    );
    let extra = [
        "--arm",
        "plugin",
        "--budget-usd",
        "1.00",
        "--plugin",
        AEP_PLANNING,
        "--plugin",
        DEV_TEAM,
    ];

    // --- the free half: without the live flag nothing is spawned at all ----------------------
    let refused = protocol_with(
        &spawn_args(&out, &cwd, &extra),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    assert!(
        stderr(&refused).contains("EVAL-RUN-002"),
        "a declared plugin does not make a spawn free: {}",
        stderr(&refused)
    );
    assert!(!argv.exists(), "and the tool was never started");

    // --- the invocation, as a real process received it ---------------------------------------
    let owned = stub_env(&binary, &argv, "");
    let mut environment = as_pairs(&owned);
    let stream_path = printable(&stream).to_owned();
    for pair in &mut environment {
        if pair.0 == "STUB_STREAM" {
            pair.1 = &stream_path;
        }
    }
    let spawned = protocol_with(&spawn_args(&out, &cwd, &extra), &environment);
    assert_eq!(code(&spawned), 0, "{}", stderr(&spawned));

    let words = recorded_argv(&argv);
    assert!(
        words
            .windows(2)
            .any(|pair| pair == ["--plugin-dir", "/plugins/aep-plan"]),
        "the two mechanisms combine rather than exclude each other: {words:?}"
    );
    let forwarded: Vec<&String> = words
        .iter()
        .zip(words.iter().skip(1))
        .filter(|(flag, _)| *flag == "--plugin")
        .map(|(_, value)| value)
        .collect();
    assert_eq!(
        forwarded,
        vec![AEP_PLANNING, DEV_TEAM],
        "every `--plugin` is forwarded verbatim, in the order it was declared. The whole \
         invocation was: {words:?}"
    );

    // --- and the manifest keeps the two apart -------------------------------------------------
    let written = manifest(&out, "claude-plugin-development-honest");
    assert!(
        written.contains(&format!("plugin_digest: {DIRECTORY_DIGEST}")),
        "`plugin_digest` stays the directory treatment's: {written}"
    );
    assert!(
        written.contains(&format!(
            "plugins:\n  - plugin: {AEP_PLANNING}\n    digest: {planning}\n  - plugin: \
             {DEV_TEAM}\n    digest: {dev_team}\n"
        )),
        "and the marketplace plugins are a list beside it, each with the digest the attestation \
         stated: {written}"
    );
}

#[cfg(unix)]
#[test]
fn an_unpinned_plugin_is_refused_before_anything_is_spawned_in_metaharness_own_words() {
    // metaharness refuses this spelling at parse, before a `RunSpec` exists. This runner refuses
    // it before the spawn, with metaharness's own sentence, so an operator meets one refusal
    // rather than paying for a run to be told the same thing by the tool underneath.
    let out = scratch("aep-eval-run-unpinned");
    let cwd = scratch("aep-eval-run-unpinned-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(
            &out,
            &cwd,
            &[
                "--arm",
                "plugin",
                "--budget-usd",
                "1.00",
                "--plugin",
                "beyond10x/agentplugins@aep-plan",
            ],
        ),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-013") && reason.contains("names no pin"),
        "{reason}"
    );
    assert!(
        reason.contains("<repo>@<name>@<version-or-commit>"),
        "and the refusal shows the shape, which is metaharness's own wording: {reason}"
    );
    assert!(
        !argv.exists(),
        "and nothing was started to be refused afterwards"
    );
}

#[cfg(unix)]
#[test]
fn a_marketplace_plugin_is_refused_by_name_on_a_harness_that_has_none() {
    // `--plugin` is Claude Code only, and metaharness refuses it by name on the other kinds rather
    // than accepting and ignoring it. Accepting it here and dropping it at the seam would be the
    // failure that refusal exists to prevent: an operator who declared a plugin believing the run
    // had one.
    for harness in ["codex", "b10x"] {
        let out = scratch(&format!("aep-eval-run-no-marketplace-{harness}"));
        let cwd = scratch(&format!("aep-eval-run-no-marketplace-{harness}-tree"));
        let binary = stub(&out);
        let argv = out.join("argv");
        let mut invocation = spawn_args(
            &out,
            &cwd,
            &[
                "--arm",
                "plugin",
                "--budget-usd",
                "1.00",
                "--plugin",
                AEP_PLANNING,
            ],
        );
        let harness_at = invocation
            .iter()
            .position(|word| *word == "--harness")
            .expect("the arguments name a harness");
        invocation[harness_at + 1] = harness;

        let refused = protocol_with(
            &invocation,
            &[
                ("METAHARNESS_BIN", printable(&binary)),
                ("METAHARNESS_LIVE", "1"),
                ("STUB_ARGV", printable(&argv)),
            ],
        );
        assert_eq!(code(&refused), 1, "{}", stdout(&refused));
        let reason = stderr(&refused);
        assert!(
            reason.contains("EVAL-RUN-014") && reason.contains("marketplace"),
            "{harness}: {reason}"
        );
        assert!(!argv.exists(), "{harness}: and nothing was started");
    }
}

#[cfg(unix)]
#[test]
fn arm_raw_with_a_marketplace_plugin_is_refused_before_the_run_and_not_after_it() {
    // Arm `raw` is the arm with no plugin in it. A stream attesting one is already refused
    // (`EVAL-STREAM-007`) — after the money was spent. The same contradiction is visible before
    // the spawn when it is written on the command line, so that is where it is refused.
    let out = scratch("aep-eval-run-raw-plugin");
    let cwd = scratch("aep-eval-run-raw-plugin-tree");
    let binary = stub(&out);
    let argv = out.join("argv");

    let refused = protocol_with(
        &spawn_args(
            &out,
            &cwd,
            &[
                "--arm",
                "raw",
                "--budget-usd",
                "1.00",
                "--plugin",
                AEP_PLANNING,
            ],
        ),
        &[
            ("METAHARNESS_BIN", printable(&binary)),
            ("METAHARNESS_LIVE", "1"),
            ("STUB_ARGV", printable(&argv)),
        ],
    );
    assert_eq!(code(&refused), 1, "{}", stdout(&refused));
    let reason = stderr(&refused);
    assert!(
        reason.contains("EVAL-RUN-015") && reason.contains("no plugin in it"),
        "{reason}"
    );
    assert!(!argv.exists(), "and nothing was started");
}
