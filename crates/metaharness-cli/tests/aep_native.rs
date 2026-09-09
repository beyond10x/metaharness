//! Native hook and transition contracts migrated from AEP.
mod support;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

/// The repository root.
fn root() -> PathBuf {
    support::aep_root()
}

/// Runs `protocol` with `args` from the repository root.
fn protocol(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .arg("aep")
        .args(args)
        .current_dir(root())
        .output()
        .expect("the protocol binary runs")
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

/// Writes a fixture file, creating the directories above it.
fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the temporary tree is writable");
    }
    std::fs::write(path, contents).expect("the fixture is writable");
}

/// A project with a planning store, a task and a step map, built from scratch.
struct Fixture {
    directory: PathBuf,
}

impl Fixture {
    /// Builds the fixture. `operator` puts an `operator` step at the head of `verify`.
    fn new(name: &str, operator: bool) -> Self {
        let directory = std::env::temp_dir().join(format!("protocol-drive-{name}"));
        std::fs::remove_dir_all(&directory).ok();
        std::fs::create_dir_all(&directory).expect("the temporary tree is writable");

        // The story, the specification of it, and a task that says which story it is: the shape a
        // driven run leaves behind, and the shape `spec-driven` now reads. Without the `specifies`
        // edge and the task's `derived_from` this store is the one run `NATIVE-1/1` walked — an
        // approved specification of nobody's work in particular — and the run stops at
        // `establish_verifiers -> implement`, which is the guard working rather than a defect here.
        write(
            &directory.join(".engineering/planning/story/passkeys.md"),
            "---\nformat: aep.planning-md/1\nid: story:passkeys\nkind: story\nstatus: active\n\
             title: Sign in with a passkey\nsummary: What the work is.\n---\n# Story\n\n\
             Signing in with a passkey replaces the password prompt.\n",
        );
        write(
            &directory.join(".engineering/planning/specification/passkeys.md"),
            "---\nformat: aep.planning-md/1\nid: specification:passkeys\nkind: specification\n\
             status: approved\ntitle: Passkey sign-in\nsummary: What signing in with a passkey \
             must do.\nrelations:\n- specifies: story:passkeys\n---\n# Specification\n\nThe \
             assertion is verified against the stored public key.\n",
        );
        write(
            &directory.join("task.yaml"),
            "id: DRIVE-1\nkind: feature\nobjective: drive-a-workflow\nprotocol: adp/1\n\
             profile: development.standard\nderived_from:\n  - story:passkeys\n",
        );
        write(&directory.join("steps.yaml"), &step_map(operator));

        Self { directory }
    }

    /// The arguments every verb needs.
    fn location(&self) -> Vec<String> {
        vec![
            "--project".to_owned(),
            printable(&self.directory).to_owned(),
            "--root".to_owned(),
            printable(&root()).to_owned(),
            "--task".to_owned(),
            printable(&self.directory.join("task.yaml")).to_owned(),
            "--map".to_owned(),
            printable(&self.directory.join("steps.yaml")).to_owned(),
        ]
    }

    /// Runs one `protocol drive` verb against this fixture.
    ///
    /// A `run` carries `--allow-evidence-gap`, and that is a statement about the fixture rather
    /// than about the flag. This map declares `test_result`, `diff` and `static_analysis` and
    /// nothing else, so F-W4.2-4's launch check refuses it: `spec-driven` wants a `specification`
    /// record and `provenance-tracking` an independent `verification` one, and no step here mints
    /// either. Every test below is about the routing loop, the lock or the report, none of which
    /// that gap changes — so the tests say *I know* rather than growing two steps that write
    /// evidence documents nobody reads. The refusal itself is tested on its own, without the flag,
    /// in `a_map_that_cannot_produce_demanded_evidence_is_refused_before_the_first_step`.
    fn drive(&self, verb: &[&str], extra: &[&str]) -> Output {
        let mut args: Vec<String> = vec!["drive".to_owned()];
        args.extend(verb.iter().map(ToString::to_string));
        args.extend(self.location());
        if verb == ["run"] {
            args.push("--allow-evidence-gap".to_owned());
        }
        args.extend(extra.iter().map(ToString::to_string));
        let borrowed: Vec<&str> = args.iter().map(String::as_str).collect();
        protocol(&borrowed)
    }

