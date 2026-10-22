//! Bounded ESS suite executed against production seams; unsupported steps fail closed.
use super::*;
use metaharness::protocol::{
    CLOSED_FAMILY, CloseReason, Emission, Event, EventLine, EventStream, RunId, TraceIrDocument,
    UNK_FAMILY, parse_event_line, project_document,
};
use serde_json::{Value, json};

fn suite() -> Value {
    serde_json::from_str(include_str!("../../../../spec/conformance.json")).unwrap()
}

#[derive(Default)]
struct Observed {
    outcome: String,
    error: Option<String>,
    events: std::collections::BTreeMap<String, Value>,
    retained: Option<(tempfile::TempDir, String, String)>,
}

fn event(outcome: &str, name: &str, payload: Value) -> Observed {
    Observed {
        outcome: outcome.to_owned(),
        events: [(name.to_owned(), payload)].into(),
        ..Observed::default()
    }
}

fn checked(ok: bool, accepted: &str, refused: &str, error: &str) -> Observed {
    Observed {
        outcome: if ok { accepted } else { refused }.to_owned(),
        error: (!ok).then(|| error.to_owned()),
        ..Observed::default()
    }
}

/// Amendment a27: a real `--hermetic strict` Claude run over the scripted runner in a directory
/// this target made, with the H7 state and H11 verdict read off `session.started`'s attestation and
/// the exit code off the production floor. The outcome is derived from what was observed, never
/// from the probe, so a shape the domain does not declare fails by scenario name.
fn scratch_cwd_plan(input: &Value) -> Observed {
    use metaharness::protocol::{CredentialSource, Event, HermeticMode, HermeticRow, Kind};
    use metaharness::{
        FakeAuditor, Input, Metaharness, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let probe = input["probe"].as_str().unwrap();
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    fs::create_dir(&copy).unwrap();
    let builder = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None);
    let builder = match probe {
        "undirected" => builder.with_scratch_cwd(),
        "declared" => builder.with_cwd(&copy).with_scratch_cwd(),
        "declared-under-memory" => {
            fs::write(root.path().join("AGENTS.md"), "above the copy").unwrap();
            builder.with_cwd(&copy).with_scratch_cwd()
        }
        "named-only" => builder.with_cwd(&copy),
        other => panic!("unsupported scratch cwd probe {other}"),
    };
    let canonical = fs::canonicalize(&copy).unwrap();
    // The record must sit on the adapter's pin or H9 fails the strict run for a reason this
    // command does not test; the pin is read off the adapter's own capabilities, never spelled.
    let pinned = metaharness::capabilities(Kind::Claude)
        .unwrap()
        .versions_pinned;
    let opening = format!(
        r#"{{"emit":"session.started","harness_version":"{}","output_style":"default","plugins":[],"mcp_servers":[],"credential_source":"none","cwd":"{}"}}"#,
        pinned[0],
        canonical.display()
    );
    let closing = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#.to_owned();
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([opening, closing], log.clone());
    let mut seams = ScriptedSeams;
    let mut run =
        match builder.start_with(Input::Prompt("fixture".to_owned()), &mut runner, &mut seams) {
            Ok(run) => run,
            Err(refusal) => {
                assert_eq!(log.spawns(), 0, "a refused declaration must spawn nothing");
                assert!(refusal.to_string().contains("--scratch-cwd"), "{refusal}");
                return checked(
                    false,
                    "",
                    "declaration-without-directory",
                    "metaharness.workspace.DeclarationWithoutDirectory",
                );
            }
        };
    run.drain().unwrap();
    // The directory the child ran in, read off the launch plan the runner was handed (the argv
    // carries no cwd), so the scenario rests on the plan and not on the scripted record alone.
    assert_eq!(log.launched_cwds(), vec![canonical]);
    let attestation = run
        .events()
        .iter()
        .find_map(|event| {
            if let Event::SessionStarted { hermetic, .. } = event {
                Some(hermetic.clone())
            } else {
                None
            }
        })
        .unwrap();
    let h7 = if attestation.declares(HermeticRow::H7) {
        "declared"
    } else if attestation
        .unavailable
        .iter()
        .any(|control| control.row == HermeticRow::H7)
    {
        "unavailable"
    } else {
        "imposed"
    };
    let h11_imposed = attestation.claims(HermeticRow::H11);
    // Amendment a36: a memory file above the copy is named in the imposed H11 row, never refused,
    // and never decides the row. Read off the row's own text, by the path the walk resolves.
    let planted_above = fs::canonicalize(root.path())
        .unwrap()
        .join("AGENTS.md")
        .display()
        .to_string();
    let memory_above_reported = attestation
        .imposed
        .iter()
        .any(|control| control.row == HermeticRow::H11 && control.how.contains(&planted_above));
    let report = run.audit(&mut FakeAuditor::missing()).unwrap();
    let exit_code = run.exit(Some(&report)).code();
    let outcome = match (h7, h11_imposed, exit_code, memory_above_reported) {
        ("declared", true, 0, false) => "strict-admits-the-declared-copy".to_owned(),
        ("declared", true, 0, true) => "memory-above-the-copy-is-reported".to_owned(),
        ("unavailable", false, 1, false) => "named-without-declaration-keeps-a6".to_owned(),
        other => format!("undeclared-shape:{other:?}"),
    };
    event(
        &outcome,
        "metaharness.workspace.ScratchCwdAttested",
        json!({
            "h7": h7,
            "h11_imposed": h11_imposed,
            "exit_code": exit_code,
            "memory_above_reported": memory_above_reported,
        }),
    )
}

/// `/bin/sh` in place of the vendor binary, under the production spawn runner: it prints the two
/// records it was handed and then runs its ending, so the child's exit is the runner's measurement.
struct ShellChild {
    ending: &'static str,
    records: [String; 2],
}

impl metaharness::ProcessRunner for ShellChild {
    fn requires_executable(&self) -> bool {
        false
    }

    fn start(
        &mut self,
        plan: &metaharness::LaunchPlanView<'_>,
    ) -> std::io::Result<Box<dyn metaharness::HarnessProcess>> {
        let [opening, closing] = &self.records;
        let args = vec![
            "-c".to_owned(),
            format!("printf '%s\\n' \"$1\" \"$2\"; {}", self.ending),
            "sh".to_owned(),
            opening.clone(),
            closing.clone(),
        ];
        metaharness::ProcessRunner::start(
            &mut metaharness::SpawnRunner::new(),
            &metaharness::LaunchPlanView {
                program: "/bin/sh",
                args: &args,
                ..*plan
            },
        )
    }
}

/// The opening and terminal records a Claude child prints for the declared opening: every field the
/// floor reads on the adapter's pin, no plugin and no MCP server list, or a version off the pin.
fn audited_records(opening: &str, cwd: &Path) -> [String; 2] {
    let pinned = metaharness::capabilities(metaharness::protocol::Kind::Claude)
        .unwrap()
        .versions_pinned;
    let mut record = json!({
        "emit": "session.started",
        "harness_version": pinned[0],
        "output_style": "default",
        "plugins": [],
        "mcp_servers": [],
        "credential_source": "none",
        "cwd": cwd.display().to_string(),
    });
    match opening {
        "complete" => {}
        "silent-on-plugins-and-mcp" => {
            let fields = record.as_object_mut().unwrap();
            fields.remove("plugins");
            fields.remove("mcp_servers");
        }
        "off-the-pin" => record["harness_version"] = json!("0.0.0-off-the-pin"),
        other => panic!("unsupported opening record {other}"),
    }
    [
        record.to_string(),
        r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#.to_owned(),
    ]
}

/// The domain word for the report line that starts with `prefix`, mapped only from the exact
/// printed words listed: anything else is `unnamed:<line>`, which no declared variant matches.
fn printed_word(rendered: &str, prefix: &str, words: &[(&str, &str)]) -> String {
    let line = rendered
        .lines()
        .find_map(|line| line.strip_prefix(prefix))
        .unwrap_or("<absent>");
    words
        .iter()
        .find(|(printed, _)| *printed == line)
        .map_or_else(|| format!("unnamed:{line}"), |(_, word)| (*word).to_owned())
}

