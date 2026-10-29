//! A catalogue the caller supplies: one `tool-contract/1` document, read once, served flat
//! (design amendment a40).
//!
//! # What it replaces, for one run
//!
//! Without it the owned surface publishes the three verbs over the whole local catalogue. With
//! it the server publishes the document's operations and nothing else, one MCP tool per
//! operation, named by the operation's own `mcp` rendering. The verbs are not published, because
//! behind `tool_invoke` they reach tools the caller did not name.
//!
//! # The reader is the format's own
//!
//! The document is parsed by `b10x-tool-contract`, the crate the `tools` repository publishes the
//! format in, and checked by its validator. Nothing here re-states the grammar. What this module
//! adds is what serving needs and the format leaves open: that every operation has an `mcp`
//! rendering a vendor can carry as a tool name, and an input schema derived from the field list.

use std::fmt;
use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};

use serde_json::value::RawValue;
use serde_json::{Map, Value, json};
use tool_contract::types::Shape;
use tool_contract::{Contract, validate};

use crate::resolve::Resolved;
use crate::server::SERVER_NAME;

/// The code a catalogue file that could not be read is refused with.
pub const CATALOGUE_UNREADABLE: &str = "catalogue-unreadable";
/// The code a document in another format is refused with.
pub const CATALOGUE_NOT_TOOL_CONTRACT: &str = "catalogue-not-tool-contract";
/// The code a `tool-contract/1` document that cannot be served is refused with.
pub const CATALOGUE_MALFORMED: &str = "catalogue-malformed";

/// The most bytes a catalogue file is read to: 4 MiB. The scratch copy is read under the same cap,
/// so a document whose copy would pass it is refused by the parent (amendment a40 § 3).
pub const CATALOGUE_BYTES_MAX: u64 = 4 * 1024 * 1024;

/// The text of the scratch copy the server reads, for a document as JSON (amendment a40 § 6).
///
/// Pretty JSON, with every character the YAML reader refuses or changes written as a `\u` escape:
/// 37 characters, U+007F–U+009F, U+2028, U+2029, U+FFFE and U+FFFF. `serde_json` already escapes
/// the C0 controls and writes these raw. The reader (`serde_yaml_ng` 0.10 over `unsafe-libyaml`
/// 0.2.11) admits only tab, line feed, carriage return, U+0020–U+007E, U+0085, U+00A0–U+D7FF,
/// U+E000–U+FFFD and U+10000–U+10FFFF (`src/reader.rs:381-388`), so 34 of them are refused raw:
/// U+007F–U+0084, U+0086–U+009F, U+FFFE and U+FFFF. The other three, U+0085, U+2028 and U+2029,
/// it admits and reads as line breaks (`src/macros.rs:253-264`), and a quoted scalar folds a line
/// break and drops the spaces beside it, so those are escaped too. Characters past U+FFFF are
/// written raw: the reader admits them, and refuses a `\u` escape of a surrogate
/// (`src/scanner.rs:2355`). Each escaped character can only stand inside a JSON string, where its
/// escape means the same character, so the copy is the same JSON.
/// `every_unicode_scalar_value_survives_the_scratch_copy` asks this of every scalar value, each
/// between spaces.
///
/// # Panics
///
/// Never: a JSON value's keys are strings, so it always renders, and writing to a `String` cannot
/// fail.
#[must_use]
pub fn render_scratch_copy(document: &Value) -> String {
    let pretty = serde_json::to_string_pretty(document).expect("a JSON value renders");
    let mut copy = String::with_capacity(pretty.len());
    for character in pretty.chars() {
        if refused_by_the_reader(character) {
            write!(copy, "\\u{:04x}", u32::from(character)).expect("a String takes any text");
        } else {
            copy.push(character);
        }
    }
    copy
}

/// A character `serde_json` writes raw that the YAML reader refuses or reads as a line break.
fn refused_by_the_reader(character: char) -> bool {
    matches!(
        character,
        '\u{7f}'..='\u{9f}' | '\u{2028}' | '\u{2029}' | '\u{fffe}' | '\u{ffff}'
    )
}

