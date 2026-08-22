//! The claude adapter.
//!
//! Everything claude-specific lives here — how the binary is launched, how its transcript maps
//! onto [`metaharness_protocol::Event`], how steering reaches it, and what hermetic means for
//! it. Nothing outside this crate may know any of that. Unbuilt until the protocol design is
//! accepted; see `docs/design/metaharness-protocol-v0.1.md`.
