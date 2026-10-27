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

use std::path::{Path, PathBuf};

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
/// local source is on — the binary's own list (research page § *H11: memory files*). Observed
/// 2026-10-21: the vendor run directly with `--setting-sources project,local` sent a canary from
/// each, and through the launch (`--setting-sources ""`) none. `.claude/rules` is read
/// recursively, so one file sits a directory deeper.
const SHAPES_THE_PINNED_LOADER_READS: [&str; 7] = [
    "CLAUDE.md",
    "CLAUDE.local.md",
    "AGENTS.md",
    ".claude/CLAUDE.md",
    ".claude/AGENTS.md",
    ".claude/rules/r.md",
    ".claude/rules/nested/r.md",
];

/// A strict Claude run in an embedder's declared scratch copy at `<root>/copy`, after `plant`
/// wrote into `root`, its opening record on `version`. Returns `root` as the walk sees it (links
/// resolved) and either H11's floor row or the launch's refusal.
fn h11_over_a_declared_copy(
    version: &str,
    plant: impl FnOnce(&Path),
) -> (PathBuf, Result<(Verdict, String), String>) {
    let root = tempfile::tempdir().unwrap();
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).unwrap();
    plant(root.path());
    let seen = std::fs::canonicalize(root.path()).unwrap();
    let log = ScriptedLog::new();
    let started = start(
        credential_free()
            .with_hermetic(HermeticMode::Strict)
            .with_cwd(&copy)
            .with_scratch_cwd(),
        vec![init_line(version, "none", Some(&copy)), RESULT.to_owned()],
        &log,
    );
    let outcome = match started {
        Ok(mut run) => {
            run.drain().expect("drains");
            Ok(floor_row(&run, HermeticRow::H11))
        }
        Err(refusal) => Err(refusal.to_string()),
    };
    (seen, outcome)
}

fn write(path: &Path, text: &str) {
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, text).unwrap();
}

/// Rewritten to decision C (`story:claude-h11-memory-walk-shapes`, design amendment a36). The pass-1
/// finding stands — the walk looked for `CLAUDE.md` and `AGENTS.md` while the pinned loader reads
/// seven shapes — and the decision answers it the other way round: H11 does not rest on the walk
/// any more. It rests on `--setting-sources ""` and the fresh scratch config home, held to the
/// memory canary recorded for the record's version, and the walk reports what it finds. So every
/// shape above the copy is **named** in an `ok` H11 row and none is refused.
#[test]
fn h11_names_every_memory_shape_the_pinned_loader_reads_and_refuses_none() {
    let pinned = metaharness_claude::PINNED_VERSIONS[0];
    // The control: nothing planted, and the row reads ok.
    let (_, clean) = h11_over_a_declared_copy(pinned, |_| {});
    assert!(matches!(clean, Ok((Verdict::Ok, _))), "{clean:?}");

    let mut wrong = Vec::new();
    for shape in SHAPES_THE_PINNED_LOADER_READS {
        let (root, outcome) =
            h11_over_a_declared_copy(pinned, |root| write(&root.join(shape), "canary"));
        let planted = root.join(shape).display().to_string();
        match outcome {
            Ok((Verdict::Ok, detail)) if detail.contains(&planted) => {}
            other => wrong.push(format!("{shape}: {other:?}")),
        }
    }
    assert!(
        wrong.is_empty(),
        "a memory file above the copy must be named in an ok H11 and never refused:\n{}",
        wrong.join("\n")
    );
}

/// The guard half of decision C: the claim is only as good as the canary of the binary that ran.
/// A record on a version with no memory canary never reads H11 `ok`, whatever the attestation
/// says; since amendment a62 it reads `unk`, because nobody measured that binary.
#[test]
fn h11_is_unk_on_a_record_version_with_no_memory_canary() {
    let (_, outcome) = h11_over_a_declared_copy("2.1.303", |_| {});
    let Ok((verdict, detail)) = outcome else {
        panic!("the launch was refused: {outcome:?}");
    };
    assert_eq!(
        verdict,
        Verdict::Unk,
        "story:off-pin-vendor-version-is-advisory: an unmeasured version is unk: {detail}"
    );
    assert!(detail.contains("2.1.303"), "{detail}");
}