/// The longest published name a vendor can carry once it prefixes it.
///
/// Claude Code names an MCP tool `mcp__<server>__<tool>`, which is 18 bytes before the tool under
/// this server, and the Messages API documents a tool name as `^[a-zA-Z0-9_-]{1,64}$`. The
/// pattern is read from the vendor's documentation and was not driven here (amendment a40 § 3).
const MAX_PUBLISHED_NAME: usize = 64 - "mcp__".len() - SERVER_NAME.len() - "__".len();

/// The rendering a published name comes from.
const MCP_RENDERING: &str = "mcp";

/// Why a catalogue file is not a catalogue this run can serve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CatalogueError {
    /// The platform would not hand over the file's text.
    Unreadable {
        /// The file the run named.
        path: PathBuf,
        /// What the platform said.
        detail: String,
    },
    /// The text is YAML and its top level is not a mapping whose `format` is `tool-contract/1`.
    NotToolContract {
        /// The file the run named.
        path: PathBuf,
        /// What the document is instead, in words.
        found: String,
    },
    /// A `tool-contract/1` document, or text that is not YAML at all, that cannot be served.
    Malformed {
        /// The file the run named.
        path: PathBuf,
        /// Every problem found, each naming where it is.
        problems: Vec<String>,
    },
}

impl CatalogueError {
    /// The code the refusal's message begins with.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unreadable { .. } => CATALOGUE_UNREADABLE,
            Self::NotToolContract { .. } => CATALOGUE_NOT_TOOL_CONTRACT,
            Self::Malformed { .. } => CATALOGUE_MALFORMED,
        }
    }

    fn malformed(path: &Path, problems: Vec<String>) -> Self {
        Self::Malformed {
            path: path.to_path_buf(),
            problems,
        }
    }

    /// What was found, without the code and the path: for a refusal that has to name the file some
    /// other way than by its path.
    #[must_use]
    pub fn problems(&self) -> Vec<String> {
        match self {
            Self::Unreadable { detail, .. } => vec![detail.clone()],
            Self::NotToolContract { found, .. } => vec![format!(
                "it is not a {} document: {found}",
                tool_contract::document::FORMAT
            )],
            Self::Malformed { problems, .. } => problems.clone(),
        }
    }
}

impl fmt::Display for CatalogueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unreadable { path, detail } => write!(
                f,
                "{CATALOGUE_UNREADABLE}: --catalogue {} could not be read: {detail}. Nothing was \
                 started",
                path.display()
            ),
            Self::NotToolContract { path, found } => write!(
                f,
                "{CATALOGUE_NOT_TOOL_CONTRACT}: --catalogue {} is not a {} document: {found}. \
                 Nothing was started",
                path.display(),
                tool_contract::document::FORMAT
            ),
            Self::Malformed { path, problems } => write!(
                f,
                "{CATALOGUE_MALFORMED}: --catalogue {} is not a {} catalogue this run can serve: \
                 {}. Nothing was started",
                path.display(),
                tool_contract::document::FORMAT,
                problems.join("; ")
            ),
        }
    }
}

impl std::error::Error for CatalogueError {}

/// One operation of the catalogue, as the server publishes it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogueEntry {
    /// The operation's name in the document, such as `fs.read`.
    pub contract_name: String,
    /// The name it is published under: its `mcp` rendering, such as `fs_read`.
    pub published: String,
    /// The operation's `summary`, with its `guidance` after a blank line when it has one.
    pub description: String,
    /// The JSON Schema of its input, derived from the field list's shape.
    pub input_schema: Value,
}

impl CatalogueEntry {
    /// Why a call's arguments are not ones this entry's input schema admits, each problem naming
    /// the value; empty when the call may be performed.
    ///
    /// Everything is read from the published schema, so the server refuses what it published and
    /// nothing else (amendments a40 § 5 and a45). The arguments must be an object with no key
    /// outside the entry's input fields, giving every required field (one whose type is not
    /// `Optional<…>`) a value other than null. Every value given is then held to its declared
    /// type, at every depth the schema declares: a struct's fields under the same two rules, a
    /// list's items and a map's values. An `integer` is a JSON number held as an `i64` or a `u64`,
    /// an enum is one of its strings, and null on a field that is not required reads as absent.
    /// A value is named by its path (`offset`, `window.from`, `edits[1].old`, `env["key"]`) and
    /// with its declared type, and so is a required field left out or given as null.
    #[must_use]
    pub fn argument_problems(&self, arguments: &Value) -> Vec<String> {
        let Value::Object(given) = arguments else {
            return vec![format!(
                "its arguments are {}, and they must be an object of its input fields",
                json_kind(arguments)
            )];
        };
        let mut problems = Vec::new();
        struct_problems(&self.input_schema, given, None, &mut problems);
        problems
    }

