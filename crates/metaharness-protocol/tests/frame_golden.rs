//! The frame the other side of the seam mints, read by the consumer that would refuse it.
//!
//! `engineering-protocols` drives runs through this workspace and hands every `llm` step a sealed
//! `metaharness.frame/1` document as a file, because it is public, this repository is not, and no
//! Cargo dependency may cross between them. The two share a vocabulary and no code — which means
//! nothing but a committed artifact can tell them apart from two implementations that have quietly
//! drifted.
//!
//! That repository's own contract suite is a **transcription** of `frame.rs`: the digest rule
//! written out by hand, because it cannot link the real one. Its stated open risk is that *"the
//! transcription's continued agreement with `frame.rs` is closed only by the metaharness-side
//! replay of these bytes."* This file is that replay, and it is the half that matters — everything
//! over there is a second opinion about what this crate does, and only here does the real
//! [`Frame::parse_document`] get the minted bytes.
//!
//! # What each test pins, and what breaking it means
//!
//! The seal is over the frame's **canonical form** — sorted keys, `operations` in wire-name order,
//! `digest` and `format` absent from the hashed bytes (design § 5.5). None of that is visible from
//! the struct, and the first cross-repository document failed on exactly the invisible part: the
//! enum's variant order was the canonical one and no outside producer could have known it. So the
//! digest is pinned here as a **literal**, not recomputed from whatever this build happens to do —
//! a suite that re-derived its own expectation would agree with itself through any change.
//!
//! A failure here is a question, not a chore. Either this repository's canonical form moved, and
//! the other side has to be told before its next driven run dies at step one with a paid-for
//! session, or the copy is stale. Re-sealing the fixture to restore green deletes the evidence of
//! which it was. Provenance and the re-recording procedure: `fixtures/golden/README.md`.

use metaharness_protocol::{
    Digest, FRAME_FORMAT, Frame, FrameDocError, Handoff, Operation, WorkflowRef,
};

/// The minted document, as bytes rather than as a path, so a test binary run from anywhere reads
/// the same one.
const GOLDEN: &str = include_str!("../fixtures/golden/metaharness-frame-canonical.json");

/// The digest the document states and its contents must imply, as both repositories pin it.
const SEALED_DIGEST: &str = "43a6f845a21f3475569323950a9d276bfed3df11979adc3edf18878da6963a12";

/// SHA-256 of the file itself, as recorded on the minting side the day it was copied here.
///
/// The copy's claim is that it is byte-identical, and this is the only assertion that checks it —
/// every other test below would still pass on a document that had been "tidied" on the way in and
/// resealed.
const MINTED_FILE_SHA256: &str = "ef897a58a624848aad942d69d2745b431f2eaad5180cd0f5b2e1c8975adcb93b";

/// The bytes in the tree are the bytes that were minted, hashed by this crate's own hasher.
#[test]
fn the_committed_copy_is_the_file_the_minter_wrote() {
    assert_eq!(
        Digest::of(GOLDEN.as_bytes()).as_str(),
        MINTED_FILE_SHA256,
        "the fixture is a byte-identical copy of the minted document, or it is evidence of nothing"
    );
}

/// The minted document is accepted, and it says what the minter meant it to say.
///
/// Asserted field by field rather than as "no error", because a consumer that accepted the bytes
/// and then read a different step out of them would be the worse failure of the two: the run would
/// go ahead, under a frame nobody wrote, and the digest in every downstream event would cite it.
#[test]
fn the_minted_frame_is_accepted_and_carries_the_step_the_minter_described() {
    let frame = Frame::parse_document(GOLDEN).expect("the minted document is accepted");

    assert_eq!(
        frame.workflow,
        WorkflowRef {
            id: "development/default".to_string(),
            version: "1".to_string(),
        },
        "the workflow is pinned for the life of the run (H10), so both halves of it survive the file"
    );
    assert_eq!(frame.node.id, "implement");
    assert_eq!(frame.step.workflow, "development/default");
    assert_eq!(frame.step.state, "implement");
    assert_eq!(frame.step.index, 2, "the third step of the run");
    assert_eq!(frame.step.attempt, 1, "the first attempt at it");

    assert!(
        frame.prior.is_empty(),
        "nothing had been established when this step was minted"
    );

    // Verbatim, in the words of the document that asked — a driver that summarised here would be
    // the only place the summary existed (§ 5.1).
    assert_eq!(frame.obligations.len(), 1);
    assert_eq!(
        frame.obligations[0].text,
        "the suite is red before the implementation"
    );
    assert_eq!(
        frame.obligations[0].asked_by, None,
        "legal, and worse: this minter attributes no obligation yet, and the consumer must not \
         invent a source for it"
    );

    // The field that exists because of a recorded failure: a run never told what the next state
    // wanted wrote neither of the two things it wanted.
    assert_eq!(frame.reaching.len(), 1);
    assert_eq!(frame.reaching[0].text, "to verify: the suite is green");

    assert!(
        frame.next.is_empty(),
        "this minter names no reachable node on this step"
    );
    assert_eq!(
        frame.handoff,
        Handoff::None,
        "a step that owes nothing says so; an unstated handoff is a step nobody can fail"
    );
    assert_eq!(
        frame.entities, None,
        "this step chooses from no enumeration"
    );

    // The admitted set, in the order the *wire* decides — the rule an external producer follows
    // without reading this crate's enum, and the one the first cross-repository document broke.
    let admitted: Vec<&str> = frame.operations.iter().map(Operation::name).collect();
    assert_eq!(
        admitted,
        [
            "dir.list",
            "file.edit",
            "file.read",
            "file.write",
            "search",
            "shell",
            "skill.load",
        ]
    );
    assert!(frame.operations.admits(&Operation::Shell));
    assert!(frame.operations.admits(&Operation::FileWrite));
    for refused in [
        Operation::WebRead,
        Operation::SubagentSpawn,
        Operation::TaskTodo,
    ] {
        assert!(
            !frame.operations.admits(&refused),
            "`{}` is outside this step's admitted set and the consumer must read it that way — \
             `subagent.spawn` in particular is a route around the per-step admission",
            refused.name()
        );
    }
}

