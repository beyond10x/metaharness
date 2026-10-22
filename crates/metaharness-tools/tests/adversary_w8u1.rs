//! Adversary cases for `story:mcp-arguments-checked-against-declared-types` (wave 8, unit 1).
//!
//! Each case drives the catalogue road from amendment a45 of the protocol design: decisions D1–D3
//! and the sentences of its sections 2 to 4, read as the specification of the code the same unit
//! wrote.

use std::path::Path;

use metaharness_tools::{CallerCatalogue, Catalogue, LocalOperations, Server, Verbs};
use serde_json::{Value, json};

const FIXTURE: &str = include_str!("fixtures/caller-catalogue.yaml");

fn tree() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("a temporary tree");
    std::fs::write(dir.path().join("README.md"), "hello catalogue\n").expect("a file");
    dir
}

/// A server that may write: the program set is declared, so `code.write` and `code.edit` run.
fn server(root: &Path, catalogue: CallerCatalogue) -> Server {
    let operations =
        LocalOperations::unconfined(root, vec!["/bin/echo".to_owned()]).expect("opens");
    Server::with_catalogue(Verbs::new(Catalogue::of(operations)), catalogue)
}

fn call(server: &mut Server, name: &str, arguments: &Value) -> (bool, String) {
    let answer = server
        .handle(&json!({
            "jsonrpc": "2.0",
            "id": 9,
            "method": "tools/call",
            "params": {"name": name, "arguments": arguments.clone()},
        }))
        .expect("answered");
    let failed = answer["result"]["isError"]
        .as_bool()
        .unwrap_or_else(|| panic!("a tool result, not a JSON-RPC error: {answer}"));
    let text = answer["result"]["content"][0]["text"]
        .as_str()
        .expect("one text block")
        .to_owned();
    (failed, text)
}

fn released(family: &str) -> CallerCatalogue {
    let document = tool_contract::documents::get(family, "v1alpha1").expect("released at 0.4.0");
    CallerCatalogue::from_text(Path::new("released.yaml"), document.yaml).expect("a catalogue")
}

/// The synthetic fixture with `fs.read` given one input field of every shape the grammar has. A
/// caller writes its own document, so these fields reach a performed entry.
fn every_shape() -> CallerCatalogue {
    let text = FIXTURE
        .replace(
            "errors:\n",
            "types:\n  - {name: fs.Kind, kind: enum, summary: A kind., variants: [file, directory]}\n  - name: fs.Window\n    kind: struct\n    summary: A window.\n    fields: [{name: from, type: Integer}, {name: to, type: Optional<Integer>}]\nerrors:\n",
        )
        .replace(
            "        description: The first line to read, numbered from 1.\n",
            "        description: The first line to read, numbered from 1.\n      - name: blob\n        type: Bytes\n      - name: lines\n        type: List<Optional<Integer>>\n      - name: env\n        type: Map<String, Boolean>\n      - name: kind\n        type: fs.Kind\n      - name: window\n        type: Optional<fs.Window>\n",
        );
    CallerCatalogue::from_text(Path::new("every-shape.yaml"), &text).expect("a catalogue")
}

/// `base` with the keys of `overrides` set over it.
fn merged(mut base: Value, overrides: Value) -> Value {
    let Value::Object(overrides) = overrides else {
        panic!("an object");
    };
    base.as_object_mut().expect("an object").extend(overrides);
    base
}

/// A call to the every-shape `fs_read` giving each required field a value it admits, with
/// `overrides` set over them.
fn every_shape_call(overrides: Value) -> Value {
    merged(
        json!({"path": "README.md", "blob": "", "lines": [], "env": {}, "kind": "file"}),
        overrides,
    )
}

/// The same for the `fs_read` of [`an_optional_list_item_or_map_value_is_walked_into`].
fn optional_inner_call(overrides: Value) -> Value {
    merged(
        json!({"path": "README.md", "windows": [], "kinds": {}}),
        overrides,
    )
}

fn problems_of(catalogue: &CallerCatalogue, published: &str, arguments: &Value) -> Vec<String> {
    catalogue
        .entries()
        .iter()
        .find(|entry| entry.published == published)
        .expect("the entry")
        .argument_problems(arguments)
}

// ---------------------------------------------------------------------------------------------
// Red against the unit's tree: the code disagrees with a45.
// ---------------------------------------------------------------------------------------------