    /// The arguments as the call is performed: every struct field given as null that its schema
    /// does not mark required is taken out, at every depth the schema declares a struct, because
    /// such a null reads as absent (amendments a45 § 2 and a50 § 2). A null list item or map value
    /// is a value rather than a field and is kept, and so is anything the schema does not declare.
    /// Meant for arguments [`argument_problems`](Self::argument_problems) found nothing wrong
    /// with, so no required field is null by then.
    #[must_use]
    pub fn without_absent_nulls(&self, mut arguments: Value) -> Value {
        if let Value::Object(given) = &mut arguments {
            drop_absent_nulls(&self.input_schema, given);
        }
        arguments
    }
}

/// Takes out of a struct each field given as null that `schema` does not mark required, then does
/// the same inside each remaining value its schema declares.
fn drop_absent_nulls(schema: &Value, given: &mut Map<String, Value>) {
    let required: Vec<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    given.retain(|key, value| !value.is_null() || required.contains(&key.as_str()));
    let Some(properties) = schema.get("properties").and_then(Value::as_object) else {
        return;
    };
    for (key, value) in given.iter_mut() {
        if let Some(field) = properties.get(key) {
            drop_absent_nulls_inside(field, value);
        }
    }
}

/// [`drop_absent_nulls`] for every struct inside `value`, read as [`value_problems`] reads it.
fn drop_absent_nulls_inside(schema: &Value, value: &mut Value) {
    if let Some(choices) = schema.get("anyOf").and_then(Value::as_array) {
        if let Some(choice) = choices
            .iter()
            .find(|choice| of_declared_kind(choice, value))
        {
            drop_absent_nulls_inside(choice, value);
        }
        return;
    }
    match value {
        Value::Array(items) => {
            if let Some(item) = schema.get("items") {
                for each in items {
                    drop_absent_nulls_inside(item, each);
                }
            }
        }
        Value::Object(given) => {
            if schema.get("properties").is_some() {
                drop_absent_nulls(schema, given);
            } else if let Some(each) = schema
                .get("additionalProperties")
                .filter(|each| each.is_object())
            {
                for entry in given.values_mut() {
                    drop_absent_nulls_inside(each, entry);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {}
    }
}

/// The problems of an object whose schema has `properties`: keys outside them, required fields
/// left out or null (each named with its declared type), then each value given against its
/// field's schema. `at` is the object's path, `None` for a call's arguments.
fn struct_problems(
    schema: &Value,
    given: &Map<String, Value>,
    at: Option<&str>,
    problems: &mut Vec<String>,
) {
    let properties = schema.get("properties").and_then(Value::as_object);
    let declared: Vec<&str> = properties
        .map(|properties| properties.keys().map(String::as_str).collect())
        .unwrap_or_default();
    let path_of = |name: &str| at.map_or_else(|| name.to_owned(), |at| format!("{at}.{name}"));
    for key in given.keys() {
        if declared.contains(&key.as_str()) {
            continue;
        }
        let fields = declared
            .iter()
            .map(|name| format!("`{name}`"))
            .collect::<Vec<_>>()
            .join(", ");
        problems.push(match (at, declared.is_empty()) {
            (None, true) => format!("`{key}` is given, and it has no input fields"),
            (None, false) => format!("`{key}` is not one of its input fields, which are {fields}"),
            (Some(at), true) => format!("`{at}.{key}` is given, and `{at}` has no fields"),
            (Some(at), false) => {
                format!("`{at}.{key}` is not one of the fields of `{at}`, which are {fields}")
            }
        });
    }
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str);
    for name in required {
        let path = path_of(name);
        let declared = properties
            .and_then(|properties| properties.get(name))
            .map(|field| format!(", and its declared type is {}", declared_type(field)))
            .unwrap_or_default();
        match given.get(name) {
            None => problems.push(format!(
                "`{path}` is required and the call leaves it out{declared}"
            )),
            Some(Value::Null) => problems.push(format!(
                "`{path}` is required and the call gives it as null{declared}"
            )),
            Some(_) => {}
        }
    }
    for (key, value) in given {
        // Null on a required field is refused above; on any other field it reads as absent.
        if value.is_null() {
            continue;
        }
        if let Some(field) = properties.and_then(|properties| properties.get(key)) {
            value_problems(field, value, &path_of(key), problems);
        }
    }
}

/// The problems of one value against the schema published for it, named by `path`.
fn value_problems(schema: &Value, value: &Value, path: &str, problems: &mut Vec<String>) {
    // Only `Optional` outside a field publishes `anyOf`: its inner schema, or null.
    if let Some(choices) = schema.get("anyOf").and_then(Value::as_array) {
        match choices
            .iter()
            .find(|choice| of_declared_kind(choice, value))
        {
            Some(choice) => value_problems(choice, value, path, problems),
            None => problems.push(mistyped(path, value, schema)),
        }
        return;
    }
    if !of_declared_kind(schema, value) {
        problems.push(mistyped(path, value, schema));
        return;
    }
    match value {
        Value::String(text) => {
            if let Some(values) = schema.get("enum").and_then(Value::as_array)
                && !values.iter().any(|named| named.as_str() == Some(text))
            {
                problems.push(format!(
                    "`{path}` is a string that is not one of its values, and its declared type is \
                     {}",
                    declared_type(schema)
                ));
            }
        }
        Value::Array(items) => {
            if let Some(item) = schema.get("items") {
                for (index, each) in items.iter().enumerate() {
                    value_problems(item, each, &format!("{path}[{index}]"), problems);
                }
            }
        }
        Value::Object(given) => {
            if schema.get("properties").is_some() {
                struct_problems(schema, given, Some(path), problems);
            } else if let Some(each) = schema
                .get("additionalProperties")
                .filter(|each| each.is_object())
            {
                for (key, entry) in given {
                    let key = Value::from(key.as_str());
                    value_problems(each, entry, &format!("{path}[{key}]"), problems);
                }
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
}

/// Whether `value` is of the JSON kind `schema`'s `type` names. A schema without a `type` names
/// no kind; one whose `type` this module does not publish admits nothing.
fn of_declared_kind(schema: &Value, value: &Value) -> bool {
    match schema.get("type").and_then(Value::as_str) {
        None => true,
        Some("string") => value.is_string(),
        Some("integer") => value.is_i64() || value.is_u64(),
        Some("boolean") => value.is_boolean(),
        Some("array") => value.is_array(),
        Some("object") => value.is_object(),
        Some("null") => value.is_null(),
        Some(_) => false,
    }
}

/// A value of the wrong kind, named by its path, the kind it is and the type it is declared.
///
/// `serde_json` holds a number written as an integer inside 64 bits as an `i64` or a `u64`, with
/// one exception without its `arbitrary_precision` feature: `-0`, which it then holds as the float
/// `-0.0` (with the feature, as the `i64` 0; a45 § 2). So a number held as neither is negative
/// zero, or one written with a fraction or an exponent, or one outside 64 bits.
fn mistyped(path: &str, value: &Value, schema: &Value) -> String {
    let found = match value {
        Value::Number(number) if number.is_i64() || number.is_u64() => json_kind(value),
        Value::Number(number)
            if number
                .as_f64()
                .is_some_and(|float| float.to_bits() == (-0.0_f64).to_bits()) =>
        {
            "negative zero, a number not held as a 64-bit integer"
        }
        Value::Number(_) => "a number written with a fraction or an exponent, or outside 64 bits",
        other => json_kind(other),
    };
    format!(
        "`{path}` is {found}, and its declared type is {}",
        declared_type(schema)
    )
}

/// The declared type in the schema's words: `` `integer` ``, `` `integer` or `null` ``,
/// `` `string`, one of `directory`, `file` ``.
fn declared_type(schema: &Value) -> String {
    if let Some(choices) = schema.get("anyOf").and_then(Value::as_array) {
        return choices
            .iter()
            .map(declared_type)
            .collect::<Vec<_>>()
            .join(" or ");
    }
    let mut words = format!(
        "`{}`",
        schema
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or("any value")
    );
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        let values = values
            .iter()
            .filter_map(Value::as_str)
            .map(|named| format!("`{named}`"))
            .collect::<Vec<_>>()
            .join(", ");
        write!(words, ", one of {values}").expect("a String takes any text");
    }
    if let Some(encoding) = schema.get("contentEncoding").and_then(Value::as_str) {
        write!(words, ", {encoding}-encoded").expect("a String takes any text");
    }
    words
}

fn json_kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "an object",
    }
}

/// The name a vendor gives a catalogue entry it offers.
///
/// Claude Code reaches the entries through an MCP server and prefixes each:
/// `mcp__metaharness__<published>`. Codex's app-server offers them as client tools under the bare
/// published name (design amendment a66).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntryNaming {
    /// `mcp__metaharness__<published>`.
    McpPrefixed,
    /// `<published>`.
    Bare,
}

impl EntryNaming {
    fn name(self, published: &str) -> String {
        match self {
            Self::McpPrefixed => prefixed(published),
            Self::Bare => published.to_owned(),
        }
    }
}

/// A caller's `tool-contract/1` document, read and checked, with the entries it publishes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallerCatalogue {
    contract: Contract,
    entries: Vec<CatalogueEntry>,
}

