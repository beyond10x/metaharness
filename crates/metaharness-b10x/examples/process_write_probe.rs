//! Owned native containment probe, staged as one immutable driver by an opt-in fixture.
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
struct Args {
    #[arg(long)]
    expect_writable: bool,
    #[arg(long)]
    outside_path: PathBuf,
}

fn main() {
    let args = Args::parse();
    // Never run these writes in an ordinary checkout. The native mount's cwd is evidence too.
    assert_eq!(
        std::env::current_dir().unwrap(),
        PathBuf::from("/workspace")
    );
    let mut result = serde_json::Map::new();
    for path in [
        "target/native-fixture-probe",
        "generated/native-fixture-probe",
        "src/native-fixture-probe",
    ] {
        let write = std::fs::write(path, b"owned-native-process-write");
        let expected = args.expect_writable && !path.starts_with("src/");
        result.insert(path.to_owned(), serde_json::json!({"written":write.is_ok(),"errno":write.as_ref().err().and_then(std::io::Error::raw_os_error)}));
        assert_eq!(write.is_ok(), expected, "{path}: {write:?}");
        if !expected {
            assert_eq!(
                write.unwrap_err().raw_os_error(),
                Some(30),
                "must be an observed read-only mount, not a missing directory"
            );
        }
    }
    let outside = std::fs::write(&args.outside_path, b"must-not-escape");
    result.insert("outside".to_owned(), serde_json::json!({"written":outside.is_ok(),"errno":outside.as_ref().err().and_then(std::io::Error::raw_os_error)}));
    assert!(
        outside.is_err(),
        "a host path outside the workspace must remain unwritable"
    );
    println!("{}", serde_json::Value::Object(result));
}
