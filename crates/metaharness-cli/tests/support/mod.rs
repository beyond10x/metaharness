use std::path::{Path, PathBuf};
/// The exact dependency's version and source, shared by fixture and compatibility assertions.
pub fn aep_package() -> &'static serde_json::Value {
    static PACKAGE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    PACKAGE.get_or_init(|| {
        let output = std::process::Command::new(env!("CARGO"))
            .args(["metadata", "--locked", "--format-version", "1"])
            .current_dir(env!("CARGO_MANIFEST_DIR"))
            .output()
            .expect("cargo metadata");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let metadata: serde_json::Value =
            serde_json::from_slice(&output.stdout).expect("metadata JSON");
        metadata["packages"]
            .as_array()
            .expect("packages")
            .iter()
            .find(|p| p["name"] == "aep-cli")
            .expect("pinned AEP library")
            .clone()
    })
}

/// Resolve fixtures from the exact AEP dependency, never an ambient sibling.
pub fn aep_root() -> PathBuf {
    Path::new(aep_package()["manifest_path"].as_str().expect("manifest"))
        .parent()
        .expect("crate")
        .join("../../..")
        .canonicalize()
        .expect("AEP source")
}