impl CallerCatalogue {
    /// Reads and checks the file.
    ///
    /// Only a regular file is read, asked before it is opened, so a pipe or a device is refused
    /// rather than waited on or read without end; and it is read to [`CATALOGUE_BYTES_MAX`] bytes
    /// and no further.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::Unreadable`] when the path is not a regular file, the file is larger than
    /// the cap or its text cannot be read, and otherwise whatever [`CallerCatalogue::from_text`]
    /// refuses.
    pub fn read(path: &Path) -> Result<Self, CatalogueError> {
        let unreadable = |detail: String| CatalogueError::Unreadable {
            path: path.to_path_buf(),
            detail,
        };
        let not_regular = |metadata: &std::fs::Metadata| {
            let what = if metadata.is_dir() {
                "a directory"
            } else {
                "something else"
            };
            unreadable(format!(
                "it is {what}, not a regular file, and only a regular file is read as a catalogue"
            ))
        };
        let metadata = std::fs::metadata(path).map_err(|error| unreadable(error.to_string()))?;
        if !metadata.is_file() {
            return Err(not_regular(&metadata));
        }
        let file = std::fs::File::open(path).map_err(|error| unreadable(error.to_string()))?;
        // The file opened is asked again: the path may have been replaced since it was looked at.
        let opened = file
            .metadata()
            .map_err(|error| unreadable(error.to_string()))?;
        if !opened.is_file() {
            return Err(not_regular(&opened));
        }
        let mut bytes = Vec::new();
        file.take(CATALOGUE_BYTES_MAX + 1)
            .read_to_end(&mut bytes)
            .map_err(|error| unreadable(error.to_string()))?;
        if u64::try_from(bytes.len()).map_or(true, |read| read > CATALOGUE_BYTES_MAX) {
            return Err(unreadable(format!(
                "it is larger than {CATALOGUE_BYTES_MAX} bytes (4 MiB), the most a catalogue is \
                 read to"
            )));
        }
        let text = String::from_utf8(bytes)
            .map_err(|error| unreadable(format!("its text is not UTF-8: {error}")))?;
        Self::from_text(path, &text)
    }

