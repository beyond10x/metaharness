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

/// What [`Refusal::AuditorNotInvokable`] says about a prefix with no word in it.
const EMPTY_PREFIX: &str = "the auditor prefix is empty";

/// The `--spec`/`--auditor` pairing, checked before anything is started (design amendment a43).
///
/// `start` calls this after `check_spec` and before the scratch tree, any proxy or the spawn, so a
/// run whose expectation document no auditor would check is refused before a harness process
/// exists or a provider is asked anything. The audit-time checks in [`run_auditor`] stay, and
/// rows 1, 2, 3, 5 and 6 below are the same rules with the same names.
///
/// # Errors
///
/// Exit `2` each, the first that matches:
///
/// 1. [`Refusal::SpecWithoutAuditor`]: `--spec` and no `--auditor`, with or without an audit.
/// 2. [`Refusal::AuditorWithoutSpec`]: `--auditor` and no `--spec`, with or without an audit.
/// 3. [`Refusal::AuditorArgsWithoutAuditor`]: auditor arguments and no `--auditor`, with or
///    without an audit. Nothing would receive them.
/// 4. [`Refusal::Launch`]: both, on a run that wants no audit. The auditor runs only on an audited
///    run, so the document would be checked by nobody.
/// 5. [`Refusal::SpecUnreadable`]: the document's path is not UTF-8, or the document is missing,
///    not a regular file, or cannot be opened.
/// 6. [`Refusal::AuditorNotInvokable`]: the prefix has no word in it, or its first word starts
///    with `-`. The argv is empty, because there is no transcript yet to build one around.
/// 7. [`Refusal::AuditorNotInvokable`]: the prefix's first word names no program this process's
///    `PATH` and file system can start, read as [`ProcessAuditor`] will start it. The argv is
///    empty, as in row 6.
pub(crate) fn check_pairing(spec: &metaharness_protocol::RunSpec) -> Result<(), Refusal> {
    let Some((document, prefix)) = named(
        spec.spec.as_deref(),
        spec.auditor.as_deref(),
        &spec.auditor_args,
    )?
    else {
        return Ok(());
    };
    if !crate::audit::spec_wants_audit(spec) {
        return Err(Refusal::Launch {
            detail: "--spec and --auditor were given on a run that wants no audit: the auditor \
                     runs only on an audited run (--audit or --hermetic strict), so the \
                     expectation document would be checked by nobody. Add --audit, or drop \
                     --spec and --auditor"
                .to_string(),
        });
    }
    check_document(document)?;
    let not_invokable = |detail| Refusal::AuditorNotInvokable {
        argv: Vec::new(),
        detail,
    };
    let program = program_word(prefix).map_err(not_invokable)?;
    check_program(program, std::env::var_os("PATH").as_deref()).map_err(not_invokable)
}

/// Rows 1 to 3 of amendment a43: who was named. `None` when nobody was and nothing is to be
/// audited; the document and the prefix when both were.
fn named<'a>(
    document: Option<&'a Path>,
    prefix: Option<&'a str>,
    arguments: &[String],
) -> Result<Option<(&'a Path, &'a str)>, Refusal> {
    match (document, prefix) {
        (Some(document), Some(prefix)) => Ok(Some((document, prefix))),
        (Some(_), None) => Err(Refusal::SpecWithoutAuditor),
        (None, Some(_)) => Err(Refusal::AuditorWithoutSpec),
        (None, None) if arguments.is_empty() => Ok(None),
        (None, None) => Err(Refusal::AuditorArgsWithoutAuditor),
    }
}

