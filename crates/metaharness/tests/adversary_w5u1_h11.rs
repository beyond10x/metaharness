//! Adversary cases, wave 5 unit 1 pass 1, against `story:claude-h11-memory-walk-shapes` (design
//! amendment a36).
//!
//! Each case drives the production builder, a scripted runner and the production floor, or reads a
//! document the unit changed. No vendor process, no credential, no network.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, Event, HermeticAttestation, HermeticMode, HermeticRow, Kind,
    Verdict,
};
use metaharness::{
    ClaudeSeams, Input, ManualClock, Metaharness, Run, ScriptStep, ScriptedLog, ScriptedRunner,
};

const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false}"#;

fn init_line(version: &str, cwd: &Path) -> String {
    format!(
        r#"{{"type":"system","subtype":"init","claude_code_version":"{version}","cwd":{},"tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[{{"name":"cc-plugin-agents-md","source":"cc-plugin-agents-md@builtin"}},{{"name":"cc-plugin-plugin-authoring","source":"cc-plugin-plugin-authoring@builtin"}}]}}"#,
        serde_json::Value::from(cwd.display().to_string())
    )
}

/// A fresh root holding `copy/`, the directory the embedder made.
fn embedder_copy() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    (root, copy)
}

/// A strict credential-free Claude run over the declared copy, its opening record on `version`.
fn claude_declared_copy(copy: &Path, version: &str) -> Run {
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(init_line(version, copy)),
            ScriptStep::line(RESULT),
        ],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .with_hermetic(HermeticMode::Strict)
        .with_cwd(copy)
        .with_scratch_cwd()
        .start_with_clock(
            Input::Prompt("synthetic".into()),
            &mut runner,
            &mut ClaudeSeams,
            Box::new(ManualClock::new()),
        )
        .expect("a memory file above the copy refuses nothing (a36)");
    run.drain().expect("drains");
    run
}

fn attestation(run: &Run) -> HermeticAttestation {
    run.events()
        .iter()
        .find_map(|event| match event {
            Event::SessionStarted { hermetic, .. } => Some(hermetic.clone()),
            _ => None,
        })
        .expect("the run opened")
}

fn h11(run: &Run) -> (Verdict, String) {
    let row = run
        .hermetic_floor()
        .into_iter()
        .find(|row| row.row == HermeticRow::H11)
        .expect("H11 is evaluated");
    (row.verdict, row.detail)
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

fn pinned() -> &'static str {
    metaharness_claude::PINNED_VERSIONS[0]
}

// ------------------------------------------------------------------ the walk under hostile trees

/// Symlinks under `.claude/rules` that point back up the tree cannot make the walk circle, and a
/// linked `.md` file is named like a real one.
#[test]
fn adversary_w5u1_walk_terminates_over_rules_symlink_loops_and_names_linked_files() {
    let (root, copy) = embedder_copy();
    let rules = root.path().join(".claude/rules");
    write(&rules.join("real.md"), "canary");
    write(&root.path().join("elsewhere/target.md"), "canary");
    std::os::unix::fs::symlink(root.path().join(".claude"), rules.join("loop")).unwrap();
    std::os::unix::fs::symlink(root.path(), rules.join("up")).unwrap();
    std::os::unix::fs::symlink(&rules, rules.join("self")).unwrap();
    std::os::unix::fs::symlink(
        root.path().join("elsewhere/target.md"),
        rules.join("linked.md"),
    )
    .unwrap();
    let seen = std::fs::canonicalize(root.path()).unwrap();

    let run = claude_declared_copy(&copy, pinned());
    let (verdict, detail) = h11(&run);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    for named in [".claude/rules/real.md", ".claude/rules/linked.md"] {
        let path = seen.join(named).display().to_string();
        assert!(detail.contains(&path), "{named} not named: {detail}");
    }
    for unfollowed in ["/loop/", "/up/", "/self/"] {
        assert!(
            !detail.contains(unfollowed),
            "{unfollowed} followed: {detail}"
        );
    }
}

/// An unreadable directory under `.claude/rules` neither panics the walk nor refuses the run.
#[test]
fn adversary_w5u1_walk_survives_an_unreadable_rules_directory() {
    use std::os::unix::fs::PermissionsExt;
    let (root, copy) = embedder_copy();
    let locked = root.path().join(".claude/rules/locked");
    write(&locked.join("hidden.md"), "canary");
    write(&root.path().join(".claude/rules/open.md"), "canary");
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();

    let run = claude_declared_copy(&copy, pinned());
    let (verdict, detail) = h11(&run);
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o755)).unwrap();
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(detail.contains("open.md"), "{detail}");
}

/// A rules file whose name is not UTF-8 is named, lossily, and panics nothing.
#[test]
fn adversary_w5u1_walk_names_a_non_utf8_rules_file() {
    use std::os::unix::ffi::OsStrExt;
    let (root, copy) = embedder_copy();
    let rules = root.path().join(".claude/rules");
    std::fs::create_dir_all(&rules).unwrap();
    let name = std::ffi::OsStr::from_bytes(b"\xffodd.md");
    std::fs::write(rules.join(name), "canary").unwrap();

    let run = claude_declared_copy(&copy, pinned());
    let (verdict, detail) = h11(&run);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    assert!(detail.contains("\u{FFFD}odd.md"), "{detail}");
}