/// D2 and a45 § 2: "On a struct field that is not required, null reads as absent and is
/// accepted". `expected_sha256` is optional on `code.write` and `code.edit`, so a call giving it
/// as null is the call without it, which is performed. The a32 § 5 digest guard after the check
/// reads the key's presence, not its value, so the call is refused as one "called with
/// `expected_sha256`" and nothing is written.
#[test]
fn a_null_expected_sha256_reads_as_absent_and_the_write_is_performed() {
    let dir = tree();
    let readme = dir.path().join("README.md");
    let mut server = server(dir.path(), released("code-editor"));

    // Control: the same write without the field is performed on this server.
    let (failed, text) = call(
        &mut server,
        "code_write",
        &json!({"path": "README.md", "text": "control\n"}),
    );
    assert!(!failed, "the control write: {text}");
    assert_eq!(
        std::fs::read_to_string(&readme).expect("the file"),
        "control\n"
    );

    let mut answered = Vec::new();
    let (failed, text) = call(
        &mut server,
        "code_write",
        &json!({"path": "README.md", "text": "replaced\n", "expected_sha256": null}),
    );
    let written = std::fs::read_to_string(&readme).expect("the file");
    if failed || written != "replaced\n" {
        answered.push(format!(
            "code_write expected_sha256 null: isError {failed}, file {written:?}: {text}"
        ));
    }
    std::fs::write(&readme, "replaced\n").expect("the file");
    let (failed, text) = call(
        &mut server,
        "code_edit",
        &json!({"path": "README.md", "old": "replaced", "new": "edited", "expected_sha256": null}),
    );
    let edited = std::fs::read_to_string(&readme).expect("the file");
    if failed || edited != "edited\n" {
        answered.push(format!(
            "code_edit expected_sha256 null: isError {failed}, file {edited:?}: {text}"
        ));
    }
    assert!(
        answered.is_empty(),
        "null on an optional field did not read as absent:\n{}",
        answered.join("\n")
    );
}

/// D1 and a45 § 3: a refused value is named by its path, "and the refusal names the JSON kind it
/// is and the declared type in the schema's words". Null on a required field is refused (D2), and
/// its refusal names neither the declared type nor, at depth, anything a model can correct from.
#[test]
fn a_required_field_given_null_is_refused_naming_its_declared_type() {
    let mut missing = Vec::new();
    let top = problems_of(&released("fs"), "fs_read", &json!({"path": null}));
    if !(top.len() == 1 && top[0].contains("`path`") && top[0].contains("`string`")) {
        missing.push(format!("fs_read path null: {top:?}"));
    }
    let nested = problems_of(
        &released("code-editor"),
        "code_patch",
        &json!({"path": "README.md", "edits": [{"old": null, "new": "x"}]}),
    );
    if !(nested.len() == 1
        && nested[0].contains("`edits[0].old`")
        && nested[0].contains("`string`"))
    {
        missing.push(format!("code_patch edits[0].old null: {nested:?}"));
    }
    let window = problems_of(
        &every_shape(),
        "fs_read",
        &every_shape_call(json!({"window": {"from": null}})),
    );
    if !(window.len() == 1
        && window[0].contains("`window.from`")
        && window[0].contains("`integer`"))
    {
        missing.push(format!("fs_read window.from null: {window:?}"));
    }
    assert!(
        missing.is_empty(),
        "a refusal that does not name the declared type:\n{}",
        missing.join("\n")
    );
}

