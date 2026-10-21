//! `story:hook-program-quotes-channel-paths`, design amendment a37 § 6: a Claude scratch root
//! that names `${CLAUDE_PLUGIN_ROOT}` is refused by name before the spawn, and Codex and b10x
//! runs under the same root are not.
//!
//! Its own test binary, because the scratch root's parent is the process's temporary directory and
//! `tempfile::env::override_temp_dir` sets it once for the whole process. The runner is scripted:
//! no vendor process, no credential, no network.

use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
use metaharness::{
    Input, Metaharness, Refusal, SCRATCH_PATH_NAMES_PLUGIN_VARIABLE, ScriptedLog, ScriptedRunner,
    ScriptedSeams,
};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

/// The code the refusal's message begins with, spelled out so a renamed constant cannot move it.
const CODE: &str = "scratch-path-names-plugin-variable";

fn builder(kind: Kind) -> Metaharness {
    let builder = Metaharness::new(kind).with_credentials(CredentialSource::None);
    match kind {
        Kind::B10x | Kind::Codex => builder
            .with_decisions(DecisionMode::Observe)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called"),
        Kind::Claude => builder,
    }
}

#[test]
fn a_claude_root_naming_a_plugin_variable_is_refused_by_name_and_other_kinds_start() {
    let base = tempfile::tempdir().expect("a directory");
    let parent = base.path().join("tmp-${CLAUDE_PLUGIN_ROOT}");
    std::fs::create_dir(&parent).expect("the awkward directory");
    tempfile::env::override_temp_dir(&parent).expect("the first override in this process");

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
    let started = builder(Kind::Claude).start_with(
        Input::Prompt("write the failing test".to_owned()),
        &mut runner,
        &mut ScriptedSeams,
    );
    let Err(refusal) = started else {
        panic!("a Claude run whose hook command names ${{CLAUDE_PLUGIN_ROOT}} started");
    };
    let words = refusal.to_string();
    assert!(
        matches!(
            refusal,
            Refusal::ScratchPathNamesPluginVariable {
                variable: "${CLAUDE_PLUGIN_ROOT}",
                ..
            }
        ),
        "its own variant, not a launch refusal in words: {refusal:?}"
    );
    assert_eq!(CODE, SCRATCH_PATH_NAMES_PLUGIN_VARIABLE);
    assert!(words.starts_with(CODE), "{words}");
    assert!(words.contains("${CLAUDE_PLUGIN_ROOT}"), "{words}");
    assert_eq!(log.spawns(), 0, "refused before the spawn");
    let leftovers: Vec<_> = std::fs::read_dir(&parent)
        .expect("the parent is readable")
        .collect();
    assert!(leftovers.is_empty(), "the refused run left {leftovers:?}");

    // Neither of the other kinds reads the substring, so neither is refused for it.
    for kind in [Kind::Codex, Kind::B10x] {
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
        let started = builder(kind).start_with(
            Input::Prompt("write the failing test".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        );
        if let Err(refusal) = started {
            panic!("{kind:?} was refused under the same root: {refusal}");
        }
        assert_eq!(log.spawns(), 1, "{kind:?}: started once");
    }
}