    /// Checks a document's text; `path` is only the name its refusals carry.
    ///
    /// The `format` is read before the grammar. The format's parser refuses an unknown field
    /// before it looks at `format`, so a document in another format would otherwise be reported
    /// as a malformed `tool-contract/1` document, which names the wrong problem.
    ///
    /// A document is accepted only when its scratch copy ([`CallerCatalogue::document`]) is one the
    /// server reads back as the same catalogue, within [`CATALOGUE_BYTES_MAX`]: what the parent
    /// checked is then what the server serves, whatever the caller's YAML said and however it
    /// said it (amendment a40 § 3).
    ///
    /// One byte-order mark at the start of the text is dropped before it is parsed. YAML admits
    /// one there (YAML 1.2.2 § 5.2) and editors write one, but `serde_yaml_ng` 0.10 declares the
    /// stream UTF-8 to the reader (`src/libyaml/parser.rs:75`), and the reader looks for a mark only
    /// when no encoding was declared (`unsafe-libyaml` 0.2.11 `src/reader.rs:164-168`), so it
    /// keeps the mark as a character.
    ///
    /// # Errors
    ///
    /// [`CatalogueError::NotToolContract`] when the text is YAML in another format, and
    /// [`CatalogueError::Malformed`] for every other reason the document cannot be served,
    /// its scratch copy not reading back among them.
    pub fn from_text(path: &Path, text: &str) -> Result<Self, CatalogueError> {
        let text = text.strip_prefix('\u{feff}').unwrap_or(text);
        let checked = Self::checked(path, text)?;
        let copy = render_scratch_copy(&checked.document_value());
        if u64::try_from(copy.len()).map_or(true, |bytes| bytes > CATALOGUE_BYTES_MAX) {
            return Err(CatalogueError::malformed(
                path,
                vec![format!(
                    "its scratch copy would be {} bytes, and the server reads a catalogue to \
                     {CATALOGUE_BYTES_MAX} bytes (4 MiB) and no further",
                    copy.len()
                )],
            ));
        }
        match Self::checked(path, &copy) {
            Ok(again) if again == checked => Ok(checked),
            Ok(_) => Err(CatalogueError::malformed(
                path,
                vec!["its scratch copy reads back as another catalogue".to_owned()],
            )),
            Err(error) => Err(CatalogueError::malformed(
                path,
                vec![format!(
                    "its scratch copy would not read back: {}",
                    error.problems().join("; ")
                )],
            )),
        }
    }

