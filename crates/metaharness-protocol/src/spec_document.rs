//! A [`RunSpec`] as a document: a mapping of `metaharness run` settings (design amendment a64).
//!
//! A caller that holds a run's settings as data, such as a run file's `harness` section, reads them
//! into the one options type here, without the `clap` feature and without spelling an argv. The
//! document is the third spelling of the struct beside the builder and the flags (design D11), so
//! it takes the flags' own words:
//!
//! * a key is a `metaharness run` long flag without its dashes, or `kind`, the positional, or
//!   `auditor-args`, the arguments after `--`;
//! * a value takes the flag's vocabulary: an enum flag's word as clap spells it (`hermetic:
//!   "strict"`), a boolean flag as a JSON boolean, a repeatable flag as an array, a number flag as
//!   a JSON number, and every other flag as a string;
//! * an absent key takes [`RunSpec::new`]'s value, except that `catalogue` without `tool-surface`
//!   implies `owned`, as it does on the command line.
//!
//! A key the document does not know is refused naming it, and so is a value that does not read as
//! its key's type: a setting dropped in silence is a run that differs from the one that was asked
//! for and says so nowhere. `null` is a value like any other and is refused; a key left out takes
//! the default. Whether the kind takes a setting is `metaharness::check_spec`'s question, not
//! the document's, so a document reads every key on every kind.

use std::collections::BTreeSet;
use std::fmt;
use std::path::PathBuf;

use serde::Deserialize;
use serde::de::{Deserializer, IgnoredAny, MapAccess, Visitor};
use serde_json::{Map, Value};

use crate::hermetic::HermeticMode;
use crate::plugin::MarketplacePlugin;
use crate::spec::{CredentialSource, DecisionMode, Kind, RunSpec, ScopeAnnounce, ToolSurface};

/// Why a run-spec document was refused. Every variant with a key names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunSpecDocumentError {
    /// The text is not one JSON value.
    Malformed {
        /// What the JSON reader said.
        detail: String,
    },
    /// The document is JSON and not an object.
    NotAnObject {
        /// What it is instead: `an array`, `a string`, `a number`, `a boolean` or `null`.
        found: &'static str,
    },
    /// The text names a key twice, and one of its two values would be dropped.
    DuplicateKey {
        /// The key.
        key: String,
    },
    /// A key that is not a `metaharness run` setting.
    UnknownKey {
        /// The key, as written.
        key: String,
    },
    /// The document names no `kind`.
    MissingKind,
    /// A value that does not read as its key's type or vocabulary.
    BadValue {
        /// The key.
        key: String,
        /// What the key takes, and what was there instead.
        detail: String,
    },
}

impl fmt::Display for RunSpecDocumentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RunSpecDocumentError::Malformed { detail } => {
                write!(f, "the run-spec document is not one JSON value: {detail}")
            }
            RunSpecDocumentError::NotAnObject { found } => write!(
                f,
                "a run-spec document is an object of `metaharness run` settings, and this one is \
                 {found}"
            ),
            RunSpecDocumentError::DuplicateKey { key } => write!(
                f,
                "the run-spec document names `{key}` twice, and one of the two values would be \
                 dropped; name it once"
            ),
            RunSpecDocumentError::UnknownKey { key } => write!(
                f,
                "the run-spec document names `{key}`, which is not a `metaharness run` setting. A \
                 key is a long flag of the run verb without its dashes, `kind` or `auditor-args`; \
                 an unknown key is refused rather than dropped"
            ),
            RunSpecDocumentError::MissingKind => f.write_str(
                "the run-spec document names no `kind`; name the harness: claude, codex or b10x",
            ),
            RunSpecDocumentError::BadValue { key, detail } => {
                write!(f, "the run-spec document's `{key}` {detail}")
            }
        }
    }
}

impl std::error::Error for RunSpecDocumentError {}

const EMPTY_PATH: &str = "takes a path, and this is the empty string, which names no file; the \
                          command line refuses the same value";

