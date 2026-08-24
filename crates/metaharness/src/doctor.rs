//! What is installed, against what the adapter was written for.
//!
//! Design § 9.2: *"`doctor` exists because H9 needs an answer before money is spent."* H9 is
//! asserted from the vendor's own opening record — which only exists once a session has been
//! paid for — so a run that is off the pin discovers it after the fact. This verb is the same
//! question asked for free.
//!
//! It lives in the library and not in the binary because the comparison is protocol logic: which
//! versions the adapter pins, and what "off the pin" means, are the adapter's to say (design
//! § 8.4 O1, D11).

use metaharness_protocol::Kind;

use crate::refusal::Refusal;

/// What `doctor` found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installed {
    /// The adapter that was asked about.
    pub adapter: String,
    /// The program that was run.
    pub program: String,
    /// The version string the binary reported, reduced to its version token.
    pub version: String,
    /// Exactly what the binary printed, before any reduction.
    pub reported: String,
    /// The versions this adapter declares it was written against.
    pub pinned: Vec<String>,
}

impl Installed {
    /// Whether the installed version is one the adapter pins.
    #[must_use]
    pub fn on_pin(&self) -> bool {
        self.pinned.contains(&self.version)
    }

    /// The line a person reads.
    #[must_use]
    pub fn render(&self) -> String {
        let pinned = self.pinned.join(", ");
        if self.on_pin() {
            format!(
                "{}: {} {} — on the adapter's pin ({pinned})",
                self.adapter, self.program, self.version
            )
        } else {
            format!(
                "{}: {} {} — OFF the adapter's pin ({pinned}). Every version-pinned claim this \
                 adapter makes was read from {pinned} and is unverified here; --strict-version \
                 refuses the run rather than reporting it",
                self.adapter, self.program, self.version
            )
        }
    }
}

/// Ask the installed vendor binary what version it is.
///
/// **Resolved on the child's `PATH`, not this process's** (CT-3). The two can disagree — Q18's
/// cause was a pacman codex 0.145.0 at `/usr/bin` and an npm codex 0.144.0 at `~/.local/bin`,
/// with the operator's shell putting `/usr/bin` first and the constructed child `PATH` putting
/// `~/.local/bin` first — and the binary this verb blesses must be the binary the spawn will
/// execute, or the pre-flight answers a question nobody asked. The resolved absolute path is
/// reported in [`Installed::program`] so a disagreement between installs is visible as a path,
/// not inferable from a version.
///
/// # Errors
///
/// [`Refusal::NoAdapter`] for a kind this build has no adapter for, and [`Refusal::Io`] when the
/// binary is absent from the child's `PATH` or would not run — both exit `2`, because neither is
/// a verdict about a run.
pub fn installed(kind: Kind) -> Result<Installed, Refusal> {
    let home = std::env::var("HOME").ok();
    let (adapter, child_path) = match kind {
        Kind::Claude => (
            metaharness_claude::ADAPTER_ID,
            metaharness_claude::child_path(home.as_deref()),
        ),
        Kind::Codex => (
            metaharness_codex::ADAPTER_ID,
            metaharness_codex::child_path(home.as_deref()),
        ),
        // The binary is `b10x-harness`, not `b10x`: the adapter id names the *adapter*, and only
        // for this kind do the two differ. Resolved on the ordinary `PATH` because this loop keeps
        // no vendor home to look inside.
        // The `PATH` the *spawn* constructs, not the operator's. Reading the operator's here was
        // the CT-3 mismatch the Claude adapter warns about, arrived at this kind: a pre-flight that
        // blesses a binary the run cannot find is not a pre-flight.
        Kind::B10x => (
            "b10x-harness",
            metaharness_b10x::child_path(std::env::var("HOME").ok().as_deref()),
        ),
    };
    let program = resolve_on(adapter, &child_path)?;
    let program = program.display().to_string();
    let output = std::process::Command::new(&program)
        .arg("--version")
        .output()
        .map_err(|error| Refusal::Io {
            detail: format!("{program} --version could not be run: {error}"),
        })?;
    if !output.status.success() {
        return Err(Refusal::Io {
            detail: format!(
                "{program} --version exited {}: {}",
                output
                    .status
                    .code()
                    .map_or_else(|| "on a signal".to_string(), |code| code.to_string()),
                String::from_utf8_lossy(&output.stderr).trim()
            ),
        });
    }
    let reported = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let pinned: Vec<String> = match kind {
        Kind::Claude => metaharness_claude::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
        Kind::Codex => metaharness_codex::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
        Kind::B10x => metaharness_b10x::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect(),
    };
    Ok(Installed {
        adapter: adapter.to_string(),
        program,
        version: version_token(&reported),
        reported,
        pinned,
    })
}