    /// The format's checks and this module's, on one text.
    fn checked(path: &Path, text: &str) -> Result<Self, CatalogueError> {
        let document: serde_yaml_ng::Value = serde_yaml_ng::from_str(text).map_err(|error| {
            CatalogueError::malformed(path, vec![format!("the text is not YAML: {error}")])
        })?;
        if let Some(found) = not_tool_contract(&document) {
            return Err(CatalogueError::NotToolContract {
                path: path.to_path_buf(),
                found,
            });
        }
        let contract = Contract::from_yaml(text)
            .map_err(|error| CatalogueError::malformed(path, vec![error.to_string()]))?;
        if let Err(violations) = validate(&contract) {
            return Err(CatalogueError::malformed(
                path,
                violations.iter().map(ToString::to_string).collect(),
            ));
        }
        let entries =
            entries(&contract).map_err(|problems| CatalogueError::malformed(path, problems))?;
        Ok(Self { contract, entries })
    }

    /// The entries, in document order.
    #[must_use]
    pub fn entries(&self) -> &[CatalogueEntry] {
        &self.entries
    }

    /// The names a vendor that prefixes MCP tools offers: `mcp__metaharness__<published>`.
    ///
    /// What a Claude Code session's opening record lists under this catalogue, if it lists what
    /// the launch asked for. A reader compares the two; nothing here asserts it.
    #[must_use]
    pub fn offered_tools(&self) -> Vec<String> {
        self.offered_tools_as(EntryNaming::McpPrefixed)
    }

    /// The names a vendor offers the entries under, by its naming, in document order.
    #[must_use]
    pub fn offered_tools_as(&self, naming: EntryNaming) -> Vec<String> {
        self.entries
            .iter()
            .map(|entry| naming.name(&entry.published))
            .collect()
    }

    /// The scratch copy the server reads, as the JSON text [`render_scratch_copy`] writes.
    ///
    /// Serialising it writes that text unchanged. JSON is YAML the format's parser reads, and
    /// [`CallerCatalogue::from_text`] accepted this catalogue only because this copy reads back as
    /// it.
    ///
    /// # Panics
    ///
    /// Never: the rendering is JSON.
    #[must_use]
    pub fn document(&self) -> Box<RawValue> {
        RawValue::from_string(render_scratch_copy(&self.document_value()))
            .expect("the scratch copy is JSON")
    }

