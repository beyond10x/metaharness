//! A caller's `tool-contract/1` catalogue, read once and served flat (amendment a40).

use std::path::{Path, PathBuf};

use metaharness_tools::{
    CATALOGUE_BYTES_MAX, CATALOGUE_MALFORMED, CATALOGUE_NOT_TOOL_CONTRACT, CATALOGUE_UNREADABLE,
    CallerCatalogue, Catalogue, CatalogueEntry, CatalogueError, LocalOperations, Resolved,
    SERVER_NAME, Server, Verbs, render_scratch_copy,
};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/caller-catalogue.yaml");

fn fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/caller-catalogue.yaml")
}

fn caller() -> CallerCatalogue {
    CallerCatalogue::read(&fixture_path()).expect("the synthetic catalogue reads")
}

fn refused(text: &str) -> CatalogueError {
    CallerCatalogue::from_text(Path::new("catalogue.yaml"), text)
        .expect_err("the text is not a usable catalogue")
}

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary tree");
    std::fs::write(dir.path().join("README.md"), "hello catalogue\n").expect("a file");
    dir
}

fn server(root: &Path, catalogue: CallerCatalogue) -> Server {
    let operations =
        LocalOperations::unconfined(root, vec!["/bin/echo".to_owned()]).expect("opens");
    Server::with_catalogue(Verbs::new(Catalogue::of(operations)), catalogue)
}

fn request(id: u64, method: &str, params: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "method": method, "params": params.clone()})
}

fn call(server: &mut Server, name: &str, arguments: &Value) -> (bool, String) {
    let answer = server
        .handle(&request(
            7,
            "tools/call",
            &json!({"name": name, "arguments": arguments.clone()}),
        ))
        .expect("answered");
    let failed = answer["result"]["isError"].as_bool().expect("a verdict");
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("one text block")
        .to_owned();
    (failed, text)
}

#[test]
fn the_published_names_are_the_mcp_renderings_in_document_order() {
    let catalogue = caller();
    let published: Vec<&str> = catalogue
        .entries()
        .iter()
        .map(|entry| entry.published.as_str())
        .collect();
    assert_eq!(published, ["fs_read", "fs_stat"]);
    let contract: Vec<&str> = catalogue
        .entries()
        .iter()
        .map(|entry| entry.contract_name.as_str())
        .collect();
    assert_eq!(contract, ["fs.read", "fs.stat"]);
    assert_eq!(
        catalogue.offered_tools(),
        [
            format!("mcp__{SERVER_NAME}__fs_read"),
            format!("mcp__{SERVER_NAME}__fs_stat")
        ]
    );
}

#[test]
fn a_released_tools_contract_reads_as_a_catalogue() {
    let released = tool_contract::documents::get("fs", "v1alpha1").expect("released at 0.2.0");
    let catalogue = CallerCatalogue::from_text(Path::new("fs.yaml"), released.yaml)
        .expect("the format's own released document is a catalogue");
    let published: Vec<&str> = catalogue
        .entries()
        .iter()
        .map(|entry| entry.published.as_str())
        .collect();
    assert_eq!(published, ["fs_read", "fs_list", "fs_glob", "fs_stat"]);
}

#[test]
fn an_entry_carries_its_summary_guidance_and_a_schema_derived_from_its_fields() {
    let catalogue = caller();
    let read = &catalogue.entries()[0];
    assert_eq!(
        read.description,
        "Read a window of a file as numbered lines.\n\nPage with offset, the first line, numbered from 1."
    );
    assert_eq!(
        read.input_schema,
        json!({
            "type": "object",
            "properties": {
                "offset": {"type": "integer", "description": "The first line to read, numbered from 1."},
                "path": {"type": "string", "description": "The file, relative to the workspace root."},
            },
            "required": ["path"],
            "additionalProperties": false,
        })
    );
    assert_eq!(catalogue.entries()[1].description, "Say what one entry is.");
}

/// The fixture with one input field of every shape the grammar has on `fs.read`.
fn every_shape_text() -> String {
    FIXTURE
        .replace(
            "errors:\n",
            "types:\n  - {name: fs.Kind, kind: enum, summary: A kind., variants: [file, directory]}\n  - name: fs.Window\n    kind: struct\n    summary: A window.\n    fields: [{name: from, type: Integer}, {name: to, type: Optional<Integer>}]\nerrors:\n",
        )
        .replace(
            "        description: The first line to read, numbered from 1.\n",
            "        description: The first line to read, numbered from 1.\n      - name: blob\n        type: Bytes\n      - name: lines\n        type: List<Optional<Integer>>\n      - name: env\n        type: Map<String, Boolean>\n      - name: kind\n        type: fs.Kind\n      - name: window\n        type: Optional<fs.Window>\n",
        )
}

