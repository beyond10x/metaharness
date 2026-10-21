//! The expectation document's path as the auditor is handed it (amendment a43).
//!
//! The check before the spawn opens the document by its path. The auditor is handed the path's
//! display form, which replaces every byte that is not UTF-8. A document whose path is not UTF-8
//! passes the check and is never the file the auditor is pointed at, so it is checked by nobody.
//! The runner is scripted: no vendor process, no credential, no network.
#![cfg(unix)]

use std::ffi::OsStr;
use std::os::unix::ffi::OsStrExt as _;

use metaharness::protocol::{CredentialSource, Kind};
use metaharness::{
    AuditorRun, FakeAuditor, Input, ManualClock, Metaharness, ScriptStep, ScriptedLog,
    ScriptedRunner, ScriptedSeams,
};

const CLOSING: &str = r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#;

#[test]
fn an_expectation_document_whose_path_is_not_utf8_is_refused_before_the_harness_starts() {
    let scratch = tempfile::tempdir().expect("a directory");
    let document = scratch
        .path()
        .join(OsStr::from_bytes(b"expectations-\xff.yaml"));
    std::fs::write(&document, "kinds: []\n").expect("a file whose name is not UTF-8");

    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::new(vec![ScriptStep::line(CLOSING)], log.clone());
    let started = Metaharness::new(Kind::Claude)
        .with_credentials(CredentialSource::None)
        .with_audit(true)
        .with_spec_file(document.clone())
        .with_auditor("protocol observe trace check")
        .start_with_clock(
            Input::Prompt("check the pairing".to_string()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        );

    match started {
        Err(refusal) => {
            assert_eq!(log.spawns(), 0, "refused before the spawn: {refusal}");
            let words = refusal.to_string();
            assert!(
                words.starts_with("the expectation document"),
                "the refusal is about the document: {words}"
            );
        }
        Ok(mut run) => {
            run.drain().expect("drains");
            let mut auditor = FakeAuditor::answering(AuditorRun {
                exit_code: Some(0),
                stdout: "one ok\n".to_string(),
                stderr: String::new(),
            });
            let report = run.audit(&mut auditor).expect("audited");
            let argv = report
                .auditor
                .as_ref()
                .expect("the auditor ran")
                .argv
                .clone();
            let handed = &argv[5];
            panic!(
                "the harness started ({} spawn) for --spec {:?}; the auditor was handed --spec \
                 {handed:?}, a path that exists: {}; the document exists: {}; exit {:?}",
                log.spawns(),
                document.as_os_str().as_bytes().escape_ascii().to_string(),
                std::path::Path::new(handed).exists(),
                document.exists(),
                run.exit(Some(&report)),
            );
        }
    }
}