    /// The document as a JSON value, for a plan that carries it as one; the file written from it
    /// with [`render_scratch_copy`] is [`CallerCatalogue::document`].
    ///
    /// # Panics
    ///
    /// Never for a catalogue this type built: its document was parsed from text into the format's
    /// own types, whose keys are strings and whose values serialise.
    #[must_use]
    pub fn document_value(&self) -> Value {
        serde_json::to_value(&self.contract)
            .expect("a parsed tool-contract/1 document serialises to JSON")
    }

    /// What a call is, under this catalogue.
    ///
    /// Only a published entry, by the name the vendor gives it, resolves, and it resolves through
    /// its contract name to the entry harness maps that name to. Everything else is
    /// [`Resolved::Unknown`]: the three verbs, a vendor's own tool, another server's tool and an
    /// entry nothing in this run performs.
    #[must_use]
    pub fn resolve(&self, tool: &str) -> Resolved {
        self.resolve_as(tool, EntryNaming::McpPrefixed)
    }

    /// [`CallerCatalogue::resolve`] for a vendor that names the entries by `naming`.
    #[must_use]
    pub fn resolve_as(&self, tool: &str, naming: EntryNaming) -> Resolved {
        self.performed_as(tool, naming)
            .and_then(harness_catalogue::operation_of)
            .map_or(Resolved::Unknown, |operation| {
                Resolved::Operations(vec![operation.to_owned()])
            })
    }

    /// What a call would touch, under this catalogue: the subjects of the entry it resolves to,
    /// read from the call's own arguments, and nothing for a call that resolves to no operation.
    #[must_use]
    pub fn subjects_of(&self, tool: &str, input: &Value) -> Vec<String> {
        self.subjects_as(tool, input, EntryNaming::McpPrefixed)
    }