#[test]
fn every_shape_in_the_grammar_has_a_schema() {
    let text = every_shape_text();
    let catalogue =
        CallerCatalogue::from_text(Path::new("catalogue.yaml"), &text).expect("still a catalogue");
    let properties = &catalogue.entries()[0].input_schema["properties"];
    assert_eq!(
        properties["kind"],
        json!({"type": "string", "enum": ["directory", "file"]})
    );
    assert_eq!(
        properties["window"],
        json!({
            "type": "object",
            "properties": {
                "from": {"type": "integer"},
                "to": {"type": "integer"},
            },
            "required": ["from"],
            "additionalProperties": false,
        })
    );
    assert_eq!(
        properties["blob"],
        json!({"type": "string", "contentEncoding": "base64"})
    );
    assert_eq!(
        properties["lines"],
        json!({"type": "array", "items": {"anyOf": [{"type": "integer"}, {"type": "null"}]}})
    );
    assert_eq!(
        properties["env"],
        json!({"type": "object", "additionalProperties": {"type": "boolean"}})
    );
    assert_eq!(
        catalogue.entries()[0].input_schema["required"],
        json!(["blob", "env", "kind", "lines", "path"])
    );
}

#[test]
fn the_scratch_copy_reads_back_as_the_same_catalogue() {
    let catalogue = caller();
    let copy = serde_json::to_string_pretty(&catalogue.document()).expect("serialises");
    let again = CallerCatalogue::from_text(Path::new("catalogue.json"), &copy)
        .expect("the JSON copy is YAML the format reads");
    assert_eq!(again, catalogue);
}

/// Whether `text`, as a string in a scratch copy, reads back through the YAML reader the server
/// uses as the same string.
fn survives_the_copy(text: &str) -> bool {
    let copy = render_scratch_copy(&json!({"value": text}));
    serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&copy)
        .ok()
        .and_then(|read| {
            read.get("value")
                .and_then(|value| value.as_str().map(str::to_owned))
        })
        .is_some_and(|read| read == text)
}

/// The copy is JSON the server reads as YAML. YAML refuses some characters in the stream that JSON
/// carries raw, and folds others as line breaks inside a quoted scalar; every one of them has to
/// reach the copy escaped. A line break in a quoted scalar also takes the spaces beside it, so each
/// value stands between spaces. Asked of every Unicode scalar value, in blocks, and of each value
/// of a block that does not survive.
#[test]
fn every_unicode_scalar_value_survives_the_scratch_copy() {
    let scalars: Vec<char> = (0..=0x10_FFFF_u32).filter_map(char::from_u32).collect();
    let mut lost = Vec::new();
    for block in scalars.chunks(4096) {
        let mut text = String::from(" ");
        for scalar in block {
            text.push(*scalar);
            text.push(' ');
        }
        if survives_the_copy(&text) {
            continue;
        }
        let alone: Vec<String> = block
            .iter()
            .filter(|scalar| !survives_the_copy(&format!(" {scalar} ")))
            .map(|scalar| format!("U+{:04X}", u32::from(*scalar)))
            .collect();
        if alone.is_empty() {
            lost.push(format!(
                "the block from U+{:04X}, though each value alone survives",
                u32::from(block[0])
            ));
        }
        lost.extend(alone);
    }
    assert!(
        lost.is_empty(),
        "{} scalar values do not survive the copy: {}",
        lost.len(),
        lost.join(" ")
    );
}

/// The YAML reader holds a flow-mapping key to 1024 characters, and the copy writes every mapping
/// in flow style. A caller's block document can carry a longer key with `? `, which the parent
/// reads; the copy of it would not read back, so the parent refuses it by name.
#[test]
fn a_document_whose_copy_would_not_read_back_is_refused_by_the_parent() {
    let key = "k".repeat(1100);
    let text = FIXTURE.replace(
        "    renderings: {mcp: fs_stat}\n",
        &format!("    renderings:\n      mcp: fs_stat\n      ? {key}\n      : long\n"),
    );
    assert_ne!(text, FIXTURE, "the fixture line was found");
    match CallerCatalogue::from_text(Path::new("catalogue.yaml"), &text) {
        Ok(accepted) => {
            let copy = serde_json::to_string_pretty(&accepted.document()).expect("renders");
            let read_back = CallerCatalogue::from_text(Path::new("catalogue.json"), &copy);
            panic!(
                "the parent accepted it, and its copy reads back as: {:?}",
                read_back.map(|again| again == accepted)
            );
        }
        Err(error) => {
            assert_eq!(error.code(), CATALOGUE_MALFORMED, "{error}");
            assert!(error.to_string().contains("scratch copy"), "{error}");
        }
    }
}

/// The copy is read under the same cap as the caller's file. A file under the cap whose copy is
/// over it (each `\x7F` is four bytes in the file and six in the copy) is refused by the parent,
/// naming the cap, rather than handed to a server that would refuse it.
#[test]
fn a_file_under_the_cap_whose_copy_is_over_it_is_refused_naming_the_cap() {
    let summary = "\\x7F".repeat(786_432);
    let text = FIXTURE.replace(
        "    summary: Read a window of a file as numbered lines.",
        &format!("    summary: \"{summary}\""),
    );
    assert_ne!(text, FIXTURE, "the fixture line was found");
    assert!(text.len() < usize::try_from(CATALOGUE_BYTES_MAX).unwrap());
    let dir = tree();
    let path = dir.path().join("wide.yaml");
    std::fs::write(&path, &text).expect("a file");
    let error = CallerCatalogue::read(&path).expect_err("its copy is past the cap");
    assert_eq!(error.code(), CATALOGUE_MALFORMED, "{error}");
    assert!(
        error.to_string().contains(&CATALOGUE_BYTES_MAX.to_string()),
        "{error}"
    );
}