/// Amendment a48: a real `--hermetic strict` Claude run over a declared scratch copy, audited. A
/// measured child is [`ShellChild`], which exits `0`, exits `7` or kills itself; an unknown child
/// is the scripted runner, which measures nothing. The three verdict lines are read off the
/// rendered report and the exit code off the production floor, and each is mapped back only from
/// the exact words this fixture can produce, so a line the domain does not declare fails by
/// scenario name instead of being read as the nearest variant.
fn audited_run_exit(input: &Value) -> Observed {
    use metaharness::protocol::{CredentialSource, HermeticMode, Kind};
    use metaharness::{
        FakeAuditor, Input, Metaharness, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    fs::create_dir(&copy).unwrap();
    let records = audited_records(
        input["opening"].as_str().unwrap(),
        &fs::canonicalize(&copy).unwrap(),
    );
    let shell = |ending| -> Box<dyn metaharness::ProcessRunner> {
        Box::new(ShellChild {
            ending,
            records: records.clone(),
        })
    };
    let mut runner: Box<dyn metaharness::ProcessRunner> = match input["child"].as_str().unwrap() {
        "exited-zero" => shell("exit 0"),
        "exited-nonzero" => shell("exit 7"),
        "signaled" => shell("kill -KILL $$"),
        "unknown" => Box::new(ScriptedRunner::of_lines(
            records.clone(),
            ScriptedLog::new(),
        )),
        other => panic!("unsupported child exit {other}"),
    };
    let mut run = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_cwd(&copy)
        .with_scratch_cwd()
        .start_with(
            Input::Prompt("fixture".to_owned()),
            &mut *runner,
            &mut ScriptedSeams,
        )
        .unwrap();
    run.drain().unwrap();
    let report = run.audit(&mut FakeAuditor::missing()).unwrap();
    let exit_code = run.exit(Some(&report)).code();
    let rendered = report.render();
    let named = |prefix: &str, words: &[(&str, &str)]| printed_word(&rendered, prefix, words);
    let child_exit = named(
        "child exit: ",
        &[
            ("0", "exited-zero"),
            ("7", "exited-nonzero"),
            ("signal 9", "signaled"),
            ("unknown", "unknown"),
        ],
    );
    let control = named(
        "control verdict: ",
        &[("ok", "ok"), ("unk", "unk"), ("gap", "gap")],
    );
    let audit = named(
        "audit verdict: ",
        &[
            ("ok (exit 0)", "ok"),
            ("gap (exit 1)", "gap"),
            ("no verdict (exit 3)", "no-verdict"),
        ],
    );
    let outcome = match (control.as_str(), audit.as_str(), exit_code) {
        ("gap", "gap", 1) => "a-gating-row-failed".to_owned(),
        ("unk", "no-verdict", 3) => "nobody-found-out".to_owned(),
        ("ok", "ok", 0) => "every-control-held".to_owned(),
        other => format!("undeclared-shape:{other:?}"),
    };
    event(
        &outcome,
        "metaharness.session.AuditedRunExited",
        json!({
            "child_exit": child_exit,
            "control": control,
            "audit": audit,
            "exit_code": exit_code,
        }),
    )
}

/// The generated `ExitAuditedRun` scenarios fix the child at the enum's first variant: no guard
/// reads it, and the payload echoes it. Each is run again with every child the event's own shape
/// declares, through the same production arm, so the measured and unmeasured children are driven
/// by the specification's expectations rather than by a table spelled here (amendment a48).
#[test]
fn every_declared_child_exit_drives_the_audited_run_scenarios() {
    let suite = suite();
    let mut driven = Vec::new();
    for (id, scenario) in suite["scenarios"].as_object().unwrap() {
        if !id.starts_with("metaharness.session.ExitAuditedRun/") {
            continue;
        }
        let steps = scenario["steps"].as_array().unwrap();
        let children = steps
            .iter()
            .find(|step| {
                step["step"] == "expect_event"
                    && step["event"] == "metaharness.session.AuditedRunExited"
            })
            .and_then(|step| step["shape"]["child_exit"]["variants"].as_array())
            .unwrap()
            .clone();
        for child in children {
            let mut varied = scenario.clone();
            for step in varied["steps"].as_array_mut().unwrap() {
                if step["step"] == "execute_command" {
                    step["input"]["child"]["value"] = child.clone();
                }
                if step["step"] == "expect_event"
                    && step["event"] == "metaharness.session.AuditedRunExited"
                {
                    step["payload"]["child_exit"] = child.clone();
                }
            }
            if let Err(reason) = run_scenario(&varied) {
                panic!("{id} with child {child}: {reason}");
            }
            driven.push(format!("{id} {child}"));
        }
    }
    assert_eq!(
        driven.len(),
        12,
        "three outcomes, four children: {driven:#?}"
    );
}

/// Run git for the fixture tree with no operator configuration.
fn fixture_git(directory: &Path, args: &[&str]) {
    let status = Process::new("git")
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .args(args)
        .status()
        .expect("git is a gate prerequisite");
    assert!(status.success(), "git {args:?}");
}

/// The two tracked files amendment a35's accepted probe hands over.
const CONTEXT_FIXTURE_FILES: [(&str, &str); 2] = [
    ("todo.py", "def add(items, item):\n    items.append(item)\n"),
    (
        "tests/test_todo.py",
        "from todo import add\n\n\ndef test_add():\n    add([], 1)\n",
    ),
];

/// The tree amendment a35's probes run in, and what the probe declares: `work/`, whose `.git` is a
/// gitfile naming the sibling `gitdir/` — the shape an embedder's scratch copy has — holding the
/// two tracked files, a tracked symbolic link, an untracked file, and whatever the probe adds.
fn context_fixture(probe: &str) -> (tempfile::TempDir, PathBuf, Vec<PathBuf>) {
    use std::os::unix::ffi::OsStrExt as _;
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("work");
    let gitdir = root.path().join("gitdir");
    fixture_git(
        root.path(),
        &[
            "init",
            "--quiet",
            "--separate-git-dir",
            gitdir.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    assert!(work.join(".git").is_file(), "the copy's .git is a gitfile");
    fs::create_dir(work.join("tests")).unwrap();
    for (path, text) in CONTEXT_FIXTURE_FILES {
        fs::write(work.join(path), text).unwrap();
    }
    std::os::unix::fs::symlink("todo.py", work.join("link.py")).unwrap();
    fs::write(work.join("untracked.txt"), "not in the index\n").unwrap();
    let mut tracked: Vec<String> = vec![
        "todo.py".into(),
        "tests/test_todo.py".into(),
        "link.py".into(),
    ];
    let declared: Vec<PathBuf> = match probe {
        "tracked" | "undirected" => CONTEXT_FIXTURE_FILES
            .iter()
            .map(|(path, _)| PathBuf::from(path))
            .collect(),
        "empty-path" => vec![PathBuf::new()],
        "absolute" => vec![work.join("todo.py")],
        "climbing" => vec!["tests/../todo.py".into()],
        "non-utf8-path" => vec![PathBuf::from(std::ffi::OsStr::from_bytes(b"bad-\xff.txt"))],
        "symlink" => vec!["link.py".into()],
        "irregular" => vec!["tests".into()],
        "absent" => vec!["absent.py".into()],
        "not-text" => {
            fs::write(work.join("blob.bin"), [0xff_u8, 0xfe, 0x00, 0x41]).unwrap();
            tracked.push("blob.bin".into());
            vec!["blob.bin".into()]
        }
        "untracked" => vec!["untracked.txt".into()],
        "over-count" => {
            let names: Vec<String> = (0..=metaharness::protocol::CONTEXT_FILES_MAX)
                .map(|index| format!("f{index:02}.txt"))
                .collect();
            for name in &names {
                fs::write(work.join(name), "x\n").unwrap();
            }
            tracked.extend(names.iter().cloned());
            names.into_iter().map(PathBuf::from).collect()
        }
        "over-bytes" => {
            let big = "y".repeat(metaharness::protocol::CONTEXT_BYTES_MAX);
            fs::write(work.join("big.txt"), big).unwrap();
            tracked.push("big.txt".into());
            vec!["big.txt".into()]
        }
        other => panic!("unsupported context file probe {other}"),
    };
    let mut add = vec!["add".to_owned(), "--".to_owned()];
    add.extend(tracked);
    fixture_git(&work, &add.iter().map(String::as_str).collect::<Vec<_>>());
    (root, work, declared)
}

/// One kind's start over the fixture: the opening record's `context_files`, or the outcome name of
/// the refusal. A refusal must spawn nothing and name a declared path.
fn context_start(
    kind: metaharness::protocol::Kind,
    work: Option<&Path>,
    declared: &[PathBuf],
) -> Result<Vec<metaharness::protocol::ContextFile>, &'static str> {
    use metaharness::protocol::{CredentialSource, DecisionMode, Event, Kind};
    use metaharness::{
        ContextRefusal, Input, Metaharness, Refusal, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let mut builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    if kind == Kind::B10x {
        builder = builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("ess-fixture");
    }
    if let Some(work) = work {
        builder = builder.with_cwd(work).with_scratch_cwd();
    }
    for path in declared {
        builder = builder.with_context(path);
    }
    let opening = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
    let closing = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([opening, closing], log.clone());
    match builder.start_with(
        Input::Prompt("ess".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    ) {
        Ok(mut run) => {
            run.drain().unwrap();
            assert_eq!(log.spawns(), 1, "{kind:?}");
            Ok(run
                .events()
                .iter()
                .find_map(|event| match event {
                    Event::SessionStarted { context_files, .. } => Some(context_files.clone()),
                    _ => None,
                })
                .unwrap_or_default())
        }
        Err(refusal) => {
            assert_eq!(log.spawns(), 0, "{kind:?}: a refusal spawns nothing");
            let text = refusal.to_string();
            let Refusal::ContextFile { path, why } = refusal else {
                panic!("{kind:?}: not a context refusal: {text}");
            };
            assert!(
                declared.contains(&path),
                "{kind:?} names a path that was not declared: {text}"
            );
            // Exhaustive and without a wildcard: a refusal production adds without an outcome in
            // the domain fails to compile here (correction round 2).
            Err(match why {
                ContextRefusal::WithoutCwd => "context-without-cwd",
                ContextRefusal::Empty => "empty-path-refused",
                ContextRefusal::Absolute => "absolute-path-refused",
                ContextRefusal::Escapes => "climbing-path-refused",
                ContextRefusal::PathNotUtf8 => "non-utf8-path-refused",
                ContextRefusal::Symlink => "symlink-refused",
                ContextRefusal::NotRegularFile => "irregular-file-refused",
                ContextRefusal::Unreadable { .. } => "unreadable-file-refused",
                ContextRefusal::NotText { .. } => "not-text-refused",
                ContextRefusal::Untracked { .. } => "untracked-file-refused",
                ContextRefusal::CountCap { .. } => "count-cap-refused",
                ContextRefusal::ByteCap { .. } => "byte-cap-refused",
            })
        }
    }
}

/// Amendment a35: one declaration per probe, started on **all three kinds** over the scripted
/// runner in the tree [`context_fixture`] made. A refusal must reach every kind before the spawn
/// with the same reason; an accepted declaration is read off each run's
/// `session.started.context_files`, every digest compared with the tree's own bytes. The outcome is
/// derived from what was observed, never from the probe, so a kind that disagrees fails by name.
fn context_files_plan(input: &Value) -> Observed {
    use metaharness::protocol::{Digest, Kind};
    let probe = input["probe"].as_str().unwrap();
    let (_root, work, declared) = context_fixture(probe);
    let work = (probe != "undirected").then_some(work.as_path());
    let mut outcomes = Vec::new();
    let mut recorded = Vec::new();
    for kind in [Kind::Claude, Kind::Codex, Kind::B10x] {
        match context_start(kind, work, &declared) {
            Ok(listed) => {
                outcomes.push("tracked-files-recorded");
                recorded.push(listed);
            }
            Err(outcome) => outcomes.push(outcome),
        }
    }
    outcomes.dedup();
    let [outcome] = outcomes.as_slice() else {
        return checked(false, "", &format!("kinds-disagree:{outcomes:?}"), "");
    };
    let error = match *outcome {
        "tracked-files-recorded" => {
            let digests_match = recorded.iter().all(|listed| {
                listed.len() == CONTEXT_FIXTURE_FILES.len()
                    && listed
                        .iter()
                        .zip(CONTEXT_FIXTURE_FILES)
                        .all(|(entry, (path, text))| {
                            entry.path == path
                                && entry.sha256 == Digest::of(text.as_bytes())
                                && entry.bytes == text.len() as u64
                        })
            });
            let files = recorded
                .iter()
                .map(Vec::len)
                .reduce(|left, right| if left == right { left } else { 0 })
                .unwrap_or(0);
            return event(
                outcome,
                "metaharness.workspace.ContextFilesRecorded",
                json!({"files": files, "kinds": recorded.len(), "digests_match": digests_match}),
            );
        }
        "context-without-cwd" => "metaharness.workspace.ContextWithoutDirectory",
        "empty-path-refused" => "metaharness.workspace.ContextPathEmpty",
        "absolute-path-refused" => "metaharness.workspace.ContextPathAbsolute",
        "climbing-path-refused" => "metaharness.workspace.ContextPathEscapes",
        "non-utf8-path-refused" => "metaharness.workspace.ContextPathNotUtf8",
        "symlink-refused" => "metaharness.workspace.ContextSymlink",
        "irregular-file-refused" => "metaharness.workspace.ContextNotRegularFile",
        "unreadable-file-refused" => "metaharness.workspace.ContextUnreadable",
        "not-text-refused" => "metaharness.workspace.ContextNotText",
        "untracked-file-refused" => "metaharness.workspace.ContextUntracked",
        "count-cap-refused" => "metaharness.workspace.ContextCountCap",
        "byte-cap-refused" => "metaharness.workspace.ContextByteCap",
        other => unreachable!("{other}"),
    };
    checked(false, "", outcome, error)
}

fn process_write_plan(input: &Value) -> Observed {
    use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
    use metaharness::{Input, Metaharness, ScriptedLog, ScriptedRunner};
    use metaharness_b10x::B10xSeams;
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("work");
    fs::create_dir(&cwd).unwrap();
    let mut builder = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model("ess-fixture")
        .with_model_endpoint("http://fixture.invalid")
        .with_cwd(cwd)
        .with_substrate_embedded(true);
    let probe = input["probe"].as_str().unwrap();
    builder = match probe {
        "empty" => builder,
        "explicit" => builder
            .with_process_write_subtree("target")
            .with_process_write_subtree("generated"),
        "invalid" => builder.with_process_write_subtree("../outside"),
        "file-scope-only" => builder.with_write_scope("**=allowed"),
        other => panic!("unsupported process write probe {other}"),
    };
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines(Vec::<String>::new(), log.clone());
    let result = builder.start_with(
        Input::Prompt("fixture".to_owned()),
        &mut runner,
        &mut B10xSeams::new(None, None),
    );
    let admitted = result.is_ok();
    let launched = log.launched();
    let paths: Vec<&str> = launched
        .iter()
        .flat_map(|argv| argv.windows(2))
        .filter(|pair| pair[0] == "--process-write-subtree")
        .map(|pair| pair[1].as_str())
        .collect();
    let count = paths.len();
    let outcome = if !admitted {
        assert_eq!(log.spawns(), 0);
        "process-invalid-directory"
    } else if count == 2 {
        assert_eq!(paths, ["target", "generated"]);
        "process-explicit-subtrees"
    } else {
        assert_eq!(
            count, 0,
            "an unexpected process grant must fail conformance"
        );
        if launched
            .iter()
            .any(|argv| argv.iter().any(|arg| arg == "--write-scope"))
        {
            "file-scope-is-not-process-scope"
        } else {
            "process-read-only-default"
        }
    };
    event(
        outcome,
        "metaharness.workspace.ProcessWritePlanned",
        json!({"admitted":admitted,"count":count}),
    )
}

fn completion(input: &Value) -> Observed {
    let record = metaharness_codex::conformance::completion(
        input["error_property"].as_str().unwrap(),
        input["last_message"].as_str().unwrap(),
    )
    .unwrap();
    let Event::SessionEnded {
        is_error,
        total_cost_usd,
        ..
    } = &record
    else {
        panic!("no terminal observation")
    };
    assert_eq!(*total_cost_usd, None, "unknown evidence is never zero cost");
    let (outcome, verdict) = match is_error {
        Some(true) => ("explicit-failure", "failure"),
        Some(false) => ("positive-completion", "success"),
        None => ("incomplete-evidence", "unknown"),
    };
    event(
        outcome,
        "metaharness.session.CompletionClassified",
        json!({"verdict":verdict}),
    )
}

fn closed_stream(input: &Value) -> Event {
    use metaharness::{
        Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner, ScriptedSeams,
    };
    let mut terminal = input.clone();
    terminal.as_object_mut().unwrap().remove("terminal_present");
    terminal["emit"] = json!("session.ended");
    let script = if input["terminal_present"] == true {
        vec![ScriptStep::line(terminal.to_string())]
    } else {
        vec![]
    };
    let mut runner = ScriptedRunner::new(script, ScriptedLog::new());
    let mut run = Metaharness::new(metaharness::protocol::Kind::Claude)
        .with_decisions(metaharness::protocol::DecisionMode::Observe)
        .start_with_clock(
            Input::Prompt("synthetic".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .unwrap();
    run.drain().unwrap();
    let Event::StreamClosed { events, .. } = run.events().last().unwrap() else {
        panic!("no closure")
    };
    assert_eq!(*events, u64::try_from(run.events().len() - 1).unwrap());
    run.events().last().unwrap().clone()
}

fn closure(input: &Value) -> Observed {
    let Event::StreamClosed { reason, .. } = closed_stream(input) else {
        unreachable!()
    };
    let reason = serde_json::to_value(reason).unwrap();
    let outcome = match reason.as_str().unwrap() {
        "completed" => "positive-terminal-record",
        "budget" => "observed-budget-stop",
        "error" => "failed-or-unknown-terminal",
        other => panic!("unexpected closure {other}"),
    };
    event(
        outcome,
        "metaharness.session.StreamClassified",
        json!({"reason":reason}),
    )
}

fn polling(input: &Value) -> Observed {
    use metaharness::protocol::{Command, CommandOutcome, CredentialSource, DecisionMode, Kind};
    use metaharness::{
        EventPoll, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner,
        ScriptedSeams,
    };
    if input["probe"] == "quiet-wireless-interrupt" {
        return wireless_interrupt();
    }
    let probe = input["probe"].as_str().unwrap();
    let mut script = vec![ScriptStep::line(r#"{"emit":"session.started"}"#)];
    if probe == "pending-halt" {
        script.extend([
            ScriptStep::line(r#"{"emit":"tool.requested","call_id":"t1","name":"Bash","input":{"command":"ls"}}"#),
            ScriptStep::awaiting("t1"),
        ]);
    } else {
        script.extend([ScriptStep::Idle, ScriptStep::Idle]);
    }
    let log = ScriptedLog::new();
    let clock = ManualClock::new();
    let mut runner = ScriptedRunner::new(script, log.clone());
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Ask)
        .start_with_clock(
            Input::Prompt("poll fixture".into()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(clock.clone()),
        )
        .unwrap();
    let mut yielded = false;
    for _ in 0..32 {
        match run.poll_event().unwrap() {
            EventPoll::Event(_) | EventPoll::Progress => (),
            EventPoll::Idle => {
                yielded = probe != "pending-halt";
                break;
            }
            EventPoll::DecisionPending => {
                yielded = probe == "pending-halt";
                break;
            }
            EventPoll::Ended => break,
        }
    }
    let (outcome, control) = match probe {
        "quiet-halt" => (
            "quiet-native-halt",
            Some(Command::Halt {
                reason: "fixture".into(),
            }),
        ),
        "quiet-interrupt" => (
            "quiet-native-interrupt",
            Some(Command::Interrupt {
                reason: "fixture".into(),
            }),
        ),
        "pending-halt" => (
            "pending-decision-steering",
            Some(Command::Halt {
                reason: "fixture".into(),
            }),
        ),
        "quiet-only" => ("quiet-output-is-not-eof", None),
        other => panic!("unknown probe {other}"),
    };
    let control_name = control.as_ref().map(Command::name);
    if let Some(command) = control {
        assert!(matches!(
            run.send_as("ess-steer", command).unwrap(),
            CommandOutcome::Ok { .. }
        ));
        for _ in 0..32 {
            if !matches!(
                run.poll_event().unwrap(),
                EventPoll::Event(_) | EventPoll::Progress
            ) {
                break;
            }
        }
    }
    let control_written = control_name.is_some_and(|name| {
        log.written()
            .iter()
            .any(|line| serde_json::from_str::<Value>(line).unwrap()["control"] == name)
    });
    let closed = run
        .events()
        .iter()
        .any(|event| matches!(event, Event::StreamClosed { .. }));
    event(
        outcome,
        "metaharness.session.PollingObserved",
        json!({
            "yielded":yielded, "control_written":control_written,
        "closed":closed, "deadline_advanced":clock.reading_ms() != 0, "process_killed":log.killed(),
        }),
    )
}

fn wireless_interrupt() -> Observed {
    use metaharness::protocol::{Command, CommandOutcome, CredentialSource, DecisionMode, Kind};
    use metaharness::{
        EventPoll, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog, ScriptedRunner,
    };
    let log = ScriptedLog::new();
    let clock = ManualClock::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::Idle, ScriptStep::Idle], log.clone());
    let mut run = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .start_with_clock(
            Input::Prompt("poll fixture".into()),
            &mut runner,
            &mut metaharness_b10x::B10xSeams::new(None, None),
            Box::new(clock.clone()),
        )
        .unwrap();
    let yielded = matches!(run.poll_event().unwrap(), EventPoll::Idle);
    assert!(matches!(
        run.send_as(
            "ess-interrupt",
            Command::Interrupt {
                reason: "fixture".into()
            }
        )
        .unwrap(),
        CommandOutcome::Ok { .. }
    ));
    for _ in 0..32 {
        if !matches!(
            run.poll_event().unwrap(),
            EventPoll::Event(_) | EventPoll::Progress
        ) {
            break;
        }
    }
    let closed = run
        .events()
        .iter()
        .any(|event| matches!(event, Event::StreamClosed { .. }));
    event(
        "quiet-wireless-interrupt",
        "metaharness.session.PollingObserved",
        json!({
            "yielded":yielded, "control_written":!log.written().is_empty(),
            "closed":closed, "deadline_advanced":clock.reading_ms() != 0, "process_killed":log.killed(),
        }),
    )
}

fn frame(input: &Value) -> Observed {
    let tools = tests::config(&[]);
    let state = "implement".parse().unwrap();
    let task = tests::driven_task();
    let context = tests::step_context(&tools, &state, &task);
    let mut frame = metaharness_frame(&context, &[], "test/linear", "1");
    match input["evidence"].as_str().unwrap() {
        "sealed" => (),
        "modified" => frame["node"]["id"] = json!("changed"),
        "untagged" => {
            frame.as_object_mut().unwrap().remove("format");
        }
        other => panic!("unknown frame evidence {other}"),
    }
    let admitted =
        metaharness::protocol::Frame::parse_document(&frame_document(&frame).unwrap()).is_ok();
    event(
        if admitted { "admitted" } else { "refused" },
        "metaharness.boundaries.FrameRead",
        json!({"admitted":admitted}),
    )
}

fn governed(input: &Value) -> Observed {
    use aep_engine::ProtocolEngine as _;
    let profile = if input["engine_allows"] == true {
        tests::AUTHORIZE_PROFILE.replace(
            "allow: [repository.read]",
            "allow: [repository.read, command.execute]",
        )
    } else {
        tests::AUTHORIZE_PROFILE.to_owned()
    };
    let (engine, mut execution) = tests::authorizing_execution_with_profile(&profile);
    let tools = tests::config(&[Capability::RepositoryRead, Capability::CommandExecution]);
    let state = "implement".parse().unwrap();
    let task = tests::driven_task();
    let context = tests::step_context(&tools, &state, &task);
    let (name, payload) =
        metaharness_codex::conformance::governed_request(input["shape"].as_str().unwrap()).unwrap();
    let line = json!({"event":"tool.requested","decision_required":true,"call_id":"ess-call","name":name,"input":payload}).to_string()+"\n";
    let mut commands = Vec::new();
    answer_events(
        Harness::Codex,
        &context,
        WriteSurface {
            scope: &[],
            root: Path::new("."),
        },
        line.as_bytes(),
        &mut commands,
        &mut Vec::new(),
        &mut |request| engine.authorize(&mut execution, request),
    );
    let response: Value = serde_json::from_slice(&commands).unwrap();
    let allowed = response["decision"]["decision"] == "allow";
    event(
        if allowed { "allowed" } else { "denied" },
        "metaharness.boundaries.CallDecided",
        json!({"allowed":allowed}),
    )
}

fn contained_read(input: &Value) -> Observed {
    use metaharness_tools::{Catalogue, LocalOperations, Server, Verbs};
    let directory = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    fs::write(directory.path().join("inside"), "inside-marker").unwrap();
    fs::write(outside.path().join("secret"), "outside-marker").unwrap();
    std::os::unix::fs::symlink(
        outside.path().join("secret"),
        directory.path().join("outside-symlink"),
    )
    .unwrap();
    let mut server = Server::new(Verbs::new(Catalogue::of(
        LocalOperations::new(directory.path()).unwrap(),
    )));
    let response = server
        .handle(
            &json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{
        "name":"tool_invoke","arguments":{"name":"file_read","arguments":{"path":input["path"]}}}}),
        )
        .unwrap();
    let contained = response["result"]["isError"] != true && response["error"].is_null();
    if contained {
        assert!(response.to_string().contains("inside-marker"));
    } else {
        assert!(
            response.to_string().contains("outside the workspace"),
            "{response}"
        );
    }
    event(
        if contained { "read" } else { "refused" },
        "metaharness.boundaries.FileRead",
        json!({"contained":contained}),
    )
}

fn retained(command: &str) -> Observed {
    let directory = tempfile::tempdir().unwrap();
    let (event_name, view_name, field) = match command {
        "metaharness.session.RecordClosedStream" => {
            let record = closed_stream(&json!({"terminal_present":false}));
            fs::write(
                directory.path().join(SPEND_FILE),
                serde_json::to_vec(&record).unwrap(),
            )
            .unwrap();
            (
                "metaharness.session.ClosureRecorded",
                "metaharness.session.RetainedClosure",
                "closure",
            )
        }
        "metaharness.spending.RecordFiniteLedger" => {
            SpendBudget::start(
                directory.path(),
                SpendTerms {
                    cap_micro_usd: 10,
                    assumed_micro_usd_per_run: 1,
                },
            )
            .unwrap();
            (
                "metaharness.spending.FiniteLedgerRecorded",
                "metaharness.spending.RetainedFiniteLedger",
                "ledger",
            )
        }
        "metaharness.spending.RecordInvocationAdmission" => {
            let mut budget = AdmissionBudget::open(
                directory.path(),
                spending::SpendPolicy::Uncapped {
                    authorization_ref: "conformance:no-model".to_owned(),
                },
                false,
            )
            .unwrap();
            let tools = tests::config(&[]);
            let state = "implement".parse().unwrap();
            let task = tests::driven_task();
            budget
                .reserve(&tests::step_context(&tools, &state, &task))
                .unwrap();
            (
                "metaharness.spending.AdmissionRecorded",
                "metaharness.spending.RetainedAdmission",
                "admission",
            )
        }
        other => panic!("unsupported retained operation {other}"),
    };
    let mut observation = event(
        "recorded",
        event_name,
        json!({"record_id":directory.path().display().to_string()}),
    );
    observation.retained = Some((directory, view_name.to_owned(), field.to_owned()));
    observation
}

fn query(observed: &Observed, view: &str) -> Result<Value, String> {
    if ![
        "metaharness.session.RetainedClosure",
        "metaharness.spending.RetainedAdmission",
        "metaharness.spending.RetainedFiniteLedger",
        "metaharness.trace.RetainedProjection",
    ]
    .contains(&view)
    {
        return Err(format!("unsupported view {view}"));
    }
    let Some((directory, expected, field)) = observed.retained.as_ref() else {
        return Ok(json!([]));
    };
    if view != expected {
        return Ok(json!([]));
    }
    if field == "projection" {
        return retained_projection(directory.path());
    }
    let record: Value = serde_json::from_slice(
        &fs::read(directory.path().join(SPEND_FILE)).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let projected = if field == "admission" {
        json!({"launches":record["invocations"].as_array().ok_or("missing actual admissions")?.len(),"authorization_ref":record["policy"]["authorization_ref"]})
    } else {
        record
    };
    Ok(json!({field:projected}))
}

fn observed_codex(command: &str, input: &Value) -> Observed {
    let events =
        metaharness_codex::conformance::observation(input["evidence"].as_str().unwrap()).unwrap();
    if command.ends_with("ReadToolOutcome") {
        let (error, code) = events
            .iter()
            .rev()
            .find_map(|event| match event {
                Event::ToolResult {
                    is_error,
                    exit_code,
                    ..
                } => Some((*is_error, *exit_code)),
                _ => None,
            })
            .unwrap();
        let known = error.is_some();
        let failed = error == Some(true);
        let numeric_exit = code.is_some();
        let outcome = match (known, failed, numeric_exit) {
            (false, _, _) => "unverified",
            (true, false, true) => "command-success",
            (true, true, true) => "command-failure",
            (true, false, false) => "success-without-code",
            (true, true, false) => "failure-without-code",
        };
        event(
            outcome,
            "metaharness.observations.ToolOutcomeRead",
            json!({"known":known,"failed":failed,"numeric_exit":numeric_exit}),
        )
    } else {
        let Event::SessionEnded {
            final_answer,
            observed_models,
            ..
        } = events.last().unwrap()
        else {
            panic!("missing terminal")
        };
        let (present, yes, no) = if command.ends_with("ReadFinalAnswer") {
            (final_answer.is_some(), "authoritative", "unavailable")
        } else {
            (
                observed_models
                    .as_ref()
                    .is_some_and(|models| models.iter().any(|model| model.model.is_some())),
                "observed-selection",
                "unreported",
            )
        };
        event(
            if present { yes } else { no },
            "metaharness.observations.PresenceRead",
            json!({"present":present}),
        )
    }
}

// --- metaharness.trace: the `trace-ir/1` document, observed through production (ADR 0096 point 7).
// Every arm reads the family, the reason, the digest and the completeness verdict off the
// document `project_document` wrote; none restates the mapping (`spec/domains/trace.yaml`).

/// The file a retained projection is written to, beside the retained ledgers' `SPEND_FILE`.
const TRACE_FILE: &str = "trace-ir.json";

/// The twelve node families the domain declares: the ten `IrFamily` names plus `unk` and
/// `stream_closed`, in the order `metaharness.trace.FamilyCensus` lists them.
const FAMILIES: [&str; 12] = [
    "session_start",
    "run_outcome",
    "assistant_text",
    "assistant_thinking",
    "thinking_estimate",
    "synthetic_injection",
    "tool_call",
    "tool_result",
    "rate_limit",
    "opaque",
    UNK_FAMILY,
    CLOSED_FAMILY,
];

/// The repository root, for the committed inputs the trace commands read by path.
fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// The committed `metaharness.event/1` stream of one recorded run. `evals/aep/runs/README.md`:
/// hand-written inputs through this driver's own reader, so reading them is not a paid run
/// (invariant 5; precedent `crates/metaharness-claude/tests/recorded_runs.rs`).
fn recorded_stream(run: &str) -> Vec<u8> {
    fs::read(repository_root().join(format!("evals/aep/runs/{run}.events.jsonl"))).unwrap()
}

/// Every non-blank line of a stream through the production parser.
fn parse_stream(bytes: &[u8]) -> Vec<EventLine> {
    std::str::from_utf8(bytes)
        .unwrap()
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| parse_event_line(line).unwrap())
        .collect()
}

/// The bytes a stream of framed lines is, one JSON object per line: what `transcript_digest` names.
fn stream_bytes(lines: &[EventLine]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for line in lines {
        bytes.extend(serde_json::to_vec(line).unwrap());
        bytes.push(b'\n');
    }
    bytes
}

/// Frame emissions the way every producer in this workspace does, through `EventStream`, the one
/// place a sequence number and the closing marker are written (amendment a17).
fn framed(
    run: &str,
    emissions: Vec<Emission>,
    close: Option<CloseReason>,
) -> (Vec<EventLine>, Vec<u8>) {
    let mut stream = EventStream::new(RunId::new(run));
    let mut lines: Vec<EventLine> = emissions
        .into_iter()
        .map(|emission| stream.stamp(emission))
        .collect();
    if let Some(reason) = close {
        lines.extend(stream.close(reason));
    }
    let bytes = stream_bytes(&lines);
    (lines, bytes)
}

/// One event of each wire kind, carrying the fields its variant requires and nothing else: the
/// family is what is observed, so the sample claims nothing about anything else.
#[allow(clippy::too_many_lines)]
fn sample_event(wire: &str) -> Event {
    use metaharness::protocol::{
        CommandOutcome, DecidedBy, Decision, DecisionCensus, Digest, HermeticAttestation,
        HermeticMode, ProcessTermination, RateLimitInfo, Seam, StepOutcome, StepRef, TranscriptRef,
        Usage,
    };
    let step = || StepRef {
        workflow: "ess".to_owned(),
        state: "implement".to_owned(),
        index: 1,
        attempt: 1,
    };
    match wire {
        "session.started" => Event::SessionStarted {
            adapter: "ess".to_owned(),
            adapter_class: "harness".to_owned(),
            harness_version: None,
            session_id: None,
            model: None,
            permission_mode: None,
            credential_source: None,
            output_style: None,
            cwd: None,
            offered_tools: None,
            available_operations: None,
            withheld: None,
            context_files: Vec::new(),
            slash_commands: None,
            skills: None,
            agents: None,
            plugins: None,
            mcp_servers: None,
            inputs_digest: None,
            transcript: TranscriptRef {
                path: None,
                digest: None,
                bytes: None,
            },
            hermetic: HermeticAttestation::none(HermeticMode::Off),
        },
        "session.ended" => Event::SessionEnded {
            final_answer: None,
            observed_models: None,
            is_error: None,
            subtype: None,
            stop_reason: None,
            terminal_reason: None,
            api_error_status: None,
            num_turns: None,
            duration_ms: None,
            duration_api_ms: None,
            ttft_ms: None,
            time_to_request_ms: None,
            total_cost_usd: None,
            permission_denials: None,
            subagents_spawned: None,
            usage: None,
            model_usage: None,
            census: DecisionCensus::default(),
        },
        "step.entered" => Event::StepEntered {
            step: step(),
            frame_digest: None,
        },
        "step.left" => Event::StepLeft {
            step: step(),
            outcome: StepOutcome::Completed,
        },
        "turn.started" => Event::TurnStarted {
            turn: 1,
            frame_digest: None,
        },
        "turn.ended" => Event::TurnEnded {
            turn: 1,
            stop_reason: None,
        },
        "text" => Event::Text {
            text: "hello".to_owned(),
            request_id: None,
        },
        "thinking" => Event::Thinking {
            text: "reasoning".to_owned(),
            request_id: None,
        },
        "thinking.estimate" => Event::ThinkingEstimate {
            estimate: 1,
            delta: None,
        },
        "injection" => Event::Injection {
            text: "frame".to_owned(),
            origin: None,
        },
        "tool.requested" => Event::ToolRequested {
            call_id: "t1".to_owned(),
            name: "Read".to_owned(),
            input: json!({}),
            operations: Vec::new(),
            subjects: Vec::new(),
            decision_required: false,
            deadline_ms: None,
            seam: Seam::None,
        },
        "tool.decided" => Event::ToolDecided {
            call_id: "t1".to_owned(),
            decision: Decision::Allow,
            decided_by: DecidedBy::Embedder,
            seam: Seam::None,
            latency_ms: None,
        },
        "tool.result" => Event::ToolResult {
            exit_code: None,
            outcome_source: None,
            call_id: "t1".to_owned(),
            is_error: None,
            content: None,
            bytes: None,
            tool_use_result: None,
        },
        "usage" => Event::Usage {
            request_id: None,
            model: None,
            usage: Usage::default(),
        },
        "rate_limit" => Event::RateLimit {
            info: RateLimitInfo {
                status: None,
                window: None,
                resets_at: None,
                utilization: None,
                using_overage: None,
            },
        },
        "command.result" => Event::CommandResult {
            id: "c1".to_owned(),
            outcome: CommandOutcome::Ok { applies_at: None },
        },
        "warning" => Event::Warning {
            code: "ESS".to_owned(),
            message: "fixture".to_owned(),
        },
        "opaque" => Event::Opaque {
            vendor_type: None,
            vendor_subtype: None,
            digest: Digest::of(b"opaque"),
            source_line: None,
        },
        "auth.expired" => Event::AuthExpired {
            credential_source: None,
            detail: None,
            source_line: None,
        },
        "stream.closed" => Event::StreamClosed {
            process: ProcessTermination::Unknown,
            events: 0,
            reason: CloseReason::Completed,
            run_id: "ess".to_owned(),
        },
        other => panic!("{other} is not on the wire"),
    }
}

/// `metaharness.trace.ClassifyEventKind`: one event of the asked-for kind through
/// `project_document`; the family and the reason are read off the node the production writer
/// emitted, and the outcome is named from that family.
fn classify(input: &Value) -> Observed {
    // The domain spells `-` where the wire spells `.`, because a guard reads `.` as field access.
    let wire = input["kind"].as_str().unwrap().replace('-', ".");
    let sample = sample_event(&wire);
    assert_eq!(sample.name(), wire, "the sample is of the kind asked for");
    let (lines, bytes) = framed("ess-classify", vec![Emission::untimed(sample)], None);
    let document = project_document(&lines, &bytes);
    assert_eq!(document.events.len(), 1, "every event is a node");
    let node = &document.events[0].kind;
    let family = node["event"].as_str().unwrap().to_owned();
    assert_eq!(document.metaharness.families.get(&family), Some(&1));
    if family == UNK_FAMILY {
        // The outcome is named from the kind the node itself states, not from the input.
        let kind = node["event_kind"].as_str().unwrap();
        assert_eq!(kind, wire, "an unk node names its kind");
        assert_eq!(document.metaharness.unk_kinds.get(kind), Some(&1));
        return event(
            &format!("unk-{}", kind.replace('.', "-")),
            "metaharness.trace.UnkClassified",
            json!({"family": family, "reason": node["reason"]}),
        );
    }
    assert!(
        document.metaharness.unk_kinds.is_empty(),
        "a mapped kind is not a vocabulary gap"
    );
    event(
        &family.replace('_', "-"),
        "metaharness.trace.FamilyClassified",
        json!({"family": family}),
    )
}

/// `metaharness.trace.VerifyStreamCompleteness`: a short stream built through the production
/// `EventStream` and read back through `project_document`. `complete` is the verified flag of
/// amendment a17. Every shape `stream_completeness` decides between is built: no marker; the
/// marker production writes; that marker followed by a line (the appended-to file); that marker
/// with its count overwritten; and that marker twice. The outcome is named from what the document
/// shows (how many markers, whether the last node is one, the verified flag), never from the input.
fn completeness(input: &Value) -> Observed {
    let text = || {
        Emission::untimed(Event::Text {
            text: "done".to_owned(),
            request_id: None,
        })
    };
    let mut stream = EventStream::new(RunId::new("ess-completeness"));
    let mut lines = vec![stream.stamp(text())];
    match input["marker"].as_str().unwrap() {
        "absent" => (),
        "last-and-counting" => lines.extend(stream.close(CloseReason::Completed)),
        "misplaced" => {
            lines.extend(stream.close(CloseReason::Completed));
            lines.push(stream.stamp(text()));
        }
        "miscounting" => {
            let mut marker = stream
                .close(CloseReason::Completed)
                .expect("a fresh stream closes once");
            let Event::StreamClosed { events, .. } = &mut marker.event else {
                unreachable!("close writes the marker")
            };
            assert_eq!(
                *events, 1,
                "the production marker counts the node before it"
            );
            *events = 7;
            lines.push(marker);
        }
        "duplicated" => {
            let first = stream
                .close(CloseReason::Completed)
                .expect("a fresh stream closes once");
            assert!(
                stream.close(CloseReason::Completed).is_none(),
                "production writes one marker"
            );
            let mut second = first.clone();
            second.seq = first.seq + 1;
            if let Event::StreamClosed { events, .. } = &mut second.event {
                *events = 2;
            }
            lines.push(first);
            lines.push(second);
        }
        other => panic!("unsupported marker evidence {other}"),
    }
    let bytes = stream_bytes(&lines);
    let document = project_document(&lines, &bytes);
    let markers = document
        .metaharness
        .families
        .get(CLOSED_FAMILY)
        .copied()
        .unwrap_or(0);
    let marker_last = document
        .events
        .last()
        .is_some_and(|node| node.kind["event"] == CLOSED_FAMILY);
    let complete = document.metaharness.stream_complete;
    // A tuple the domain does not name (a marker-less or duplicated stream read as complete) is an
    // outcome no scenario declares, so the scenario fails by name on `expect_outcome` with the
    // observed tuple in the message rather than taking the whole run down.
    let outcome = match (markers, marker_last, complete) {
        (0, false, false) => "truncated".to_owned(),
        (1, true, true) => "complete".to_owned(),
        (1, false, false) => "inconsistent-misplaced".to_owned(),
        (1, true, false) => "inconsistent-miscounting".to_owned(),
        (2.., true, false) => "inconsistent-duplicated".to_owned(),
        other => format!("unnamed-completeness-shape:{other:?}"),
    };
    event(
        &outcome,
        "metaharness.trace.CompletenessVerified",
        json!({"markers": markers, "marker_last": marker_last, "complete": complete}),
    )
}

/// `metaharness.trace.NameDigest`: `transcript_digest` names the event stream's own bytes, and the
/// vendor's reference travels beside it under its own name (P3, amendment a15 point 2).
fn digest(input: &Value) -> Observed {
    use metaharness::protocol::{Digest, TranscriptRef};
    const VENDOR_BYTES: &[u8] = b"the vendor's own transcript";
    let vendor = match input["vendor_reference"].as_str().unwrap() {
        "digest-named" => TranscriptRef {
            path: Some("vendor/transcript.jsonl".to_owned()),
            digest: Some(Digest::of(VENDOR_BYTES)),
            bytes: Some(u64::try_from(VENDOR_BYTES.len()).unwrap()),
        },
        "digest-absent" => TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        other => panic!("unsupported vendor reference {other}"),
    };
    let mut opening = sample_event("session.started");
    let Event::SessionStarted { transcript, .. } = &mut opening else {
        unreachable!()
    };
    transcript.clone_from(&vendor);
    let (lines, bytes) = framed("ess-digest", vec![Emission::untimed(opening)], None);
    let document = project_document(&lines, &bytes);
    let over_stream_bytes = document.transcript_digest == Digest::of(&bytes).as_str();
    let vendor_digest = document
        .metaharness
        .vendor_transcript
        .as_ref()
        .and_then(|reference| reference.digest.clone());
    assert_eq!(
        vendor_digest, vendor.digest,
        "the vendor's reference travels as recorded"
    );
    if let Some(named) = &vendor_digest {
        assert_ne!(
            named.as_str(),
            document.transcript_digest,
            "two digests, two files"
        );
    }
    let vendor_digest_named = vendor_digest.is_some();
    let outcome = if vendor_digest_named {
        "stream-digest-beside-vendor-digest"
    } else {
        "stream-digest-alone"
    };
    event(
        outcome,
        "metaharness.trace.DigestNamed",
        json!({
            "over_stream_bytes": over_stream_bytes,
            "source": document.metaharness.source,
            "vendor_digest_named": vendor_digest_named,
        }),
    )
}

/// One committed recorded run, projected and rendered: the document, the stream's line count and
/// the rendered bytes.
fn project_recorded(run: &str) -> (TraceIrDocument, usize, String) {
    let bytes = recorded_stream(run);
    let lines = parse_stream(&bytes);
    let document = project_document(&lines, &bytes);
    let rendered = document.render().unwrap();
    (document, lines.len(), rendered)
}

/// `metaharness.trace.ProjectRecordedRun`: project one recorded run, render it twice, and retain
/// the rendered document as an immutable record. The outcome is named from the run id the
/// stream's own lines carry, read off the document rather than off the input.
fn project_recorded_run(input: &Value) -> Observed {
    let run = input["run"].as_str().unwrap();
    let (document, lines, first) = project_recorded(run);
    let (_, _, second) = project_recorded(run);
    assert_eq!(document.events.len(), lines, "every event is a node");
    let directory = tempfile::tempdir().unwrap();
    fs::write(directory.path().join(TRACE_FILE), &first).unwrap();
    let recorded = document
        .metaharness
        .run
        .as_deref()
        .expect("every line of a stream carries its run");
    let unk: u32 = document.metaharness.unk_kinds.values().sum();
    let mut observation = event(
        &format!("{recorded}-recorded"),
        "metaharness.trace.ProjectionRecorded",
        json!({
            "record_id": directory.path().display().to_string(),
            "nodes": document.events.len(),
            "unk": unk,
            "stream_complete": document.metaharness.stream_complete,
            "byte_stable": first == second,
            // The vendor's recorded opening timestamp as milliseconds, read off the node; `null`
            // when the writer derived none, which no declared payload matches.
            "opening_timestamp_ms": document.events[0].timestamp_ms,
        }),
    );
    observation.retained = Some((
        directory,
        "metaharness.trace.RetainedProjection".to_owned(),
        "projection".to_owned(),
    ));
    observation
}

/// The retained document read back, with what `metaharness.trace.RetainedDocument` declares of
/// it: the header, the census, the family counts, the node and line counts, the opening node, and
/// whether a fresh projection of the committed stream the record names renders to the retained
/// bytes (P1). Values come from the file and from that stream, never from target bookkeeping.
fn retained_projection(directory: &Path) -> Result<Value, String> {
    let retained = fs::read_to_string(directory.join(TRACE_FILE)).map_err(|e| e.to_string())?;
    let document: Value = serde_json::from_str(&retained).map_err(|e| e.to_string())?;
    let block = &document["metaharness"];
    let run = block["run"]
        .as_str()
        .ok_or("the retained document names no run")?;
    let (_, lines, fresh) = project_recorded(run);
    let nodes = document["events"]
        .as_array()
        .ok_or("the retained document has no events")?
        .len();
    let census = block["families"]
        .as_object()
        .ok_or("the retained document has no family census")?;
    let families: serde_json::Map<String, Value> = FAMILIES
        .iter()
        .map(|family| {
            let count = census.get(*family).and_then(Value::as_u64).unwrap_or(0);
            ((*family).to_owned(), json!(count))
        })
        .collect();
    let unk: u64 = block["unk_kinds"]
        .as_object()
        .ok_or("the retained document has no unk census")?
        .values()
        .filter_map(Value::as_u64)
        .sum();
    let opening = &document["events"][0];
    // P1 across builds: the committed golden `crates/metaharness-cli/tests/project.rs` holds is
    // the fixed point a fresh render of the same stream must land on, byte for byte.
    let golden =
        fs::read_to_string(repository_root().join(format!("evals/aep/runs/{run}.trace-ir.json")))
            .map_err(|e| e.to_string())?;
    Ok(json!({"projection": {
        "header": {
            "format": document["format"],
            "transcript_digest": document["transcript_digest"],
            "adapter": document["adapter"]["name"],
            "source": block["source"],
        },
        "census": {
            "events_total": block["events_total"],
            "unk": unk,
            "stream_complete": block["stream_complete"],
        },
        "families": families,
        "counts": {"nodes": nodes, "lines": lines},
        "opening_node": {
            "index": opening["index"],
            "source_line": opening["source_line"],
            "timestamp": opening["timestamp"],
            "timestamp_ms": opening["timestamp_ms"],
            "family": opening["kind"]["event"],
        },
        "opening_timestamp_present": opening["timestamp"].is_string(),
        "byte_stable": fresh == retained,
        "golden_stable": fresh == golden,
    }}))
}

/// The emissions a reader owes for a record: every line through `push`, then `finish`.
fn read_record<R>(
    reader: &mut R,
    text: &str,
    push: impl Fn(&mut R, &str) -> Vec<Emission>,
    finish: impl Fn(&mut R) -> Vec<Emission>,
) -> Vec<Emission> {
    let mut emissions = Vec::new();
    for line in text.lines() {
        emissions.extend(push(reader, line));
    }
    emissions.extend(finish(reader));
    emissions
}

/// A vendor record through the production reader that owns its source kind: Claude through
/// `metaharness::ClaudeSeams` (which is `TranscriptReader::new(..).with_seam(..)`,
/// `crates/metaharness-claude/src/bridge.rs`), Codex through `RolloutReader`, the loop through
/// `B10xSeams`. A replay is not a launch: the attestation claims nothing and the seam is `None`,
/// the way `crates/metaharness-claude/tests/recorded_runs.rs` drives the reader (invariant 3).
fn vendor_emissions(source: &str, relative: &str, record: &[u8], text: &str) -> Vec<Emission> {
    use metaharness::protocol::{
        Digest, HermeticAttestation, HermeticMode, Seam, SeamFactory as _, TranscriptRef,
    };
    let transcript = TranscriptRef {
        path: Some(relative.to_owned()),
        digest: Some(Digest::of(record)),
        bytes: Some(u64::try_from(record.len()).unwrap()),
    };
    let attestation = HermeticAttestation::none(HermeticMode::Off);
    match source {
        "claude-code-stream-json" => {
            let mut seam = metaharness::ClaudeSeams.build(transcript, attestation, Seam::None);
            read_record(
                &mut seam,
                text,
                |seam, line| seam.push_line(line),
                |seam| seam.finish(),
            )
        }
        "codex-rollout" => {
            let mut reader = metaharness_codex::RolloutReader::new(transcript, attestation);
            read_record(
                &mut reader,
                text,
                metaharness_codex::RolloutReader::push_line,
                metaharness_codex::RolloutReader::finish,
            )
        }
        "harness-json" => {
            let mut seam = metaharness_b10x::B10xSeams::new(None, None).build(
                transcript,
                attestation,
                Seam::None,
            );
            read_record(
                &mut seam,
                text,
                |seam, line| seam.push_line(line),
                |seam| seam.finish(),
            )
        }
        other => panic!("{other} has no production reader here"),
    }
}

/// `metaharness.trace.NormalizeRecord`: a committed golden vendor record through its production
/// reader, framed by the production `EventStream`, then `project_document`. The recorded
/// `metaharness.event/1` stream needs no reader, and that is observed from its first line rather
/// than assumed from the input; the adapter is the opening node's own.
fn normalize(input: &Value) -> Observed {
    let source = input["source"].as_str().unwrap();
    let relative = match source {
        "claude-code-stream-json" => "crates/metaharness-claude/fixtures/golden/transcript.jsonl",
        "codex-rollout" => "crates/metaharness-codex/fixtures/golden/rollout.jsonl",
        "harness-json" => "crates/metaharness-b10x/fixtures/golden/loop.jsonl",
        "metaharness-event-stream" => "evals/aep/runs/decomposer-clean.events.jsonl",
        other => panic!("unsupported source kind {other}"),
    };
    let record = fs::read(repository_root().join(relative)).unwrap();
    let text = std::str::from_utf8(&record).unwrap();
    let already_wire = text
        .lines()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| parse_event_line(line).is_ok());
    let (lines, bytes) = if already_wire {
        (parse_stream(&record), record.clone())
    } else {
        let emissions = vendor_emissions(source, relative, &record, text);
        // The same reader's committed expectation, as a sequence of event names: a disagreement
        // here is a finding about the reader path, not a property of the projection.
        let expected = fs::read_to_string(
            repository_root().join(relative.replace(".jsonl", ".expected.jsonl")),
        )
        .unwrap();
        let expected_names: Vec<&str> = expected
            .lines()
            .map(|line| parse_event_line(line).unwrap().event.name())
            .collect();
        let observed_names: Vec<&str> = emissions
            .iter()
            .map(|emission| emission.event.name())
            .collect();
        if observed_names != expected_names {
            return Observed {
                outcome: format!("reader-sequence-differs-from-committed-expectation:{relative}"),
                ..Observed::default()
            };
        }
        framed("ess-normalize", emissions, None)
    };
    let document = project_document(&lines, &bytes);
    assert_eq!(document.events.len(), lines.len(), "every event is a node");
    let opening = &document.events[0].kind;
    let opening_family = opening["event"].as_str().unwrap().to_owned();
    let adapter = opening["adapter"].as_str().unwrap_or_default().to_owned();
    let outcome = match (adapter.as_str(), already_wire) {
        (_, true) => "event-stream-projected",
        ("claude", false) => "claude-transcript-normalized",
        ("codex", false) => "codex-rollout-normalized",
        ("b10x", false) => "harness-record-normalized",
        (other, false) => panic!("the opening node names an undeclared adapter {other}"),
    };
    event(
        outcome,
        "metaharness.trace.RecordNormalized",
        json!({
            "adapter": adapter,
            "reader_applied": !already_wire,
            "opening_family": opening_family,
        }),
    )
}

/// `metaharness.trace.ProjectVendorRecord` (amendment a28): the public verb's library entry,
/// `metaharness::project_vendor_record`, over one committed golden vendor record, called twice.
/// Every value is read off the rendered document and off the record on disk; the outcome is named
/// from the opening node's own adapter, never from the input, so a shape the domain does not
/// declare fails by scenario name. The one `SourceKind` that is not a `--from` reaches the
/// refusal, which is observed for naming the source it refused.
fn project_vendor_record(input: &Value) -> Observed {
    let source = input["source"].as_str().unwrap();
    let vendor = match source.parse::<metaharness::VendorSource>() {
        Ok(vendor) => vendor,
        Err(refusal) => {
            return event(
                "event-stream-is-not-a-from",
                "metaharness.trace.VendorSourceRefused",
                json!({"refused": true, "names_the_source": refusal.to_string().contains(source)}),
            );
        }
    };
    let relative = match vendor {
        metaharness::VendorSource::ClaudeCodeStreamJson => {
            "crates/metaharness-claude/fixtures/golden/transcript.jsonl"
        }
        metaharness::VendorSource::CodexRollout => {
            "crates/metaharness-codex/fixtures/golden/rollout.jsonl"
        }
        metaharness::VendorSource::HarnessJson => {
            "crates/metaharness-b10x/fixtures/golden/loop.jsonl"
        }
    };
    let path = repository_root().join(relative);
    let document = match metaharness::project_vendor_record(&path, vendor) {
        Ok(document) => document,
        Err(refusal) => {
            return Observed {
                outcome: format!("vendor-record-refused:{relative}:{refusal}"),
                ..Observed::default()
            };
        }
    };
    let first = document.render().unwrap();
    let second = metaharness::project_vendor_record(&path, vendor)
        .unwrap()
        .render()
        .unwrap();
    let rendered: Value = serde_json::from_str(&first).unwrap();
    let block = &rendered["metaharness"];
    let opening = &rendered["events"][0]["kind"];
    let adapter = opening["adapter"].as_str().unwrap_or_default().to_owned();
    let outcome = match adapter.as_str() {
        "claude" => "claude-transcript-projected",
        "codex" => "codex-rollout-projected",
        "b10x" => "harness-record-projected",
        other => panic!("the opening node names an undeclared adapter {other}"),
    };
    // The adapter's own committed expectation for the same record, through the wire-case writer:
    // the census a replay owes is the census the reader owes, and nothing more.
    let expected_bytes =
        fs::read(repository_root().join(relative.replace(".jsonl", ".expected.jsonl"))).unwrap();
    let expected = project_document(&parse_stream(&expected_bytes), &expected_bytes);
    let record_digest = metaharness::protocol::Digest::of(&fs::read(&path).unwrap());
    event(
        outcome,
        "metaharness.trace.VendorRecordProjected",
        json!({
            "source": block["source"],
            "adapter": adapter,
            "opening_family": opening["event"],
            "hermetic_mode": block["hermetic"]["mode"],
            // `null` when the block carries no list, which no declared payload matches.
            "imposed": block["hermetic"]["imposed"].as_array().map(Vec::len),
            "stream_complete": block["stream_complete"],
            "byte_stable": first == second,
            "digest_over_record": rendered["transcript_digest"] == json!(record_digest.as_str()),
            "census_as_expected": document.metaharness.families == expected.metaharness.families,
        }),
    )
}

/// `metaharness.trace.SummarizeRecord` (amendment a42): `metaharness project --to
/// trace-ir-summary` through its library entries. A vendor source is read by
/// `metaharness::project_vendor_record` over the golden record `ProjectVendorRecord` reads; the
/// event stream by `metaharness::project_file` over the recorded `decomposer-clean` run, as the
/// verb reads it without `--from`. The document is summarized twice by
/// `metaharness_protocol::summarize`. Every payload field but `byte_stable` is the compact JSON
/// text of the rendered field of the same name, parsed from the production render's bytes and
/// re-derived from nothing, so `null` and `0` differ byte for byte. The outcome is named from the summary's own
/// `source`, never from the input.
fn summarize_record(input: &Value) -> Observed {
    let source = input["source"].as_str().unwrap();
    let document = match source.parse::<metaharness::VendorSource>() {
        Ok(vendor) => {
            let relative = match vendor {
                metaharness::VendorSource::ClaudeCodeStreamJson => {
                    "crates/metaharness-claude/fixtures/golden/transcript.jsonl"
                }
                metaharness::VendorSource::CodexRollout => {
                    "crates/metaharness-codex/fixtures/golden/rollout.jsonl"
                }
                metaharness::VendorSource::HarnessJson => {
                    "crates/metaharness-b10x/fixtures/golden/loop.jsonl"
                }
            };
            metaharness::project_vendor_record(&repository_root().join(relative), vendor)
        }
        Err(_) => metaharness::project_file(
            &repository_root().join("evals/aep/runs/decomposer-clean.events.jsonl"),
        ),
    };
    let document = match document {
        Ok(document) => document,
        Err(refusal) => {
            return Observed {
                outcome: format!("summary-refused:{source}:{refusal}"),
                ..Observed::default()
            };
        }
    };
    let summary = metaharness::protocol::summarize(&document);
    let first = summary.render().unwrap();
    let second = metaharness::protocol::summarize(&document)
        .render()
        .unwrap();
    let outcome = match summary.source {
        "claude-code-stream-json" => "claude-transcript-summarized",
        "codex-rollout" => "codex-rollout-summarized",
        "harness-json" => "harness-record-summarized",
        "metaharness.event/1" => "event-stream-summarized",
        other => panic!("the summary names an undeclared source {other}"),
    };
    // Every payload value is read off the production render's own bytes, parsed and written back
    // as compact JSON text, never re-serialized from the typed summary, so a field the render
    // writes wrong cannot be corrected on the way to the scenario. A key the render carries beyond
    // the declared ones is observed too and fails as an undeclared field. A nested object's keys
    // come out sorted, because the parse does not keep the render's order.
    let rendered: Value = serde_json::from_str(&first).unwrap();
    let mut payload = json!({ "byte_stable": first == second });
    for (key, value) in rendered.as_object().unwrap() {
        payload[key] = Value::String(value.to_string());
    }
    event(outcome, "metaharness.trace.SessionSummarized", payload)
}

#[allow(
    clippy::too_many_lines,
    reason = "one arm per specified command, read top to bottom"
)]
fn execute(command: &str, input: &Value) -> Observed {
    match command {
        "metaharness.trace.ClassifyEventKind" => classify(input),
        "metaharness.trace.VerifyStreamCompleteness" => completeness(input),
        "metaharness.trace.NameDigest" => digest(input),
        "metaharness.trace.ProjectRecordedRun" => project_recorded_run(input),
        "metaharness.trace.NormalizeRecord" => normalize(input),
        "metaharness.trace.ProjectVendorRecord" => project_vendor_record(input),
        "metaharness.trace.SummarizeRecord" => summarize_record(input),
        "metaharness.session.PollSteering" => polling(input),
        "metaharness.observations.ReadFinalAnswer"
        | "metaharness.observations.ReadObservedModel"
        | "metaharness.observations.ReadToolOutcome" => observed_codex(command, input),
        "metaharness.observations.ReadNativeTermination" => {
            use metaharness::protocol::ProcessTermination;
            let mut closure = json!({"event":"stream.closed","events":0,"run_id":"ess-native","reason":"completed"});
            match input["evidence"].as_str().unwrap() {
                "missing" => (),
                "exited-zero" => closure["process"] = json!({"kind":"exited","code":0}),
                "exited-nonzero" => closure["process"] = json!({"kind":"exited","code":7}),
                "signaled" => closure["process"] = json!({"kind":"signaled","signal":9}),
                other => panic!("unsupported native evidence {other}"),
            }
            let Event::StreamClosed { process, .. } = serde_json::from_value(closure).unwrap()
            else {
                unreachable!()
            };
            let (outcome, known, native_success) = match process {
                ProcessTermination::Unknown => ("unobserved", false, false),
                ProcessTermination::Exited { code: 0 } => ("measured-success", true, true),
                _ => ("measured-failure", true, false),
            };
            event(
                outcome,
                "metaharness.observations.NativeStatusClassified",
                json!({"known":known,"native_success":native_success}),
            )
        }
        "metaharness.session.RecordClosedStream"
        | "metaharness.spending.RecordFiniteLedger"
        | "metaharness.spending.RecordInvocationAdmission" => retained(command),
        "metaharness.session.FinishCodexCompletion" => completion(input),
        "metaharness.session.CloseUnsteeredStream" => closure(input),
        "metaharness.session.ExitAuditedRun" => audited_run_exit(input),
        "metaharness.workspace.DeclareProcessWrites" => process_write_plan(input),
        "metaharness.workspace.DeclareScratchWorkingDirectory" => scratch_cwd_plan(input),
        "metaharness.workspace.DeclareContextFiles" => context_files_plan(input),
        "metaharness.workspace.CheckSelectedName" => checked(
            metaharness_b10x::workspace_component_is_adoptable(input["name"].as_str().unwrap()),
            "eligible-component",
            "invalid-name-refused",
            "metaharness.workspace.InvalidName",
        ),
        "metaharness.spending.CheckFiniteTerms" => {
            let usd = |value: &Value| {
                let encoded = value.to_string();
                // ESS's integer witnesses may be encoded as 1.0. Accept only an exact
                // integral spelling in u64 range; never round a floating-point amount.
                encoded
                    .strip_suffix(".0")
                    .unwrap_or(&encoded)
                    .parse::<u64>()
                    .ok()
                    .map_or_else(
                        || "-1".to_owned(),
                        |value| format!("{}.{:06}", value / 1_000_000, value % 1_000_000),
                    )
            };
            let terms = &input["terms"];
            checked(
                spend_terms_with_live(
                    &tests::b10x_map(),
                    Some(&usd(&terms["cap_micro_usd"])),
                    Some(&usd(&terms["assumed_micro_usd_per_run"])),
                    true,
                )
                .is_ok(),
                "valid-finite-terms",
                "invalid-finite-terms",
                "metaharness.spending.InvalidFiniteTerms",
            )
        }
        "metaharness.spending.CheckUncappedAuthorization" => {
            let reference = match input["reference_evidence"].as_str().unwrap() {
                "missing" => None,
                "blank" => Some(" \t".to_owned()),
                "nonblank" => Some("operator:ess".to_owned()),
                other => panic!("unsupported reference shape {other}"),
            };
            let options = SpendOptions {
                uncapped_budget: input["explicit_opt_in"].as_bool().unwrap(),
                spend_authorization: reference,
            };
            checked(
                spending::policy(&tests::b10x_map(), None, None, None, &options, true).is_ok(),
                "explicitly-authorized",
                "unauthorized",
                "metaharness.spending.UncappedAuthorizationRequired",
            )
        }
        "metaharness.boundaries.ReadSealedFrame" => frame(input),
        "metaharness.boundaries.GovernCodexCall" => governed(input),
        "metaharness.boundaries.ReadSelectedWorkspaceFile" => contained_read(input),
        other => panic!("no production target for {other}; cannot silently skip"),
    }
}

fn event_matches(actual: Option<&Value>, step: &Value) -> Result<bool, String> {
    let Some(actual) = actual else {
        return Ok(false);
    };
    let shapes = step["shape"]
        .as_object()
        .ok_or("event has no declared shape")?;
    if let Some(payload) = step.get("payload") {
        let mut expected = payload.clone();
        // ESS integer literals may serialize as 2.0. Parse exact decimal spelling,
        // never f64: rounding could make different large integers falsely match.
        for (field, shape) in shapes {
            if shape["holds"] == "primitive"
                && shape["kind"] == "integer"
                && expected.get(field).is_some()
            {
                let spelling = expected[field]
                    .as_number()
                    .ok_or("integer payload absent")?
                    .to_string();
                let integral = if spelling.contains('.') {
                    spelling
                        .trim_end_matches('0')
                        .strip_suffix('.')
                        .ok_or("nonintegral or unsupported integer spelling")?
                } else {
                    &spelling
                };
                let integer = integral
                    .parse::<i64>()
                    .map_err(|_| "integer payload exceeds the target's exact numeric range")?;
                expected[field] = json!(integer);
            }
        }
        // A field the specification generates (`record_id: {generated: true}`) is absent from the
        // literal payload and checked by shape below; every literal field must match exactly.
        for (field, value) in expected.as_object().ok_or("payload is not an object")? {
            if actual[field] != *value {
                return Ok(false);
            }
        }
    }
    // Every field the event carries is one the specification declares; an undeclared one fails.
    if actual
        .as_object()
        .ok_or("event payload is not an object")?
        .keys()
        .any(|field| !shapes.contains_key(field))
    {
        return Ok(false);
    }
    for (field, shape) in shapes {
        let matches = match shape["holds"].as_str() {
            Some("enum") => shape["variants"]
                .as_array()
                .ok_or("enum has no variants")?
                .contains(&actual[field]),
            Some("primitive") => match shape["kind"].as_str() {
                Some("string") => actual[field].is_string(),
                Some("boolean") => actual[field].is_boolean(),
                Some("integer") => actual[field].is_i64() || actual[field].is_u64(),
                other => return Err(format!("unsupported primitive {other:?}")),
            },
            other => return Err(format!("unsupported event shape {other:?}")),
        };
        if !matches {
            return Ok(false);
        }
    }
    Ok(true)
}

#[test]
fn integer_payload_comparison_preserves_exact_values() {
    let step = |count: Value| json!({"payload":{"count":count},"shape":{"count":{"holds":"primitive","kind":"integer"}}});
    assert!(event_matches(Some(&json!({"count":2})), &step(json!(2.0))).unwrap());
    assert!(
        !event_matches(
            Some(&json!({"count":9_007_199_254_740_992_i64})),
            &step(json!(9_007_199_254_740_993_i64))
        )
        .unwrap()
    );
    assert!(event_matches(Some(&json!({"count":2})), &step(json!(2.25))).is_err());
}

fn view_matches(actual: &Value, expectation: &Value) -> Result<bool, String> {
    match expectation["expect"].as_str() {
        Some("contains") => Ok(actual.as_object().is_some_and(|row| !row.is_empty())
            && expectation["fields"]
                .as_object()
                .ok_or("missing expected fields")?
                .iter()
                .all(|(key, value)| actual[key] == *value)),
        Some("satisfies") => {
            let words: Vec<_> = expectation["predicate"]
                .as_str()
                .ok_or("missing predicate")?
                .split_whitespace()
                .collect();
            if words.len() != 3 {
                return Err("unsupported predicate grammar".to_owned());
            }
            let field = |path: &str| {
                let mut value = actual;
                for part in path.split('.') {
                    value = &value[part];
                }
                value
            };
            let left = field(words[0]);
            // ESS's own rule: a right-hand side with a dot is another field of the same view;
            // without one it is a literal. Only integer and boolean literals are admitted.
            let right = if words[2].contains('.') {
                field(words[2]).clone()
            } else if let Ok(integer) = words[2].parse::<i64>() {
                json!(integer)
            } else if let Ok(truth) = words[2].parse::<bool>() {
                json!(truth)
            } else {
                return Err(format!("unsupported invariant literal {}", words[2]));
            };
            if left.is_null() || right.is_null() {
                return Err("invariant field is absent".to_owned());
            }
            match words[1] {
                "==" => Ok(*left == right),
                ">=" | ">" => {
                    let left = left
                        .as_i64()
                        .ok_or("invariant field is not a representable integer")?;
                    let right = right
                        .as_i64()
                        .ok_or("invariant bound is not a representable integer")?;
                    Ok(if words[1] == ">=" {
                        left >= right
                    } else {
                        left > right
                    })
                }
                other => Err(format!("unsupported invariant operator {other}")),
            }
        }
        other => Err(format!("unsupported view expectation {other:?}")),
    }
}

fn run_scenario(scenario: &Value) -> Result<(), String> {
    let mut actual = Observed::default();
    let mut snapshots = std::collections::BTreeMap::new();
    for step in scenario["steps"]
        .as_array()
        .ok_or("scenario has no steps")?
    {
        let agrees = match step["step"].as_str().ok_or("step has no kind")? {
            "execute_command" => {
                let mut input = serde_json::Map::new();
                let empty = serde_json::Map::new();
                let fields = step.get("input").map_or(Ok(&empty), |value| {
                    value.as_object().ok_or("input is not an object")
                })?;
                for (key, value) in fields {
                    if value["kind"] != "literal" {
                        return Err(format!("unsupported input binding: {value}"));
                    }
                    input.insert(key.clone(), value["value"].clone());
                }
                let mut next = execute(
                    step["command"].as_str().ok_or("missing command")?,
                    &Value::Object(input),
                );
                if next.retained.is_none() {
                    next.retained = actual.retained.take();
                }
                actual = next;
                true
            }
            "expect_outcome" => actual.outcome == step["outcome"]["outcome"],
            "expect_event" => {
                event_matches(actual.events.get(step["event"].as_str().unwrap()), step)?
            }
            "query_view" => {
                query(&actual, step["view"].as_str().ok_or("missing view")?)?;
                true
            }
            "snapshot_view" => {
                let view = step["view"].as_str().ok_or("missing view")?;
                snapshots.insert(view.to_owned(), query(&actual, view)?);
                true
            }
            "expect_view_unchanged" => {
                let view = step["view"].as_str().ok_or("missing view")?;
                snapshots.get(view).ok_or("view was not snapshotted")? == &query(&actual, view)?
            }
            "expect_view" => view_matches(
                &query(&actual, step["view"].as_str().ok_or("missing view")?)?,
                &step["expectation"],
            )?,
            "expect_no_event" => !actual.events.contains_key(step["event"].as_str().unwrap()),
            "expect_no_events" => actual.events.is_empty(),
            "expect_error" => actual.error.as_deref() == step["error"].as_str(),
            "expect_no_error" => actual.error.is_none(),
            other => {
                return Err(format!(
                    "unsupported expectation {other}; cannot silently skip"
                ));
            }
        };
        if !agrees {
            return Err(format!(
                "unmet {step}; observed outcome {}, error {:?}, events {:?}",
                actual.outcome, actual.error, actual.events
            ));
        }
    }
    Ok(())
}

#[test]
fn ess_generated_scenarios_drive_production_targets() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let external = std::env::var_os("METAHARNESS_ESS_SUITE").map(PathBuf::from);
    let source = external
        .clone()
        .unwrap_or_else(|| root.join("spec/conformance.json"));
    let suite: Value = serde_json::from_slice(&fs::read(&source).unwrap()).unwrap();
    assert!(
        [
            "ess-conformance/22",
            "ess-conformance/23",
            "ess-conformance/34",
            "ess-conformance/35"
        ]
        .iter()
        .any(|version| suite["provenance"]["suite_version"] == *version),
        "unsupported suite version"
    );
    // Every scenario starts from `Observed::default()`, so only an empty initial state is honoured.
    let initial = &suite["provenance"]["scenario_initial_state"];
    assert!(
        initial.is_null() || initial == "empty",
        "unsupported scenario initial state: {initial}"
    );
    let scenarios = suite["scenarios"].as_object().unwrap();
    if external.is_none() {
        assert_eq!(
            scenarios.len(),
            111,
            "review and update coverage deliberately"
        );
        assert_eq!(
            suite["coverage"]["counts"]["refused"], 0,
            "every invariant now has an observable production record"
        );
        for command in [
            "PollSteering",
            "FinishCodexCompletion",
            "CloseUnsteeredStream",
            "ExitAuditedRun",
            "CheckSelectedName",
            "DeclareProcessWrites",
            "DeclareScratchWorkingDirectory",
            "DeclareContextFiles",
            "CheckFiniteTerms",
            "CheckUncappedAuthorization",
            "ReadSealedFrame",
            "GovernCodexCall",
            "ReadSelectedWorkspaceFile",
            "ReadNativeTermination",
            "ReadFinalAnswer",
            "ReadObservedModel",
            "ReadToolOutcome",
            "ClassifyEventKind",
            "VerifyStreamCompleteness",
            "NameDigest",
            "ProjectRecordedRun",
            "NormalizeRecord",
            "ProjectVendorRecord",
            "SummarizeRecord",
        ] {
            assert!(
                scenarios.keys().any(|id| id.contains(command)),
                "missing required obligation {command}"
            );
        }
    }
    let mut failed = Vec::new();
    let mut results = Vec::new();
    for (id, scenario) in scenarios {
        let status = match run_scenario(scenario) {
            Ok(()) => "passed",
            Err(reason) => {
                failed.push(format!("{id}: {reason}"));
                "failed"
            }
        };
        results.push(json!({"scenario_id":id,"status":status}));
    }
    let report = json!({"format":"metaharness.ess-conformance/1","claim":"selected executable boundary projection; not full-system or current-vendor qualification",
        "provenance":suite["provenance"],"total":scenarios.len(),"passed":scenarios.len()-failed.len(),"failed":failed.len(),"skipped":0,
        "excluded_invariant_obligations":suite["coverage"]["counts"]["refused"],"failures":failed});
    fs::create_dir_all(root.join("target")).unwrap();
    fs::write(
        root.join("target/metaharness-ess-report.json"),
        serde_json::to_vec_pretty(&report).unwrap(),
    )
    .unwrap();
    println!("{report}");
    let output = std::env::var_os("METAHARNESS_ESS_REPORT").map_or_else(
        || root.join("target/ess-conformance-report.json"),
        PathBuf::from,
    );
    official_report(&source, &output, &results);
    if external.is_none() {
        let official: Value = serde_json::from_slice(&fs::read(&output).unwrap()).unwrap();
        assert_eq!(official["conformance_status"], "passed", "{official}");
    }
    assert!(failed.is_empty(), "{}", failed.join("\n"));
}

fn official_report(source: &Path, output: &Path, results: &[Value]) {
    let completed_at = u64::try_from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis(),
    )
    .unwrap();
    let input = output.with_extension("results.json");
    fs::write(&input,serde_json::to_vec_pretty(&json!({"format":"ess-conformance-results/1","completed_at":completed_at,"results":results})).unwrap()).unwrap();
    let result = Process::new("ess")
        .args(["verify", "conform", "report", "--suite"])
        .arg(source)
        .arg("--results")
        .arg(input)
        .args([
            "--implementation",
            "metaharness-issue-repair",
            "--runner",
            "metaharness-rust-target@1",
            "--report-out",
        ])
        .arg(output)
        .output()
        .unwrap();
    assert!(
        result.status.success(),
        "{}{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );
}

#[test]
fn ess_suite_matches_the_current_specification() {
    let scratch = tempfile::tempdir().unwrap();
    let output = scratch.path().join("suite.json");
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let status = Process::new("ess")
        .args(["verify", "conform", "synthesize", "--path"])
        .arg(root.join("spec"))
        .arg("--scenarios")
        .arg(root.join("spec"))
        .args(["--suite-format", "5", "--strict-requires", "--out"])
        .arg(&output)
        .output()
        .expect("ESS 0.67.0 is a gate prerequisite");
    assert!(
        status.status.success(),
        "{}",
        String::from_utf8_lossy(&status.stderr)
    );
    let fresh: Value = serde_json::from_slice(&fs::read(output).unwrap()).unwrap();
    assert_eq!(
        fresh,
        suite(),
        "regenerate spec/conformance.json with ESS 0.67.0"
    );
}

/// Every terminal-record shape `CloseUnsteeredStream` takes: `terminal_present`, then `is_error`
/// true, false or absent, `subtype` `success`, the budget word, another word or absent, and
/// `stop_reason` and `terminal_reason` each the budget word, another word or absent. 216 shapes;
/// an absent field is left out of the input, as a vendor record leaves it out.
fn terminal_record_shapes() -> Vec<(String, Value)> {
    const BUDGET: &str = "budget-exhausted";
    fn tag(value: Option<&Value>) -> &'static str {
        match value {
            None => "n",
            Some(Value::Bool(true)) => "t",
            Some(Value::Bool(false)) => "f",
            Some(Value::String(word)) if word == "success" => "s",
            Some(Value::String(word)) if word == BUDGET => "b",
            Some(_) => "o",
        }
    }
    let words = [Some(json!(BUDGET)), Some(json!("other")), None];
    let mut shapes = Vec::new();
    for terminal_present in [true, false] {
        for is_error in [Some(json!(true)), Some(json!(false)), None] {
            for subtype in [
                Some(json!("success")),
                Some(json!(BUDGET)),
                Some(json!("other")),
                None,
            ] {
                for stop_reason in words.clone() {
                    for terminal_reason in words.clone() {
                        let mut input = json!({"terminal_present": terminal_present});
                        let fields = [
                            ("is_error", &is_error),
                            ("subtype", &subtype),
                            ("stop_reason", &stop_reason),
                            ("terminal_reason", &terminal_reason),
                        ];
                        let mut name = format!("grid-{}", if terminal_present { "t" } else { "f" });
                        for (field, value) in fields {
                            name.push('-');
                            name.push_str(tag(value.as_ref()));
                            if let Some(value) = value {
                                input[field] = value.clone();
                            }
                        }
                        shapes.push((name, input));
                    }
                }
            }
        }
    }
    shapes
}

