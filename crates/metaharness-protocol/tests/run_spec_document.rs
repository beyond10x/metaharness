//! A session request is a run-spec document (story:run-spec-document, design amendment a64).
//!
//! A caller holding a mapping of `metaharness run` settings, such as a run file's `harness`
//! section, reads it into a [`RunSpec`] through the protocol crate and without the `clap` feature.
//! Keys are the run verb's argument names, `kind` included; values take each flag's own vocabulary.
//! A key the document does not know and a value that does not read as its key's type are refused
//! by name and never dropped.
//!
//! The cases outside `against_the_command_line` build without `clap`, which is how a library
//! embedder links this crate. That module is the anti-drift half and needs the real argument
//! definitions, so it builds with the feature (`cargo test --workspace` unifies it on).

// `json!` over a document naming all 42 keys recurses past the default limit of 128.
#![recursion_limit = "256"]

use std::collections::BTreeSet;
use std::path::PathBuf;

use metaharness_protocol::{
    CredentialSource, DecisionMode, HermeticMode, Kind, MarketplacePlugin, PromptCacheTtl, RunSpec,
    RunSpecDocumentError, ScopeAnnounce, ToolSurface,
};
use serde_json::{Value, json};

fn refused(text: &str) -> RunSpecDocumentError {
    RunSpec::from_document_str(text).expect_err("refused")
}

fn refused_value(document: &Value) -> RunSpecDocumentError {
    RunSpec::from_document(document).expect_err("refused")
}

/// A document naming every admitted key, each away from `RunSpec::new`'s value, written by hand
/// so that a key read into the wrong field fails rather than round-tripping through the same
/// mistake twice.
fn populated_document() -> Value {
    json!({
        "kind": "claude",
        "hermetic": "strict",
        "prompt": "tidy the imports",
        "frame": "frames/step.json",
        "decisions": "ask",
        "actor": "agent:implementor",
        "hooks": "hooks.toml",
        "driver": "bin/aep",
        "tool-surface": "owned",
        "catalogue": "catalogue.yaml",
        "allow-program": ["cargo", "git"],
        "credentials": "api-key",
        "model": "sonnet",
        "model-endpoint": "http://127.0.0.1:1",
        "model-wire": "anthropic-messages",
        "subscription-token-file": "token.json",
        "subscription-token-env": "TOKEN",
        "subscription-token-pointer": "/access",
        "api-key-secret": "llm/key",
        "effort": "high",
        "prompt-cache-ttl": "5m",
        "max-turns": 60,
        "max-budget-usd": "12.50",
        "plugin-dir": ["plugins/one", "plugins/two"],
        "plugin": ["beyond10x/agentplugins@aep@0.25.0"],
        "cwd": "work",
        "scratch-cwd": true,
        "scratch-name": "case-7",
        "retain-dir": "retained",
        "strict-version": true,
        "audit": true,
        "spec": "expectations.yaml",
        "substrate": "substrate.sock",
        "substrate-embedded": false,
        "cgroup-root": "cgroup",
        "toolchain": "rust",
        "prices": "prices.toml",
        "write-scope": ["src/**=allowed"],
        "process-write-subtree": ["target"],
        "scope-announce": "silent",
        "context": ["README.md"],
        "auditor": "protocol observe trace check",
        "auditor-args": ["--advisory", "billed"],
    })
}