/// The expectation document is a regular file this process can open, named by a UTF-8 path.
///
/// The path comes first: the auditor is handed it as text ([`auditor_argv`]), and a path that is
/// not UTF-8 would reach it as another one. The metadata is read before the open: a directory
/// opens on Linux and must still be refused, and a FIFO would hold the open until something wrote
/// to it. The content is not read; it is the auditor's (design D12).
fn check_document(document: &Path) -> Result<(), Refusal> {
    let refused = |detail: String| Refusal::SpecUnreadable {
        path: document.to_path_buf(),
        detail,
    };
    if document.to_str().is_none() {
        return Err(refused(format!(
            "its path {} is not UTF-8, and the auditor is handed the path as text, in which it \
             would name another file. Name the document by a path that is UTF-8",
            document.as_os_str().as_encoded_bytes().escape_ascii()
        )));
    }
    let metadata = std::fs::metadata(document).map_err(|error| refused(error.to_string()))?;
    if !metadata.is_file() {
        return Err(refused("it is not a regular file".to_string()));
    }
    std::fs::File::open(document).map_err(|error| refused(error.to_string()))?;
    Ok(())
}

/// Row 6: the prefix's first word, the program. A prefix with no word in it names no program, and
/// neither does one whose first word starts with `-`, which a reader of the argv takes for an
/// option. `Err` is the refusal's detail.
fn program_word(prefix: &str) -> Result<&str, String> {
    match prefix.split_whitespace().next() {
        None => Err(EMPTY_PREFIX.to_string()),
        Some(word) if word.starts_with('-') => Err(format!(
            "the auditor prefix starts with {word:?}, which reads as an option and names no \
             program"
        )),
        Some(word) => Ok(word),
    }
}

/// Row 7: the program can be started, found as [`ProcessAuditor`] finds it.
///
/// `ProcessAuditor` starts it with [`std::process::Command::new`], which passes this process's
/// environment on, so `path` is this process's `PATH`. A word with `/` in it is taken as written,
/// relative to the current directory when it is relative. A bare word is looked for in each
/// directory of `path` in order, an empty entry naming the current directory, and a file with no
/// execute bit is passed over. `Err` is the refusal's detail.
fn check_program(word: &str, path: Option<&std::ffi::OsStr>) -> Result<(), String> {
    if word.contains('/') {
        let metadata = std::fs::metadata(word)
            .map_err(|error| format!("the auditor program {word:?} cannot be found: {error}"))?;
        if !metadata.is_file() {
            return Err(format!(
                "the auditor program {word:?} is not a regular file"
            ));
        }
        if !has_execute_bit(&metadata) {
            return Err(format!("the auditor program {word:?} has no execute bit"));
        }
        return Ok(());
    }
    let Some(path) = path else {
        return Err(format!(
            "the auditor program {word:?} has no '/' in it and PATH is not set, so there is no \
             directory to find it in"
        ));
    };
    let found = search_directories(path).any(|directory| {
        std::fs::metadata(directory.join(word))
            .is_ok_and(|metadata| metadata.is_file() && has_execute_bit(&metadata))
    });
    if found {
        Ok(())
    } else {
        Err(format!(
            "the auditor program {word:?} is not an executable regular file in any directory of \
             PATH"
        ))
    }
}

/// The directories of `path` a bare word is looked for in, in order. An empty entry is the
/// current directory, as `execvp` reads it.
fn search_directories(path: &std::ffi::OsStr) -> impl Iterator<Item = std::path::PathBuf> + '_ {
    std::env::split_paths(path).map(|directory| {
        if directory.as_os_str().is_empty() {
            std::path::PathBuf::from(".")
        } else {
            directory
        }
    })
}

/// Any of the three execute bits. Whether this user holds one is the invoker's question.
fn has_execute_bit(metadata: &std::fs::Metadata) -> bool {
    std::os::unix::fs::PermissionsExt::mode(&metadata.permissions()) & 0o111 != 0
}