const KINDS: [(&str, Kind); 3] = [
    ("claude", Kind::Claude),
    ("codex", Kind::Codex),
    ("b10x", Kind::B10x),
];

const HERMETIC_MODES: [(&str, HermeticMode); 3] = [
    ("off", HermeticMode::Off),
    ("on", HermeticMode::On),
    ("strict", HermeticMode::Strict),
];

const DECISION_MODES: [(&str, DecisionMode); 3] = [
    ("frame", DecisionMode::Frame),
    ("ask", DecisionMode::Ask),
    ("observe", DecisionMode::Observe),
];

const TOOL_SURFACES: [(&str, ToolSurface); 2] = [
    ("native", ToolSurface::Native),
    ("owned", ToolSurface::Owned),
];

const CREDENTIAL_SOURCES: [(&str, CredentialSource); 4] = [
    ("operator-login", CredentialSource::OperatorLogin),
    ("api-key", CredentialSource::ApiKey),
    ("loopback", CredentialSource::Loopback),
    ("none", CredentialSource::None),
];

const SCOPE_ANNOUNCEMENTS: [(&str, ScopeAnnounce); 2] = [
    ("stated", ScopeAnnounce::Stated),
    ("silent", ScopeAnnounce::Silent),
];

impl RunSpec {
    /// Every key a run-spec document admits, in field order: the run verb's long flags, `kind`
    /// and `auditor-args`. A test holds it equal to the flag set `derive(clap::Args)` makes.
    pub const DOCUMENT_KEYS: &'static [&'static str] = &[
        "kind",
        "hermetic",
        "prompt",
        "frame",
        "decisions",
        "actor",
        "hooks",
        "driver",
        "tool-surface",
        "catalogue",
        "allow-program",
        "credentials",
        "model",
        "model-endpoint",
        "model-wire",
        "subscription-token-file",
        "subscription-token-env",
        "subscription-token-pointer",
        "api-key-secret",
        "effort",
        "max-turns",
        "max-budget-usd",
        "plugin-dir",
        "plugin",
        "cwd",
        "scratch-cwd",
        "retain-dir",
        "strict-version",
        "audit",
        "spec",
        "substrate",
        "substrate-embedded",
        "cgroup-root",
        "toolchain",
        "prices",
        "write-scope",
        "process-write-subtree",
        "scope-announce",
        "context",
        "auditor",
        "auditor-args",
    ];

    /// Read a run-spec document from JSON text.
    ///
    /// The text form of [`RunSpec::from_document`], with one refusal more: a key written twice,
    /// which a parsed value can no longer show.
    ///
    /// # Errors
    ///
    /// [`RunSpecDocumentError::Malformed`] for text that is not one JSON value,
    /// [`RunSpecDocumentError::DuplicateKey`] for a key written twice, and every refusal of
    /// [`RunSpec::from_document`].
    pub fn from_document_str(text: &str) -> Result<Self, RunSpecDocumentError> {
        let malformed = |error: serde_json::Error| RunSpecDocumentError::Malformed {
            detail: error.to_string(),
        };
        let document: Value = serde_json::from_str(text).map_err(malformed)?;
        if document.is_object() {
            let KeyScan(duplicate) = serde_json::from_str(text).map_err(malformed)?;
            if let Some(key) = duplicate {
                return Err(RunSpecDocumentError::DuplicateKey { key });
            }
        }
        Self::from_document(&document)
    }

    /// Read a run-spec document a caller already holds as a value, such as a YAML mapping it
    /// converted.
    ///
    /// # Errors
    ///
    /// [`RunSpecDocumentError::NotAnObject`], then [`RunSpecDocumentError::UnknownKey`] for the
    /// first key outside [`RunSpec::DOCUMENT_KEYS`], then [`RunSpecDocumentError::MissingKind`],
    /// then [`RunSpecDocumentError::BadValue`] for the first value in field order that does not
    /// read as its key's type.
    pub fn from_document(document: &Value) -> Result<Self, RunSpecDocumentError> {
        let Value::Object(settings) = document else {
            return Err(RunSpecDocumentError::NotAnObject {
                found: shape(document),
            });
        };
        if let Some(key) = settings
            .keys()
            .find(|key| !Self::DOCUMENT_KEYS.contains(&key.as_str()))
        {
            return Err(RunSpecDocumentError::UnknownKey { key: key.clone() });
        }
        let read = Settings(settings);
        let kind = read
            .word("kind", &KINDS)?
            .ok_or(RunSpecDocumentError::MissingKind)?;
        let mut spec = RunSpec::new(kind);
        set(&mut spec.hermetic, read.word("hermetic", &HERMETIC_MODES)?);
        set_some(&mut spec.prompt, read.string("prompt")?);
        set_some(&mut spec.frame, read.path("frame")?);
        set(
            &mut spec.decisions,
            read.word("decisions", &DECISION_MODES)?,
        );
        set_some(&mut spec.actor, read.string("actor")?);
        set_some(&mut spec.hooks, read.path("hooks")?);
        set_some(&mut spec.driver, read.path("driver")?);
        set_some(&mut spec.catalogue, read.path("catalogue")?);
        // `default_value_if("catalogue", IsPresent, "owned")` on the flag: the same here.
        match read.word("tool-surface", &TOOL_SURFACES)? {
            Some(surface) => spec.tool_surface = surface,
            None if spec.catalogue.is_some() => spec.tool_surface = ToolSurface::Owned,
            None => {}
        }
        set(&mut spec.allow_program, read.strings("allow-program")?);
        set(
            &mut spec.credentials,
            read.word("credentials", &CREDENTIAL_SOURCES)?,
        );
        set_some(&mut spec.model, read.string("model")?);
        set_some(&mut spec.model_endpoint, read.string("model-endpoint")?);
        set_some(&mut spec.model_wire, read.string("model-wire")?);
        set_some(
            &mut spec.subscription_token_file,
            read.string("subscription-token-file")?,
        );
        set_some(
            &mut spec.subscription_token_env,
            read.string("subscription-token-env")?,
        );
        set_some(
            &mut spec.subscription_token_pointer,
            read.string("subscription-token-pointer")?,
        );
        set_some(&mut spec.api_key_secret, read.string("api-key-secret")?);
        set_some(&mut spec.effort, read.string("effort")?);
        set_some(&mut spec.max_turns, read.count("max-turns")?);
        set_some(&mut spec.max_budget_usd, read.string("max-budget-usd")?);
        set(&mut spec.plugin_dir, read.paths("plugin-dir")?);
        set(&mut spec.plugin, read.plugins("plugin")?);
        set_some(&mut spec.cwd, read.path("cwd")?);
        set(&mut spec.scratch_cwd, read.flag("scratch-cwd")?);
        set_some(&mut spec.retain_dir, read.path("retain-dir")?);
        set(&mut spec.strict_version, read.flag("strict-version")?);
        set(&mut spec.audit, read.flag("audit")?);
        set_some(&mut spec.spec, read.path("spec")?);
        set_some(&mut spec.substrate, read.path("substrate")?);
        set(
            &mut spec.substrate_embedded,
            read.flag("substrate-embedded")?,
        );
        set_some(&mut spec.cgroup_root, read.path("cgroup-root")?);
        set_some(&mut spec.toolchain, read.string("toolchain")?);
        set_some(&mut spec.prices, read.path("prices")?);
        set(&mut spec.write_scope, read.strings("write-scope")?);
        set(
            &mut spec.process_write_subtree,
            read.strings("process-write-subtree")?,
        );
        set(
            &mut spec.scope_announce,
            read.word("scope-announce", &SCOPE_ANNOUNCEMENTS)?,
        );
        set(&mut spec.context, read.paths("context")?);
        set_some(&mut spec.auditor, read.string("auditor")?);
        set(&mut spec.auditor_args, read.strings("auditor-args")?);
        Ok(spec)
    }
}

