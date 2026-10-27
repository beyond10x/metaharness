//! Adversary cases, pass 1, against `story:off-pin-vendor-version-is-advisory` (design amendment
//! a62).
//!
//! Each case drives the production builder and the production readers (`ClaudeSeams`,
//! `CodexSeams`) over a scripted runner, then reads the production floor. No vendor process, no
//! credential, no network.

use std::path::{Path, PathBuf};

use metaharness::protocol::{
    CredentialSource, DecisionMode, Event, HermeticMode, HermeticRow, Kind, Verdict, warning_code,
};
use metaharness::{
    ClaudeSeams, CodexSeams, FakeAuditor, Input, ManualClock, Metaharness, Run, RunExit,
    ScriptStep, ScriptedLog, ScriptedRunner,
};

const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false}"#;

/// The two built-ins every credential-free opening record of every pinned binary carries
/// (`metaharness_claude::BUILTIN_PLUGINS`, "observed on all three").
const BUILTINS: &str = r#"[{"name":"cc-plugin-agents-md","source":"cc-plugin-agents-md@builtin"},{"name":"cc-plugin-plugin-authoring","source":"cc-plugin-plugin-authoring@builtin"}]"#;

fn copy_dir() -> (tempfile::TempDir, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    let canonical = std::fs::canonicalize(&copy).unwrap();
    (root, canonical)
}

fn claude_init(version: Option<&str>, cwd: &Path, plugins: &str) -> String {
    let mut record = serde_json::json!({
        "type": "system",
        "subtype": "init",
        "cwd": cwd.display().to_string(),
        "tools": [],
        "mcp_servers": [],
        "output_style": "default",
        "apiKeySource": "none",
        "plugins": serde_json::from_str::<serde_json::Value>(plugins).unwrap(),
    });
    if let Some(version) = version {
        record["claude_code_version"] = version.into();
    }
    record.to_string()
}

/// A strict credential-free Claude run over a declared copy, its opening record naming
/// `version` and listing `plugins`.
fn claude_strict(version: Option<&str>, plugins: &str) -> (tempfile::TempDir, Run) {
    let (root, copy) = copy_dir();
    let mut runner = ScriptedRunner::new(
        vec![
            ScriptStep::line(claude_init(version, &copy, plugins)),
            ScriptStep::line(RESULT),
        ],
        ScriptedLog::new(),
    );
    let mut run = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
        .with_hermetic(HermeticMode::Strict)
        .with_cwd(&copy)
        .with_scratch_cwd()
        .start_with_clock(
            Input::Prompt("synthetic".into()),
            &mut runner,
            &mut ClaudeSeams,
            Box::new(ManualClock::new()),
        )
        .expect("a declared copy plans");
    run.drain().expect("drains");
    (root, run)
}

fn codex_rollout(version: Option<&str>, cwd: &Path) -> Vec<String> {
    let mut payload = serde_json::json!({
        "id": "s-1",
        "session_id": "s-1",
        "cwd": cwd.display().to_string(),
    });
    if let Some(version) = version {
        payload["cli_version"] = version.into();
    }
    vec![
        serde_json::json!({"timestamp":"2026-10-27T00:00:00.000Z","type":"session_meta","payload":payload}).to_string(),
        r#"{"timestamp":"2026-10-27T00:00:00.001Z","type":"event_msg","payload":{"type":"task_started","turn_id":"t1"}}"#.to_owned(),
        r#"{"timestamp":"2026-10-27T00:00:00.002Z","type":"turn_context","payload":{"turn_id":"t1","model":"m","personality":"pragmatic"}}"#.to_owned(),
        r#"{"timestamp":"2026-10-27T00:00:01.000Z","type":"event_msg","payload":{"type":"task_complete","turn_id":"t1","error":null,"last_agent_message":"done"}}"#.to_owned(),
    ]
}

fn codex_strict(version: Option<&str>) -> (tempfile::TempDir, Run) {
    let (root, copy) = copy_dir();
    let mut runner = ScriptedRunner::of_lines(codex_rollout(version, &copy), ScriptedLog::new());
    let mut run = Metaharness::new(Kind::Codex)
        .with_hermetic(HermeticMode::Strict)
        .with_decisions(DecisionMode::Observe)
        .with_credentials(CredentialSource::None)
        .with_cwd(&copy)
        .with_scratch_cwd()
        .start_with_clock(
            Input::Prompt("synthetic".to_owned()),
            &mut runner,
            &mut CodexSeams,
            Box::new(ManualClock::new()),
        )
        .expect("a declared copy plans");
    run.drain().unwrap();
    (root, run)
}

