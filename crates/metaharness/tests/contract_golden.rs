//! The `contract_result` record, pinned as the bytes a consumer reads (adapter contract CT-1).
//!
//! `AEP` ingests `metaharness conformance <kind> --contract` as evidence. It is
//! public and this workspace is not, so no Cargo dependency crosses; what crosses is the
//! **vocabulary** — `{checked, failed, breaking_changes, provider, consumer}` (design
//! `docs/design/adapter-contract-v0.1.md`). Two implementations that share a vocabulary and no code
//! cannot tell each other apart from two implementations that have quietly drifted, and the only
//! thing that can is a committed artifact.
//!
//! So the record is pinned twice over. The golden files under `fixtures/golden/` are the exact
//! stdout of the two live runs on the day they were recorded, and the tests below rebuild the
//! record through the real library path — `contract_result(kind, &conformance_vectors(kind))`, the
//! same call the binary makes — and compare it to them **byte for byte**. A test that re-derived
//! its own expectation would agree with itself through any change.
//!
//! # What breaking one means
//!
//! Not a chore, and not always a bug:
//!
//! | what moved | what it means |
//! |---|---|
//! | `checked` | a vector was added or removed. Legitimate, and still deliberate: regenerate, and say so |
//! | `provider` | the pin moved, or the adapter id did. The consumer is now reading about a different binary |
//! | `failed` / `breaking_changes` | the contract is red. The record is not the thing to fix |
//! | the key order | the serialisation moved under the record. A consumer reads bytes, so this is a break even though the values are equal |
//!
//! Regeneration is `regenerate_the_contract_records`, `#[ignore]`d because it writes into the
//! source tree. Provenance and the procedure: `fixtures/golden/README.md`.

use metaharness::protocol::Kind;
use metaharness::{conformance_vectors, contract_result};

/// The recorded stdout of each run, as bytes rather than as a path, so a test binary run from
/// anywhere reads the same one. The trailing newline is part of the record: it is the line the CLI
/// wrote, not a value it happened to hold.
const CLAUDE_GOLDEN: &str = include_str!("../fixtures/golden/contract-result-claude.json");
const CODEX_GOLDEN: &str = include_str!("../fixtures/golden/contract-result-codex.json");
const B10X_GOLDEN: &str = include_str!("../fixtures/golden/contract-result-b10x.json");

/// The six keys, in the one order the record is serialised in.
///
/// Written out rather than read off the emitted record, because the claim is about the **bytes** a
/// consumer parses in sequence, and an expectation derived from the same serialiser would hold
/// through a change to it.
const KEY_ORDER: [&str; 6] = [
    "breaking_changes",
    "checked",
    "consumer",
    "failed",
    "kind",
    "provider",
];

/// How the record for one kind is rebuilt: the library path the binary itself takes.
fn emit(kind: Kind) -> String {
    let vectors = conformance_vectors(kind).expect("the adapter exists");
    let record = contract_result(kind, &vectors).expect("the record is built");
    format!("{record}\n")
}

/// Name what moved, field by field, and say what to do about it.
fn drift(golden: &str, emitted: &str, kind: Kind) -> String {
    use std::fmt::Write as _;

    let mut report = format!(
        "the `contract_result` record for {} is no longer the committed one.\n",
        kind.as_str()
    );
    let parse = |text: &str| serde_json::from_str::<serde_json::Value>(text).ok();
    match (parse(golden), parse(emitted)) {
        (Some(expected), Some(observed)) => {
            for key in KEY_ORDER {
                let (was, now) = (expected.get(key), observed.get(key));
                if was != now {
                    let _ = writeln!(
                        report,
                        "  {key}: was {}, is now {}",
                        was.map_or_else(|| "absent".to_string(), ToString::to_string),
                        now.map_or_else(|| "absent".to_string(), ToString::to_string),
                    );
                }
            }
            if expected == observed {
                report.push_str(
                    "  every value is equal, so what moved is the bytes: key order, spacing or \
                     the trailing newline. A consumer reads bytes.\n",
                );
            }
        }
        _ => {
            let _ = writeln!(report, "  golden {golden:?}, emitted {emitted:?}");
        }
    }
    report.push_str(
        "A count that moved because a vector was added or removed is legitimate — and the golden \
         record is still regenerated **deliberately**, never to restore green: \
         `cargo test -p metaharness --test contract_golden regenerate -- --ignored`, then read the \
         diff, move the vector-count pins with it, and tell the consumer that is building against \
         these bytes.",
    );
    report
}

/// The claude record this repository emits is the record it published.
#[test]
fn the_claude_contract_record_is_the_bytes_the_consumer_reads() {
    let emitted = emit(Kind::Claude);
    assert_eq!(
        emitted,
        CLAUDE_GOLDEN,
        "{}",
        drift(CLAUDE_GOLDEN, &emitted, Kind::Claude)
    );
}