/// Memory shapes at the copy's root are the copy's own: listed in `ambient_inputs`, never named
/// in H11 as above it.
#[test]
fn adversary_w5u1_every_shape_at_the_copy_root_is_the_copys_own_and_not_in_h11() {
    let (_root, copy) = embedder_copy();
    let shapes = [
        "CLAUDE.local.md",
        ".claude/CLAUDE.md",
        ".claude/AGENTS.md",
        ".claude/rules/deep/inner.md",
    ];
    for shape in shapes {
        write(&copy.join(shape), "the copy's own");
    }
    let seen = std::fs::canonicalize(&copy).unwrap();

    let run = claude_declared_copy(&copy, pinned());
    let attested = attestation(&run);
    let (verdict, detail) = h11(&run);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
    for shape in shapes {
        let path = seen.join(shape).display().to_string();
        assert!(
            attested
                .ambient_inputs
                .iter()
                .any(|input| input.contains(&path)),
            "{shape} not listed: {:?}",
            attested.ambient_inputs
        );
        assert!(!detail.contains(&path), "{shape} named in H11: {detail}");
    }
}

// ------------------------------------------------------------------ the canary key

/// The canary is keyed by the record's version exactly: a banner spelling of a pinned version
/// finds no canary and H11 is not `ok`.
#[test]
fn adversary_w5u1_a_version_banner_is_not_a_pinned_canary_key() {
    let (_root, copy) = embedder_copy();
    let banner = format!("{} (Claude Code)", pinned());
    let run = claude_declared_copy(&copy, &banner);
    let (verdict, detail) = h11(&run);
    assert_eq!(verdict, Verdict::Gap, "{detail}");
    assert!(detail.contains("no memory canary"), "{detail}");
}

/// The second pin has its own row and reads `ok` on it, not on the first pin's.
#[test]
fn adversary_w5u1_each_pin_reads_its_own_canary() {
    for version in metaharness_claude::PINNED_VERSIONS {
        let (_root, copy) = embedder_copy();
        let run = claude_declared_copy(&copy, version);
        let (verdict, detail) = h11(&run);
        assert_eq!(verdict, Verdict::Ok, "{version}: {detail}");
        assert!(
            detail.contains(&format!("memory canary recorded for {version}:")),
            "{version}: {detail}"
        );
    }
}

// ------------------------------------------------------------------ b10x keeps its two shapes

const B10X_STARTED: &str =
    r#"{"kind":"started","model":"synthetic","published_tools":[],"credential_source":"none"}"#;
const B10X_FINISHED: &str = r#"{"kind":"finished","stop":{"kind":"completed"}}"#;

fn b10x_declared_copy(copy: &Path) -> Run {
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut runner = ScriptedRunner::of_lines([B10X_STARTED, B10X_FINISHED], ScriptedLog::new());
    let mut run = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .with_cwd(copy)
        .with_scratch_cwd()
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("a declared copy plans on b10x");
    run.drain().unwrap();
    run
}

/// a36 is Claude-only: the shared walk still counts `CLAUDE.md` and `AGENTS.md` alone on b10x,
/// and one of them above the copy still makes H11 unavailable there.
#[test]
fn adversary_w5u1_b10x_walk_keeps_two_shapes_and_its_verdict() {
    let (root, copy) = embedder_copy();
    for shape in [
        "CLAUDE.local.md",
        ".claude/CLAUDE.md",
        ".claude/AGENTS.md",
        ".claude/rules/r.md",
    ] {
        write(&root.path().join(shape), "not a b10x memory shape");
    }
    let run = b10x_declared_copy(&copy);
    assert!(
        attestation(&run).claims(HermeticRow::H11),
        "{:?}",
        attestation(&run)
    );

    let (root, copy) = embedder_copy();
    write(&root.path().join("AGENTS.md"), "a b10x memory shape");
    let run = b10x_declared_copy(&copy);
    let attested = attestation(&run);
    assert!(!attested.claims(HermeticRow::H11), "{attested:?}");
    assert!(
        attested
            .unavailable
            .iter()
            .any(|control| control.row == HermeticRow::H11
                && control.why.contains("1 memory file(s)")),
        "{attested:?}"
    );
}

// ------------------------------------------------------------------ documents the unit changed

fn repository_file(relative: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative);
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// The research page the canary rows cite opens with its own title. The unit's edit put a
/// sentence fragment in front of the `#` heading on line 1.
#[test]
fn adversary_w5u1_research_page_opens_with_its_title() {
    let page = repository_file("docs/research/2026-10-21-claude-code-2.1.304-adapter-surface.md");
    let first = page.lines().next().unwrap_or_default();
    assert_eq!(
        first, "# Claude Code 2.1.304 and 2.1.305 adapter surface",
        "line 1 of the research page"
    );
}

/// The public hermetic page says what the code does on the declared copy: since a36 Claude's H11
/// is no longer measured by the walk there. A page that still says H11 "stays measured" without
/// naming the Claude exception contradicts `website/docs/reference/cli.md` and the code.
#[test]
fn adversary_w5u1_hermetic_page_does_not_say_h11_stays_measured_on_claude() {
    let page = repository_file("website/docs/hermetic.md");
    let paragraph = page
        .split("\n\n")
        .find(|paragraph| paragraph.contains("H11 stays measured"));
    if let Some(paragraph) = paragraph {
        assert!(
            paragraph.contains("a36") || paragraph.contains("Claude"),
            "website/docs/hermetic.md still says, for every kind: {paragraph}"
        );
    }
}
