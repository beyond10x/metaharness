//! Adversary pass 2, wave 3 of 2026-10-20, unit `story:metaharness-tools-adopts-tool-contract`.
//!
//! Correction round 1 gave `resolve_verb` and `subjects_of_verb` a `VerbSurface`, and `Run`
//! picks `HarnessLoop` for every b10x run (`run.rs:893`). These cases drive that choice through
//! the **production** b10x seam over the loop record `b10x-harness 0.15.1` wrote
//! (`crates/metaharness-b10x/fixtures/golden/loop.jsonl`), not the generic `{"emit": …}` fixture
//! the unit's own case uses. No vendor process, no credential, no network.
//!
//! That record shows what the loop publishes when metaharness launches it: `published_tools` is
//! `["file_read", "dir_list", "search", "find"]` — the flat surface, which is `b10x-harness run`'s
//! default (`f0870131:crates/harness-cli/src/lib.rs:744`, `default_value = "flat"`), and the b10x
//! launch passes no `--surface` (`crates/metaharness-b10x/src/launch.rs`). Its one call is
//! `file_read {"path": "README.md"}`, recorded by the loop with `"subjects": ["file:README.md"]`.

use metaharness::protocol::{CredentialSource, DecisionMode, Event, Kind};
use metaharness::{Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner};

const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// The recorded loop, with its one `tool-requested` line replaced by `replacement` when given.
fn recorded_loop(replacement: Option<&str>) -> Vec<String> {
    B10X_RECORDED_LOOP
        .lines()
        .map(|line| match replacement {
            Some(replacement) if line.contains(r#""kind":"tool-requested""#) => {
                replacement.to_owned()
            }
            _ => line.to_owned(),
        })
        .collect()
}

/// The `operations` and `subjects` metaharness put on the run's one `tool.requested`.
fn what_metaharness_recorded(replacement: Option<&str>) -> (Vec<String>, Vec<String>) {
    let mut runner = ScriptedRunner::of_lines(recorded_loop(replacement), ScriptedLog::new());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut run = Metaharness::new(Kind::B10x)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("b10x-emulated")
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run plans on b10x");
    run.drain().expect("pumps");
    run.events()
        .iter()
        .find_map(|event| match event {
            Event::ToolRequested {
                operations,
                subjects,
                ..
            } => Some((operations.clone(), subjects.clone())),
            _ => None,
        })
        .expect("the loop's call is recorded")
}

/// F2, through the production seam: a contract name behind `tool_invoke` on a b10x record names
/// no effect, because the loop refused it.
#[test]
fn a_contract_name_on_a_b10x_record_read_through_the_b10x_seam_names_no_effect() {
    let invented = r#"{"kind":"tool-requested","call_id":"call_b10x_001","name":"tool_invoke","arguments":{"name":"fs.read","arguments":{"path":"README.md"}}}"#;
    assert_eq!(
        what_metaharness_recorded(Some(invented)),
        (Vec::new(), Vec::new())
    );
}

/// What the recorded call touched, as the b10x loop itself recorded it, reaches metaharness's
/// record.
///
/// `Run::subjects` sends every b10x call to `subjects_of_verb` because the run "publishes the
/// verbs" (`run.rs:880-885`, `publishes_verbs` at `:907-909`), and `subjects_of_verb` answers empty
/// for any tool that is not `tool_invoke` (`resolve.rs:161-163`). The loop metaharness launches
/// publishes its entries flat, so its calls are `file_read`, never `tool_invoke`.
#[test]
#[ignore = "pre-existing; the acceptance case of story:b10x-record-keeps-the-loops-subjects"]
fn the_file_a_flat_b10x_call_read_is_on_metaharness_record() {
    let (operations, subjects) = what_metaharness_recorded(None);
    assert_eq!(
        operations,
        vec!["file.read".to_owned()],
        "operation resolved"
    );
    assert_eq!(
        subjects,
        vec!["file:README.md".to_owned()],
        "the loop recorded `subjects: [\"file:README.md\"]` for this call; metaharness's \
         `tool.requested` for it names none"
    );
}
