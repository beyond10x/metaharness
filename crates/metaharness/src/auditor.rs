//! `--audit`: the pluggable ceiling.
//!
//! metaharness embeds **no expectation language** (design D12). A rival specification language
//! is the same mistake as a rival IR one layer up, and the auditor the design was written for
//! already carries 51 expectation kinds, three verdicts and a severity model.
//!
//! Three things the first draft of the contract got wrong and this module gets right (finding
//! F2):
//!
//! 1. **`--auditor` is an argv prefix and extra arguments pass through.** A single-word program
//!    name is a degenerate prefix; a two-word subcommand is not a special case.
//! 2. **The subject is the raw vendor transcript**, not a trace-ir document. metaharness has the
//!    bytes because § 8.4 O8 requires it to keep them, and the trace-ir document form is Q9's.
//! 3. **An audit that produced no verdict rows is exit `2`.** Everything the reference auditor
//!    rejects about *itself* also leaves as `1`, so an empty table must not be read as a
//!    contradiction.
//!
//! The invocation is behind a trait with a fake, so argv construction and exit-code mapping are
//! tested with no process spawn.

use std::path::Path;

use crate::refusal::Refusal;

/// What one auditor invocation produced.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AuditorRun {
    /// The exit code, or `None` when a signal took it.
    pub exit_code: Option<i32>,
    /// Everything it wrote to stdout.
    pub stdout: String,
    /// Everything it wrote to stderr, kept so a setup failure can say what the auditor said.
    pub stderr: String,
}

/// The auditor's verdict as metaharness records it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditorVerdict {
    /// The argv metaharness constructed, so a report can be reproduced by hand.
    pub argv: Vec<String>,
    /// The auditor's exit code.
    pub exit_code: Option<i32>,
    /// How many verdict rows it produced. Zero is a setup failure, never a verdict.
    pub verdict_rows: u32,
}

/// How an auditor is invoked.
pub trait AuditorInvoker {
    /// Run this argv and report what it did.
    ///
    /// # Errors
    ///
    /// Whatever the platform said. A missing or unexecutable auditor is exit `2`: metaharness
    /// could not do its job.
    fn invoke(&mut self, argv: &[String]) -> std::io::Result<AuditorRun>;
}

/// The real invoker: one child process.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessAuditor;

impl AuditorInvoker for ProcessAuditor {
    fn invoke(&mut self, argv: &[String]) -> std::io::Result<AuditorRun> {
        let Some((program, arguments)) = argv.split_first() else {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "the auditor prefix is empty",
            ));
        };
        let output = std::process::Command::new(program)
            .args(arguments)
            .output()?;
        Ok(AuditorRun {
            exit_code: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// An invoker that answers from a script and records what it was asked. **Test support.**
///
/// Public because the exit-code mapping is the part of `--audit` most worth asserting on, and a
/// fake that lives in a `#[cfg(test)]` module cannot be borrowed by an embedder's own tests.
#[derive(Debug, Default)]
pub struct FakeAuditor {
    answers: Vec<std::io::Result<AuditorRun>>,
    calls: Vec<Vec<String>>,
}

impl FakeAuditor {
    /// An auditor that answers this, once.
    #[must_use]
    pub fn answering(run: AuditorRun) -> Self {
        Self {
            answers: vec![Ok(run)],
            calls: Vec::new(),
        }
    }

    /// An auditor that is not there.
    #[must_use]
    pub fn missing() -> Self {
        Self {
            answers: vec![Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no such program",
            ))],
            calls: Vec::new(),
        }
    }

    /// Every argv it was asked to run.
    #[must_use]
    pub fn calls(&self) -> &[Vec<String>] {
        &self.calls
    }
}

impl AuditorInvoker for FakeAuditor {
    fn invoke(&mut self, argv: &[String]) -> std::io::Result<AuditorRun> {
        self.calls.push(argv.to_vec());
        if self.answers.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "the fake auditor has no answer left",
            ));
        }
        self.answers.remove(0)
    }
}

/// The full invocation: `<prefix…> --spec <spec> --transcript <path> [pass-through…]`.
///
/// The prefix is split on whitespace, so `protocol observe trace check` is four words and
/// `protocol` is one; the pass-through goes **last**, after everything metaharness adds, because that is where
/// an auditor's own options have to be for its parser to see them.
#[must_use]
pub fn auditor_argv(
    prefix: &str,
    spec: &Path,
    transcript: &Path,
    pass_through: &[String],
) -> Vec<String> {
    let mut argv: Vec<String> = prefix.split_whitespace().map(ToString::to_string).collect();
    argv.push("--spec".to_string());
    argv.push(spec.display().to_string());
    argv.push("--transcript".to_string());
    argv.push(transcript.display().to_string());
    argv.extend(pass_through.iter().cloned());
    argv
}

/// How many verdict rows an auditor produced.
///
/// **metaharness cannot read an auditor's table and does not pretend to.** It counts non-blank
/// stdout lines, and the one thing it needs from that number is whether it is zero: an auditor
/// that said nothing judged nothing, and a table with nothing in it would otherwise go green —
/// or red — while checking nothing.
#[must_use]
pub fn count_verdict_rows(stdout: &str) -> u32 {
    u32::try_from(
        stdout
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count(),
    )
    .unwrap_or(u32::MAX)
}

/// Run the external auditor, if the run asked for one.
///
/// # Errors
///
/// [`Refusal::SpecWithoutAuditor`] when a specification was given and nobody was named to check
/// it, [`Refusal::AuditorWithoutSpec`] for the mirror case, [`Refusal::SpecUnreadable`],
/// [`Refusal::AuditorNotInvokable`] and [`Refusal::NoVerdictRows`]. Every one of them is exit
/// `2`.
pub fn run_auditor(
    spec_path: Option<&Path>,
    prefix: Option<&str>,
    pass_through: &[String],
    transcript: &Path,
    invoker: &mut dyn AuditorInvoker,
) -> Result<Option<AuditorVerdict>, Refusal> {
    let (spec_path, prefix) = match (spec_path, prefix) {
        (None, None) => return Ok(None),
        (Some(_), None) => return Err(Refusal::SpecWithoutAuditor),
        (None, Some(_)) => return Err(Refusal::AuditorWithoutSpec),
        (Some(spec_path), Some(prefix)) => (spec_path, prefix),
    };

    if let Err(error) = std::fs::File::open(spec_path) {
        return Err(Refusal::SpecUnreadable {
            path: spec_path.to_path_buf(),
            detail: error.to_string(),
        });
    }

    let argv = auditor_argv(prefix, spec_path, transcript, pass_through);
    if argv.first().is_none_or(String::is_empty) {
        return Err(Refusal::AuditorNotInvokable {
            argv,
            detail: "the auditor prefix is empty".to_string(),
        });
    }

    let run = invoker
        .invoke(&argv)
        .map_err(|error| Refusal::AuditorNotInvokable {
            argv: argv.clone(),
            detail: error.to_string(),
        })?;

    let verdict_rows = count_verdict_rows(&run.stdout);
    if verdict_rows == 0 {
        return Err(Refusal::NoVerdictRows { argv });
    }

    Ok(Some(AuditorVerdict {
        argv,
        exit_code: run.exit_code,
        verdict_rows,
    }))
}
