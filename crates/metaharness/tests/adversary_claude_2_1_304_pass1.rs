//! Adversary cases, pass 1, against `story:claude-code-2-1-304-declared` (design amendment a33).
//!
//! Each case drives the production builder, the Claude adapter's own reader (through
//! [`ClaudeSeams`]), a scripted runner and the production floor. No vendor process, no credential,
//! no network: the scripted runner records a credential copy and a spawn without performing either.
//!
//! The opening records below are shaped as the 2.1.304 and 2.1.305 binaries wrote theirs in a
//! credential-free run against a loopback stub (`--credentials none --model-endpoint`), read on
//! 2026-10-21: `apiKeySource` is `"ANTHROPIC_API_KEY"` there, because the launch hands the child
//! its endpoint placeholder in that variable.

use std::path::Path;

use metaharness::protocol::{CredentialSource, HermeticMode, HermeticRow, Kind, Verdict};
use metaharness::{
    ClaudeSeams, Input, ManualClock, Metaharness, Refusal, Run, ScriptStep, ScriptedLog,
    ScriptedRunner,
};

const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false}"#;

/// A Claude `system/init` line: the version and `apiKeySource` it reports, and the cwd it names.
fn init_line(version: &str, api_key_source: &str, cwd: Option<&Path>) -> String {
    let cwd = cwd.map_or_else(String::new, |cwd| {
        format!(
            r#""cwd":{},"#,
            serde_json::Value::from(cwd.display().to_string())
        )
    });
    format!(
        r#"{{"type":"system","subtype":"init","claude_code_version":"{version}",{cwd}"tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"{api_key_source}","plugins":[{{"name":"cc-plugin-agents-md","source":"cc-plugin-agents-md@builtin"}},{{"name":"cc-plugin-plugin-authoring","source":"cc-plugin-plugin-authoring@builtin"}}]}}"#
    )
}

/// The credential-free launch the native fixture drives: `credentials: none` and a declared
/// endpoint, so no operator file is read and the child holds the plan's placeholder.
fn credential_free() -> Metaharness {
    Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("synthetic-never-called")
}

fn start(builder: Metaharness, lines: Vec<String>, log: &ScriptedLog) -> Result<Run, Refusal> {
    let mut runner = ScriptedRunner::new(
        lines.into_iter().map(ScriptStep::line).collect(),
        log.clone(),
    );
    builder.start_with_clock(
        Input::Prompt("synthetic".into()),
        &mut runner,
        &mut ClaudeSeams,
        Box::new(ManualClock::new()),
    )
}

fn floor_row(run: &Run, which: HermeticRow) -> (Verdict, String) {
    let rows = run.hermetic_floor();
    let row = rows
        .iter()
        .find(|row| row.row == which)
        .expect("every row is evaluated");
    (row.verdict, row.detail.clone())
}

// -------------------------------------------------------------------- --strict-version on Claude

/// `metaharness_claude::PINNED_VERSIONS`' own contract: *"A version outside this pin is a
/// `warning`, or a refusal before the run under `--strict-version`"*, and the story's pin-move
/// protocol (AGENTS.md § Live-evaluating: *"`--strict-version` refuses before the spawn when the
/// resolved binary is off the pin"*).
///
/// The b10x arm keeps that promise (`stream_closed.rs`,
/// `scripted_b10x_does_not_fabricate_strict_version_evidence`). The Claude arm never reads the flag:
/// the launch spawns, and the off-pin version is first seen in the opening record — after the
/// spawn, after the vendor has been handed the prompt. Driven against the real binary on
/// 2026-10-21: `metaharness run claude --strict-version` with 2.1.305 on the child's `PATH` spawned
/// it, sent a model request and read H9 `gap` only in the audit afterwards.
#[test]
fn strict_version_refuses_a_claude_launch_before_the_spawn() {
    let log = ScriptedLog::new();
    let result = start(
        credential_free().with_strict_version(true),
        vec![
            init_line("2.1.305", "ANTHROPIC_API_KEY", None),
            RESULT.to_owned(),
        ],
        &log,
    );
    let spawned = log.spawns();
    let Err(refusal) = result else {
        panic!(
            "--strict-version let a Claude launch through without reading the vendor's version \
             first; the runner spawned {spawned} child(ren) and the off-pin version could only be \
             seen in the opening record afterwards"
        );
    };
    assert!(
        refusal.to_string().contains("--strict-version"),
        "{refusal}"
    );
    assert_eq!(spawned, 0, "a refusal before the run spawns nothing");
}

// ------------------------------------------------- H4 under `credentials: none` with an endpoint

