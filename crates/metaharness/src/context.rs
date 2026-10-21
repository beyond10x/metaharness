//! `--context <FILE>` on every kind (design amendment a35): the checks a declaration can fail
//! before anything is read, and the read itself — beneath the `--cwd` tree, through no symbolic
//! link, of tracked regular files, inside both caps.
//!
//! The adapters are pure and render what they are handed; this is the I/O they are handed it
//! from, once, in the core, so every kind refuses the same declaration with the same words.

use std::io::Read as _;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use metaharness_protocol::{
    CONTEXT_BYTES_MAX, CONTEXT_FILES_MAX, ContextContentError, ContextPathError,
    DeclaredContextFile, RunSpec, context_cost, context_path,
};

use crate::refusal::{ContextRefusal, Refusal};

fn refuse(path: &Path, why: ContextRefusal) -> Refusal {
    Refusal::ContextFile {
        path: path.to_path_buf(),
        why,
    }
}

/// The declared path in its recorded form, or the refusal that names it.
fn shaped(declared: &Path) -> Result<String, Refusal> {
    context_path(declared).map_err(|error| {
        refuse(
            declared,
            match error {
                ContextPathError::Empty => ContextRefusal::Empty,
                ContextPathError::Absolute => ContextRefusal::Absolute,
                ContextPathError::Escapes => ContextRefusal::Escapes,
                ContextPathError::NotUtf8 => ContextRefusal::PathNotUtf8,
            },
        )
    })
}

/// What a declaration can be refused for without reading anything: no `--cwd`, a path's shape,
/// the count cap. Part of `check_spec`, so both faces refuse identically.
pub(crate) fn check_declared(spec: &RunSpec) -> Result<(), Refusal> {
    let Some(first) = spec.context.first() else {
        return Ok(());
    };
    if spec.cwd.is_none() {
        return Err(refuse(first, ContextRefusal::WithoutCwd));
    }
    for (index, declared) in spec.context.iter().enumerate() {
        shaped(declared)?;
        if index == CONTEXT_FILES_MAX {
            return Err(refuse(
                declared,
                ContextRefusal::CountCap {
                    declared: spec.context.len(),
                },
            ));
        }
    }
    Ok(())
}

/// Read every declared file beneath `cwd`, in order, or refuse naming the first that cannot be
/// handed over. `cwd` is the resolved (canonical) `--cwd` tree.
///
/// Each file is opened through no symbolic link and never above the tree, checked to be a regular
/// file on the open descriptor, read at most one byte past the byte cap, and checked to be text.
/// Then git is asked once which of them its index names. The bytes returned are the bytes the
/// record digests and the adapter renders: nothing reads the file twice.
pub(crate) fn read_declared(
    spec: &RunSpec,
    cwd: &Path,
) -> Result<Vec<DeclaredContextFile>, Refusal> {
    check_declared(spec)?;
    if spec.context.is_empty() {
        return Ok(Vec::new());
    }
    let tree = std::fs::File::open(cwd).map_err(|error| {
        refuse(
            &spec.context[0],
            ContextRefusal::Unreadable {
                detail: format!("the --cwd tree {} did not open: {error}", cwd.display()),
            },
        )
    })?;
    let mut files = Vec::with_capacity(spec.context.len());
    let mut total = 0_usize;
    for declared in &spec.context {
        let path = shaped(declared)?;
        let file = open_beneath(&tree, &path).map_err(|why| refuse(declared, why))?;
        let metadata = file.metadata().map_err(|error| {
            refuse(
                declared,
                ContextRefusal::Unreadable {
                    detail: error.to_string(),
                },
            )
        })?;
        if !metadata.file_type().is_file() {
            return Err(refuse(declared, ContextRefusal::NotRegularFile));
        }
        let mut content = Vec::new();
        file.take(u64::try_from(CONTEXT_BYTES_MAX).unwrap_or(u64::MAX) + 1)
            .read_to_end(&mut content)
            .map_err(|error| {
                refuse(
                    declared,
                    ContextRefusal::Unreadable {
                        detail: error.to_string(),
                    },
                )
            })?;
        total += context_cost(&path, content.len());
        if total > CONTEXT_BYTES_MAX {
            return Err(refuse(declared, ContextRefusal::ByteCap { bytes: total }));
        }
        let file = DeclaredContextFile::new(path, content).map_err(|error| {
            refuse(
                declared,
                ContextRefusal::NotText {
                    detail: match error {
                        ContextContentError::NotUtf8 { valid_up_to } => {
                            format!("not UTF-8 after byte {valid_up_to}")
                        }
                        ContextContentError::Nul { at } => format!("a NUL at byte {at}"),
                    },
                },
            )
        })?;
        files.push(file);
    }
    untracked(cwd, &spec.context, &files)?;
    Ok(files)
}