/// A catalogue is read to 4 MiB and no further: a file of exactly that many bytes is read, and one
/// byte more is refused as unreadable, naming the cap.
#[test]
fn a_file_past_the_size_cap_is_refused_naming_the_cap_and_one_at_it_is_read() {
    let cap = usize::try_from(CATALOGUE_BYTES_MAX).unwrap();
    let padded = |bytes: usize| {
        let mut text = FIXTURE.to_owned();
        let room = bytes - text.len() - 2;
        text.push('#');
        text.push_str(&"x".repeat(room));
        text.push('\n');
        assert_eq!(text.len(), bytes);
        text
    };
    let dir = tree();

    let at = dir.path().join("at.yaml");
    std::fs::write(&at, padded(cap)).expect("a file");
    let read = CallerCatalogue::read(&at).expect("a file of exactly the cap is read");
    assert_eq!(read, caller());

    let past = dir.path().join("past.yaml");
    std::fs::write(&past, padded(cap + 1)).expect("a file");
    let error = CallerCatalogue::read(&past).expect_err("one byte past the cap");
    assert_eq!(error.code(), CATALOGUE_UNREADABLE, "{error}");
    assert!(error.to_string().contains("4194304 bytes"), "{error}");
    assert!(error.to_string().contains("past.yaml"), "{error}");
}

/// Only a regular file is read. Asked before anything is opened, so a pipe nobody writes to or a
/// device that never ends is refused at once, not read until memory runs out.
#[test]
fn a_path_that_is_not_a_regular_file_is_refused_before_it_is_read() {
    let dir = tree();
    let mut refused_paths = vec![dir.path().to_path_buf()];
    #[cfg(unix)]
    {
        refused_paths.push(PathBuf::from("/dev/zero"));
        let fifo = dir.path().join("catalogue.fifo");
        let made = std::process::Command::new("mkfifo")
            .arg(&fifo)
            .status()
            .expect("mkfifo runs");
        assert!(made.success(), "mkfifo made the pipe");
        refused_paths.push(fifo);
    }
    for path in refused_paths {
        let error = CallerCatalogue::read(&path).expect_err("not a regular file");
        assert_eq!(
            error.code(),
            CATALOGUE_UNREADABLE,
            "{}: {error}",
            path.display()
        );
        assert!(
            error.to_string().contains("not a regular file"),
            "{}: {error}",
            path.display()
        );
    }
}

#[test]
fn an_unreadable_file_is_refused_by_name() {
    let missing = tree().path().join("absent.yaml");
    let error = CallerCatalogue::read(&missing).expect_err("there is no file");
    assert_eq!(error.code(), CATALOGUE_UNREADABLE);
    assert!(
        error.to_string().starts_with("catalogue-unreadable: "),
        "{error}"
    );
    assert!(error.to_string().contains("absent.yaml"), "{error}");
}

#[test]
fn a_document_of_another_format_is_refused_as_not_tool_contract() {
    for text in [
        "format: tool-contract/2\ntoolset: fs\n",
        "format: ess/1\nentities: []\n",
        "toolset: fs\nversion: v1\n",
        "- format: tool-contract/1\n",
        "just a sentence\n",
    ] {
        let error = refused(text);
        assert_eq!(
            error.code(),
            CATALOGUE_NOT_TOOL_CONTRACT,
            "{text:?}: {error}"
        );
        assert!(
            error
                .to_string()
                .starts_with("catalogue-not-tool-contract: "),
            "{error}"
        );
    }
}

#[test]
fn text_that_is_not_yaml_or_breaks_the_grammar_is_refused_as_malformed() {
    let broken_yaml = "format: tool-contract/1\noperations: [unclosed\n";
    let unknown_field = FIXTURE.replace("toolset: fs\n", "toolset: fs\nowner: someone\n");
    for text in [broken_yaml, unknown_field.as_str()] {
        let error = refused(text);
        assert_eq!(error.code(), CATALOGUE_MALFORMED, "{error}");
        assert!(
            error.to_string().starts_with("catalogue-malformed: "),
            "{error}"
        );
    }
}

#[test]
fn every_violation_the_validator_reports_is_named() {
    let text = FIXTURE
        .replace("capability: workspace.read\n", "capability: Workspace\n")
        .replace(
            "timeout_ms: {default: 10000, max: 60000}",
            "timeout_ms: {default: 10000}",
        );
    let error = refused(&text);
    assert_eq!(error.code(), CATALOGUE_MALFORMED);
    let words = error.to_string();
    assert!(words.contains("fs.read: capability: "), "{words}");
    assert!(words.contains("fs.stat: capability: "), "{words}");
    assert!(
        words.contains("fs.read: bounds: timeout_ms has no max"),
        "{words}"
    );
}