    /// The `.engineering/runs` directory.
    fn runs(&self) -> PathBuf {
        self.directory.join(".engineering/runs")
    }

    /// The cursor of one run.
    fn cursor(&self, run: &str) -> serde_json::Value {
        serde_json::from_str(&self.cursor_text(run)).expect("the cursor is JSON")
    }

    /// The cursor of one run as it is written on disk.
    ///
    /// The bytes rather than the parsed document, because `serde` maps an absent key and an
    /// explicit `null` to the same `None`: an invariant about what the document *holds* has to be
    /// asserted on the document.
    fn cursor_text(&self, run: &str) -> String {
        let (task, ordinal) = run.rsplit_once('/').expect("a run id");
        let path = self.runs().join(task).join(ordinal).join("cursor.json");
        std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
    }
}

/// so the pause is exercised rather than only the pre-flight refusal that names it.
fn step_map(operator: bool) -> String {
    let pause = if operator {
        "      - kind: operator\n        prompt: judge this change before the suites run\n"
    } else {
        ""
    };
    format!(
        "format: aep.driver-steps/1\n\
         id: fixture/drive\n\
         workflow: adp/default/2\n\
         states:\n\
        \x20 establish_verifiers:\n\
        \x20   steps:\n\
        \x20     - kind: command\n\
        \x20       description: the red suite\n\
        \x20       run: [sh, -c, \"exit 1\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: unit\n\
        \x20         verifier: test-runner\n\
        \x20 implement:\n\
        \x20   steps:\n\
        \x20     - kind: command\n\
        \x20       description: the working tree changed\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: diff\n\
        \x20         verifier: git\n\
        \x20 verify:\n\
        \x20   steps:\n\
         {pause}\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: unit\n\
        \x20         verifier: test-runner\n\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: test_result\n\
        \x20         suite: contract\n\
        \x20         verifier: test-runner\n\
        \x20     - kind: command\n\
        \x20       run: [sh, -c, \"exit 0\"]\n\
        \x20       evidence:\n\
        \x20         kind: static_analysis\n\
        \x20         verifier: static-analyzer\n"
    )
}

// ---------------------------------------------------------------------------------------------
// `protocol drive transition` — the governor a native flow consults at a section boundary.
// ---------------------------------------------------------------------------------------------

