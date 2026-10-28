//! Adversary cases, pass 1, for `story:run-spec-document` (wave D170, unit 1; design amendment a64).
//!
//! Read against the amendment's claim that the document is the third spelling of the flags: the
//! same settings as an argv and as a document are the same spec, and the document admits nothing
//! the command line refuses. The cases outside `against_the_command_line` build without `clap`.

use metaharness_protocol::{Kind, RunSpec, RunSpecDocumentError};
use serde_json::{Value, json};

/// Every key whose flag is a `PathBuf` (one value) or a `Vec<PathBuf>` (repeatable).
const PATH_KEYS: [&str; 10] = [
    "frame",
    "hooks",
    "driver",
    "catalogue",
    "cwd",
    "retain-dir",
    "spec",
    "substrate",
    "cgroup-root",
    "prices",
];
const PATH_LIST_KEYS: [&str; 2] = ["plugin-dir", "context"];

fn empty_path_documents() -> Vec<(&'static str, Value)> {
    let mut documents = Vec::new();
    for key in PATH_KEYS {
        documents.push((key, json!({"kind": "b10x", key: ""})));
    }
    for key in PATH_LIST_KEYS {
        documents.push((key, json!({"kind": "b10x", key: [""]})));
    }
    documents
}

/// RED. `--cwd ""` (and every other path flag) is refused by clap's `PathBufValueParser` as an
/// empty value; the document reads `""` into `Some(PathBuf::from(""))`. The amendment says a
/// document and its argv read to the same spec and that the document admits no word the command
/// line refuses, so an empty path should be a `BadValue` naming its key.
#[test]
fn an_empty_path_is_refused_by_the_document_as_the_command_line_refuses_it() {
    let mut admitted = Vec::new();
    for (key, document) in empty_path_documents() {
        match RunSpec::from_document(&document) {
            Err(RunSpecDocumentError::BadValue { key: named, .. }) if named == key => {}
            other => admitted.push(format!("{key}: {other:?}")),
        }
    }
    assert!(
        admitted.is_empty(),
        "the document admits an empty path the command line refuses:\n{}",
        admitted.join("\n")
    );
}

/// Green probe: a key written twice is caught on the raw text even when the second spelling uses
/// a JSON escape, and when the two values are equal.
#[test]
fn a_duplicate_key_is_caught_through_an_escape_and_with_equal_values() {
    for text in [
        r#"{"kind":"claude","kind":"codex"}"#,
        r#"{"kind":"claude","model":"a","model":"a"}"#,
    ] {
        assert!(
            matches!(
                RunSpec::from_document_str(text),
                Err(RunSpecDocumentError::DuplicateKey { .. })
            ),
            "{text}"
        );
    }
}

/// Green probe: keys that differ from an admitted key by case or by the flag's dashes are unknown.
#[test]
fn a_key_with_dashes_or_another_case_is_unknown() {
    for key in ["--max-turns", "Max-Turns", "max_turns", "KIND"] {
        let document = json!({"kind": "claude", key: 1});
        assert_eq!(
            RunSpec::from_document(&document),
            Err(RunSpecDocumentError::UnknownKey {
                key: key.to_owned()
            }),
            "{key}"
        );
    }
}

/// Green probe: `max-turns` outside `u32`, fractional, negative, a float spelling of a whole
/// number, and `null` are each a `BadValue` naming the key.
#[test]
fn a_turn_count_outside_u32_is_a_bad_value_naming_the_key() {
    for text in [
        r#"{"kind":"claude","max-turns":4294967296}"#,
        r#"{"kind":"claude","max-turns":18446744073709551616}"#,
        r#"{"kind":"claude","max-turns":-1}"#,
        r#"{"kind":"claude","max-turns":1.5}"#,
        r#"{"kind":"claude","max-turns":null}"#,
        r#"{"kind":"claude","max-turns":[60]}"#,
    ] {
        match RunSpec::from_document_str(text) {
            Err(RunSpecDocumentError::BadValue { key, .. }) => assert_eq!(key, "max-turns"),
            other => panic!("{text}: {other:?}"),
        }
    }
    assert_eq!(
        RunSpec::from_document_str(r#"{"kind":"claude","max-turns":4294967295}"#)
            .map(|spec| spec.max_turns),
        Ok(Some(u32::MAX))
    );
}

/// Green probe: a `kind` that is not a harness kind is a `BadValue` on `kind`, never
/// `MissingKind`, and every keyed variant's `Display` names its key.
#[test]
fn a_bad_kind_is_a_bad_value_and_every_keyed_refusal_names_its_key() {
    for kind in [json!("Claude"), json!("gpt"), json!(null), json!(1)] {
        match RunSpec::from_document(&json!({"kind": kind})) {
            Err(error @ RunSpecDocumentError::BadValue { .. }) => {
                assert!(error.to_string().contains("`kind`"), "{error}");
            }
            other => panic!("{kind}: {other:?}"),
        }
    }
    for error in [
        RunSpec::from_document_str(r#"{"kind":"claude","kind":"codex"}"#).unwrap_err(),
        RunSpec::from_document_str(r#"{"kind":"claude","retain-dirr":"x"}"#).unwrap_err(),
        RunSpec::from_document_str(r#"{"kind":"claude","audit":"yes"}"#).unwrap_err(),
    ] {
        let key = match &error {
            RunSpecDocumentError::DuplicateKey { key }
            | RunSpecDocumentError::UnknownKey { key }
            | RunSpecDocumentError::BadValue { key, .. } => key.clone(),
            other => panic!("{other:?}"),
        };
        assert!(error.to_string().contains(&format!("`{key}`")), "{error}");
    }
}

/// Green probe: the error travels across threads and into `Box<dyn Error>`.
#[test]
fn the_refusal_is_send_sync_and_static() {
    fn bounds<T: std::error::Error + Send + Sync + 'static>() {}
    bounds::<RunSpecDocumentError>();
    let boxed: Box<dyn std::error::Error + Send + Sync> =
        Box::new(RunSpecDocumentError::MissingKind);
    assert!(boxed.to_string().contains("kind"));
}

/// Green probe: an empty document object is `MissingKind`; `{}` inside `[]` is `NotAnObject`.
#[test]
fn an_empty_object_is_missing_its_kind() {
    assert_eq!(
        RunSpec::from_document_str("{}"),
        Err(RunSpecDocumentError::MissingKind)
    );
    assert_eq!(
        RunSpec::from_document_str("[{}]"),
        Err(RunSpecDocumentError::NotAnObject { found: "an array" })
    );
    assert_eq!(
        RunSpec::from_document_str(r#"{"kind":"codex"}"#),
        Ok(RunSpec::new(Kind::Codex))
    );
}

/// The command-line half of the empty-path case: clap refuses each empty path, so the document
/// reading one is a drift and not a shared rule.
#[cfg(feature = "clap")]
mod against_the_command_line {
    use clap::{Args as _, Command};

    use super::{PATH_KEYS, PATH_LIST_KEYS};

    #[test]
    fn the_command_line_refuses_every_empty_path() {
        for key in PATH_KEYS.iter().chain(PATH_LIST_KEYS.iter()) {
            let flag = format!("--{key}");
            let matched = metaharness_protocol::RunSpec::augment_args(Command::new("run"))
                .try_get_matches_from(["run", "b10x", flag.as_str(), ""]);
            let error = matched.expect_err(key);
            assert_eq!(
                error.kind(),
                clap::error::ErrorKind::InvalidValue,
                "{key}: {error}"
            );
        }
    }
}