/// The launch attests H4 for this run as *"`ANTHROPIC_API_KEY` carries this plan's own placeholder
/// for the declared model endpoint; no operator credential is in the child at all"*
/// (`api_key_posture`), and the floor's H4 row, reading the record that placeholder produces,
/// says the opposite: *"an exported key takes precedence over the operator login and may point at
/// an account with no credits"*. A strict credential-free run with an endpoint — the mode the
/// native fixture qualifies the 2.1.304 pin in — therefore fails H4 on a key metaharness itself put
/// there (research page § *The native fixture at 2.1.304*, row *floor with `--audit`: H4*).
#[test]
fn h4_does_not_call_the_plans_own_endpoint_placeholder_an_undeclared_key() {
    let log = ScriptedLog::new();
    let mut run = start(
        credential_free().with_audit(true),
        vec![
            init_line(
                metaharness_claude::PINNED_VERSIONS[0],
                "ANTHROPIC_API_KEY",
                None,
            ),
            RESULT.to_owned(),
        ],
        &log,
    )
    .expect("a credential-free run with an endpoint starts");
    run.drain().expect("drains");
    let (verdict, detail) = floor_row(&run, HermeticRow::H4);
    assert_eq!(verdict, Verdict::Ok, "{detail}");
}

// --------------------------------------------- H11: the walk against the pinned loader's shapes

/// Every file shape the 2.1.304 loader reads from a directory above the cwd once a project or
/// local source is on, besides the `CLAUDE.md` the walk already looks for. Observed 2026-10-21: the
/// vendor run directly with `--setting-sources project,local` sent a canary from each of these,
/// planted one to four levels above the cwd, and with `--setting-sources project` it sent one from
/// `<HOME>/.claude/CLAUDE.md` — the operator's own user memory — whenever the cwd sat below HOME.
const SHAPES_THE_WALK_DOES_NOT_READ: [&str; 3] =
    [".claude/CLAUDE.md", "CLAUDE.local.md", ".claude/rules/r.md"];

/// A strict Claude run in an embedder's declared scratch copy at `<root>/copy`, after `plant`
/// wrote into `root`. `Ok(verdict)` is H11's floor row; `Err` is the launch's refusal.
fn h11_over_a_declared_copy(plant: impl FnOnce(&Path)) -> Result<(Verdict, String), String> {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    plant(root.path());
    let log = ScriptedLog::new();
    let started = start(
        credential_free()
            .with_hermetic(HermeticMode::Strict)
            .with_cwd(&copy)
            .with_scratch_cwd(),
        vec![
            init_line(metaharness_claude::PINNED_VERSIONS[0], "none", Some(&copy)),
            RESULT.to_owned(),
        ],
        &log,
    );
    match started {
        Ok(mut run) => {
            run.drain().expect("drains");
            Ok(floor_row(&run, HermeticRow::H11))
        }
        Err(refusal) => Err(refusal.to_string()),
    }
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Amendment a33 § 4 keeps the ancestor walk *because* the 2.1.304 gate is *"one pin's
/// behaviour, not a control metaharness holds, and a binary that loaded memory regardless of the
/// source would put the file in context with nothing in the record to say so"*. The walk is that
/// control, and it looks for `CLAUDE.md` and `AGENTS.md` only (`builder.rs`, `memory_ancestors`).
/// A memory file of any other shape the pinned loader reads sits above the cwd with H11 attested
/// `imposed` — *"no memory file outside the copied tree is discoverable"* — and kept out of the
/// context only by the gate a33 says is not a control. On a machine whose TMPDIR is under HOME,
/// `<HOME>/.claude/CLAUDE.md` is such a file above every scratch cwd.
#[test]
#[ignore = "pre-existing; decided as option C in story:claude-h11-memory-walk-shapes, to be rewritten there"]
fn h11_is_not_ok_over_a_memory_file_the_pinned_loader_reads_and_the_walk_does_not() {
    // The control: nothing planted, so the test's own ancestors are clean and the row can read ok.
    let clean = h11_over_a_declared_copy(|_| {});
    assert!(
        matches!(clean, Ok((Verdict::Ok, _))),
        "the control must read H11 ok, or the temporary directory's ancestors already hold a \
         memory file and nothing below is measured: {clean:?}"
    );
    // The mechanism works for the shape it knows.
    let known = h11_over_a_declared_copy(|root| write(&root.join("CLAUDE.md"), "canary"));
    assert!(
        !matches!(known, Ok((Verdict::Ok, _))),
        "a CLAUDE.md above the copy is caught: {known:?}"
    );

    let mut missed = Vec::new();
    for shape in SHAPES_THE_WALK_DOES_NOT_READ {
        let outcome = h11_over_a_declared_copy(|root| write(&root.join(shape), "canary"));
        if let Ok((Verdict::Ok, detail)) = outcome {
            missed.push(format!("{shape}: H11 ok — {detail}"));
        }
    }
    assert!(
        missed.is_empty(),
        "memory files the 2.1.304 loader reads sat above the cwd and H11 read ok:\n{}",
        missed.join("\n")
    );
}