/// a45 § 3: "A number that is not an `i64` or a `u64` is named for what it is, a number written
/// with a fraction or an exponent, or outside 64 bits". JSON `-0` has no fraction, no exponent and
/// is inside 64 bits; `serde_json` without `arbitrary_precision` holds it as the float `-0.0`, so
/// D3 refuses it, and the refusal then says something of it that is not so. With that feature it
/// is held as the integer 0 and admitted, and the assertion holds with no refusal to read.
#[test]
fn minus_zero_is_not_described_as_a_fraction_an_exponent_or_outside_64_bits() {
    let arguments: Value =
        serde_json::from_str(r#"{"path": "README.md", "offset": -0}"#).expect("JSON");
    let problems = problems_of(&released("fs"), "fs_read", &arguments);
    assert!(
        problems
            .iter()
            .all(|problem| !problem.contains("a fraction or an exponent, or outside 64 bits")),
        "`-0` is described as what it is not: {problems:?}"
    );
}

// ---------------------------------------------------------------------------------------------
// Green against the unit's tree: each kills a mutant the unit's own suite lets live.
// ---------------------------------------------------------------------------------------------

/// a45 § 2: "A struct inside the arguments is held to the three top-level rules too: no key
/// outside its fields, and every required field given a value other than null." Asked through
/// the server on a performed entry, so the call is also shown not to be performed.
#[test]
fn a_struct_inside_the_arguments_refuses_a_key_outside_its_fields_and_a_missing_field() {
    let dir = tree();
    let mut server = server(dir.path(), every_shape());
    for (window, named) in [
        (json!({"from": 1, "extra": 2}), "`window.extra`"),
        (json!({}), "`window.from`"),
        (json!({"to": 3}), "`window.from`"),
    ] {
        let arguments = every_shape_call(json!({"window": window}));
        let (failed, text) = call(&mut server, "fs_read", &arguments);
        assert!(failed, "{arguments}: {text}");
        assert!(text.contains(named), "{arguments}: {text}");
        assert!(text.contains("nothing was done"), "{arguments}: {text}");
        assert!(!text.contains("hello catalogue"), "{arguments}: {text}");
    }
    // The positive control: the struct check admits a window with its required field given and
    // its optional one null.
    let admitted = every_shape_call(json!({"window": {"from": 1, "to": null}}));
    let problems = problems_of(&every_shape(), "fs_read", &admitted);
    assert!(problems.is_empty(), "{admitted}: {problems:?}");
    // Through the server the call reaches harness, which since 0.16.1 refuses a field the
    // performed entry's schema does not declare (`crates/harness-wire/src/undeclared.rs:66` at
    // `330e2cde`). `file_read` declares none of the fields this caller's document adds, so the
    // call is refused there, by name, and nothing is read.
    let (failed, text) = call(&mut server, "fs_read", &admitted);
    assert!(failed, "{text}");
    assert!(text.contains("is not an argument of `file_read`"), "{text}");
    assert!(!text.contains("hello catalogue"), "{text}");
}

/// The a45 § 3 example, verbatim: the refusal names the JSON kind received, not only the path and
/// the declared type.
#[test]
fn the_documented_refusal_is_the_refusal_sent() {
    let dir = tree();
    let mut server = server(dir.path(), released("fs"));
    let (failed, text) = call(
        &mut server,
        "fs_read",
        &json!({"path": "README.md", "offset": "second"}),
    );
    assert!(failed, "{text}");
    assert_eq!(
        text,
        "`fs_read` was called with arguments its input schema does not admit, so nothing was \
         done: `offset` is a string, and its declared type is `integer`."
    );
    for (offset, kind) in [
        (json!(false), "is a boolean,"),
        (json!([]), "is an array,"),
        (json!({}), "is an object,"),
        (
            json!(1.5),
            "is a number written with a fraction or an exponent, or outside 64 bits,",
        ),
    ] {
        let problems = problems_of(
            &released("fs"),
            "fs_read",
            &json!({"path": "README.md", "offset": offset}),
        );
        assert_eq!(problems.len(), 1, "{offset}: {problems:?}");
        assert!(problems[0].contains(kind), "{offset}: {problems:?}");
    }
}

/// a45 § 2: "Another string is refused and the refusal lists the values."
#[test]
fn an_enum_refusal_lists_the_values() {
    let problems = problems_of(
        &every_shape(),
        "fs_read",
        &every_shape_call(json!({"kind": "socket"})),
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains("`kind`"), "{problems:?}");
    assert!(problems[0].contains("`directory`, `file`"), "{problems:?}");
}

/// a45 § 3: "It quotes no value the caller sent". A large value carrying control characters is
/// not echoed into the tool result, at the top level, in an enum or inside a list.
#[test]
fn a_refusal_quotes_no_value_the_caller_sent() {
    let sent = format!("MARK\u{7}\u{1b}[2J{}", "x".repeat(65_536));
    let dir = tree();
    let mut server = server(dir.path(), every_shape());
    for arguments in [
        every_shape_call(json!({"offset": sent})),
        every_shape_call(json!({"kind": sent})),
        every_shape_call(json!({"lines": [sent]})),
        every_shape_call(json!({"env": {"key": sent}})),
    ] {
        let (failed, text) = call(&mut server, "fs_read", &arguments);
        assert!(failed, "{text}");
        assert!(
            !text.contains("MARK"),
            "the value is echoed: {}",
            &text[..200.min(text.len())]
        );
        assert!(text.len() < 1024, "{} bytes answered", text.len());
    }
}

/// a45 § 3: a map key is written as a JSON string, so a key carrying a quote or a control
/// character cannot end the path it is named in.
#[test]
fn a_map_key_is_named_as_a_json_string() {
    let problems = problems_of(
        &every_shape(),
        "fs_read",
        &every_shape_call(json!({"env": {"a\"b\n": 1}})),
    );
    assert_eq!(problems.len(), 1, "{problems:?}");
    assert!(problems[0].contains(r#"`env["a\"b\n"]`"#), "{problems:?}");
    assert!(problems[0].contains("`boolean`"), "{problems:?}");
}

/// D3 at its edges: every integer `serde_json` holds as an `i64` or a `u64` passes the check,
/// zero, a negative and both extremes included; the range is the entry's to read.
#[test]
fn every_64_bit_integer_passes_the_type_check() {
    let catalogue = released("fs");
    for offset in [
        json!(0),
        json!(-1),
        json!(i64::MIN),
        json!(i64::MAX),
        json!(u64::MAX),
    ] {
        assert_eq!(
            problems_of(
                &catalogue,
                "fs_read",
                &json!({"path": "README.md", "offset": offset})
            ),
            Vec::<String>::new(),
            "{offset}"
        );
    }
    for text in ["1e3", "-0.0", "18446744073709551616"] {
        let arguments: Value =
            serde_json::from_str(&format!(r#"{{"path": "README.md", "offset": {text}}}"#))
                .expect("JSON");
        let problems = problems_of(&catalogue, "fs_read", &arguments);
        assert_eq!(problems.len(), 1, "{text}: {problems:?}");
        assert!(problems[0].contains("`integer`"), "{text}: {problems:?}");
    }
}

/// a45 § 2: "at every depth that schema declares". An `Optional` inside a list or a map publishes
/// `anyOf`, and the choice the value matches is still walked: an enum's values and a struct's
/// fields hold inside it. The unit's fixtures put only `Optional<Integer>` there.
#[test]
fn an_optional_list_item_or_map_value_is_walked_into() {
    let text = FIXTURE
        .replace(
            "errors:\n",
            "types:\n  - {name: fs.Kind, kind: enum, summary: A kind., variants: [file, directory]}\n  - name: fs.Window\n    kind: struct\n    summary: A window.\n    fields: [{name: from, type: Integer}, {name: to, type: Optional<Integer>}]\nerrors:\n",
        )
        .replace(
            "        description: The first line to read, numbered from 1.\n",
            "        description: The first line to read, numbered from 1.\n      - name: windows\n        type: List<Optional<fs.Window>>\n      - name: kinds\n        type: Map<String, Optional<fs.Kind>>\n",
        );
    let catalogue =
        CallerCatalogue::from_text(Path::new("optional-inner.yaml"), &text).expect("a catalogue");
    for (arguments, named, declared) in [
        (
            optional_inner_call(json!({"windows": [{"from": "x"}]})),
            "`windows[0].from`",
            "`integer`",
        ),
        (
            optional_inner_call(json!({"windows": [{"from": 1, "extra": 2}]})),
            "`windows[0].extra`",
            "`from`",
        ),
        (
            optional_inner_call(json!({"kinds": {"a": "socket"}})),
            r#"`kinds["a"]`"#,
            "`directory`, `file`",
        ),
    ] {
        let problems = problems_of(&catalogue, "fs_read", &arguments);
        assert_eq!(problems.len(), 1, "{arguments}: {problems:?}");
        assert!(problems[0].contains(named), "{arguments}: {problems:?}");
        assert!(problems[0].contains(declared), "{arguments}: {problems:?}");
    }
    assert_eq!(
        problems_of(
            &catalogue,
            "fs_read",
            &optional_inner_call(
                json!({"windows": [null, {"from": 1}], "kinds": {"a": null, "b": "file"}})
            ),
        ),
        Vec::<String>::new()
    );
}
