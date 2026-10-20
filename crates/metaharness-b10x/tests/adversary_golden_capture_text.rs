//! Adversary case, wave 3 of 2026-10-20, unit `story:b10x-adapter-on-harness-over-llm`.
//!
//! The golden loop record was re-recorded from `b10x-harness 0.15.0` (`fixtures/golden/version.txt`)
//! and its `started` line now carries `"mcp": []`. The adapter's own documentation still describes
//! "the 0.9.1 capture in `fixtures/golden/`" and says it does not carry an `mcp` field
//! (`src/lib.rs`, `src/seam.rs`). A reader who trusts that text reasons about a fixture that is no
//! longer there.

fn crate_file(relative: &str) -> String {
    let path = format!("{}/{relative}", env!("CARGO_MANIFEST_DIR"));
    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{path}: {error}"))
}

/// The comment text of a Rust source, prefixes dropped and whitespace collapsed, so a phrase broken
/// across two comment lines reads as one.
fn comment_text(source: &str) -> String {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_start();
            line.strip_prefix("//!")
                .or_else(|| line.strip_prefix("///"))
                .or_else(|| line.strip_prefix("//"))
        })
        .flat_map(str::split_whitespace)
        .collect::<Vec<_>>()
        .join(" ")
}

#[test]
fn the_adapter_describes_the_golden_capture_as_the_version_it_was_recorded_from() {
    let banner = crate_file("fixtures/golden/version.txt");
    let recorded = banner
        .trim()
        .strip_prefix("b10x-harness ")
        .expect("the banner names the binary");

    let started: serde_json::Value = serde_json::from_str(
        crate_file("fixtures/golden/loop.jsonl")
            .lines()
            .next()
            .expect("the record opens"),
    )
    .expect("JSON");
    assert_eq!(started["kind"], "started");

    let mut stale = Vec::new();
    for file in ["src/lib.rs", "src/seam.rs"] {
        let text = comment_text(&crate_file(file));
        let mut rest = text.as_str();
        while let Some(at) = rest.find(" capture ") {
            let before = &rest[..at];
            let version = before.rsplit(' ').next().unwrap_or_default();
            let after: String = rest[at..].chars().take(48).collect();
            let is_version = !version.is_empty()
                && version.split('.').count() == 3
                && version.split('.').all(|part| part.parse::<u32>().is_ok());
            if is_version && after.contains("golden") && version != recorded {
                stale.push(format!("{file}: \"{version}{after}\""));
            }
            rest = &rest[at + " capture ".len()..];
        }
    }
    assert!(
        stale.is_empty(),
        "fixtures/golden/ holds a {recorded} capture whose started record carries mcp = {}, and \
         the adapter's documentation still describes another capture:\n{}",
        started["mcp"],
        stale.join("\n")
    );
}
