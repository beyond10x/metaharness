//! Adversary pass 2 on `story:session-request-carries-context-files` (design amendment a35): the
//! `channel` the record gives a b10x context file, against what the pinned loop does with it.
//!
//! a35 decision 7: "`channel` ... says what each file became ... what gained system-prompt authority
//! is exactly the files whose channel says so". `ContextChannel::SystemPrompt` is documented as
//! "appended to the harness's system prompt: the file's text carries system-prompt authority";
//! b10x records `harness-context` ("handed to the harness's own context mechanism").
//!
//! At `HARNESS_REVISION` the loop's CLI turns every `--context` path into one context layer
//! (`crates/harness-cli/src/lib.rs`, `push_file_context`: `ContextKind::ProvidedContext`,
//! `ContextTrust::Operator`), and `LoopConfig::with_context` renders the package into
//! `LoopConfig::instructions` — "the standing instruction for the run, sent separately from the
//! person's input", which the wire codecs send as the system prompt. This case builds that layer and
//! runs the pinned loop library's own `with_context` on it. No process, no model.

use b10x_harness_loop::{
    ContextCacheClass, ContextKind, ContextLayer, ContextPackage, ContextTrust, LoopConfig,
};
use metaharness_protocol::ContextChannel;

#[test]
fn a_context_file_the_pinned_loop_puts_in_its_standing_instruction_is_recorded_as_system_prompt() {
    let text = "if a < b && c:\n    pass\n";
    let copy = "/scratch/run/context/todo.py";
    let layer = ContextLayer::new(
        "operator.context.0",
        ContextKind::ProvidedContext,
        ContextTrust::Operator,
        ContextCacheClass::Session,
        text,
    )
    .with_source(copy);
    let config = LoopConfig::new("m", "").with_context(ContextPackage::new(vec![layer]));

    // Observed in the pinned library: the file is the standing instruction, labelled with operator
    // authority, and its `<` and `&` are escaped on the way.
    assert_eq!(
        config.instructions,
        format!(
            "<context kind=\"provided_context\" trust=\"operator\" source=\"{copy}\">\n\
             if a &lt; b &amp;&amp; c:\n    pass\n</context>"
        )
    );

    assert_eq!(
        metaharness_b10x::CONTEXT_CHANNEL,
        ContextChannel::SystemPrompt,
        "the pinned loop puts a --context file in its standing instruction (the system prompt) \
         under trust=\"operator\", so on b10x the file gains system-prompt authority while the \
         record says harness-context; either the channel or a35 decision 7's claim has to move"
    );
}