/// One authored `ess-scenario/1` for `CloseUnsteeredStream` over `input`, whose outcome and
/// `StreamClassified` reason are the ones production's close produced over the same record.
fn production_scenario(name: &str, input: &Value) -> String {
    let observed = closure(input);
    let reason = &observed.events["metaharness.session.StreamClassified"]["reason"];
    let mut lines = vec![
        "type: ess-scenario/1".to_owned(),
        "domain: metaharness.session".to_owned(),
        format!("scenario: {name}"),
        "summary: Production's close of one terminal-record shape.".to_owned(),
        "timeline:".to_owned(),
        "  - at: 2026-10-21T00:00:00Z".to_owned(),
        "    command: metaharness.session.CloseUnsteeredStream".to_owned(),
        "    input:".to_owned(),
    ];
    for field in [
        "terminal_present",
        "is_error",
        "subtype",
        "stop_reason",
        "terminal_reason",
    ] {
        lines.push(format!(
            "      {field}: {}",
            input.get(field).unwrap_or(&Value::Null)
        ));
    }
    lines.extend([
        format!("    outcome: {}", observed.outcome),
        "    events:".to_owned(),
        "      - event: metaharness.session.StreamClassified".to_owned(),
        format!("        payload: {{reason: {reason}}}"),
    ]);
    lines.join("\n") + "\n"
}

