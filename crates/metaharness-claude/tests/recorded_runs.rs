//! The two recorded `evals/aep` runs, as `metaharness.event/1` streams.
//!
//! `epic:runs-side-by-side` names *"the two recorded `evals/aep` runs"* as the fixture for
//! `metaharness project` and for the viewer. What is recorded there is Claude Code `stream-json`
//! (`evals/aep/checks/transcripts/*-clean.jsonl`), and `story:trace-ir-reader` puts a foreign
//! transcript out of scope by name — *"a Claude Code JSONL that did not pass through this driver
//! is not this story"*. So the fixture the two later stories read is those bytes **passed through
//! this driver's own reader**: this test is the pass, and `evals/aep/runs/*.events.jsonl` is its
//! committed output.
//!
//! # This does not violate invariant 5
//!
//! *"Nothing under `evals/` runs in `task check`"* is about a **paid run** never being part of a
//! gate. Nothing here runs an eval: it reads two committed files, converts them in memory, and
//! compares bytes. The inputs are themselves hand-written — `evals/aep/checks/transcripts/README.md`
//! says so out loud, *"nothing here came from a model, and no claim in this repository rests on one
//! of these files describing a real run"* — and that label travels with the derived streams, which
//! carry it in `evals/aep/runs/README.md`.
//!
//! # What the conversion is, and what it is not
//!
//! It is the adapter's transcript reader, driven over the recorded lines in order, with an
//! attestation that claims **nothing** (`HermeticAttestation::none(HermeticMode::Off)`) and a seam
//! of `Seam::None`. A replay is not a launch: there was no scratch home, no credential and no
//! imposition, so an attestation that claimed one would be metaharness asserting a control it
//! never applied (invariant 3).

use std::path::PathBuf;

use metaharness_claude::TranscriptReader;
use metaharness_protocol::{
    Digest, EventStream, HermeticAttestation, HermeticMode, RunId, Seam, TranscriptRef,
};

/// The two runs, as `(run id, recorded transcript, derived event stream)`.
const RUNS: [(&str, &str, &str); 2] = [
    (
        "decomposer-clean",
        "checks/transcripts/decomposer-clean.jsonl",
        "runs/decomposer-clean.events.jsonl",
    ),
    (
        "plan-reviewer-clean",
        "checks/transcripts/plan-reviewer-clean.jsonl",
        "runs/plan-reviewer-clean.events.jsonl",
    ),
];

fn evals_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../evals/aep")
}

/// The conversion, as one function, so the check and the regeneration cannot disagree about it.
fn convert(run: &str, transcript_path: &str) -> String {
    let path = evals_dir().join(transcript_path);
    let bytes = std::fs::read(&path).expect("the recorded transcript is committed");
    let text = String::from_utf8(bytes.clone()).expect("the recorded transcript is UTF-8");

    let mut reader = TranscriptReader::new(
        TranscriptRef {
            path: Some(transcript_path.to_string()),
            digest: Some(Digest::of(&bytes)),
            bytes: Some(bytes.len() as u64),
        },
        HermeticAttestation::none(HermeticMode::Off),
    )
    .with_seam(Seam::None);

    let mut stream = EventStream::new(RunId::new(run));
    let mut out = String::new();
    for line in text.lines() {
        for emission in reader.push_line(line) {
            let framed = stream.stamp(emission);
            out.push_str(&serde_json::to_string(&framed).expect("an event line renders"));
            out.push('\n');
        }
    }
    for emission in reader.finish() {
        let framed = stream.stamp(emission);
        out.push_str(&serde_json::to_string(&framed).expect("an event line renders"));
        out.push('\n');
    }
    out
}

#[test]
fn the_two_recorded_runs_convert_to_the_committed_event_streams() {
    for (run, transcript, events) in RUNS {
        let expected = std::fs::read_to_string(evals_dir().join(events))
            .expect("the event stream is committed");
        assert_eq!(
            convert(run, transcript),
            expected,
            "{run}: the derived event stream moved; regenerate it deliberately and read the diff"
        );
    }
}

/// The same conversion, byte for byte, twice — the property the whole projection rests on.
#[test]
fn the_conversion_reads_no_clock_and_is_the_same_bytes_twice() {
    for (run, transcript, _) in RUNS {
        assert_eq!(convert(run, transcript), convert(run, transcript));
    }
}

/// Rewrite the derived streams. `#[ignore]`d because it writes into the source tree; it is the
/// second half of a deliberate change, not of the gate:
///
/// ```console
/// cargo test -p metaharness-claude --test recorded_runs regenerate -- --ignored
/// ```
#[test]
#[ignore = "writes evals/aep/runs/*.events.jsonl from the recorded transcripts; run after a deliberate change, then read the diff"]
fn regenerate_the_recorded_event_streams() {
    for (run, transcript, events) in RUNS {
        let path = evals_dir().join(events);
        std::fs::create_dir_all(path.parent().expect("a parent")).expect("the runs directory");
        std::fs::write(&path, convert(run, transcript)).expect("the event stream is written");
    }
}