#[test]
fn a_catalogue_with_no_operations_is_malformed() {
    let text =
        "format: tool-contract/1\ntoolset: fs\nversion: v1\nsummary: nothing\noperations: []\n";
    let error = refused(text);
    assert_eq!(error.code(), CATALOGUE_MALFORMED);
    assert!(error.to_string().contains("no operations"), "{error}");
}

#[test]
fn an_operation_without_a_usable_mcp_rendering_is_malformed() {
    for (from, to, says) in [
        (
            "renderings: {mcp: fs_stat}",
            "renderings: {claude: Stat}",
            "no `mcp` rendering",
        ),
        (
            "renderings: {mcp: fs_stat}",
            "renderings: {mcp: fs.stat}",
            "fs.stat",
        ),
        (
            "renderings: {mcp: fs_stat}",
            "renderings: {mcp: fs_stat_with_a_name_that_runs_past_the_vendor_limit}",
            "46 bytes",
        ),
    ] {
        let error = refused(&FIXTURE.replace(from, to));
        assert_eq!(error.code(), CATALOGUE_MALFORMED, "{to}: {error}");
        assert!(error.to_string().contains(says), "{to}: {error}");
    }
}

#[test]
fn a_published_entry_resolves_through_its_contract_name_and_nothing_else_does() {
    let catalogue = caller();
    let read = format!("mcp__{SERVER_NAME}__fs_read");
    assert_eq!(
        catalogue.resolve(&read),
        Resolved::Operations(vec!["file.read".to_owned()])
    );
    assert_eq!(
        catalogue.subjects_of(&read, &json!({"path": "src/lib.rs"})),
        ["file:src/lib.rs"]
    );

    let stat = format!("mcp__{SERVER_NAME}__fs_stat");
    assert_eq!(catalogue.resolve(&stat), Resolved::Unknown);
    assert_eq!(
        catalogue.subjects_of(&stat, &json!({"path": "src/lib.rs"})),
        [] as [String; 0]
    );

    for outside in [
        format!("mcp__{SERVER_NAME}__tool_invoke"),
        format!("mcp__{SERVER_NAME}__file_read"),
        "mcp__another__fs_read".to_owned(),
        "fs_read".to_owned(),
        "Read".to_owned(),
    ] {
        assert_eq!(catalogue.resolve(&outside), Resolved::Unknown, "{outside}");
        assert!(
            catalogue
                .subjects_of(&outside, &json!({"name": "file_read", "path": "x"}))
                .is_empty(),
            "{outside}"
        );
    }
}

#[test]
fn the_server_lists_the_catalogue_and_nothing_else() {
    let dir = tree();
    let mut server = server(dir.path(), caller());
    let answer = server
        .handle(&request(1, "tools/list", &json!({})))
        .expect("answered");
    let tools = answer["result"]["tools"].as_array().expect("a list");
    let names: Vec<&str> = tools
        .iter()
        .filter_map(|tool| tool["name"].as_str())
        .collect();
    assert_eq!(names, ["fs_read", "fs_stat"]);
    assert_eq!(tools[0]["inputSchema"], caller().entries()[0].input_schema);
    assert_eq!(tools[0]["description"], caller().entries()[0].description);
}

#[test]
fn a_mapped_entry_is_performed_as_the_entry_harness_maps_it_to() {
    let dir = tree();
    let mut server = server(dir.path(), caller());
    let (failed, text) = call(&mut server, "fs_read", &json!({"path": "README.md"}));
    assert!(!failed, "{text}");
    assert!(text.contains("hello catalogue"), "{text}");
}

/// A released document as the run's catalogue.
fn released(family: &str) -> CallerCatalogue {
    let document = tool_contract::documents::get(family, "v1alpha1").expect("released at 0.2.0");
    CallerCatalogue::from_text(Path::new("released.yaml"), document.yaml).expect("a catalogue")
}

/// `code.write` declares `expected_sha256`, so its call passes the argument check and reaches the
/// digest guard, which refuses it and leaves the file as it was.
#[test]
fn the_digest_guard_refusal_holds_on_the_catalogue_road() {
    let dir = tree();
    let mut server = server(dir.path(), released("code-editor"));
    let (failed, text) = call(
        &mut server,
        "code_write",
        &json!({"path": "README.md", "text": "replaced\n", "expected_sha256": "00"}),
    );
    assert!(failed, "{text}");
    assert!(text.contains("`expected_sha256`"), "{text}");
    assert!(text.contains("precondition-failed"), "{text}");
    assert_eq!(
        std::fs::read_to_string(dir.path().join("README.md")).expect("the file"),
        "hello catalogue\n"
    );
}