/// Runs `drive transition` against the fixture with `document` on stdin.
fn transition(fixture: &Fixture, extra: &[&str], document: &str) -> Output {
    use std::io::Write as _;
    let mut args: Vec<String> = vec!["drive".to_owned(), "transition".to_owned()];
    args.extend(fixture.location());
    args.extend(extra.iter().map(ToString::to_string));
    let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .arg("aep")
        .args(&args)
        .current_dir(root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the protocol binary runs");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(document.as_bytes())
        .expect("the document is written");
    child.wait_with_output().expect("the process exits")
}

/// The loop's document, as design 0003 § 3 spells it.
fn consultation(path: &str, moment: &str, failed: bool) -> String {
    serde_json::json!({
        "hook": "transition", "flow": "adp/default", "path": path, "moment": moment,
        "attempt": 1, "of": 3, "failed": failed, "handoff": {}, "workspace": "/nowhere"
    })
    .to_string()
}

/// Entering a section with no run behind it proceeds: the engine is put on the state the path
/// names, and that is where it is.
#[test]
fn transition_enter_without_a_run_proceeds_on_a_state_the_workflow_declares() {
    let fixture = Fixture::new("transition-enter", false);
    let output = transition(
        &fixture,
        &[],
        &consultation("root.implement", "enter", false),
    );
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
}

/// Leaving a section whose rung costs evidence the store does not hold is refused, in the
/// engine's words: the reason names the state and what is missing.
#[test]
fn transition_leave_is_refused_by_the_engine_when_the_rung_is_not_earned() {
    let fixture = Fixture::new("transition-leave", false);
    let output = transition(
        &fixture,
        &[],
        &consultation("root.establish_verifiers", "leave", false),
    );
    let text = stdout(&output);
    assert_eq!(code(&output), 2, "{text}{}", stderr(&output));
    let reason: serde_json::Value = serde_json::from_str(text.trim()).expect("a JSON refusal");
    let reason = reason["reason"].as_str().expect("a reason string");
    assert!(
        reason.contains("establish_verifiers"),
        "the refusal names the state it is about:\n{reason}"
    );
}

/// A section that came out failed is left alone: the loop has already recorded the failure and
/// the engine adds nothing (design 0003 § 3, third row).
#[test]
fn transition_leave_of_a_failed_section_proceeds() {
    let fixture = Fixture::new("transition-failed", false);
    let output = transition(
        &fixture,
        &[],
        &consultation("root.establish_verifiers", "leave", true),
    );
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
}

/// A path naming something the workflow does not declare is a refusal, not a guess.
#[test]
fn transition_refuses_a_path_that_names_no_state() {
    let fixture = Fixture::new("transition-unknown", false);
    let output = transition(
        &fixture,
        &[],
        &consultation("root.polishing", "enter", false),
    );
    let text = stdout(&output);
    assert_eq!(code(&output), 2, "{text}{}", stderr(&output));
    assert!(text.contains("polishing"), "{text}");
    assert!(text.contains("not a state"), "{text}");
}

/// A retreat group `<first>-to-<last>` is entered at its first state and left at its last.
#[test]
fn transition_reads_a_retreat_group_as_its_first_and_last_state() {
    let fixture = Fixture::new("transition-group", false);
    // Entering `implement-to-verify` is entering `implement`: no run, so proceed.
    let output = transition(
        &fixture,
        &[],
        &consultation("root.implement-to-verify", "enter", false),
    );
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
    // Leaving it is leaving `verify`, whose rung is not earned in an empty store.
    let output = transition(
        &fixture,
        &[],
        &consultation("root.implement-to-verify", "leave", false),
    );
    let text = stdout(&output);
    assert_eq!(code(&output), 2, "{text}{}", stderr(&output));
    assert!(text.contains("verify"), "{text}");
}

/// A document for another hook point proceeds, said rather than assumed.
#[test]
fn transition_answers_only_the_transition_point() {
    let fixture = Fixture::new("transition-other-point", false);
    let output = transition(
        &fixture,
        &[],
        r#"{"hook":"before-call","entry":"file_write","call":{"arguments":{"path":"x"}}}"#,
    );
    assert_eq!(code(&output), 0, "{}{}", stdout(&output), stderr(&output));
}

/// A document the verb cannot read is neither yes nor no: exit 1, which the loop reads fail
/// closed.
#[test]
fn transition_cannot_answer_an_unreadable_document() {
    let fixture = Fixture::new("transition-unreadable", false);
    let output = transition(&fixture, &[], "this is not JSON");
    assert_eq!(code(&output), 1, "{}{}", stdout(&output), stderr(&output));
    assert!(
        stderr(&output).contains("not the loop's JSON"),
        "{}",
        stderr(&output)
    );
}

/// With `--run`, the engine is positioned on the run's own cursor.
///
/// Entering the state the run is in proceeds; leaving a state the run is *not* in is refused
/// rather than answered from a state the flow only claims — the two disagree about where the work
/// is, and a governor does not guess.
#[test]
fn transition_with_a_run_answers_from_the_runs_cursor() {
    let fixture = Fixture::new("transition-run", false);
    fixture.drive(&["run"], &[]);
    let cursor = fixture.cursor("DRIVE-1/1");
    let state = cursor["state"].as_str().expect("the cursor names a state");

    let output = transition(
        &fixture,
        &["--run", "DRIVE-1/1"],
        &consultation(&format!("root.{state}"), "enter", false),
    );
    assert_eq!(
        code(&output),
        0,
        "entering the state the run is in proceeds:\n{}{}",
        stdout(&output),
        stderr(&output)
    );

    let elsewhere = if state == "verify" {
        "implement"
    } else {
        "verify"
    };
    let output = transition(
        &fixture,
        &["--run", "DRIVE-1/1"],
        &consultation(&format!("root.{elsewhere}"), "leave", false),
    );
    let text = stdout(&output);
    assert_eq!(code(&output), 2, "{text}{}", stderr(&output));
    assert!(text.contains("disagree"), "{text}");
    assert!(
        text.contains(state),
        "the refusal names where the run is:\n{text}"
    );

    // Nothing was written: a governor answers, it does not walk.
    let after = fixture.cursor("DRIVE-1/1");
    assert_eq!(after, cursor, "the cursor is untouched by a consultation");
}

/// A run that does not exist is a verb that cannot answer, not a refusal of the move.
#[test]
fn transition_with_an_unknown_run_cannot_answer() {
    let fixture = Fixture::new("transition-no-run", false);
    let output = transition(
        &fixture,
        &["--run", "DRIVE-9/9"],
        &consultation("root.implement", "enter", false),
    );
    assert_eq!(code(&output), 1, "{}{}", stdout(&output), stderr(&output));
    assert!(
        stderr(&output).contains("no run DRIVE-9/9"),
        "{}",
        stderr(&output)
    );
}

/// The root is asked about first, and it is the flow's own container, not a state: entering it
/// proceeds, and leaving it without a run proceeds — the sections inside were governed one by one.
///
/// The first paid native walk was refused at `enter root` and ran nothing (2026-08-29); this is
/// the test that would have cost nothing to write first.
#[test]
fn transition_proceeds_at_the_root_which_is_a_container_and_not_a_state() {
    let fixture = Fixture::new("transition-root", false);
    for moment in ["enter", "leave"] {
        let output = transition(&fixture, &[], &consultation("root", moment, false));
        assert_eq!(
            code(&output),
            0,
            "{moment} root proceeds without a run:\n{}{}",
            stdout(&output),
            stderr(&output)
        );
    }
}

/// With a run, leaving the root asks the engine whether the task may move on from the cursor —
/// and on a run that stopped short, it may not, in the engine's words.
#[test]
fn transition_leave_root_with_a_run_is_the_engines_answer() {
    let fixture = Fixture::new("transition-root-run", false);
    fixture.drive(&["run"], &[]);
    let output = transition(
        &fixture,
        &["--run", "DRIVE-1/1"],
        &consultation("root", "leave", false),
    );
    let text = stdout(&output);
    assert_eq!(code(&output), 2, "{text}{}", stderr(&output));
    let reason: serde_json::Value = serde_json::from_str(text.trim()).expect("a JSON refusal");
    assert!(
        reason["reason"].as_str().unwrap_or_default().contains(':'),
        "the reason is the engine's: state, then what is unmet\n{text}"
    );
}

// --------------------------------------------------------------- `protocol drive hook`

/// One `before-call` consultation, run through the shipped binary.
///
/// Spawned rather than called, because *the rule is runnable as a program* is the whole claim the
/// native arm rests on: a unit test of the function would hold whether or not the verb existed.
fn hook(document: &str) -> Output {
    use std::io::Write as _;
    let mut child = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["aep", "drive", "hook"])
        .current_dir(root())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the protocol binary runs");
    child
        .stdin
        .take()
        .expect("stdin is piped")
        .write_all(document.as_bytes())
        .expect("the document is written");
    child.wait_with_output().expect("the process exits")
}