/// The digest the consumer derives from the minted bytes is the value both repositories pin.
///
/// This is the sealing scheme itself under test, not the document: the literal below is what the
/// other side computed from its own transcription of the rule. Two implementations agreeing on it
/// is the whole content of the claim that a frame can be cited by digest across a process boundary.
#[test]
fn the_consumer_derives_the_digest_both_repositories_pin() {
    let frame = Frame::parse_document(GOLDEN).expect("the minted document is accepted");
    assert_eq!(
        frame.digest.as_str(),
        SEALED_DIGEST,
        "the document states the sealed digest"
    );
    assert_eq!(
        frame.computed_digest().as_str(),
        SEALED_DIGEST,
        "and this consumer, re-deriving it from the contents, arrives at the same one"
    );
    assert!(frame.digest_intact());
    assert!(
        GOLDEN.contains(FRAME_FORMAT),
        "the document is self-describing (D2): the tag is in the file, not in a handshake"
    );
}

/// One byte changed, everything else untouched: the document is refused as a digest mismatch.
///
/// `"index": 2` becomes `"index": 3` — the smallest edit that lies about something, since it moves
/// the frame to a step the engine never minted while leaving every word the model is shown intact.
/// A digest that survived this would be decoration, and a frame cited by digest downstream would
/// pin nothing.
#[test]
fn a_single_flipped_byte_in_the_minted_frame_is_refused_as_a_digest_mismatch() {
    let flipped = GOLDEN.replace("\"index\": 2", "\"index\": 3");
    assert_ne!(flipped, GOLDEN, "the mutation reached the document");

    let error = Frame::parse_document(&flipped).expect_err("an edited document is refused");
    let FrameDocError::DigestMismatch { stated, computed } = &error else {
        panic!("expected a digest mismatch, got {error:?}");
    };
    assert_eq!(
        stated.as_str(),
        SEALED_DIGEST,
        "the document still states the digest it was sealed with"
    );
    assert_ne!(stated, computed, "and its contents no longer imply it");
    assert!(
        error.to_string().contains("edited after sealing"),
        "the refusal has to say which of the two happened, or it is a bug report nobody can act \
         on: {error}"
    );
}

/// The document this repository would write for that frame is the document the other one minted.
///
/// The seal is over the canonical form, and the *file* is a second canonical form on top of it —
/// pretty-printed, keys sorted, one trailing newline. Two minters that agreed on the digest and
/// disagreed on the file would still be interoperable, so this is the stronger claim of the two,
/// and it is the one that makes a diff of the fixture readable at all.
#[test]
fn re_emitting_the_minted_frame_reproduces_the_minted_bytes() {
    let frame = Frame::parse_document(GOLDEN).expect("the minted document is accepted");
    assert_eq!(
        frame.to_document(),
        GOLDEN,
        "the round trip through this crate is byte-exact, tag and trailing newline included"
    );
}

/// What the model is shown, from a frame this repository did not build.
///
/// The instruction text is rendered by one function shared by every adapter, so the last thing the
/// seam owes is that a *foreign* frame renders the same way a local one does: verbatim obligations
/// naming who asked, the admitted operations by their wire names, and the digest cited in the text
/// so a frame mutated after the model saw it is detectable in the transcript too.
#[test]
fn the_minted_frame_renders_the_instruction_the_model_would_be_shown() {
    let text = Frame::parse_document(GOLDEN)
        .expect("the minted document is accepted")
        .render_instruction();

    assert!(
        text.contains("Workflow development/default (1), state implement, step 2 attempt 1."),
        "{text}"
    );
    assert!(
        text.contains("- the suite is red before the implementation"),
        "the obligation reaches the model in the words the document asked for it in: {text}"
    );
    assert!(text.contains("- to verify: the suite is green"), "{text}");
    assert!(text.contains("This step owes: nothing."), "{text}");
    assert!(text.contains("- skill.load"), "{text}");
    assert!(
        text.contains(&format!("Frame {SEALED_DIGEST}.")),
        "the rendered text cites the frame it came from: {text}"
    );
}