/// The published input schema closes the object (`additionalProperties: false`). A field the
/// entry does not declare is refused by name, rather than dropped on the way to an entry that
/// would then list the workspace root.
#[test]
fn a_call_with_a_field_its_entry_does_not_declare_is_refused_and_nothing_is_listed() {
    let dir = tree();
    std::fs::create_dir(dir.path().join("x")).expect("a directory");
    let mut server = server(dir.path(), released("fs"));
    let (failed, text) = call(&mut server, "fs_list", &json!({"dir": "x"}));
    assert!(failed, "{text}");
    assert!(text.contains("`fs_list`"), "{text}");
    assert!(text.contains("`dir`"), "{text}");
    assert!(text.contains("`path`"), "{text}");
    assert!(text.contains("nothing was done"), "{text}");
    assert!(!text.contains("README.md"), "{text}");

    let (failed, text) = call(&mut server, "fs_list", &json!({"path": "."}));
    assert!(!failed, "{text}");
    assert!(text.contains("README.md"), "{text}");
    let (failed, text) = call(&mut server, "fs_list", &json!({}));
    assert!(!failed, "{text}");
    assert!(text.contains("README.md"), "{text}");
}

/// A required field (one whose type is not `Optional<…>`) that the call leaves out, or gives as
/// null, is refused by name; so are arguments that are not an object.
#[test]
fn a_call_lacking_a_required_field_or_an_object_is_refused_and_nothing_is_read() {
    let dir = tree();
    let mut server = server(dir.path(), caller());
    for (arguments, named) in [
        (json!({}), "`path`"),
        (json!({"path": null}), "`path`"),
        (json!({"offset": 1}), "`path`"),
        (json!("README.md"), "a string"),
        (json!(["README.md"]), "an array"),
        (Value::Null, "null"),
    ] {
        let (failed, text) = call(&mut server, "fs_read", &arguments);
        assert!(failed, "{arguments}: {text}");
        assert!(text.contains("`fs_read`"), "{arguments}: {text}");
        assert!(text.contains(named), "{arguments}: {text}");
        assert!(text.contains("nothing was done"), "{arguments}: {text}");
        assert!(!text.contains("hello catalogue"), "{arguments}: {text}");
    }
    let answer = server
        .handle(&request(8, "tools/call", &json!({"name": "fs_read"})))
        .expect("answered");
    assert_eq!(answer["result"]["isError"], true, "{answer}");
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("text");
    assert!(text.contains("`path`"), "{text}");

    let (failed, text) = call(
        &mut server,
        "fs_read",
        &json!({"path": "README.md", "offset": 1}),
    );
    assert!(!failed, "{text}");
    assert!(text.contains("hello catalogue"), "{text}");
}