/// What [`populated_document`] says, built field by field.
fn populated() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("tidy the imports".to_owned());
    spec.frame = Some(PathBuf::from("frames/step.json"));
    spec.decisions = DecisionMode::Ask;
    spec.actor = Some("agent:implementor".to_owned());
    spec.hooks = Some(PathBuf::from("hooks.toml"));
    spec.driver = Some(PathBuf::from("bin/aep"));
    spec.tool_surface = ToolSurface::Owned;
    spec.catalogue = Some(PathBuf::from("catalogue.yaml"));
    spec.allow_program = vec!["cargo".to_owned(), "git".to_owned()];
    spec.credentials = CredentialSource::ApiKey;
    spec.model = Some("sonnet".to_owned());
    spec.model_endpoint = Some("http://127.0.0.1:1".to_owned());
    spec.model_wire = Some("anthropic-messages".to_owned());
    spec.subscription_token_file = Some("token.json".to_owned());
    spec.subscription_token_env = Some("TOKEN".to_owned());
    spec.subscription_token_pointer = Some("/access".to_owned());
    spec.api_key_secret = Some("llm/key".to_owned());
    spec.effort = Some("high".to_owned());
    spec.prompt_cache_ttl = Some(PromptCacheTtl::FiveMinutes);
    spec.max_turns = Some(60);
    spec.max_budget_usd = Some("12.50".to_owned());
    spec.plugin_dir = vec![PathBuf::from("plugins/one"), PathBuf::from("plugins/two")];
    spec.plugin = vec![
        "beyond10x/agentplugins@aep@0.25.0"
            .parse::<MarketplacePlugin>()
            .expect("pinned"),
    ];
    spec.cwd = Some(PathBuf::from("work"));
    spec.scratch_cwd = true;
    spec.scratch_name = Some("case-7".to_owned());
    spec.retain_dir = Some(PathBuf::from("retained"));
    spec.strict_version = true;
    spec.audit = true;
    spec.spec = Some(PathBuf::from("expectations.yaml"));
    spec.substrate = Some(PathBuf::from("substrate.sock"));
    spec.cgroup_root = Some(PathBuf::from("cgroup"));
    spec.toolchain = Some("rust".to_owned());
    spec.prices = Some(PathBuf::from("prices.toml"));
    spec.write_scope = vec!["src/**=allowed".to_owned()];
    spec.process_write_subtree = vec!["target".to_owned()];
    spec.scope_announce = ScopeAnnounce::Silent;
    spec.context = vec![PathBuf::from("README.md")];
    spec.auditor = Some("protocol observe trace check".to_owned());
    spec.auditor_args = vec!["--advisory".to_owned(), "billed".to_owned()];
    spec
}

/// The populated pair with `substrate-embedded` set instead of `substrate`, which the command
/// line refuses together.
fn populated_embedded() -> (Value, RunSpec) {
    let mut document = populated_document();
    let object = document.as_object_mut().expect("an object");
    object.remove("substrate");
    object.insert("substrate-embedded".to_owned(), json!(true));
    let mut spec = populated();
    spec.substrate = None;
    spec.substrate_embedded = true;
    (document, spec)
}

