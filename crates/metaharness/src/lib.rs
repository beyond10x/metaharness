//! One interface to many agent harnesses.
//!
//! ```ignore
//! Metaharness::new(Kind::Claude).hermetic().run("...").await?;
//! ```
//!
//! The builder is the library face of the same protocol the CLI speaks; both are defined by
//! `docs/design/metaharness-protocol-v0.1.md` and unbuilt until that design is accepted.

/// Which harness drives the run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Claude Code.
    Claude,
    /// Codex.
    Codex,
}
