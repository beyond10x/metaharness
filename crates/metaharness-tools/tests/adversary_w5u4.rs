//! Adversary cases for `story:caller-supplied-catalogue-on-every-kind` (wave 5, unit 4).
//!
//! The scratch copy the server reads is the parent's checked document written back out as JSON
//! (`CallerCatalogue::document`, then `serde_json::to_string_pretty`, as the builder's
//! `write_scratch_files` does). The claim under attack is that the copy reads back as the
//! catalogue that was checked, for every document the parent accepted.

use std::path::Path;

use metaharness_tools::{
    CATALOGUE_BYTES_MAX, CATALOGUE_MALFORMED, CATALOGUE_UNREADABLE, CallerCatalogue, Resolved,
    SERVER_NAME, render_scratch_copy,
};

const FIXTURE: &str = include_str!("fixtures/caller-catalogue.yaml");

fn accepted(text: &str) -> CallerCatalogue {
    CallerCatalogue::from_text(Path::new("catalogue.yaml"), text)
        .expect("the parent accepts the caller's document")
}

/// What `mcp-serve --catalogue <copy>` reads: the parent's document, rendered as the builder
/// renders the scratch file.
fn scratch_copy(catalogue: &CallerCatalogue) -> String {
    serde_json::to_string_pretty(&catalogue.document()).expect("the document renders")
}

/// A summary carrying a character that YAML admits only escaped. The caller's YAML emitter
/// escaped it, so the parent reads it; `serde_json` writes it raw, and the copy then holds a
/// character the YAML reader refuses as a control character.
#[test]
fn every_document_the_parent_accepts_reads_back_from_its_scratch_copy() {
    let mut lost = Vec::new();
    for (escape, label) in [
        ("\\x7F", "DEL"),
        ("\\x92", "C1 PRIVATE USE TWO"),
        ("\\uFFFE", "noncharacter U+FFFE"),
    ] {
        let text = FIXTURE.replace(
            "    summary: Read a window of a file as numbered lines.",
            &format!("    summary: \"Read a window{escape} of a file as numbered lines.\""),
        );
        assert_ne!(text, FIXTURE, "the fixture line was found");
        let checked = accepted(&text);
        let copy = scratch_copy(&checked);
        match CallerCatalogue::from_text(Path::new("catalogue.json"), &copy) {
            Ok(read_back) if read_back == checked => {}
            Ok(_) => lost.push(format!("{label}: the copy reads back as another catalogue")),
            Err(error) => lost.push(format!("{label}: the copy is refused: {error}")),
        }
    }
    assert!(
        lost.is_empty(),
        "the parent accepted these documents and the server would refuse or change them:\n{}",
        lost.join("\n")
    );
}

/// The Claude vector `c1-caller-catalogue` pins the scratch file `plan_launch` writes and the
/// `offered_tools` a session would report. a40 § 8 says that file is the scratch catalogue: it is
/// what `mcp-serve --catalogue` reads in that launch, so it has to be a catalogue the server
/// serves, publishing the names the vector pins.
#[test]
fn the_scratch_catalogue_the_claude_vector_pins_is_one_the_server_serves() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../metaharness-claude/fixtures/c1/caller-catalogue.json"
    ))
    .expect("the vector fixture is JSON");
    let file = fixture["scratch_files"]
        .as_array()
        .expect("scratch files")
        .iter()
        .find(|file| {
            file["path"]
                .as_str()
                .is_some_and(|path| path.ends_with("/catalogue.json"))
        })
        .expect("the vector pins a scratch catalogue");
    let copy = serde_json::to_string_pretty(&file["document"]).expect("renders");
    let served = CallerCatalogue::from_text(Path::new("catalogue.json"), &copy)
        .unwrap_or_else(|error| panic!("mcp-serve would refuse the pinned copy: {error}"));
    let pinned: Vec<String> = fixture["offered_tools"]
        .as_array()
        .expect("offered tools")
        .iter()
        .filter_map(serde_json::Value::as_str)
        .map(str::to_owned)
        .collect();
    assert_eq!(served.offered_tools(), pinned);
}

/// The published-name limit is 46 bytes: 46 is published, 47 is refused. The existing case
/// refuses a 51-byte name only, so `>` turned into `>=`, or the limit moved by up to four, stays
/// green there.
#[test]
fn a_rendering_of_exactly_the_limit_is_published_and_one_byte_more_is_not() {
    let at_limit = format!("fs_{}", "s".repeat(43));
    assert_eq!(at_limit.len(), 46);
    assert_eq!(
        format!("mcp__{SERVER_NAME}__{at_limit}").len(),
        64,
        "the vendor's tool-name limit"
    );
    let catalogue = accepted(&FIXTURE.replace(
        "renderings: {mcp: fs_stat}",
        &format!("renderings: {{mcp: {at_limit}}}"),
    ));
    assert!(
        catalogue
            .offered_tools()
            .contains(&format!("mcp__{SERVER_NAME}__{at_limit}"))
    );

    let past = format!("{at_limit}s");
    let error = CallerCatalogue::from_text(
        Path::new("catalogue.yaml"),
        &FIXTURE.replace(
            "renderings: {mcp: fs_stat}",
            &format!("renderings: {{mcp: {past}}}"),
        ),
    )
    .expect_err("a 47-byte rendering is past the vendor's limit");
    assert_eq!(error.code(), CATALOGUE_MALFORMED, "{error}");
    assert!(error.to_string().contains("47 bytes"), "{error}");
}

