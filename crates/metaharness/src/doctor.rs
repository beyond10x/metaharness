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
/// # Errors
///
/// [`Refusal::NoAdapter`] for a kind this build has no adapter for, and [`Refusal::Io`] when the
/// binary is absent or would not run — both exit `2`, because neither is a verdict about a run.
pub fn installed(kind: Kind) -> Result<Installed, Refusal> {
    let program = match kind {
        Kind::Claude => metaharness_claude::ADAPTER_ID,
        Kind::Codex => metaharness_codex::ADAPTER_ID,
    };
    let output = std::process::Command::new(program)
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
    };
    Ok(Installed {
        adapter: program.to_string(),
        program: program.to_string(),
        version: version_token(&reported),
        reported,
        pinned,
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

    /// The codex adapter exists (CX-M1), so `doctor codex` asks the machine rather than
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
