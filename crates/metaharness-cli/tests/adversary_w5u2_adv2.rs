//! Adversary pass 2 over `story:hook-program-quotes-channel-paths` (amendment a37, fix F1).
//!
//! The unit's own cases prove the plugin-variable refusal through the builder. These drive the
//! built binary, so the exit code and the refusal's first words are read off the process rather
//! than assumed. No vendor binary can run: `HOME` is a scratch directory and `PATH` is
//! `/usr/bin:/bin`, neither of which holds `claude`, so a refusal that failed to fire would end at
//! "not found", never at a spawn.

use std::path::Path;
use std::process::Command;

struct Outcome {
    code: i32,
    stderr: String,
}

fn run_claude_with_tmpdir(home: &Path, tmpdir: &Path) -> Outcome {
    let output = Command::new(env!("CARGO_BIN_EXE_metaharness"))
        .args(["run", "claude", "-p", "go", "--credentials", "none"])
        .env_clear()
        .env("HOME", home)
        .env("PATH", "/usr/bin:/bin")
        .env("TMPDIR", tmpdir)
        .output()
        .expect("the binary runs");
    Outcome {
        code: output.status.code().unwrap_or(-1),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// a37 F1 through the command line: a scratch root under a `TMPDIR` that names
/// `${CLAUDE_PLUGIN_ROOT}` exits 2, the code every other refusal uses, and stderr opens with the
/// refusal's code, so a caller that greps the code (aep drive reads this process) sees it.
#[test]
fn adversary_w5u2_adv2_the_cli_refuses_a_plugin_variable_scratch_root_with_exit_2() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    std::fs::create_dir(&home).expect("home");
    let tmpdir = base.path().join("tmp-${CLAUDE_PLUGIN_ROOT}");
    std::fs::create_dir(&tmpdir).expect("tmpdir");

    let outcome = run_claude_with_tmpdir(&home, &tmpdir);
    assert_eq!(outcome.code, 2, "stderr: {}", outcome.stderr);
    assert!(
        outcome
            .stderr
            .starts_with("metaharness: scratch-path-names-plugin-variable:"),
        "stderr: {}",
        outcome.stderr
    );
    assert_eq!(
        std::fs::read_dir(&tmpdir).expect("tmpdir").count(),
        0,
        "the refused run leaves no scratch root behind"
    );
}

/// The control: the same command under a plain `TMPDIR` does not name the refusal, so the case
/// above discriminates on the variable and not on the missing binary.
#[test]
fn adversary_w5u2_adv2_the_same_cli_run_under_a_plain_tmpdir_is_not_that_refusal() {
    let base = tempfile::tempdir().expect("a scratch directory");
    let home = base.path().join("home");
    std::fs::create_dir(&home).expect("home");
    let tmpdir = base.path().join("tmp-plain");
    std::fs::create_dir(&tmpdir).expect("tmpdir");

    let outcome = run_claude_with_tmpdir(&home, &tmpdir);
    assert!(
        !outcome
            .stderr
            .contains("scratch-path-names-plugin-variable"),
        "stderr: {}",
        outcome.stderr
    );
}