fn verdict(run: &Run, which: HermeticRow) -> (Verdict, String) {
    let row = run
        .hermetic_floor()
        .into_iter()
        .find(|row| row.row == which)
        .unwrap();
    (row.verdict, row.detail)
}

fn exit(run: &Run) -> RunExit {
    let report = run.audit(&mut FakeAuditor::missing()).expect("no auditor");
    run.exit(Some(&report))
}

fn rendered(run: &Run) -> String {
    run.audit(&mut FakeAuditor::missing())
        .expect("no auditor")
        .render()
}

fn version_warnings(run: &Run) -> usize {
    run.events()
        .iter()
        .filter(|event| {
            matches!(event, Event::Warning { code, .. } if code == warning_code::VERSION_OUTSIDE_PIN)
        })
        .count()
}

/// Spellings that look pinned and are not, beside the pin itself and no version at all.
fn spellings(pin: &str) -> Vec<Option<String>> {
    vec![
        Some(pin.to_owned()),
        Some(format!(" {pin}")),
        Some(format!("{pin} ")),
        Some(format!("v{pin}")),
        Some(format!("{pin}-alpha")),
        Some(format!("{pin}\n")),
        Some(format!("{pin}.0")),
        Some(String::new()),
        None,
    ]
}

/// The warning and H9 read one comparison: for every spelling, exactly one warning when H9 is
/// `gap`, none otherwise, and H9 `unk` only when the record names no version. Claude and Codex.
#[test]
fn adversary_off_pin_pass1_the_warning_and_h9_agree_on_every_spelling() {
    let claude_pin = metaharness_claude::PINNED_VERSIONS[2];
    for version in spellings(claude_pin) {
        let (_root, run) = claude_strict(version.as_deref(), BUILTINS);
        let (h9, detail) = verdict(&run, HermeticRow::H9);
        let expected = match version.as_deref() {
            None => Verdict::Unk,
            Some(v) if v == claude_pin => Verdict::Ok,
            Some(_) => Verdict::Gap,
        };
        assert_eq!(h9, expected, "claude {version:?}: {detail}");
        assert_eq!(
            version_warnings(&run),
            usize::from(h9 == Verdict::Gap),
            "claude {version:?}: {:?}",
            run.events()
        );
    }
    let codex_pin = metaharness_codex::PINNED_VERSIONS[1];
    for version in spellings(codex_pin) {
        let (_root, run) = codex_strict(version.as_deref());
        let (h9, detail) = verdict(&run, HermeticRow::H9);
        let expected = match version.as_deref() {
            None => Verdict::Unk,
            Some(v) if v == codex_pin => Verdict::Ok,
            Some(_) => Verdict::Gap,
        };
        assert_eq!(h9, expected, "codex {version:?}: {detail}");
        assert_eq!(
            version_warnings(&run),
            usize::from(h9 == Verdict::Gap),
            "codex {version:?}: {:?}",
            run.events()
        );
        assert_eq!(
            exit(&run),
            exit(&codex_strict(Some(codex_pin)).1),
            "codex {version:?}: a version alone moves no exit code\n{}",
            rendered(&run)
        );
    }
}

/// The story's outcome: "A Claude or Codex binary outside the adapter's pin is measured and
/// reported, never refused for its version alone". A strict Claude run whose opening record is
/// the shape every credential-free record of every pinned binary has — the two built-ins — and
/// differs from a pinned run only in its version, exits 3 (H11 `unk`, nobody measured it) and
/// never exits 1. It does exit 1: H1a reads the two built-ins as `gap` because built-ins are
/// declared per pinned version, so the version alone fails the run through another row.
#[test]
fn adversary_off_pin_pass1_a_realistic_off_pin_claude_record_is_not_failed_for_its_version() {
    let pin = metaharness_claude::PINNED_VERSIONS[2];
    let (_root, pinned) = claude_strict(Some(pin), BUILTINS);
    assert_eq!(
        verdict(&pinned, HermeticRow::H1a).0,
        Verdict::Ok,
        "precondition: the built-ins are admitted on the pin\n{}",
        rendered(&pinned)
    );
    let (_root, off) = claude_strict(Some("2.1.400"), BUILTINS);
    let not_ok_gating: Vec<String> = off
        .hermetic_floor()
        .into_iter()
        .filter(metaharness::protocol::RowVerdict::gates)
        .map(|row| format!("{} {:?}", row.row.id(), row.verdict))
        .collect();
    assert_eq!(
        exit(&off),
        RunExit::NoVerdict,
        "story:off-pin-vendor-version-is-advisory outcome: never refused for its version alone; \
         gating rows not ok: {not_ok_gating:?}\n{}",
        rendered(&off)
    );
}