/// One `file_edit` call in the loop's own spelling: `path`, `old`, `new`.
fn before_edit(path: &str, old: &str, new: &str) -> String {
    serde_json::json!({
        "hook": "before-call",
        "entry": "file_edit",
        "call": {"arguments": {"path": path, "old": old, "new": new}},
    })
    .to_string()
}

/// The fence rule, spawnable: an edit that crosses a planning document's closing `---` is refused.
///
/// **The fixture reaches the state where the rule decides.** A body edit under the same path is
/// asserted to proceed first, so the refusal below is the fence answering and not the path — the
/// path half of this rule left for the step map's `scope:`, and a test that only checked the
/// refusal would keep passing if this verb went back to refusing everything under the store.
#[test]
fn the_hook_refuses_an_edit_that_crosses_a_planning_documents_fence() {
    let store_file = ".engineering/planning/story/one.md";

    let body = hook(&before_edit(store_file, "a body sentence", "a better one"));
    assert_eq!(
        code(&body),
        0,
        "a targeted body edit is not this rule's business:\n{}{}",
        stdout(&body),
        stderr(&body)
    );

    let crossing = hook(&before_edit(store_file, "---", "-- -"));
    assert_eq!(
        code(&crossing),
        2,
        "exit 2 is how the loop's port reads a refusal:\n{}{}",
        stdout(&crossing),
        stderr(&crossing)
    );
    let reason: serde_json::Value =
        serde_json::from_str(stdout(&crossing).trim()).expect("the refusal is the port's JSON");
    let text = reason["reason"].as_str().expect("a reason for the model");
    assert!(text.contains("frontmatter fence"), "{text}");
    assert!(text.contains(store_file), "and names the document: {text}");

    let written = hook(&before_edit(store_file, "prose", "---\nstatus: done"));
    assert_eq!(
        code(&written),
        2,
        "the replacement text is read as well as the quoted one"
    );

    let elsewhere = hook(&before_edit("docs/design/a.md", "---", "***"));
    assert_eq!(
        code(&elsewhere),
        0,
        "a horizontal rule in a design document is not a store fence"
    );
}