/// `story:session-close-decides-absent-fields`: ESS's reference model decides every terminal-record
/// shape, and decides it as production closes it. Each expectation is read off a real run over
/// `ScriptedRunner` (`closure`), never written by hand, so a shape the specification leaves Unknown
/// or closes otherwise fails by its name.
#[test]
fn the_reference_model_closes_every_terminal_record_shape_as_production_does() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let scenarios = tempfile::tempdir().unwrap();
    let shapes = terminal_record_shapes();
    assert_eq!(shapes.len(), 216, "the grid is 2 x 3 x 4 x 3 x 3 shapes");
    let mut expected = Vec::new();
    for (name, input) in &shapes {
        fs::write(
            scenarios.path().join(format!("{name}.yaml")),
            production_scenario(name, input),
        )
        .unwrap();
        expected.push(format!("metaharness.session/authored/{name}"));
    }
    let report = scenarios.path().join("report.json");
    let output = Process::new("ess")
        .args(["verify", "conform", "run", "--strict-requires", "--path"])
        .arg(root.join("spec"))
        .arg("--scenarios")
        .arg(scenarios.path())
        .args(["--suite-format", "5", "--target", "interpreted"])
        .args(["--report-format", "2", "--report-out"])
        .arg(&report)
        .output()
        .expect("ESS 0.67.0 is a gate prerequisite");
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(
        &fs::read(&report).unwrap_or_else(|_| panic!("no report written:\n{text}")),
    )
    .unwrap();
    let passed = report["outcomes"]["passed"].as_array().unwrap();
    // A shape whose expectation the guards decide otherwise is not run: ESS refuses it at
    // selection (`ESS-AUTHOR-041`) and names it only under `coverage.refused`.
    let refused = report["coverage"]["refused"].as_array().unwrap();
    let undecided: Vec<String> = expected
        .iter()
        .filter(|id| !passed.iter().any(|held| held == id.as_str()))
        .map(|id| {
            if let Some(refusal) = refused.iter().find(|r| r["scenario"] == id.as_str()) {
                let why: String = refusal["message"]
                    .as_str()
                    .unwrap()
                    .chars()
                    .take(400)
                    .collect();
                return format!("refused {id}\n    {why}");
            }
            let mut lines = text
                .lines()
                .skip_while(|line| !line.trim_end().ends_with(id));
            let status = lines
                .next()
                .map_or(format!("absent {id}"), |line| line.trim().to_owned());
            let why: String = lines
                .next()
                .unwrap_or("")
                .trim()
                .chars()
                .take(160)
                .collect();
            format!("{status}\n    {why}")
        })
        .collect();
    assert!(
        undecided.is_empty(),
        "{} of {} terminal-record shapes the specification does not close as production does:\n{}",
        undecided.len(),
        expected.len(),
        undecided.join("\n")
    );
}

#[test]
fn ess_specification_is_part_of_the_repository_gate() {
    let taskfile = include_str!("../../../../Taskfile.yml");
    assert!(taskfile.contains("ess specify validate --path spec"));
}
