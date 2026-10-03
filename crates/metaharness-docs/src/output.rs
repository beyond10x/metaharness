//! Stage complete builds and replace only an identified, complete generated site.
use crate::{BASE, Files, check_commit};
use std::{collections::BTreeSet, fs, path::Path};

fn name(path: &Path) -> String {
    path.to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

fn inventory(
    root: &Path,
    directory: &Path,
    files: &Files,
    found: &mut BTreeSet<String>,
) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let relative = name(path.strip_prefix(root).map_err(|e| e.to_string())?);
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_file() && files.contains_key(&relative) {
            found.insert(relative);
        } else if kind.is_dir()
            && files
                .keys()
                .any(|file| file.starts_with(&format!("{relative}/")))
        {
            inventory(root, &path, files, found)?;
        } else {
            return Err(format!(
                "refusing non-generated file, directory or symlink: {}",
                path.display()
            ));
        }
    }
    Ok(())
}

// True means a directory exists and will need a reversible rename. Empty directories
// are acceptable first-build destinations; nonempty ones require the full owned inventory.
fn existing(out: &Path, files: &Files) -> Result<bool, String> {
    let metadata = match fs::symlink_metadata(out) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(error.to_string()),
    };
    if !metadata.is_dir() || metadata.is_symlink() {
        return Err("output must be a directory, never a symlink".into());
    }
    if fs::read_dir(out)
        .map_err(|e| e.to_string())?
        .next()
        .is_none()
    {
        return Ok(true);
    }
    let mut found = BTreeSet::new();
    inventory(out, out, files, &mut found)?;
    if found.len() != files.len() {
        return Err("refusing incomplete generated output inventory".into());
    }
    let manifest: serde_json::Value = serde_json::from_slice(
        &fs::read(out.join(".well-known/b10x-site.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("invalid existing site provenance: {e}"))?;
    if manifest["schema"] != "b10x-project-site/v1"
        || manifest["repository"] != "metaharness"
        || manifest["baseUrl"] != BASE
    {
        return Err("refusing output without Metaharness project-site provenance".into());
    }
    check_commit(
        manifest["commit"]
            .as_str()
            .ok_or("missing existing commit provenance")?,
    )?;
    Ok(true)
}

pub fn write(out: &Path, files: &Files) -> Result<(), String> {
    existing(out, files)?;
    let parent = out
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let staging = tempfile::Builder::new()
        .prefix(".metaharness-docs-")
        .tempdir_in(parent)
        .map_err(|e| e.to_string())?;
    let next = staging.path().join("next");
    for (relative, bytes) in files {
        let path = next.join(relative);
        fs::create_dir_all(path.parent().ok_or("output has no parent")?)
            .map_err(|e| e.to_string())?;
        fs::write(&path, bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    // Recheck after staging: refusal leaves every existing output byte untouched.
    let replace = existing(out, files)?;
    let previous = staging.path().join("previous");
    if replace {
        fs::rename(out, &previous)
            .map_err(|e| format!("could not preserve previous output: {e}"))?;
    }
    if let Err(error) = fs::rename(&next, out) {
        if replace && let Err(restore) = fs::rename(&previous, out) {
            let recovery = staging.keep();
            return Err(format!(
                "publication rename failed ({error}); rollback failed ({restore}); previous output retained at {}",
                recovery.join("previous").display()
            ));
        }
        return Err(format!(
            "publication rename failed; previous output preserved: {error}"
        ));
    }
    Ok(())
}
