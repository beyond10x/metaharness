//! A caller's `tool-contract/1` catalogue, read once and served flat (amendment a40).

use std::path::{Path, PathBuf};

use metaharness_tools::{
    CATALOGUE_BYTES_MAX, CATALOGUE_MALFORMED, CATALOGUE_NOT_TOOL_CONTRACT, CATALOGUE_UNREADABLE,
    CallerCatalogue, Catalogue, CatalogueError, LocalOperations, Resolved, SERVER_NAME, Server,
    Verbs, render_scratch_copy,
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

#[test]
fn every_shape_in_the_grammar_has_a_schema() {
    let text = FIXTURE
        .replace(
            "errors:\n",
            "types:\n  - {name: fs.Kind, kind: enum, summary: A kind., variants: [file, directory]}\n  - name: fs.Window\n    kind: struct\n    summary: A window.\n    fields: [{name: from, type: Integer}, {name: to, type: Optional<Integer>}]\nerrors:\n",
        )
        .replace(
            "        description: The first line to read, numbered from 1.\n",
            "        description: The first line to read, numbered from 1.\n      - name: blob\n        type: Bytes\n      - name: lines\n        type: List<Optional<Integer>>\n      - name: env\n        type: Map<String, Boolean>\n      - name: kind\n        type: fs.Kind\n      - name: window\n        type: Optional<fs.Window>\n",
        );
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
