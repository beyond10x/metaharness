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

/// The recorded `started` line's publication, and the one `--surface verbs` would have written in
/// its place (harness `3eca074:crates/harness-cli/src/lib.rs`, `Surface::Verbs`: `tool_search`,
/// `tool_describe`, `tool_invoke` over the catalogue).
const FLAT_PUBLICATION: &str = r#""published_tools":["file_read","dir_list","search","find"]"#;
const VERBS_PUBLICATION: &str =
    r#""published_tools":["tool_search","tool_describe","tool_invoke"]"#;

/// The recorded loop, with its one `tool-requested` line replaced by `replacement` when given.
fn recorded_loop(replacement: Option<&str>) -> Vec<String> {
    recorded_loop_published(FLAT_PUBLICATION, replacement)
}

/// The recorded loop with its `started` line publishing `publication` instead of the recorded
/// flat surface, and its one `tool-requested` line replaced by `replacement` when given.
fn recorded_loop_published(publication: &str, replacement: Option<&str>) -> Vec<String> {
    assert!(
        B10X_RECORDED_LOOP.contains(FLAT_PUBLICATION),
        "the recorded loop publishes the flat surface"
    );
    B10X_RECORDED_LOOP
        .replacen(FLAT_PUBLICATION, publication, 1)
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
    what_metaharness_recorded_of(recorded_loop(replacement))
}

/// The same, over the given loop record.
fn what_metaharness_recorded_of(record: Vec<String>) -> (Vec<String>, Vec<String>) {
    let mut runner = ScriptedRunner::of_lines(record, ScriptedLog::new());
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
/// The loop metaharness launches publishes its entries flat, so its calls are `file_read`, never
/// `tool_invoke`, and the run reads them on the surface the loop published (design amendment a75).
#[test]
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

/// A `tool_invoke` on a flat b10x run is a tool the loop never published: the loop refused it, so
/// the record names no effect, even when the entry inside it is a harness name the loop has.
///
/// The loop records no `operation` and no `subjects` for it, because its flat surface looks the
/// call up by its own name and finds none (harness `3eca074:crates/harness-catalogue/src/flat.rs`,
/// `Flat::subjects_of`: "An unknown name touches nothing").
#[test]
fn a_harness_name_behind_tool_invoke_on_a_flat_b10x_record_names_no_effect() {
    let refused = r#"{"kind":"tool-requested","call_id":"call_b10x_001","name":"tool_invoke","arguments":{"name":"file_read","arguments":{"path":"README.md"}}}"#;
    assert_eq!(
        what_metaharness_recorded(Some(refused)),
        (Vec::new(), Vec::new()),
        "the loop published `file_read` flat and refused `tool_invoke`"
    );
}

/// A b10x run whose loop published the three verbs reads a `tool_invoke` as the entry it names.
#[test]
fn a_b10x_run_that_published_the_verbs_reads_tool_invoke_as_its_entry() {
    let invoked = r#"{"kind":"tool-requested","call_id":"call_b10x_001","name":"tool_invoke","arguments":{"name":"file_read","arguments":{"path":"README.md"}},"operation":"file.read","subjects":["file:README.md"]}"#;
    assert_eq!(
        what_metaharness_recorded_of(recorded_loop_published(VERBS_PUBLICATION, Some(invoked))),
        (
            vec!["file.read".to_owned()],
            vec!["file:README.md".to_owned()]
        )
    );
}

/// On a run that published the verbs, a flat entry name is a tool the loop did not publish, and it
/// is not read through the rendering table as if it had been.
#[test]
fn a_flat_entry_name_on_a_b10x_run_that_published_the_verbs_names_no_effect() {
    let unpublished = r#"{"kind":"tool-requested","call_id":"call_b10x_001","name":"file_read","arguments":{"path":"README.md"}}"#;
    assert_eq!(
        what_metaharness_recorded_of(recorded_loop_published(
            VERBS_PUBLICATION,
            Some(unpublished)
        )),
        (Vec::new(), Vec::new())
    );
}

/// A flat call's subjects come from the loop's own record where it names them, and not only from
/// the arguments. The record here is constructed so the two readings differ, which only shows
/// whose answer the run kept; the loop computes its subjects by the same catalogue rule.
#[test]
fn a_flat_b10x_call_keeps_the_subjects_the_loop_recorded() {
    let recorded = r#"{"kind":"tool-requested","call_id":"call_b10x_001","name":"file_read","arguments":{"path":"README.md"},"operation":"file.read","subjects":["file:docs/README.md"]}"#;
    assert_eq!(
        what_metaharness_recorded(Some(recorded)).1,
        vec!["file:docs/README.md".to_owned()],
        "the record's subjects win over a reading of the arguments"
    );
}