/// `fs_read.offset` is declared `integer`. A string, a boolean, or a number written with a fraction
/// or an exponent is refused, naming `offset` and `integer`, and nothing is read; each was read from
/// line 1. `2` still reads from line 2, and null on the optional field reads as absent (a45, D2, D3).
#[test]
fn an_offset_that_is_not_an_integer_is_refused_naming_offset_and_integer() {
    let dir = tree();
    std::fs::write(dir.path().join("lines.txt"), "alpha\nbeta\ngamma\n").expect("a file");
    let mut server = server(dir.path(), caller());
    let mut answered = Vec::new();
    for offset in ["\"second\"", "1.5", "false", "1.0", "\"2\"", "1e0"] {
        let arguments: Value =
            serde_json::from_str(&format!(r#"{{"path": "lines.txt", "offset": {offset}}}"#))
                .expect("JSON");
        let (failed, text) = call(&mut server, "fs_read", &arguments);
        let refused = failed
            && text.contains("`fs_read`")
            && text.contains("`offset`")
            && text.contains("`integer`")
            && text.contains("nothing was done")
            && !text.contains("alpha");
        if !refused {
            answered.push(format!("offset {offset}: isError {failed}: {text}"));
        }
    }
    assert!(
        answered.is_empty(),
        "not refused naming `offset` and `integer`:\n{}",
        answered.join("\n")
    );

    let (failed, text) = call(
        &mut server,
        "fs_read",
        &json!({"path": "lines.txt", "offset": 2}),
    );
    assert!(!failed, "{text}");
    assert!(text.contains("beta"), "{text}");
    assert!(!text.contains("alpha"), "{text}");

    let (failed, text) = call(
        &mut server,
        "fs_read",
        &json!({"path": "lines.txt", "offset": null}),
    );
    assert!(!failed, "{text}");
    assert!(text.contains("alpha"), "{text}");
}

/// One step from a value to a value the published schema declares inside it.
#[derive(Debug, Clone)]
enum Step {
    /// A field of an object with `properties`.
    Field(String),
    /// The first item of an array.
    Item,
    /// The value under the key `key` of an object with `additionalProperties`.
    Entry,
}

/// How a refusal names the value at `steps`: `window.from`, `lines[0]`, `env["key"]`.
fn shown(steps: &[Step]) -> String {
    let mut text = String::new();
    for step in steps {
        match step {
            Step::Field(name) => {
                if !text.is_empty() {
                    text.push('.');
                }
                text.push_str(name);
            }
            Step::Item => text.push_str("[0]"),
            Step::Entry => text.push_str("[\"key\"]"),
        }
    }
    text
}

/// The JSON pointer of the value at `steps` in a value [`admitted`] built.
fn pointer(steps: &[Step]) -> String {
    steps
        .iter()
        .map(|step| match step {
            Step::Field(name) => format!("/{name}"),
            Step::Item => "/0".to_owned(),
            Step::Entry => "/key".to_owned(),
        })
        .collect()
}

/// The schema an `anyOf` with `null` (an `Optional` that is not a field) stands for, and the
/// schema itself otherwise.
fn inner(schema: &Value) -> &Value {
    schema["anyOf"].as_array().map_or(schema, |choices| {
        choices
            .iter()
            .find(|choice| choice["type"] != "null")
            .expect("a choice besides null")
    })
}

/// A value the published schema admits, with every declared field given, the optional ones too.
fn admitted(schema: &Value) -> Value {
    let schema = inner(schema);
    match schema["type"].as_str() {
        Some("string") => schema["enum"]
            .get(0)
            .cloned()
            .unwrap_or_else(|| json!("text")),
        Some("integer") => json!(3),
        Some("boolean") => json!(true),
        Some("array") => json!([admitted(&schema["items"])]),
        Some("object") => match schema["properties"].as_object() {
            Some(properties) => properties
                .iter()
                .map(|(name, property)| (name.clone(), admitted(property)))
                .collect(),
            None => json!({"key": admitted(&schema["additionalProperties"])}),
        },
        other => panic!("a published schema type the walk does not know: {other:?}: {schema}"),
    }
}

/// One declared value the walk sends wrong kinds to.
#[derive(Debug)]
struct Declared {
    steps: Vec<Step>,
    schema: Value,
    /// A field of an object that is not in its `required`: null reads as absent (D2).
    optional_field: bool,
}

/// Every value `schema` declares at and below `steps`.
fn declared(schema: &Value, steps: &[Step], optional_field: bool, into: &mut Vec<Declared>) {
    into.push(Declared {
        steps: steps.to_vec(),
        schema: schema.clone(),
        optional_field,
    });
    let schema = inner(schema);
    let deeper = |step: Step| {
        let mut steps = steps.to_vec();
        steps.push(step);
        steps
    };
    match schema["type"].as_str() {
        Some("array") => declared(&schema["items"], &deeper(Step::Item), false, into),
        Some("object") => match schema["properties"].as_object() {
            Some(properties) => {
                let required = schema["required"].as_array().expect("a required list");
                for (name, property) in properties {
                    let optional = !required.contains(&Value::from(name.as_str()));
                    declared(property, &deeper(Step::Field(name.clone())), optional, into);
                }
            }
            None => declared(
                &schema["additionalProperties"],
                &deeper(Step::Entry),
                false,
                into,
            ),
        },
        _ => {}
    }
}

/// Whether `value` is of the JSON kind the schema declares; a string must also be one of an
/// enum's values. Null is the caller's to decide.
fn of_the_declared_kind(schema: &Value, value: &Value) -> bool {
    let schema = inner(schema);
    match schema["type"].as_str() {
        Some("string") => value.as_str().is_some_and(|text| {
            schema["enum"]
                .as_array()
                .is_none_or(|values| values.contains(&Value::from(text)))
        }),
        Some("integer") => value.is_i64() || value.is_u64(),
        Some("boolean") => value.is_boolean(),
        Some("array") => value.is_array(),
        Some("object") => value.is_object(),
        other => panic!("a published schema type the walk does not know: {other:?}"),
    }
}

/// Every catalogue the walk reads: the synthetic fixture, the fixture with every shape of the
/// grammar, and every document `b10x-tool-contract` releases.
fn every_catalogue() -> Vec<(String, CallerCatalogue)> {
    let mut catalogues = vec![
        ("the fixture".to_owned(), caller()),
        (
            "every shape".to_owned(),
            CallerCatalogue::from_text(Path::new("every-shape.yaml"), &every_shape_text())
                .expect("a catalogue"),
        ),
    ];
    for document in tool_contract::documents::all() {
        catalogues.push((
            format!("{}/{}", document.toolset, document.version),
            CallerCatalogue::from_text(Path::new("released.yaml"), document.yaml)
                .expect("a released document is a catalogue"),
        ));
    }
    catalogues
}

/// Every value an entry's input schema declares: each top-level field, and everything inside one.
fn declared_values(schema: &Value) -> Vec<Declared> {
    let required = schema["required"].as_array().expect("a required list");
    let mut values = Vec::new();
    for (name, property) in schema["properties"].as_object().expect("properties") {
        let optional = !required.contains(&Value::from(name.as_str()));
        declared(
            property,
            &[Step::Field(name.clone())],
            optional,
            &mut values,
        );
    }
    values
}

/// The words the walk records a declared value's shape under, for the check that it reached
/// every shape the published schemas use.
fn shapes(value: &Declared) -> Vec<String> {
    let mut words = vec![
        inner(&value.schema)["type"]
            .as_str()
            .expect("a type")
            .to_owned(),
    ];
    if value.schema.get("anyOf").is_some() {
        words.push("anyOf".to_owned());
    }
    if inner(&value.schema).get("enum").is_some() {
        words.push("enum".to_owned());
    }
    words.push(
        match value.steps.as_slice() {
            [_] => "an argument",
            [.., Step::Field(_)] => "a field inside one",
            [.., Step::Item] => "a list item",
            [.., Step::Entry] => "a map value",
            [] => unreachable!("every value is under an argument"),
        }
        .to_owned(),
    );
    words
}

/// Each value the walk sends, written as JSON, and what a refusal must say it is; `-0` is added by
/// [`minus_zero`].
const KINDS: [(&str, &str); 9] = [
    ("null", "null"),
    ("true", "a boolean"),
    ("7", "a number"),
    (
        "1.0",
        "a number written with a fraction or an exponent, or outside 64 bits",
    ),
    (
        "1.5",
        "a number written with a fraction or an exponent, or outside 64 bits",
    ),
    (
        "18446744073709551616",
        "a number written with a fraction or an exponent, or outside 64 bits",
    ),
    ("\"text\"", "a string"),
    ("[]", "an array"),
    ("{}", "an object"),
];

/// JSON `-0` as this build holds it, and what a refusal must say it is. `serde_json` without
/// `arbitrary_precision` (this crate built alone) holds it as the float `-0.0`, the one number
/// written as an integer inside 64 bits that is not held as an `i64` or a `u64`, so it is refused
/// and named for what it is and not as a fraction (a45 § 3, D23). With that feature (the workspace
/// and the `metaharness` binary) it is held as the integer 0, so it is a number like `7` and is
/// admitted where an integer is declared (a45 § 2).
fn minus_zero() -> (Value, &'static str) {
    let value: Value = serde_json::from_str("-0").expect("JSON");
    if value.is_i64() {
        assert_eq!(value, json!(0), "`-0` held as an integer is 0");
        (value, "a number")
    } else {
        assert_eq!(
            value.as_f64().map(f64::to_bits),
            Some((-0.0_f64).to_bits()),
            "`-0` not held as an integer is the float -0.0"
        );
        (
            value,
            "negative zero, a number not held as a 64-bit integer",
        )
    }
}

/// Whether `problem` says the value sent at `value` is `said`: null on a required field as the
/// call giving it as null, a string outside an enum as a string that is not one of its values.
fn describes(problem: &str, value: &Declared, said: &str) -> bool {
    if said == "null" && matches!(value.steps.last(), Some(Step::Field(_))) {
        return problem.contains("is required and the call gives it as null,");
    }
    problem.contains(&format!("is {said},"))
        || problem.contains(&format!("is {said} that is not one of its values,"))
}

/// The declared type a refusal must name for `value`, as the schema's `type` in backticks after
/// the words that introduce it.
fn its_declared_type(value: &Declared) -> String {
    format!(
        "its declared type is `{}`",
        inner(&value.schema)["type"].as_str().expect("a type")
    )
}

/// `whole` with the value at `value` replaced by `kind`, and the one problem the entry names it
/// with, which must say it is `said` and name its declared type; `None` when `kind` is of the
/// declared kind, or is null where D2 reads it as absent (which is asserted to raise nothing).
fn refusal_of(
    entry: &CatalogueEntry,
    whole: &Value,
    value: &Declared,
    (kind, said): (&Value, &str),
    context: &str,
) -> Option<(Value, String)> {
    if !kind.is_null() && of_the_declared_kind(&value.schema, kind) {
        return None;
    }
    let mut arguments = whole.clone();
    *arguments
        .pointer_mut(&pointer(&value.steps))
        .expect("the walk's value is in the arguments") = kind.clone();
    let problems = entry.argument_problems(&arguments);
    if kind.is_null() && (value.optional_field || value.schema.get("anyOf").is_some()) {
        assert_eq!(problems, Vec::<String>::new(), "{context}");
        return None;
    }
    assert_eq!(problems.len(), 1, "{context}: {problems:?}");
    let path = shown(&value.steps);
    assert!(
        problems[0].contains(&format!("`{path}`")),
        "{context}: {problems:?}"
    );
    assert!(
        describes(&problems[0], value, said),
        "{context}: {problems:?}"
    );
    assert!(
        problems[0].contains(&its_declared_type(value)),
        "{context}: {problems:?}"
    );
    let problem = problems.into_iter().next().expect("one problem");
    Some((arguments, problem))
}

/// `whole` with the field at `value` left out, and the one problem the entry names it with, which
/// must name the field and its declared type; `None` when `value` is not a required field.
fn left_out_refusal(
    entry: &CatalogueEntry,
    whole: &Value,
    value: &Declared,
    context: &str,
) -> Option<(Value, String)> {
    let (Some(Step::Field(name)), false) = (value.steps.last(), value.optional_field) else {
        return None;
    };
    let mut arguments = whole.clone();
    arguments
        .pointer_mut(&pointer(&value.steps[..value.steps.len() - 1]))
        .and_then(Value::as_object_mut)
        .expect("the field's object is in the arguments")
        .remove(name)
        .expect("the field was given");
    let problems = entry.argument_problems(&arguments);
    assert_eq!(problems.len(), 1, "{context}: {problems:?}");
    let path = shown(&value.steps);
    assert!(
        problems[0].contains(&format!(
            "`{path}` is required and the call leaves it out, and {}",
            its_declared_type(value)
        )),
        "{context}: {problems:?}"
    );
    let problem = problems.into_iter().next().expect("one problem");
    Some((arguments, problem))
}

/// Every declared argument of every entry of every catalogue, and every value declared inside one
/// (a struct's field, a list's item, a map's value), is sent a value of each JSON kind it is not.
/// Each is refused with one problem naming the argument, what was sent and its declared type,
/// except null where D2 reads it as absent. Every required field is also left out, and refused
/// naming it and its declared type. Every entry is asked directly, since an entry nothing performs
/// is refused before its arguments are read; every entry that is performed is also called through
/// the server, which answers with the same refusal and performs nothing.
#[test]
fn every_declared_argument_of_every_catalogue_entry_refuses_a_value_of_each_wrong_kind() {
    let kinds: Vec<(Value, &str)> = KINDS
        .iter()
        .map(|(text, said)| (serde_json::from_str(text).expect("JSON"), *said))
        .chain([minus_zero()])
        .collect();
    let mut shapes_seen = std::collections::BTreeSet::new();
    let mut refused_count = 0_usize;
    let mut left_out_count = 0_usize;
    let mut served_count = 0_usize;
    for (label, catalogue) in every_catalogue() {
        let dir = tree();
        let mut server = server(dir.path(), catalogue.clone());
        for entry in catalogue.entries() {
            let whole = admitted(&entry.input_schema);
            assert_eq!(
                entry.argument_problems(&whole),
                Vec::<String>::new(),
                "{label} {}: every field admitted: {whole}",
                entry.published
            );
            let performed = catalogue.resolve(&format!("mcp__{SERVER_NAME}__{}", entry.published))
                != Resolved::Unknown;
            for value in declared_values(&entry.input_schema) {
                shapes_seen.extend(shapes(&value));
                let path = shown(&value.steps);
                let mut refusals = Vec::new();
                for (kind, said) in &kinds {
                    let context = format!("{label} {} `{path}` = {kind}", entry.published);
                    if let Some(refusal) =
                        refusal_of(entry, &whole, &value, (kind, *said), &context)
                    {
                        refusals.push((context, refusal));
                    }
                }
                let context = format!("{label} {} `{path}` left out", entry.published);
                if let Some(refusal) = left_out_refusal(entry, &whole, &value, &context) {
                    left_out_count += 1;
                    refusals.push((context, refusal));
                }
                for (context, (arguments, problem)) in refusals {
                    refused_count += 1;
                    if performed {
                        let (failed, text) = call(&mut server, &entry.published, &arguments);
                        assert!(failed, "{context}: {text}");
                        assert!(
                            text.contains("its input schema does not admit"),
                            "{context}: {text}"
                        );
                        assert!(text.contains(&problem), "{context}: {text}");
                        served_count += 1;
                    }
                }
            }
        }
    }
    let shapes_seen: Vec<&str> = shapes_seen.iter().map(String::as_str).collect();
    assert_eq!(
        shapes_seen,
        [
            "a field inside one",
            "a list item",
            "a map value",
            "an argument",
            "anyOf",
            "array",
            "boolean",
            "enum",
            "integer",
            "object",
            "string"
        ],
        "the walk reaches every shape the published schemas use"
    );
    assert!(
        refused_count > 0 && left_out_count > 0 && served_count > 0,
        "{refused_count} {left_out_count} {served_count}"
    );
}

#[test]
fn an_entry_nothing_performs_answers_is_error_and_does_nothing() {
    let dir = tree();
    let mut server = server(dir.path(), caller());
    let (failed, text) = call(&mut server, "fs_stat", &json!({"path": "README.md"}));
    assert!(failed);
    assert!(text.contains("`fs_stat`"), "{text}");
    assert!(text.contains("`fs.stat`"), "{text}");
    assert!(text.contains("nothing was done"), "{text}");
}

#[test]
fn a_name_outside_the_catalogue_is_not_performed_and_the_answer_names_the_catalogue() {
    let dir = tree();
    for outside in ["tool_invoke", "tool_search", "file_read", "Read", "fs.read"] {
        let mut server = server(dir.path(), caller());
        let arguments =
            json!({"name": "file_read", "arguments": {"path": "README.md"}, "path": "README.md"});
        let (failed, text) = call(&mut server, outside, &arguments);
        assert!(failed, "{outside}: {text}");
        assert!(!text.contains("hello catalogue"), "{outside}: {text}");
        assert!(text.contains("`fs_read`, `fs_stat`"), "{outside}: {text}");
        assert!(text.contains("nothing was done"), "{outside}: {text}");
    }
}