/// Open `path` for reading beneath `tree`, refusing any symbolic link on the way (Linux:
/// `openat2` with `RESOLVE_BENEATH | RESOLVE_NO_SYMLINKS`).
///
/// `O_NONBLOCK` because a FIFO opened for reading without it blocks until a writer appears, and
/// the regular-file check can only run on a descriptor that exists. `O_NOCTTY` so a terminal
/// device never becomes the process's controlling terminal on the way to being refused.
#[cfg(any(target_os = "linux", target_os = "android"))]
fn open_beneath(tree: &std::fs::File, path: &str) -> Result<std::fs::File, ContextRefusal> {
    use rustix::fs::{Mode, OFlags, ResolveFlags, openat2};
    openat2(
        tree,
        path,
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY,
        Mode::empty(),
        ResolveFlags::BENEATH | ResolveFlags::NO_SYMLINKS,
    )
    .map(std::fs::File::from)
    .map_err(open_refusal)
}

/// The same refusal on a Unix without `openat2`: every component is opened with `O_NOFOLLOW`
/// from the one before it, so no link is followed and, the path holding no `..`, nothing above
/// the tree is reached.
///
/// A link met as a directory component fails `O_DIRECTORY | O_NOFOLLOW` with `ENOTDIR` on Linux
/// rather than `ELOOP` (observed by forcing this walk on Linux), so a failed component is looked
/// at without following it before the failure is named.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn open_beneath(tree: &std::fs::File, path: &str) -> Result<std::fs::File, ContextRefusal> {
    use rustix::fd::AsFd;
    use rustix::fs::{Mode, OFlags, openat};
    let mut components: Vec<&str> = path.split('/').collect();
    let last = components.pop().unwrap_or(path);
    let mut directory: Option<rustix::fd::OwnedFd> = None;
    for component in components {
        let parent = directory.as_ref().map_or(tree.as_fd(), AsFd::as_fd);
        let next = openat(
            parent,
            component,
            OFlags::RDONLY | OFlags::CLOEXEC | OFlags::DIRECTORY | OFlags::NOFOLLOW,
            Mode::empty(),
        )
        .map_err(|errno| component_refusal(parent, component, errno))?;
        directory = Some(next);
    }
    let parent = directory.as_ref().map_or(tree.as_fd(), AsFd::as_fd);
    let flags =
        OFlags::RDONLY | OFlags::CLOEXEC | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::NOFOLLOW;
    openat(parent, last, flags, Mode::empty())
        .map(std::fs::File::from)
        .map_err(|errno| component_refusal(parent, last, errno))
}

/// Why one component failed to open on the `O_NOFOLLOW` walk: a symbolic link, looked at without
/// following it, or what the platform said.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
fn component_refusal(
    parent: rustix::fd::BorrowedFd<'_>,
    component: &str,
    errno: rustix::io::Errno,
) -> ContextRefusal {
    use rustix::fs::{AtFlags, FileType, statat};
    match statat(parent, component, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::Symlink => {
            ContextRefusal::Symlink
        }
        _ => open_refusal(errno),
    }
}

/// `ELOOP` is a symbolic link met under `RESOLVE_NO_SYMLINKS` or `O_NOFOLLOW`, and `EXDEV` an
/// escape `RESOLVE_BENEATH` stopped; both are the symlink refusal, since the path holds no `..`.
fn open_refusal(errno: rustix::io::Errno) -> ContextRefusal {
    if errno == rustix::io::Errno::LOOP || errno == rustix::io::Errno::XDEV {
        ContextRefusal::Symlink
    } else {
        ContextRefusal::Unreadable {
            detail: std::io::Error::from(errno).to_string(),
        }
    }
}