/// The vendor's argument names arrive at this verb too, and mean the same thing.
///
/// The loop sends `path`/`old`/`new`; Claude Code's `Edit` sends `file_path`/`old_string`/
/// `new_string`. A rule that read one spelling silently allowed everything on the other arm once,
/// and the store took a forged `revision: 99` for it.
#[test]
fn the_hook_reads_both_arms_spellings_of_the_same_edit() {
    let document = serde_json::json!({
        "hook": "before-call",
        "entry": "file_edit",
        "call": {"arguments": {
            "file_path": ".engineering/planning/story/one.md",
            "old_string": "---",
            "new_string": "x",
        }},
    })
    .to_string();
    let output = hook(&document);
    assert_eq!(code(&output), 2, "{}{}", stdout(&output), stderr(&output));
}

/// `file_write` is not this verb's question: a whole file is the step map's `scope:`.
///
/// The path-and-granularity half of the old `store_integrity` is declared in the map and travels
/// to this arm as `--write-scope`, which the loop's own tools enforce before a hook is spawned.
/// Answering it here as well would be a second copy of one rule.
#[test]
fn the_hook_leaves_a_whole_file_write_to_the_declared_scope() {
    let document = serde_json::json!({
        "hook": "before-call",
        "entry": "file_write",
        "call": {"arguments": {
            "path": ".engineering/planning/story/one.md",
            "text": "---\nid: story:forged\n---\n",
        }},
    })
    .to_string();
    let output = hook(&document);
    assert_eq!(
        code(&output),
        0,
        "the hook proceeds; the scope is what refuses this:\n{}{}",
        stdout(&output),
        stderr(&output)
    );
}

/// A document this program cannot read is neither yes nor no: the loop's port reads it fail closed.
#[test]
fn the_hook_cannot_answer_an_unreadable_document() {
    let output = hook("this is not JSON");
    assert_eq!(code(&output), 1, "{}{}", stdout(&output), stderr(&output));
    assert!(stderr(&output).contains("not JSON"), "{}", stderr(&output));
}
