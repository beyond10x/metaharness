//! The publication allowlist, in reading order. No private design or research is included.
use crate::Source;

macro_rules! page {
    ($key:literal, $group:literal) => {
        Source {
            key: $key,
            group: $group,
            markdown: include_str!(concat!("../../../website/docs/", $key, ".md")),
        }
    };
}

pub const PAGES: &[Source<'_>] = &[
    page!("index", "Start here"),
    page!("quickstart", "Start here"),
    page!("hermetic", "The contract"),
    page!("control-seam", "The contract"),
    page!("frames", "The contract"),
    page!("protocol/events", "Protocol"),
    page!("protocol/commands", "Protocol"),
    page!("harnesses/claude", "Harnesses"),
    page!("harnesses/codex", "Harnesses"),
    page!("harnesses/b10x", "Harnesses"),
    page!("reference/cli", "Reference"),
    page!("reference/library", "Reference"),
    page!("status", "Reference"),
];