/// A value the document named replaces the default; an absent one leaves it.
fn set<T>(slot: &mut T, value: Option<T>) {
    if let Some(value) = value {
        *slot = value;
    }
}

fn set_some<T>(slot: &mut Option<T>, value: Option<T>) {
    if let Some(value) = value {
        *slot = Some(value);
    }
}

/// What a JSON value is, in the words a refusal uses.
fn shape(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// What a value is, and for `null` and a number where a string was wanted, what to write instead.
fn instead(value: &Value) -> String {
    match value {
        Value::Null => "this is null; leave the key out to take the default".to_owned(),
        other => format!("this is {}", shape(other)),
    }
}

/// The document's settings, read one key at a time.
struct Settings<'a>(&'a Map<String, Value>);

impl Settings<'_> {
    fn bad(key: &str, detail: String) -> RunSpecDocumentError {
        RunSpecDocumentError::BadValue {
            key: key.to_owned(),
            detail,
        }
    }

    fn word<T: Copy>(
        &self,
        key: &str,
        words: &[(&str, T)],
    ) -> Result<Option<T>, RunSpecDocumentError> {
        let Some(value) = self.0.get(key) else {
            return Ok(None);
        };
        let list = words
            .iter()
            .map(|(word, _)| *word)
            .collect::<Vec<_>>()
            .join(", ");
        let Value::String(given) = value else {
            return Err(Self::bad(
                key,
                format!("takes one of {list} as a string, and {}", instead(value)),
            ));
        };
        words
            .iter()
            .find(|(word, _)| word == given)
            .map(|(_, variant)| Some(*variant))
            .ok_or_else(|| {
                Self::bad(
                    key,
                    format!("takes one of {list}, and `{given}` is none of them"),
                )
            })
    }

    fn string(&self, key: &str) -> Result<Option<String>, RunSpecDocumentError> {
        match self.0.get(key) {
            None => Ok(None),
            Some(Value::String(text)) => Ok(Some(text.clone())),
            Some(Value::Number(number)) => Err(Self::bad(
                key,
                format!(
                    "takes a string, and this is the number {number}; write it as the string \
                     \"{number}\", because the flag's value is passed on as written"
                ),
            )),
            Some(other) => Err(Self::bad(
                key,
                format!("takes a string, and {}", instead(other)),
            )),
        }
    }

    /// A path, refused when empty as clap's path parser refuses `--<key> ""`, so a document and
    /// its argv read to the same spec (a64; adversary pass 1, story:run-spec-document).
    fn path(&self, key: &str) -> Result<Option<PathBuf>, RunSpecDocumentError> {
        match self.string(key)? {
            Some(text) if text.is_empty() => Err(Self::bad(key, EMPTY_PATH.to_owned())),
            other => Ok(other.map(PathBuf::from)),
        }
    }

    fn flag(&self, key: &str) -> Result<Option<bool>, RunSpecDocumentError> {
        match self.0.get(key) {
            None => Ok(None),
            Some(Value::Bool(on)) => Ok(Some(*on)),
            Some(other) => Err(Self::bad(
                key,
                format!("takes true or false, and {}", instead(other)),
            )),
        }
    }

    fn count(&self, key: &str) -> Result<Option<u32>, RunSpecDocumentError> {
        let Some(value) = self.0.get(key) else {
            return Ok(None);
        };
        let range = format!("takes a whole number from 0 to {}", u32::MAX);
        match value {
            Value::Number(number) => number
                .as_u64()
                .and_then(|whole| u32::try_from(whole).ok())
                .map(Some)
                .ok_or_else(|| Self::bad(key, format!("{range}, and this is {number}"))),
            other => Err(Self::bad(key, format!("{range}, and {}", instead(other)))),
        }
    }

    /// A repeatable flag: an array of strings, one per occurrence of the flag.
    fn strings(&self, key: &str) -> Result<Option<Vec<String>>, RunSpecDocumentError> {
        let Some(value) = self.0.get(key) else {
            return Ok(None);
        };
        let Value::Array(items) = value else {
            return Err(Self::bad(
                key,
                format!(
                    "takes an array of strings, one per --{key}, and {}",
                    instead(value)
                ),
            ));
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| match item {
                Value::String(text) => Ok(text.clone()),
                other => Err(Self::bad(
                    key,
                    format!(
                        "takes an array of strings, and item {index} is {}",
                        shape(other)
                    ),
                )),
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }

    fn paths(&self, key: &str) -> Result<Option<Vec<PathBuf>>, RunSpecDocumentError> {
        let Some(items) = self.strings(key)? else {
            return Ok(None);
        };
        if let Some(index) = items.iter().position(String::is_empty) {
            return Err(Self::bad(key, format!("item {index}: {EMPTY_PATH}")));
        }
        Ok(Some(items.into_iter().map(PathBuf::from).collect()))
    }

    /// `--plugin`'s own spelling, `<repo>@<name>@<pin>`, refused as the flag refuses it.
    fn plugins(&self, key: &str) -> Result<Option<Vec<MarketplacePlugin>>, RunSpecDocumentError> {
        let Some(items) = self.strings(key)? else {
            return Ok(None);
        };
        items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                item.parse::<MarketplacePlugin>()
                    .map_err(|error| Self::bad(key, format!("item {index}: {error}")))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some)
    }
}