/// The first executable named `program` on the child's `PATH`, in the child's own order.
///
/// A hand-rolled walk rather than a `which` dependency, because the whole point is to use
/// **exactly** the string the launch plan constructs — a resolver with its own opinions about
/// order would reintroduce the disagreement this exists to close.
fn resolve_on(program: &str, child_path: &str) -> Result<std::path::PathBuf, Refusal> {
    use std::os::unix::fs::PermissionsExt as _;
    for dir in child_path.split(':').filter(|dir| !dir.is_empty()) {
        let candidate = std::path::Path::new(dir).join(program);
        let Ok(meta) = std::fs::metadata(&candidate) else {
            continue;
        };
        if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
            return Ok(candidate);
        }
    }
    Err(Refusal::Io {
        detail: format!(
            "{program} is not on the child's PATH ({child_path}); the operator's own shell \
             finding one somewhere else would not help, because no spawned run could execute it"
        ),
    })
}

/// The version out of whatever the binary printed.
///
/// 2.1.239 prints `2.1.239 (Claude Code)`, so the token is the first whitespace-separated word.
/// Reduced rather than matched whole, because the parenthesised product name is prose the vendor
/// is free to change and a pin that broke on it would be a pin on the wrong thing.
fn version_token(reported: &str) -> String {
    // The first token that starts with a digit: `claude` prints `2.1.239 (Claude Code)` and
    // `codex` prints `codex-cli 0.145.0`, and a picker that took the first word read the
    // second one's own name as its version.
    reported
        .split_whitespace()
        .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))
        .unwrap_or(reported)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn executable_named(dir: &std::path::Path, name: &str, mode: u32) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt as _;
        let path = dir.join(name);
        std::fs::write(&path, b"#!/bin/sh\n").expect("a program body");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(mode)).expect("its mode");
        path
    }

    /// Q18's mechanism, closed: with the same name installed twice, the resolution follows the
    /// **child's** `PATH` order — the string the launch plan constructs — not the shell's.
    #[test]
    fn resolution_follows_the_childs_path_order_not_the_shells() {
        let front = tempfile::TempDir::new().expect("a directory");
        let back = tempfile::TempDir::new().expect("another");
        let wanted = executable_named(front.path(), "vendor", 0o755);
        executable_named(back.path(), "vendor", 0o755);
        let path = format!("{}:{}", front.path().display(), back.path().display());
        assert_eq!(resolve_on("vendor", &path).expect("resolved"), wanted);
    }

    /// A file without an execute bit is not an install: the child's `execvp` would skip it, so
    /// the pre-flight must too, or it blesses a binary the run cannot start.
    #[test]
    fn a_non_executable_file_is_skipped_and_the_next_directory_wins() {
        let front = tempfile::TempDir::new().expect("a directory");
        let back = tempfile::TempDir::new().expect("another");
        executable_named(front.path(), "vendor", 0o644);
        let wanted = executable_named(back.path(), "vendor", 0o755);
        let path = format!("{}:{}", front.path().display(), back.path().display());
        assert_eq!(resolve_on("vendor", &path).expect("resolved"), wanted);
    }

    /// The refusal names the searched `PATH`, because "not found" without the where sends the
    /// operator to `which`, whose answer is the wrong resolution by construction.
    #[test]
    fn an_absent_program_is_refused_naming_the_searched_path() {
        let empty = tempfile::TempDir::new().expect("a directory");
        let path = empty.path().display().to_string();
        let refusal = resolve_on("vendor", &path).expect_err("nothing to find");
        let detail = format!("{refusal:?}");
        assert!(detail.contains(&path), "{detail}");
    }

    #[test]
    fn the_version_token_is_the_first_word_and_not_the_product_name() {
        assert_eq!(version_token("2.1.239 (Claude Code)"), "2.1.239");
        assert_eq!(version_token("2.1.239"), "2.1.239");
        assert_eq!(version_token(""), "");
    }

    #[test]
    fn a_version_the_adapter_pins_is_on_the_pin_and_says_so() {
        let installed = Installed {
            adapter: "claude".to_string(),
            program: "claude".to_string(),
            version: "2.1.239".to_string(),
            reported: "2.1.239 (Claude Code)".to_string(),
            pinned: vec!["2.1.239".to_string()],
        };
        assert!(installed.on_pin());
        assert!(installed.render().contains("on the adapter's pin"));
    }

    /// An off-pin report says what it costs: the adapter's verified rows were read from another
    /// binary, so they are claims about a binary that is not the one installed.
    #[test]
    fn a_version_outside_the_pin_says_what_is_now_unverified() {
        let installed = Installed {
            adapter: "claude".to_string(),
            program: "claude".to_string(),
            version: "2.2.0".to_string(),
            reported: "2.2.0 (Claude Code)".to_string(),
            pinned: vec!["2.1.239".to_string()],
        };
        assert!(!installed.on_pin());
        assert!(installed.render().contains("OFF the adapter's pin"));
        assert!(installed.render().contains("unverified"));
    }

    /// The codex adapter drives a real binary (CX-M2), so `doctor codex` asks the machine rather than
    /// refusing: the answer is the installed version against the pin, or `Io` where no codex
    /// binary is installed — never a session.
    #[test]
    fn codex_is_answered_about_from_the_installed_binary_or_refused_as_io() {
        match installed(Kind::Codex) {
            Ok(installed) => assert_eq!(installed.pinned, vec!["0.145.0".to_string()]),
            Err(refusal) => assert!(matches!(refusal, Refusal::Io { .. })),
        }
    }
}