/// The codex record likewise — pinned separately, because the two adapters move separately.
#[test]
fn the_codex_contract_record_is_the_bytes_the_consumer_reads() {
    let emitted = emit(Kind::Codex);
    assert_eq!(
        emitted,
        CODEX_GOLDEN,
        "{}",
        drift(CODEX_GOLDEN, &emitted, Kind::Codex)
    );
}

/// The direct-provider adapter's record is pinned by the same consumer-facing bytes.
#[test]
fn the_b10x_contract_record_is_the_bytes_the_consumer_reads() {
    let emitted = emit(Kind::B10x);
    assert_eq!(
        emitted,
        B10X_GOLDEN,
        "{}",
        drift(B10X_GOLDEN, &emitted, Kind::B10x)
    );
}

/// The keys arrive in one order, and that order is part of the record.
///
/// Byte-exactness above already catches a reordering; this test exists so that the *failure* names
/// it. `serde_json`'s map is sorted today and nothing here asks for that — turning on its
/// `preserve_order` feature anywhere in the workspace would silently re-order every record this
/// binary prints, and the consumer would be the one to find out.
#[test]
fn the_records_keys_are_serialised_in_one_pinned_order() {
    for (kind, golden) in [
        (Kind::Claude, CLAUDE_GOLDEN),
        (Kind::Codex, CODEX_GOLDEN),
        (Kind::B10x, B10X_GOLDEN),
    ] {
        for text in [golden.to_string(), emit(kind)] {
            let mut seen: Vec<(usize, &str)> = KEY_ORDER
                .iter()
                .map(|key| {
                    let at = text.find(&format!("\"{key}\":")).unwrap_or_else(|| {
                        panic!("{} carries no `{key}` key: {text}", kind.as_str())
                    });
                    (at, *key)
                })
                .collect();
            seen.sort_unstable();
            let order: Vec<&str> = seen.into_iter().map(|(_, key)| key).collect();
            assert_eq!(
                order,
                KEY_ORDER,
                "the {} record's keys arrive in a different order than the consumer pins; the \
                 values may be equal and the bytes are not",
                kind.as_str()
            );
        }
    }
}

/// The record says the same thing the run's exit code does: nothing failed, nothing broke, and
/// something was actually checked.
///
/// Pinned beside the bytes because these three are the record's whole verdict, and a golden file
/// that had been regenerated over a red run would otherwise publish the red as the new normal.
#[test]
fn the_committed_records_are_green_runs_that_checked_something() {
    for (kind, golden) in [
        (Kind::Claude, CLAUDE_GOLDEN),
        (Kind::Codex, CODEX_GOLDEN),
        (Kind::B10x, B10X_GOLDEN),
    ] {
        let record: serde_json::Value = serde_json::from_str(golden).expect("the record parses");
        assert_eq!(record["kind"], "contract_result");
        assert_eq!(record["consumer"], metaharness::protocol::EVENT_FORMAT);
        assert_eq!(record["failed"].as_u64(), Some(0), "{}", kind.as_str());
        assert_eq!(
            record["breaking_changes"].as_u64(),
            Some(0),
            "{}",
            kind.as_str()
        );
        assert!(
            record["checked"]
                .as_u64()
                .is_some_and(|checked| checked > 0),
            "a run that checked nothing also has zero failures: {}",
            kind.as_str()
        );
        let provider = record["provider"].as_str().expect("a provider string");
        assert!(
            provider.starts_with(&format!("{} ", kind.as_str())),
            "provider {provider:?} names the vendor and then its pin"
        );
    }
}

/// Rewrite both golden records from the library path. `#[ignore]`d because it writes into the
/// source tree; it is the second half of a deliberate change, not of the gate:
///
/// ```console
/// cargo test -p metaharness --test contract_golden regenerate -- --ignored
/// ```
///
/// It writes what the *library* builds rather than shelling out to the binary, so a regeneration
/// cannot pick up a stale `target/debug/metaharness`. The two are the same bytes by construction:
/// the CLI prints this record and nothing else on stdout.
#[test]
#[ignore = "writes fixtures/golden/contract-result-*.json from the current vectors; run after a deliberate change, then read the diff"]
fn regenerate_the_contract_records() {
    for (kind, name) in [
        (Kind::Claude, "contract-result-claude.json"),
        (Kind::Codex, "contract-result-codex.json"),
        (Kind::B10x, "contract-result-b10x.json"),
    ] {
        let path = format!("{}/fixtures/golden/{name}", env!("CARGO_MANIFEST_DIR"));
        std::fs::write(&path, emit(kind)).expect("the record is written");
    }
}
