//! Adversary pass 2 on `story:session-request-carries-context-files` (design amendment a35).
//!
//! Drives the production builder over a scripted runner in git trees this test makes, on the lines
//! pass 1 did not reach: what the workspace domain models against what production refuses, files in
//! a submodule, the opening record a codex seam holds to the end, and which bytes are handed over
//! when the index and the work tree disagree (a modified file, an intent-to-add entry, a sparse
//! checkout, a hard link). No vendor process, no credential, no network.
//!
//! The cases marked *characterisation* are green: they pin what the code does where a35 says
//! nothing, so a reader can see the answer without re-deriving it.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;

use metaharness::protocol::{CredentialSource, DecisionMode, Digest, Event, Kind};
use metaharness::{
    ContextRefusal, Input, ManualClock, Metaharness, Refusal, Run, ScriptedLog, ScriptedRunner,
    ScriptedSeams,
};
use serde_json::Value;

const TODO: &str = "def add(items, item):\n    items.append(item)\n";

/// Run git with no operator configuration, failing the test on a non-zero exit.
fn git(directory: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .current_dir(directory)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_NO_LAZY_FETCH", "1")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.invalid",
            "-c",
            "init.defaultBranch=main",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        output.status.success(),
        "git {args:?} in {}: {}",
        directory.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

/// A plain git work tree `work/` under a fresh root, holding a tracked `todo.py`.
struct Tree {
    root: tempfile::TempDir,
    work: PathBuf,
}

fn tree() -> Tree {
    let root = tempfile::tempdir().unwrap();
    let work = root.path().join("work");
    std::fs::create_dir(&work).unwrap();
    git(&work, &["init", "--quiet"]);
    std::fs::write(work.join("todo.py"), TODO).unwrap();
    git(&work, &["add", "todo.py"]);
    Tree { root, work }
}

fn builder(kind: Kind) -> Metaharness {
    let builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    match kind {
        Kind::B10x => builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called"),
        Kind::Claude | Kind::Codex => builder,
    }
}

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

fn start(builder: Metaharness, log: &ScriptedLog) -> Result<Run, Refusal> {
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let mut run = builder.start_with(
        Input::Prompt("write the failing test".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    )?;
    run.drain().expect("the scripted run drains");
    Ok(run)
}

fn context_files_of(run: &Run) -> Value {
    let event = run
        .events()
        .iter()
        .find(|event| matches!(event, Event::SessionStarted { .. }))
        .expect("the run opened");
    serde_json::to_value(event).unwrap()["context_files"].clone()
}

/// The value of `--append-system-prompt`, where Claude Code gets the block.
fn appended(log: &ScriptedLog) -> String {
    let argv = log.launched().pop().expect("one launch");
    argv.windows(2)
        .find(|pair| pair[0] == "--append-system-prompt")
        .map(|pair| pair[1].clone())
        .expect("claude carries the block in --append-system-prompt")
}

fn entry(path: &str, bytes: &[u8], channel: &str) -> Value {
    serde_json::json!([{
        "path": path,
        "sha256": Digest::of(bytes).as_str(),
        "bytes": bytes.len(),
        "channel": channel,
    }])
}

/// The variant's name, as `Debug` spells it.
fn variant(why: &ContextRefusal) -> String {
    format!("{why:?}")
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap()
        .to_owned()
}

/// Every `error:` the workspace domain's `DeclareContextFiles` command names, in order.
fn declare_context_files_errors() -> Vec<String> {
    let domain = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../spec/domains/workspace.yaml");
    let text = std::fs::read_to_string(&domain).unwrap();
    let mut inside = false;
    let mut errors = Vec::new();
    for line in text.lines() {
        if line == "  - name: metaharness.workspace.DeclareContextFiles" {
            inside = true;
            continue;
        }
        if inside && (line.starts_with("  - name:") || (!line.is_empty() && !line.starts_with(' ')))
        {
            break;
        }
        if inside && let Some(error) = line.trim_start().strip_prefix("error: ") {
            errors.push(error.to_owned());
        }
    }
    assert!(
        inside,
        "no DeclareContextFiles command in {}",
        domain.display()
    );
    errors
}

/// The acceptance: "the workspace domain gains a `DeclareContextFiles` command with an outcome per
/// refusal". Production refuses a declaration for twelve reasons (`ContextRefusal`), each reached
/// here through the builder before any spawn; the domain names eight. The four a35 decision 1 added
/// beyond the story's list — an empty path, a non-UTF-8 path, a file that is not there, content that
/// is not text — have no outcome, and the ESS target (`ess_conformance.rs`, `context_start`) panics
/// on each as "an undeclared refusal" instead of deriving an outcome.
#[test]
fn every_refusal_production_raises_for_a_declaration_has_an_outcome_in_the_workspace_domain() {
    let tree = tree();
    let work: &Path = &tree.work;
    std::os::unix::fs::symlink("todo.py", work.join("link.py")).unwrap();
    std::fs::create_dir(work.join("tests")).unwrap();
    std::fs::write(work.join("tests/keep.py"), "x\n").unwrap();
    std::fs::write(work.join("untracked.txt"), "not in the index\n").unwrap();
    std::fs::write(work.join("blob.bin"), [0xff_u8, 0xfe, 0x00, 0x41]).unwrap();
    std::fs::write(
        work.join("big.txt"),
        "y".repeat(metaharness::protocol::CONTEXT_BYTES_MAX),
    )
    .unwrap();
    let many: Vec<String> = (0..=metaharness::protocol::CONTEXT_FILES_MAX)
        .map(|index| format!("f{index:02}.txt"))
        .collect();
    for name in &many {
        std::fs::write(work.join(name), "x\n").unwrap();
    }
    let mut add = vec!["add", "link.py", "tests/keep.py", "blob.bin", "big.txt"];
    add.extend(many.iter().map(String::as_str));
    git(work, &add);

    let not_utf8 = PathBuf::from(OsStr::from_bytes(b"bad-\xff.txt"));
    let declarations: Vec<(Option<&Path>, Vec<PathBuf>)> = vec![
        (None, vec!["todo.py".into()]),
        (Some(work), vec!["".into()]),
        (Some(work), vec![work.join("todo.py")]),
        (Some(work), vec!["tests/../todo.py".into()]),
        (Some(work), vec![not_utf8]),
        (Some(work), vec!["link.py".into()]),
        (Some(work), vec!["tests".into()]),
        (Some(work), vec!["absent.py".into()]),
        (Some(work), vec!["blob.bin".into()]),
        (Some(work), vec!["untracked.txt".into()]),
        (Some(work), many.iter().map(PathBuf::from).collect()),
        (Some(work), vec!["big.txt".into()]),
    ];
    let mut raised = Vec::new();
    for (cwd, declared) in declarations {
        let mut declaring = builder(Kind::Claude);
        if let Some(cwd) = cwd {
            declaring = declaring.with_cwd(cwd);
        }
        for path in &declared {
            declaring = declaring.with_context(path);
        }
        let log = ScriptedLog::new();
        match start(declaring, &log) {
            Err(Refusal::ContextFile { why, .. }) => {
                assert_eq!(log.spawns(), 0);
                let name = variant(&why);
                if !raised.contains(&name) {
                    raised.push(name);
                }
            }
            Err(other) => panic!("{declared:?}: not a context refusal: {other}"),
            Ok(_) => panic!("{declared:?} was accepted"),
        }
    }
    assert_eq!(
        raised.len(),
        12,
        "every ContextRefusal variant is reachable: {raised:?}"
    );

    let errors = declare_context_files_errors();
    assert_eq!(
        errors.len(),
        raised.len(),
        "production refuses a --context declaration for {} reasons {raised:?}; the workspace \
         domain's DeclareContextFiles has {} refusal outcomes {errors:?}",
        raised.len(),
        errors.len()
    );
}

/// "A file not tracked by git in the `--cwd` tree" is refused; a file an initialised submodule of
/// that tree tracks is tracked by git in it. The check runs `git ls-files` in the superproject,
/// which lists the gitlink `sub` and never `sub/lib.py`, so the file is refused as untracked. The
/// control below shows git does track it, from the submodule's own index.
#[test]
fn a_file_tracked_in_an_initialised_submodule_of_the_cwd_tree_is_not_refused_as_untracked() {
    let tree = tree();
    let origin = tree.root.path().join("lib-origin");
    std::fs::create_dir(&origin).unwrap();
    git(&origin, &["init", "--quiet"]);
    let lib = "def lib():\n    return 1\n";
    std::fs::write(origin.join("lib.py"), lib).unwrap();
    git(&origin, &["add", "lib.py"]);
    git(&origin, &["commit", "--quiet", "-m", "lib"]);
    git(
        &tree.work,
        &[
            "-c",
            "protocol.file.allow=always",
            "submodule",
            "add",
            "--quiet",
            origin.to_str().unwrap(),
            "sub",
        ],
    );
    assert!(
        tree.work.join("sub/.git").is_file(),
        "an initialised submodule"
    );
    assert_eq!(
        git(&tree.work.join("sub"), &["ls-files", "--", "lib.py"]),
        "lib.py\n",
        "control: the submodule's index tracks the file"
    );

    for kind in [Kind::Claude, Kind::Codex, Kind::B10x] {
        let log = ScriptedLog::new();
        let run = start(
            builder(kind)
                .with_cwd(&tree.work)
                .with_context("sub/lib.py"),
            &log,
        )
        .unwrap_or_else(|refusal| panic!("{kind:?}: {refusal}"));
        assert_eq!(context_files_of(&run)[0]["path"], "sub/lib.py", "{kind:?}");
    }
}

const META: &str = r#"{"timestamp":"2026-10-21T00:00:00.000Z","type":"session_meta","payload":{"id":"s-1","session_id":"s-1","cli_version":"0.153.4","cwd":"/w"}}"#;
const TASK_STARTED: &str = r#"{"timestamp":"2026-10-21T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#;
const TURN_CONTEXT: &str = r#"{"timestamp":"2026-10-21T00:00:00.003Z","type":"turn_context","payload":{"turn_id":"t1","model":"m"}}"#;

/// A codex run with the production seam (`CodexSeams`) over a scripted rollout.
fn codex_run(work: &Path, lines: &[&str]) -> Run {
    let mut runner = ScriptedRunner::of_lines(lines.iter().copied(), ScriptedLog::new());
    let mut run = Metaharness::new(Kind::Codex)
        .with_decisions(DecisionMode::Observe)
        .with_credentials(CredentialSource::None)
        .with_cwd(work)
        .with_context("todo.py")
        .start_with_clock(
            Input::Prompt("adversary".to_owned()),
            &mut runner,
            &mut metaharness::CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("a synthetic codex run starts");
    run.drain().expect("the synthetic run drains");
    run
}

/// a35 decision 7: the run fills `context_files` "including on an opening record a seam releases
/// only at the end (codex, a34)". The reader holds `session.started` until a `turn_context`
/// completes it; a stream that ends first has it released by the seam's `finish`, in `wind_up`.
/// No other case drives that road with a declared file: `context_files.rs` and the ESS target use
/// `ScriptedSeams`, which never hold. Green; it is the case that catches the line being dropped.
#[test]
fn codex_names_its_context_files_on_an_opening_record_held_until_the_stream_ended() {
    let tree = tree();
    let run = codex_run(&tree.work, &[META, TASK_STARTED]);
    assert_eq!(
        context_files_of(&run),
        entry("todo.py", TODO.as_bytes(), "user-prompt")
    );
}

/// The same opening record, released mid-stream by the `turn_context` that completes it.
#[test]
fn codex_names_its_context_files_on_an_opening_record_a_turn_context_released() {
    let tree = tree();
    let run = codex_run(&tree.work, &[META, TASK_STARTED, TURN_CONTEXT]);
    assert_eq!(
        context_files_of(&run),
        entry("todo.py", TODO.as_bytes(), "user-prompt")
    );
}

/// Characterisation. A tracked file changed in the work tree after `git add`: the work-tree bytes
/// are digested and rendered, the index's are not, and the record carries nothing that says the two
/// differ (its sha256 is of the content, git's blob id is of a header and the content).
#[test]
fn a_tracked_file_modified_in_the_work_tree_is_handed_over_as_its_work_tree_bytes() {
    let tree = tree();
    let changed = "def add(items, item):\n    raise SystemExit\n";
    std::fs::write(tree.work.join("todo.py"), changed).unwrap();
    assert_eq!(git(&tree.work, &["diff", "--name-only"]), "todo.py\n");
    let log = ScriptedLog::new();
    let run = start(
        builder(Kind::Claude)
            .with_cwd(&tree.work)
            .with_context("todo.py"),
        &log,
    )
    .unwrap();
    assert_eq!(
        context_files_of(&run),
        entry("todo.py", changed.as_bytes(), "system-prompt")
    );
    let block = appended(&log);
    assert!(block.contains(changed) && !block.contains(TODO), "{block}");
}

/// Characterisation. `git add -N` puts an entry with no content in the index; the check counts it
/// as tracked and the work-tree bytes are handed over.
#[test]
fn an_intent_to_add_entry_passes_the_tracked_check_with_its_work_tree_bytes() {
    let tree = tree();
    let fresh = "written after the last add\n";
    std::fs::write(tree.work.join("fresh.txt"), fresh).unwrap();
    git(&tree.work, &["add", "--intent-to-add", "fresh.txt"]);
    assert_eq!(
        git(&tree.work, &["ls-files", "--stage", "--", "fresh.txt"]),
        "100644 e69de29bb2d1d6434b8b29ae775ad8c2e48c5391 0\tfresh.txt\n",
        "control: the index holds the empty blob"
    );
    let log = ScriptedLog::new();
    let run = start(
        builder(Kind::Claude)
            .with_cwd(&tree.work)
            .with_context("fresh.txt"),
        &log,
    )
    .unwrap();
    assert_eq!(
        context_files_of(&run),
        entry("fresh.txt", fresh.as_bytes(), "system-prompt")
    );
}

/// Characterisation. A cone-mode sparse checkout with a sparse index: `outside/b.txt` is out of the
/// cone, so git removed it from disk; a file written back at that path is handed over. The index
/// expands without a fetch (the objects are local), so the tracked check lists it, and the bytes
/// are whatever was written, not the blob git holds.
#[test]
fn a_file_written_outside_a_sparse_cone_is_handed_over_from_disk() {
    let root = tempfile::tempdir().unwrap();
    let origin = root.path().join("origin");
    std::fs::create_dir_all(origin.join("inside")).unwrap();
    std::fs::create_dir_all(origin.join("outside")).unwrap();
    git(&origin, &["init", "--quiet"]);
    std::fs::write(origin.join("inside/a.txt"), "in\n").unwrap();
    std::fs::write(origin.join("outside/b.txt"), "committed\n").unwrap();
    git(&origin, &["add", "-A"]);
    git(&origin, &["commit", "--quiet", "-m", "one"]);
    let work = root.path().join("work");
    git(
        root.path(),
        &[
            "clone",
            "--quiet",
            origin.to_str().unwrap(),
            work.to_str().unwrap(),
        ],
    );
    git(
        &work,
        &[
            "sparse-checkout",
            "set",
            "--cone",
            "--sparse-index",
            "inside",
        ],
    );
    assert!(!work.join("outside/b.txt").exists(), "out of the cone");
    std::fs::create_dir_all(work.join("outside")).unwrap();
    let planted = "planted outside the cone\n";
    std::fs::write(work.join("outside/b.txt"), planted).unwrap();
    let log = ScriptedLog::new();
    let run = start(
        builder(Kind::Claude)
            .with_cwd(&work)
            .with_context("outside/b.txt"),
        &log,
    )
    .unwrap_or_else(|refusal| panic!("{refusal}"));
    assert_eq!(
        context_files_of(&run),
        entry("outside/b.txt", planted.as_bytes(), "system-prompt")
    );
}

/// Characterisation. A tracked path that is a hard link to a file beside the tree is not a symbolic
/// link, so `openat2` opens it and the shared inode's bytes are handed over — including bytes
/// written to the outside name after the `git add`, which the index never saw.
#[test]
fn a_tracked_hard_link_to_a_file_outside_the_tree_hands_over_the_shared_bytes() {
    let tree = tree();
    let outside = tree.root.path().join("outside-notes.md");
    std::fs::write(&outside, "outside bytes\n").unwrap();
    std::fs::hard_link(&outside, tree.work.join("notes.md")).unwrap();
    git(&tree.work, &["add", "notes.md"]);
    let later = "outside bytes\nappended through the outside name\n";
    std::fs::write(&outside, later).unwrap();
    let log = ScriptedLog::new();
    let run = start(
        builder(Kind::Claude)
            .with_cwd(&tree.work)
            .with_context("notes.md"),
        &log,
    )
    .unwrap();
    assert_eq!(
        context_files_of(&run),
        entry("notes.md", later.as_bytes(), "system-prompt")
    );
    assert!(appended(&log).contains(later));
}