/// A hyphen is a tool-name character (`^[a-zA-Z0-9_-]{1,64}$`); nothing in the suite publishes
/// one, so dropping `-` from the admitted set stays green.
#[test]
fn a_hyphenated_rendering_is_published_and_resolves() {
    let catalogue = accepted(&FIXTURE.replace("mcp: fs_read}", "mcp: fs-read}"));
    let name = format!("mcp__{SERVER_NAME}__fs-read");
    assert!(catalogue.offered_tools().contains(&name));
    assert_eq!(
        catalogue.resolve(&name),
        Resolved::Operations(vec!["file.read".to_owned()])
    );
}

// Pass 2.

/// U+2028 and U+2029 are line breaks to the YAML reader, as U+0085 is (`IS_BREAK`,
/// unsafe-libyaml 0.2.11 `src/macros.rs:253-264`). Inside a double-quoted scalar the reader drops
/// the blanks before a break and skips the blanks after it (`src/scanner.rs`, the flow-scalar
/// whitespace loop), so `"a \u{2028} b"` reads back as `"a\u{2028}b"`. The copy escapes U+0085 and
/// writes these two raw. `every_unicode_scalar_value_survives_the_scratch_copy` never puts a space
/// beside either, so it stays green.
#[test]
fn a_line_or_paragraph_separator_beside_a_space_survives_the_scratch_copy() {
    let mut lost = Vec::new();
    for text in [
        "a \u{2028} b",
        "a \u{2029} b",
        "a\u{2028} b",
        "a \u{2029}b",
        "a \u{85} b",
    ] {
        let copy = render_scratch_copy(&serde_json::json!({ "value": text }));
        let read: Option<String> = serde_yaml_ng::from_str::<serde_yaml_ng::Value>(&copy)
            .ok()
            .and_then(|value| {
                value
                    .get("value")
                    .and_then(|value| value.as_str().map(str::to_owned))
            });
        if read.as_deref() != Some(text) {
            lost.push(format!("{text:?} reads back as {read:?}"));
        }
    }
    assert!(
        lost.is_empty(),
        "the scratch copy changes these strings:\n{}",
        lost.join("\n")
    );
}

/// The same through the parent: a summary the caller's YAML wrote as ` ` between two spaces is
/// a valid `tool-contract/1` document. The read-back sees the copy change and refuses the document,
/// so the gap is detected; the escape set leaves it open.
#[test]
fn a_summary_with_a_line_separator_between_spaces_is_accepted() {
    let mut refused = Vec::new();
    for escape in ["\\u2028", "\\u2029"] {
        let text = FIXTURE.replace(
            "    summary: Read a window of a file as numbered lines.",
            &format!("    summary: \"Read a window {escape} of a file as numbered lines.\""),
        );
        assert_ne!(text, FIXTURE, "the fixture line was found");
        if let Err(error) = CallerCatalogue::from_text(Path::new("catalogue.yaml"), &text) {
            refused.push(format!("{escape}: {error}"));
        }
    }
    assert!(
        refused.is_empty(),
        "the parent refuses valid documents:\n{}",
        refused.join("\n")
    );
}

/// The parent refuses a document whose scratch copy is past the cap, and the server reads a copy
/// of exactly the cap. The existing case's copy is about 1.5 MiB past it, so `>` turned into `>=`
/// in `from_text` stays green there.
#[test]
fn a_document_whose_copy_is_exactly_the_cap_is_accepted_and_one_byte_more_is_not() {
    let cap = usize::try_from(CATALOGUE_BYTES_MAX).unwrap();
    let with_summary = |length: usize| {
        FIXTURE.replace(
            "    summary: Read a window of a file as numbered lines.",
            &format!("    summary: {}", "a".repeat(length)),
        )
    };
    let base = accepted(&with_summary(1)).document().get().len();
    let at = with_summary(cap - base + 1);
    assert!(at.len() < cap, "the caller's file is under the cap");
    let catalogue = CallerCatalogue::from_text(Path::new("catalogue.yaml"), &at)
        .unwrap_or_else(|error| panic!("a copy of exactly the cap: {error}"));
    let copy = catalogue.document().get().to_owned();
    assert_eq!(copy.len(), cap);
    let dir = tempfile::tempdir().expect("a scratch tree");
    let path = dir.path().join("catalogue.json");
    std::fs::write(&path, &copy).expect("the copy");
    assert_eq!(
        CallerCatalogue::read(&path).expect("the server reads a copy of exactly the cap"),
        catalogue
    );

    let error =
        CallerCatalogue::from_text(Path::new("catalogue.yaml"), &with_summary(cap - base + 2))
            .expect_err("a copy one byte past the cap");
    assert_eq!(error.code(), CATALOGUE_MALFORMED, "{error}");
    assert!(
        error.to_string().contains(&format!("{} bytes", cap + 1)),
        "{error}"
    );
}