/// The first key a JSON object's text names twice, read before the text becomes a [`Value`],
/// whose map keeps one value per key.
struct KeyScan(Option<String>);

impl<'de> Deserialize<'de> for KeyScan {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Keys;

        impl<'de> Visitor<'de> for Keys {
            type Value = KeyScan;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an object")
            }

            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<KeyScan, A::Error> {
                let mut seen = BTreeSet::new();
                let mut duplicate = None;
                while let Some(key) = map.next_key::<String>()? {
                    map.next_value::<IgnoredAny>()?;
                    if duplicate.is_none() && !seen.insert(key.clone()) {
                        duplicate = Some(key);
                    }
                }
                Ok(KeyScan(duplicate))
            }
        }

        deserializer.deserialize_map(Keys)
    }
}

/// The other direction of the anti-drift test in `tests/run_spec_document.rs`: each word table
/// holds exactly the words clap derives for its enum, so the document admits no word the command
/// line refuses.
#[cfg(all(test, feature = "clap"))]
mod tests {
    use clap::ValueEnum;

    use super::{
        CREDENTIAL_SOURCES, DECISION_MODES, HERMETIC_MODES, KINDS, SCOPE_ANNOUNCEMENTS,
        TOOL_SURFACES,
    };

    fn clap_words<T: ValueEnum + PartialEq + std::fmt::Debug>(table: &[(&str, T)]) {
        let derived: Vec<(String, &T)> = T::value_variants()
            .iter()
            .map(|variant| {
                let word = variant
                    .to_possible_value()
                    .expect("no variant is skipped")
                    .get_name()
                    .to_owned();
                (word, variant)
            })
            .collect();
        assert_eq!(table.len(), derived.len(), "{table:?}");
        for (word, variant) in table {
            assert!(
                derived.iter().any(
                    |(derived, derived_variant)| derived == word && *derived_variant == variant
                ),
                "`{word}` for {variant:?} is not clap's word: {derived:?}"
            );
        }
    }

    #[test]
    fn every_word_table_is_the_words_clap_derives() {
        clap_words(&KINDS);
        clap_words(&HERMETIC_MODES);
        clap_words(&DECISION_MODES);
        clap_words(&TOOL_SURFACES);
        clap_words(&CREDENTIAL_SOURCES);
        clap_words(&SCOPE_ANNOUNCEMENTS);
    }
}