    /// [`CallerCatalogue::subjects_of`] for a vendor that names the entries by `naming`.
    #[must_use]
    pub fn subjects_as(&self, tool: &str, input: &Value, naming: EntryNaming) -> Vec<String> {
        if matches!(self.resolve_as(tool, naming), Resolved::Unknown) {
            return Vec::new();
        }
        self.performed_as(tool, naming)
            .map(|entry| {
                harness_catalogue::subjects_of(entry, input)
                    .into_iter()
                    .map(|subject| subject.as_str().to_owned())
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The entry published under `name`, by its bare published name.
    pub(crate) fn published(&self, name: &str) -> Option<&CatalogueEntry> {
        self.entries.iter().find(|entry| entry.published == name)
    }

    /// The harness entry a vendor-named call is performed as, when it is one.
    fn performed_as(&self, tool: &str, naming: EntryNaming) -> Option<&'static str> {
        self.entries
            .iter()
            .find(|entry| naming.name(&entry.published) == tool)
            .and_then(|entry| harness_catalogue::harness_operation(&entry.contract_name))
    }
}

fn prefixed(published: &str) -> String {
    format!("mcp__{SERVER_NAME}__{published}")
}

/// What the document is, when it is not a `tool-contract/1` document.
fn not_tool_contract(document: &serde_yaml_ng::Value) -> Option<String> {
    let serde_yaml_ng::Value::Mapping(mapping) = document else {
        return Some(format!(
            "its top level is {}, not a mapping",
            yaml_kind(document)
        ));
    };
    match mapping.get("format") {
        None => Some("it carries no format".to_owned()),
        Some(serde_yaml_ng::Value::String(format)) if format == tool_contract::document::FORMAT => {
            None
        }
        Some(serde_yaml_ng::Value::String(format)) => Some(format!("its format is {format:?}")),
        Some(other) => Some(format!("its format is {}, not a string", yaml_kind(other))),
    }
}

fn yaml_kind(value: &serde_yaml_ng::Value) -> &'static str {
    match value {
        serde_yaml_ng::Value::Null => "null",
        serde_yaml_ng::Value::Bool(_) => "a boolean",
        serde_yaml_ng::Value::Number(_) => "a number",
        serde_yaml_ng::Value::String(_) => "a string",
        serde_yaml_ng::Value::Sequence(_) => "a sequence",
        serde_yaml_ng::Value::Mapping(_) => "a mapping",
        serde_yaml_ng::Value::Tagged(_) => "a tagged value",
    }
}

/// Every operation as an entry, or every problem that stops one being published.
fn entries(contract: &Contract) -> Result<Vec<CatalogueEntry>, Vec<String>> {
    if contract.operations.is_empty() {
        return Err(vec![
            "the document has no operations, and a run served from it would have no tools at all"
                .to_owned(),
        ]);
    }
    let mut entries = Vec::new();
    let mut problems = Vec::new();
    for operation in &contract.operations {
        let Some(published) = operation.renderings.get(MCP_RENDERING) else {
            problems.push(format!(
                "{}: renderings: no `{MCP_RENDERING}` rendering, so it has no name a tool can \
                 carry",
                operation.name
            ));
            continue;
        };
        if let Some(problem) = unpublishable(published) {
            problems.push(format!(
                "{}: renderings: the `{MCP_RENDERING}` rendering {published:?} {problem}",
                operation.name
            ));
            continue;
        }
        let shape = match Shape::of_fields(&operation.input, &contract.types) {
            Ok(shape) => shape,
            Err(error) => {
                problems.push(format!("{}: input: {error}", operation.name));
                continue;
            }
        };
        let mut input_schema = schema(&shape);
        describe_properties(&mut input_schema, &operation.input);
        let description = match &operation.guidance {
            Some(guidance) => format!("{}\n\n{guidance}", operation.summary),
            None => operation.summary.clone(),
        };
        entries.push(CatalogueEntry {
            contract_name: operation.name.clone(),
            published: published.clone(),
            description,
            input_schema,
        });
    }
    if problems.is_empty() {
        Ok(entries)
    } else {
        Err(problems)
    }
}

/// Why a rendering cannot be published as a tool name, or `None` when it can.
fn unpublishable(name: &str) -> Option<String> {
    if name.is_empty() {
        return Some("is empty".to_owned());
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '_' || *c == '-'))
    {
        return Some(format!(
            "contains {bad:?}, and a tool name is letters, digits, `_` and `-` only"
        ));
    }
    if name.len() > MAX_PUBLISHED_NAME {
        return Some(format!(
            "is {} bytes, and a name the vendor prefixes with `mcp__{SERVER_NAME}__` may be at \
             most {MAX_PUBLISHED_NAME} bytes",
            name.len()
        ));
    }
    None
}

/// The JSON Schema of a shape (amendment a40 § 5).
fn schema(shape: &Shape) -> Value {
    match shape {
        Shape::String => json!({"type": "string"}),
        Shape::Integer => json!({"type": "integer"}),
        Shape::Boolean => json!({"type": "boolean"}),
        Shape::Bytes => json!({"type": "string", "contentEncoding": "base64"}),
        Shape::Optional(inner) => json!({"anyOf": [schema(inner), {"type": "null"}]}),
        Shape::List(item) => json!({"type": "array", "items": schema(item)}),
        Shape::Map(_, value) => json!({"type": "object", "additionalProperties": schema(value)}),
        Shape::Enum(variants) => json!({"type": "string", "enum": variants}),
        Shape::Struct(fields) => {
            let mut properties = Map::new();
            let mut required = Vec::new();
            for (name, field) in fields {
                let property = match field {
                    Shape::Optional(inner) => schema(inner),
                    other => {
                        required.push(Value::from(name.as_str()));
                        schema(other)
                    }
                };
                properties.insert(name.clone(), property);
            }
            json!({
                "type": "object",
                "properties": properties,
                "required": required,
                "additionalProperties": false,
            })
        }
    }
}

/// Each top-level property carries its field's `description`, where the field has one.
fn describe_properties(schema: &mut Value, fields: &[tool_contract::Field]) {
    let Some(properties) = schema.get_mut("properties").and_then(Value::as_object_mut) else {
        return;
    };
    for field in fields {
        if let (Some(description), Some(Value::Object(property))) =
            (&field.description, properties.get_mut(&field.name))
        {
            property.insert("description".to_owned(), Value::from(description.as_str()));
        }
    }
}