/// The child environment git gets: constructed, never inherited, so an operator's `GIT_DIR`,
/// `GIT_INDEX_FILE` or `GIT_WORK_TREE` cannot point the question at another index, and no system
/// or global configuration answers it.
const GIT_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// Refuse the first declared file git's index does not name in the tree at `cwd`.
///
/// One `git ls-files -z` over the declared paths, run in `cwd` with literal pathspecs, and every
/// declared path must come back **exactly**: a directory or a glob that matched other files is not
/// the file. git finds the repository as it always does, so a `.git` gitfile naming a sibling git
/// directory — an embedder's scratch copy — answers like any work tree. `--recurse-submodules`
/// (supported with `--cached`) asks each initialised submodule's own index too, which lists its
/// files under the submodule's path, so `sub/lib.py` matches exactly as a superproject file does.
/// A tree git cannot answer for at all (no repository, no git) refuses the first file, with what
/// git said.
///
/// What the environment guarantees (correction round 1, W2): no system or global configuration is
/// read; no object is fetched — `GIT_NO_LAZY_FETCH=1` stops a partial clone's promisor fetch, which
/// expanding a sparse index would otherwise start, and `protocol.allow=never` refuses every
/// transport besides; no fsmonitor program runs. What it does not: the repository's own
/// configuration is still read, and nothing here audits what else it names.
fn untracked(
    cwd: &Path,
    declared: &[PathBuf],
    files: &[DeclaredContextFile],
) -> Result<(), Refusal> {
    let output = Command::new("git")
        .env_clear()
        .env("PATH", GIT_PATH)
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_NO_LAZY_FETCH", "1")
        .env("GIT_ADVICE", "0")
        .env("LC_ALL", "C")
        .arg("--literal-pathspecs")
        .arg("-C")
        .arg(cwd)
        .args([
            "-c",
            "core.fsmonitor=false",
            "-c",
            "protocol.allow=never",
            "ls-files",
            "-z",
            "--cached",
            "--recurse-submodules",
            "--",
        ])
        .args(files.iter().map(DeclaredContextFile::path))
        .stdin(Stdio::null())
        .output();
    let output = match output {
        Ok(output) => output,
        Err(error) => {
            return Err(refuse(
                &declared[0],
                ContextRefusal::Untracked {
                    detail: format!("git did not run on PATH {GIT_PATH}: {error}"),
                },
            ));
        }
    };
    // git's first own word, past any advice a build that predates `GIT_ADVICE` still prints.
    let said = String::from_utf8_lossy(&output.stderr);
    let said = said
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("hint:"));
    if !output.status.success() {
        return Err(refuse(
            &declared[0],
            ContextRefusal::Untracked {
                detail: format!(
                    "git ls-files could not answer for this tree: {}",
                    said.unwrap_or("no message")
                ),
            },
        ));
    }
    let listed: Vec<&[u8]> = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|entry| !entry.is_empty())
        .collect();
    for (declared, file) in declared.iter().zip(files) {
        if !listed.contains(&file.path().as_bytes()) {
            return Err(refuse(
                declared,
                ContextRefusal::Untracked {
                    detail: match said {
                        Some(said) => format!("git ls-files does not list it; git said: {said}"),
                        None => "git ls-files does not list it".to_owned(),
                    },
                },
            ));
        }
    }
    Ok(())
}

/// Write each file's bytes under `<scratch>/context/<path>` for a kind that reads them itself, and
/// return where. The copy is what the loop's own `--context` reads, so it reads the bytes that were
/// digested and not whatever the tree holds by the time it starts.
pub(crate) fn write_copies(
    scratch: &Path,
    files: &[DeclaredContextFile],
) -> Result<Vec<PathBuf>, Refusal> {
    let root = scratch.join("context");
    files
        .iter()
        .map(|file| {
            let copy = root.join(file.path());
            if let Some(parent) = copy.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(&copy, file.text())?;
            Ok(copy)
        })
        .collect()
}