/// Run the external auditor, if the run asked for one.
///
/// # Errors
///
/// [`Refusal::SpecWithoutAuditor`] when a specification was given and nobody was named to check
/// it, [`Refusal::AuditorWithoutSpec`] for the mirror case, [`Refusal::AuditorArgsWithoutAuditor`]
/// for auditor arguments with nobody to receive them, [`Refusal::SpecUnreadable`] for a document
/// whose path is not UTF-8 or that is missing, not a regular file or cannot be opened,
/// [`Refusal::AuditorNotInvokable`] and [`Refusal::NoVerdictRows`]. Every one of them is exit `2`.
pub fn run_auditor(
    spec_path: Option<&Path>,
    prefix: Option<&str>,
    pass_through: &[String],
    transcript: &Path,
    invoker: &mut dyn AuditorInvoker,
) -> Result<Option<AuditorVerdict>, Refusal> {
    let Some((spec_path, prefix)) = named(spec_path, prefix, pass_through)? else {
        return Ok(None);
    };

    check_document(spec_path)?;

    let argv = auditor_argv(prefix, spec_path, transcript, pass_through);
    // Read off the prefix, not off `argv`: with no word in the prefix, `argv` starts at `--spec`
    // and the invoker would be handed a program by that name.
    if let Err(detail) = program_word(prefix) {
        return Err(Refusal::AuditorNotInvokable { argv, detail });
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

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use super::*;

    /// A file named `name` in `directory`, with this mode.
    fn file(directory: &Path, name: &str, mode: u32) -> PathBuf {
        let path = directory.join(name);
        std::fs::write(&path, "#!/bin/sh\n").expect("written");
        let mut permissions = std::fs::metadata(&path).expect("there").permissions();
        std::os::unix::fs::PermissionsExt::set_mode(&mut permissions, mode);
        std::fs::set_permissions(&path, permissions).expect("mode set");
        path
    }

    fn path_of(directories: &[&Path]) -> OsString {
        std::env::join_paths(directories).expect("joinable")
    }

    #[test]
    fn a_bare_word_is_found_in_the_second_directory_of_path() {
        let first = tempfile::tempdir().expect("a directory");
        let second = tempfile::tempdir().expect("a directory");
        file(second.path(), "auditor", 0o700);
        let path = path_of(&[first.path(), second.path()]);
        assert_eq!(check_program("auditor", Some(&path)), Ok(()));
        assert_eq!(
            check_program("other", Some(&path)),
            Err(
                "the auditor program \"other\" is not an executable regular file in any directory \
                 of PATH"
                    .to_string()
            )
        );
    }

    #[test]
    fn a_file_with_no_execute_bit_is_passed_over_for_one_that_has_it() {
        let first = tempfile::tempdir().expect("a directory");
        let second = tempfile::tempdir().expect("a directory");
        file(first.path(), "auditor", 0o644);
        let only_the_first = path_of(&[first.path()]);
        assert!(check_program("auditor", Some(&only_the_first)).is_err());
        file(second.path(), "auditor", 0o750);
        let both = path_of(&[first.path(), second.path()]);
        assert_eq!(check_program("auditor", Some(&both)), Ok(()));
        // A directory by that name is passed over too.
        let third = tempfile::tempdir().expect("a directory");
        std::fs::create_dir(third.path().join("auditor")).expect("made");
        let directory_first = path_of(&[third.path(), second.path()]);
        assert_eq!(check_program("auditor", Some(&directory_first)), Ok(()));
    }

    /// Asserted on the directory list rather than through a file in the current directory: a
    /// test may not change the process's current directory, and the package root it runs in is
    /// not this test's to write.
    #[test]
    fn an_empty_entry_names_the_current_directory() {
        let directories: Vec<PathBuf> = search_directories(&OsString::from(":/a::/b:")).collect();
        let expected: Vec<PathBuf> = [".", "/a", ".", "/b", "."]
            .into_iter()
            .map(PathBuf::from)
            .collect();
        assert_eq!(directories, expected);
        let empty: Vec<PathBuf> = search_directories(&OsString::new()).collect();
        assert_eq!(
            empty,
            [PathBuf::from(".")],
            "an empty PATH is one empty entry"
        );
    }

    #[test]
    fn a_bare_word_with_no_path_is_refused_and_a_word_with_a_slash_needs_none() {
        assert_eq!(
            check_program("auditor", None),
            Err(
                "the auditor program \"auditor\" has no '/' in it and PATH is not set, so there \
                 is no directory to find it in"
                    .to_string()
            )
        );
        let directory = tempfile::tempdir().expect("a directory");
        let program = file(directory.path(), "auditor", 0o700);
        let program = program.to_str().expect("a UTF-8 scratch path");
        assert_eq!(check_program(program, None), Ok(()));
    }
}