/// A link to a device is followed and refused as not a regular file; a file that is not UTF-8 is
/// refused naming UTF-8. Nothing in the suite reads either.
#[cfg(unix)]
#[test]
fn a_link_to_a_device_and_a_file_that_is_not_utf8_are_refused_as_unreadable() {
    let dir = tempfile::tempdir().expect("a scratch tree");
    let link = dir.path().join("zero.yaml");
    std::os::unix::fs::symlink("/dev/zero", &link).expect("a link");
    let error = CallerCatalogue::read(&link).expect_err("a link to a device");
    assert_eq!(error.code(), CATALOGUE_UNREADABLE, "{error}");
    assert!(error.to_string().contains("not a regular file"), "{error}");

    let latin = dir.path().join("latin.yaml");
    let mut bytes = FIXTURE.as_bytes().to_vec();
    bytes.extend_from_slice(b"# caf\xe9\n");
    std::fs::write(&latin, bytes).expect("a file");
    let error = CallerCatalogue::read(&latin).expect_err("not UTF-8");
    assert_eq!(error.code(), CATALOGUE_UNREADABLE, "{error}");
    assert!(error.to_string().contains("not UTF-8"), "{error}");
}

/// YAML admits a byte-order mark at the start of a stream (YAML 1.2.2 § 5.2), and editors write
/// one. `checked` hands the text to `serde_yaml_ng::from_str`, which sets the encoding to UTF-8,
/// so the reader keeps the mark as a character and the document no longer parses as one document.
#[test]
fn a_catalogue_file_starting_with_a_byte_order_mark_is_read() {
    let dir = tempfile::tempdir().expect("a scratch tree");
    let path = dir.path().join("bom.yaml");
    std::fs::write(&path, format!("\u{feff}{FIXTURE}")).expect("a file");
    let read = CallerCatalogue::read(&path)
        .unwrap_or_else(|error| panic!("a byte-order mark refuses the file: {error}"));
    assert_eq!(read, accepted(FIXTURE));
}

/// Every YAML context a caller's text can take reaches the copy as a JSON string, which is a
/// double-quoted scalar to the reader: CRLF line ends, a literal block scalar, a single-quoted
/// `\`, a `\u` in a comment, an astral character, and a document written as JSON with its own
/// escapes.
#[test]
fn every_yaml_context_a_caller_writes_reads_back_from_the_copy() {
    let line = "    summary: Read a window of a file as numbered lines.";
    let cases = [
        ("CRLF", FIXTURE.replace('\n', "\r\n")),
        (
            "block scalar",
            FIXTURE.replace(
                line,
                "    summary: |\n      Read a window \\u0085 of\n      a \"file\".\n",
            ),
        ),
        (
            "single-quoted backslash",
            FIXTURE.replace(line, "    summary: 'Read \\ a \\u0085 \\\\ window.'"),
        ),
        (
            "escape in a comment",
            FIXTURE.replace(line, &format!("{line} # \\u0085 \\x7F")),
        ),
        (
            "astral",
            FIXTURE.replace(
                line,
                "    summary: \"Read \\U0001F600 \u{1D11E} a window.\"",
            ),
        ),
        (
            "JSON with escapes",
            render_scratch_copy(
                &accepted(&FIXTURE.replace(line, "    summary: \"a\\\\b \\\" \\t \\x7F c\""))
                    .document_value(),
            ),
        ),
    ];
    let mut lost = Vec::new();
    for (label, text) in cases {
        let checked = match CallerCatalogue::from_text(Path::new("catalogue.yaml"), &text) {
            Ok(checked) => checked,
            Err(error) => {
                lost.push(format!("{label}: the parent refuses it: {error}"));
                continue;
            }
        };
        match CallerCatalogue::from_text(Path::new("catalogue.json"), checked.document().get()) {
            Ok(read_back) if read_back == checked => {}
            Ok(_) => lost.push(format!("{label}: the copy reads back as another catalogue")),
            Err(error) => lost.push(format!("{label}: the copy is refused: {error}")),
        }
    }
    assert!(lost.is_empty(), "{}", lost.join("\n"));
}
