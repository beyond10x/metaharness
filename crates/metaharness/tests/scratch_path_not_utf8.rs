//! `story:hook-program-quotes-channel-paths` (design amendment a37): a scratch root that is not
//! UTF-8 is refused by name before anything is spawned, on every kind.
//!
//! Its own test binary, because the scratch root's parent is the process's temporary directory and
//! `tempfile::env::override_temp_dir` sets it once for the whole process. The builder has no other
//! seam for it. The runner is scripted: no vendor process, no credential, no network.

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;

use metaharness::protocol::{CredentialSource, DecisionMode, Kind};
use metaharness::{
    Input, Metaharness, SCRATCH_PATH_NOT_UTF8, ScriptedLog, ScriptedRunner, ScriptedSeams,
};

const OPENING: &str = r#"{"emit":"session.started","harness_version":"0.0.0"}"#;
const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

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

#[test]
fn a_scratch_root_that_is_not_utf8_is_refused_by_name_before_the_spawn() {
    let base = tempfile::tempdir().expect("a directory");
    let parent = base.path().join(OsStr::from_bytes(b"tmp-\xff"));
    std::fs::create_dir(&parent).expect("a non-UTF-8 directory");
    tempfile::env::override_temp_dir(&parent).expect("the first override in this process");

    for kind in [Kind::Claude, Kind::Codex, Kind::B10x] {
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::of_lines([OPENING, CLOSING], log.clone());
        let started = builder(kind).start_with(
            Input::Prompt("write the failing test".to_owned()),
            &mut runner,
            &mut ScriptedSeams,
        );
        let Err(refusal) = started else {
            panic!("{kind:?}: a run under a non-UTF-8 scratch root started");
        };
        let words = refusal.to_string();
        assert!(
            words.starts_with(SCRATCH_PATH_NOT_UTF8),
            "{kind:?}: {words}"
        );
        assert!(words.contains(r"/tmp-\xff/"), "{kind:?}: {words}");
        assert!(
            !words.contains(char::REPLACEMENT_CHARACTER),
            "{kind:?}: {words}"
        );
        assert_eq!(log.spawns(), 0, "{kind:?}: refused before the spawn");
        let leftovers: Vec<_> = std::fs::read_dir(&parent)
            .expect("the parent is readable")
            .collect();
        assert!(
            leftovers.is_empty(),
            "{kind:?}: the refused run left {leftovers:?} behind"
        );
    }
}
