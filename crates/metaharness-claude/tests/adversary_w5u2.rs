//! Adversary pass 1, wave 5 unit 2 (`story:hook-program-quotes-channel-paths`, amendment a37).
//!
//! Drives the public surface only: `plan_launch`, `hook_program`, `hook_program_path`. No vendor
//! binary, no credential, no network. `/bin/sh` is the only process started.

// Debug escapes a newline in a root; Display would split the panic message.
#![allow(clippy::unnecessary_debug_formatting)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use metaharness_claude::{
    HookChannelPaths, LaunchContext, LaunchPlan, hook_program, hook_program_path, plan_launch,
};
use metaharness_protocol::{Digest, HermeticMode, Kind, RunSpec};

fn context(root: &Path) -> LaunchContext {
    LaunchContext {
        context_files: Vec::new(),
        scratch_root: root.to_path_buf(),
        cwd: root.join("work"),
        credentials_file: Some(PathBuf::from("/operator/.claude/.credentials.json")),
        inherited_env: BTreeMap::from([("HOME".to_string(), "/operator".to_string())]),
        memory_ancestors: Vec::new(),
        inputs_digest: Some(Digest::of(b"inputs")),
        plugins: Vec::new(),
        marketplace_plugins: Vec::new(),
        loopback: None,
        tool_server: Some(PathBuf::from("/usr/local/bin/metaharness")),
        managed_settings: Vec::new(),
    }
}

fn spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::Claude);
    spec.hermetic = HermeticMode::Strict;
    spec.prompt = Some("go".to_string());
    spec
}

fn plan(root: &Path) -> LaunchPlan {
    plan_launch(&spec(), &context(root)).unwrap_or_else(|refusal| panic!("{root:?}: {refusal}"))
}

fn command(plan: &LaunchPlan) -> String {
    plan.hook["hooks"][0]["command"]
        .as_str()
        .expect("a command string")
        .to_owned()
}

/// What `/bin/sh -c` makes of `script`, as NUL-separated words on stdout.
fn sh_words(script: &str) -> Result<Vec<String>, String> {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("sh runs");
    if !output.status.success() {
        return Err(format!(
            "exit {:?}: {}",
            output.status.code(),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(String::from_utf8(output.stdout)
        .expect("UTF-8 words")
        .split_terminator('\0')
        .map(str::to_owned)
        .collect())
}

/// A small deterministic generator: xorshift64, fixed seed.
struct Roots(u64);

impl Roots {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn below(&mut self, bound: usize) -> usize {
        let bound = u64::try_from(bound).expect("a small bound");
        usize::try_from(self.next() % bound).expect("below a usize bound")
    }

    /// One absolute root built from the pieces the shell, JSON or a quoting rule treat specially.
    fn root(&mut self) -> String {
        const PIECES: [&str; 44] = [
            "'",
            "''",
            "'''",
            "\\",
            "\\'",
            "'\\''",
            "\"",
            "$",
            "$HOME",
            "${x}",
            "$(true)",
            "`true`",
            " ",
            "\t",
            "\n",
            "\r",
            "~",
            "*",
            "?",
            "[a]",
            "{a,b}",
            "!",
            "#",
            ";",
            "&",
            "|",
            "<",
            ">",
            "(",
            ")",
            "-",
            "--",
            "=",
            "%",
            "^",
            "é",
            "\u{1F600}",
            "\u{2028}",
            "/",
            "//",
            "a",
            "run",
            "\u{7f}",
            "\u{1b}",
        ];
        let mut root = String::from("/");
        if self.next().is_multiple_of(3) {
            root.push_str(["-x", "~", "'", "-", "*"][self.below(5)]);
        }
        for _ in 0..=self.below(8) {
            root.push_str(PIECES[self.below(PIECES.len())]);
        }
        root.push_str("/.tmpXXXXXX");
        root
    }
}

/// Property: for 300 generated roots, the vendor `command` is exactly one shell word naming the
/// program, the program parses under `sh -n`, and its two assignments hold the channel paths.
#[test]
fn adversary_w5u2_generated_roots_keep_one_word_and_their_channel() {
    let mut roots = Roots(0x5eed_a37a_37a3_7a37);
    let scratch = tempfile::tempdir().expect("a directory");
    for index in 0..300 {
        let root = PathBuf::from(roots.root());
        let plan = plan(&root);
        let expected = hook_program_path(&root).to_str().expect("UTF-8").to_owned();
        let words = sh_words(&format!("printf '%s\\0' {}", command(&plan)))
            .unwrap_or_else(|error| panic!("{index} {root:?}: {error}"));
        assert_eq!(words, vec![expected], "{index} {root:?}");

        let channel = HookChannelPaths::under(&root);
        let program = hook_program(&channel).expect("a UTF-8 channel");
        let file = scratch.path().join(format!("program-{index}"));
        std::fs::write(&file, &program).expect("written");
        let syntax = Command::new("/bin/sh")
            .arg("-n")
            .arg(&file)
            .output()
            .expect("sh runs");
        assert!(
            syntax.status.success(),
            "{index} {root:?}: {}",
            String::from_utf8_lossy(&syntax.stderr)
        );

        let start = program.find("\nREQUESTS=").expect("an assignment") + 1;
        let end = program.find("\nTICKS=").expect("the budget");
        let assignments = &program[start..end];
        let values = sh_words(&format!(
            "{assignments}\nprintf '%s\\0' \"$REQUESTS\" \"$RESPONSES\""
        ))
        .unwrap_or_else(|error| panic!("{index} {root:?}: {error}"));
        assert_eq!(
            values,
            vec![
                channel.requests.to_str().expect("UTF-8").to_owned(),
                channel.responses.to_str().expect("UTF-8").to_owned()
            ],
            "{index} {root:?}"
        );
    }
}

/// Story outcome: a root either yields a hook that reaches metaharness, or is refused by name
/// before the spawn. On the pinned 2.1.304 the hook runner, for a settings-sourced hook in shell
/// form, throws before spawning anything when `command` contains `${CLAUDE_PLUGIN_ROOT}` or
/// `${CLAUDE_PLUGIN_DATA}` ("Hook command references ${...} but the hook is not associated with a
/// plugin"). The check reads the raw string, so single quotes do not protect the path, and the
/// hook never reaches the channel. The coordinator's decision (F1): the plan refuses such a root
/// by name, `scratch-path-names-plugin-variable`, and names the variable it found.
#[test]
fn adversary_w5u2_a_root_naming_a_plugin_variable_is_refused_by_name() {
    for variable in ["${CLAUDE_PLUGIN_ROOT}", "${CLAUDE_PLUGIN_DATA}"] {
        let root = PathBuf::from(format!("/scratch/{variable}/.tmpAbC123"));
        let refusal = match plan_launch(&spec(), &context(&root)) {
            Err(refusal) => refusal,
            Ok(plan) => panic!(
                "the pinned binary refuses to run this settings hook, so the seam is never \
                 consulted: {:?}",
                command(&plan)
            ),
        };
        let words = refusal.to_string();
        assert!(
            words.starts_with("scratch-path-names-plugin-variable"),
            "{words}"
        );
        assert!(words.contains(variable), "{words}");
    }
}