/// Acceptance 1.
#[test]
fn a_document_names_its_kind_and_settings_and_every_other_field_is_the_kinds_default() {
    let spec = RunSpec::from_document_str(r#"{"kind":"codex","effort":"high"}"#).expect("reads");
    assert_eq!(spec.kind, Kind::Codex);
    assert_eq!(spec.effort.as_deref(), Some("high"));
    let mut expected = RunSpec::new(Kind::Codex);
    expected.effort = Some("high".to_owned());
    assert_eq!(spec, expected);

    assert_eq!(
        RunSpec::from_document_str(r#"{"kind":"b10x"}"#),
        Ok(RunSpec::new(Kind::B10x))
    );
}

/// The two entry points read the same document to the same spec.
#[test]
fn the_value_entry_point_reads_what_the_text_entry_point_reads() {
    let document = json!({"kind": "claude", "max-turns": 30, "strict-version": true});
    let from_value = RunSpec::from_document(&document).expect("reads");
    let from_text = RunSpec::from_document_str(&document.to_string()).expect("reads");
    assert_eq!(from_value, from_text);
    assert_eq!(from_value.max_turns, Some(30));
    assert!(from_value.strict_version);
}

/// Every admitted key is read, and read into its own field.
#[test]
fn a_document_naming_every_key_reads_into_every_field() {
    let document = populated_document();
    let named: BTreeSet<&str> = document
        .as_object()
        .expect("an object")
        .keys()
        .map(String::as_str)
        .collect();
    let admitted: BTreeSet<&str> = RunSpec::DOCUMENT_KEYS.iter().copied().collect();
    assert_eq!(named, admitted, "the populated document names every key");
    assert_eq!(
        admitted.len(),
        RunSpec::DOCUMENT_KEYS.len(),
        "no key is listed twice"
    );

    assert_eq!(RunSpec::from_document(&document), Ok(populated()));
    assert_eq!(
        RunSpec::from_document_str(&document.to_string()),
        Ok(populated())
    );
    let (document, spec) = populated_embedded();
    assert_eq!(RunSpec::from_document(&document), Ok(spec));
}

/// Acceptance 2.
#[test]
fn a_key_outside_the_admitted_list_is_refused_naming_the_key() {
    let refusal = refused(r#"{"kind":"claude","retain-dirr":"out"}"#);
    assert_eq!(
        refusal,
        RunSpecDocumentError::UnknownKey {
            key: "retain-dirr".to_owned()
        }
    );
    assert!(refusal.to_string().contains("`retain-dirr`"), "{refusal}");

    // The Rust field name and the flag as typed are not keys: the document has one spelling.
    for key in ["max_turns", "--max-turns", "Kind", "help", "p"] {
        let document = json!({"kind": "claude", key: 1});
        assert_eq!(
            refused_value(&document),
            RunSpecDocumentError::UnknownKey {
                key: key.to_owned()
            },
            "{key}"
        );
    }
}

/// An unknown key is reported ahead of a missing kind: a misspelt `kind` is the likelier fault.
#[test]
fn a_misspelt_kind_is_reported_as_the_key_it_is() {
    assert_eq!(
        refused(r#"{"knd":"codex"}"#),
        RunSpecDocumentError::UnknownKey {
            key: "knd".to_owned()
        }
    );
}

/// Acceptance 3, the value half.
#[test]
fn a_value_that_does_not_read_as_its_keys_type_is_refused_naming_the_key() {
    let refusal = refused(r#"{"kind":"claude","max-turns":"many"}"#);
    let RunSpecDocumentError::BadValue { key, detail } = &refusal else {
        panic!("expected BadValue, got {refusal:?}");
    };
    assert_eq!(key, "max-turns");
    assert_ne!(detail, "");
    assert!(refusal.to_string().contains("`max-turns`"), "{refusal}");

    for (key, value) in [
        ("kind", json!("gemini")),
        ("kind", json!(null)),
        ("kind", json!(1)),
        ("max-turns", json!(-1)),
        ("max-turns", json!(1.5)),
        ("max-turns", json!(4_294_967_296_u64)),
        ("max-turns", json!(null)),
        ("hermetic", json!("loud")),
        ("hermetic", json!(true)),
        ("decisions", json!("Frame")),
        ("credentials", json!("operator_login")),
        ("tool-surface", json!(["owned"])),
        ("scope-announce", json!("quiet")),
        ("strict-version", json!("yes")),
        ("audit", json!(1)),
        ("model", json!(null)),
        ("model", json!(7)),
        ("effort", json!({"level": "high"})),
        ("prompt-cache-ttl", json!("30m")),
        ("max-budget-usd", json!(5)),
        ("cwd", json!(false)),
        ("plugin-dir", json!("plugins/one")),
        ("allow-program", json!(["cargo", 1])),
        ("plugin", json!(["beyond10x/agentplugins@aep"])),
        ("plugin", json!(["@aep@0.25.0"])),
        ("auditor-args", json!("--advisory")),
    ] {
        let mut document = json!({"kind": "claude"});
        document[key] = value.clone();
        let refusal = refused_value(&document);
        match &refusal {
            RunSpecDocumentError::BadValue { key: named, .. } => {
                assert_eq!(named, key, "{key}: {value}");
            }
            other => panic!("{key}: {value} should be a bad value, got {other:?}"),
        }
        assert!(
            refusal.to_string().contains(&format!("`{key}`")),
            "{refusal}"
        );
    }
}

/// Acceptance 3, the shape half.
#[test]
fn a_document_without_a_kind_or_that_is_not_an_object_is_refused_by_name() {
    assert_eq!(refused("{}"), RunSpecDocumentError::MissingKind);
    assert_eq!(
        refused(r#"{"model":"sonnet"}"#),
        RunSpecDocumentError::MissingKind
    );
    for (text, found) in [
        ("[]", "an array"),
        (r#""claude""#, "a string"),
        ("7", "a number"),
        ("true", "a boolean"),
        ("null", "null"),
    ] {
        assert_eq!(
            refused(text),
            RunSpecDocumentError::NotAnObject { found },
            "{text}"
        );
    }
    assert_eq!(
        refused_value(&json!(["kind", "claude"])),
        RunSpecDocumentError::NotAnObject { found: "an array" }
    );
    for text in ["", "{", r#"{"kind":"claude",}"#, "{} {}"] {
        assert!(
            matches!(refused(text), RunSpecDocumentError::Malformed { .. }),
            "{text:?}"
        );
    }
}

/// A key written twice in the text would otherwise lose one of its values in silence.
#[test]
fn a_key_written_twice_is_refused_rather_than_one_value_dropped() {
    let refusal = refused(r#"{"kind":"claude","max-turns":1,"max-turns":2}"#);
    assert_eq!(
        refusal,
        RunSpecDocumentError::DuplicateKey {
            key: "max-turns".to_owned()
        }
    );
    assert!(refusal.to_string().contains("`max-turns`"), "{refusal}");
}

#[test]
fn every_refusal_is_an_error_that_names_what_to_change() {
    for refusal in [
        refused("{"),
        refused("[]"),
        refused("{}"),
        refused(r#"{"kind":"claude","x":1}"#),
        refused(r#"{"kind":"claude","audit":1}"#),
        refused(r#"{"kind":"claude","audit":true,"audit":false}"#),
    ] {
        let error: &dyn std::error::Error = &refusal;
        assert!(!error.to_string().is_empty(), "{refusal:?}");
    }
    assert!(
        RunSpecDocumentError::MissingKind
            .to_string()
            .contains("`kind`")
    );
}

/// The flag's own vocabulary: the words `metaharness run` takes, booleans as booleans and a
/// repeatable flag as an array.
#[test]
fn values_take_the_flags_own_vocabulary() {
    let spec = RunSpec::from_document(&json!({
        "kind": "claude",
        "hermetic": "strict",
        "decisions": "observe",
        "credentials": "operator-login",
        "tool-surface": "native",
        "scope-announce": "silent",
        "strict-version": true,
        "audit": false,
        "max-budget-usd": "5",
        "plugin-dir": ["plugins/one", "plugins/two"],
        "plugin": ["beyond10x/agentplugins@aep@0.25.0"],
        "auditor-args": ["--advisory"],
    }))
    .expect("reads");
    assert_eq!(spec.hermetic, HermeticMode::Strict);
    assert_eq!(spec.decisions, DecisionMode::Observe);
    assert_eq!(spec.credentials, CredentialSource::OperatorLogin);
    assert_eq!(spec.tool_surface, ToolSurface::Native);
    assert_eq!(spec.scope_announce, ScopeAnnounce::Silent);
    assert!(spec.strict_version);
    assert!(!spec.audit);
    assert_eq!(spec.max_budget_usd.as_deref(), Some("5"));
    assert_eq!(
        spec.plugin_dir,
        vec![PathBuf::from("plugins/one"), PathBuf::from("plugins/two")]
    );
    assert_eq!(
        spec.plugin[0].to_string(),
        "beyond10x/agentplugins@aep@0.25.0"
    );
    assert_eq!(spec.auditor_args, vec!["--advisory".to_owned()]);

    for (word, credentials) in [
        ("api-key", CredentialSource::ApiKey),
        ("loopback", CredentialSource::Loopback),
        ("none", CredentialSource::None),
    ] {
        let spec =
            RunSpec::from_document(&json!({"kind": "codex", "credentials": word})).expect("reads");
        assert_eq!(spec.credentials, credentials, "{word}");
    }
    assert_eq!(
        RunSpec::from_document(&json!({"kind": "codex", "max-turns": 0}))
            .expect("reads")
            .max_turns,
        Some(0)
    );
    assert_eq!(
        RunSpec::from_document(&json!({"kind": "codex", "max-turns": 4_294_967_295_u64}))
            .expect("reads")
            .max_turns,
        Some(u32::MAX)
    );
    assert_eq!(
        RunSpec::from_document(&json!({"kind": "codex", "plugin-dir": []}))
            .expect("reads")
            .plugin_dir,
        Vec::<PathBuf>::new()
    );
}

/// `--catalogue` without `--tool-surface` means `owned` on the command line, so it does here.
#[test]
fn a_catalogue_without_a_tool_surface_implies_owned_as_on_the_command_line() {
    let implied =
        RunSpec::from_document(&json!({"kind": "claude", "catalogue": "c.yaml"})).expect("reads");
    assert_eq!(implied.tool_surface, ToolSurface::Owned);

    let named = RunSpec::from_document(
        &json!({"kind": "claude", "catalogue": "c.yaml", "tool-surface": "native"}),
    )
    .expect("reads");
    assert_eq!(
        named.tool_surface,
        ToolSurface::Native,
        "an explicit surface is kept, for check_spec to refuse"
    );
}

/// Acceptance 5: the document and `metaharness run` cannot drift apart.
#[cfg(feature = "clap")]
mod against_the_command_line {
    use std::collections::BTreeSet;

    use clap::{Args as _, Command, FromArgMatches as _};
    use metaharness_protocol::RunSpec;
    use serde_json::{Value, json};

    use super::{populated, populated_document, populated_embedded};

    fn run_verb() -> Command {
        RunSpec::augment_args(Command::new("run"))
    }

    /// Each argument of the run verb by its document key: a long flag by its long name, a
    /// positional (`kind`, and `auditor-args` after `--`) by its id in the same spelling.
    fn argument_keys(command: &Command) -> BTreeSet<String> {
        command
            .get_arguments()
            .filter(|argument| !matches!(argument.get_id().as_str(), "help" | "version"))
            .map(|argument| match argument.get_long() {
                Some(long) => long.to_owned(),
                None => argument.get_id().as_str().replace('_', "-"),
            })
            .collect()
    }

    fn read_as_the_command_line(argv: &[String]) -> RunSpec {
        let matches = run_verb()
            .try_get_matches_from(argv)
            .unwrap_or_else(|error| panic!("{argv:?}: {error}"));
        RunSpec::from_arg_matches(&matches).expect("the matches are a spec")
    }

    fn text(value: &Value) -> String {
        match value {
            Value::String(text) => text.clone(),
            Value::Number(number) => number.to_string(),
            other => panic!("no argv spelling for {other}"),
        }
    }

    /// The argv a document spells: `kind` positional, `true` a bare flag, `false` nothing, an
    /// array one flag per item, `auditor-args` after `--`.
    fn argv_of(document: &Value) -> Vec<String> {
        let object = document.as_object().expect("an object");
        let mut argv = vec!["run".to_owned(), text(&object["kind"])];
        let mut trailing = Vec::new();
        for (key, value) in object {
            match (key.as_str(), value) {
                ("kind", _) | (_, Value::Bool(false)) => {}
                ("auditor-args", Value::Array(items)) => trailing.extend(items.iter().map(text)),
                (_, Value::Bool(true)) => argv.push(format!("--{key}")),
                (_, Value::Array(items)) => {
                    for item in items {
                        argv.push(format!("--{key}"));
                        argv.push(text(item));
                    }
                }
                (_, other) => {
                    argv.push(format!("--{key}"));
                    argv.push(text(other));
                }
            }
        }
        if !trailing.is_empty() {
            argv.push("--".to_owned());
            argv.extend(trailing);
        }
        argv
    }

    /// Acceptance 5. A long flag with no document key, or a key with no flag, fails here.
    #[test]
    fn every_argument_of_the_run_verb_is_a_document_key_and_every_key_is_an_argument() {
        let from_the_flags = argument_keys(&run_verb());
        let from_the_document: BTreeSet<String> = RunSpec::DOCUMENT_KEYS
            .iter()
            .map(|key| (*key).to_owned())
            .collect();
        assert_eq!(
            from_the_flags, from_the_document,
            "a flag the document cannot express, or a key the command line does not take"
        );
        assert!(from_the_flags.contains("kind"));
        assert!(from_the_flags.contains("max-turns"));
    }

    /// Every word clap admits for a value-enum argument reads to the same spec both ways. The
    /// other direction, that the document admits no word clap does not, is the crate's own unit
    /// test over its word tables (`spec_document::tests`).
    #[test]
    fn every_word_the_command_line_admits_reads_the_same_in_the_document() {
        let command = run_verb();
        let mut checked = 0;
        for argument in command.get_arguments() {
            if !argument.get_action().takes_values() {
                continue;
            }
            for word in argument.get_possible_values() {
                let word = word.get_name();
                let (argv, document) = match argument.get_long() {
                    None => (
                        vec!["run".to_owned(), word.to_owned()],
                        json!({"kind": word}),
                    ),
                    Some(long) => (
                        vec![
                            "run".to_owned(),
                            "claude".to_owned(),
                            format!("--{long}"),
                            word.to_owned(),
                        ],
                        json!({"kind": "claude", long: word}),
                    ),
                };
                assert_eq!(
                    RunSpec::from_document(&document),
                    Ok(read_as_the_command_line(&argv)),
                    "{argv:?}"
                );
                checked += 1;
            }
        }
        // claude, codex, b10x; off, on, strict; frame, ask, observe; native, owned; four
        // credential sources; stated, silent.
        assert_eq!(checked, 17);
    }

    /// The same settings, spelled as a document and as an argv, are the same spec.
    #[test]
    fn a_populated_document_and_its_argv_read_to_the_same_spec() {
        let (embedded_document, embedded) = populated_embedded();
        for (document, spec) in [
            (populated_document(), populated()),
            (embedded_document, embedded),
            (
                json!({"kind": "codex"}),
                RunSpec::new(metaharness_protocol::Kind::Codex),
            ),
        ] {
            let argv = argv_of(&document);
            assert_eq!(read_as_the_command_line(&argv), spec, "{argv:?}");
            assert_eq!(RunSpec::from_document(&document), Ok(spec));
        }
    }

    /// `--catalogue` alone: both spellings imply the owned surface.
    #[test]
    fn a_catalogue_alone_implies_the_same_surface_both_ways() {
        let argv: Vec<String> = ["run", "claude", "--catalogue", "c.yaml"]
            .map(str::to_owned)
            .to_vec();
        assert_eq!(
            RunSpec::from_document(&json!({"kind": "claude", "catalogue": "c.yaml"})),
            Ok(read_as_the_command_line(&argv))
        );
    }
}
