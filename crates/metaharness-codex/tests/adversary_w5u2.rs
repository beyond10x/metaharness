//! Adversary pass 1, wave 5 unit 2 (`story:hook-program-quotes-channel-paths`, amendment a37).
//!
//! The scratch `config.toml` is parsed by a real TOML reader and its hook `command` compared with
//! the plan's, for roots carrying every control character, quotes, backslashes, C1 controls and
//! characters outside the BMP. No vendor binary, no credential, no network.

// Debug escapes a newline in a root; Display would split the panic message.
#![allow(clippy::unnecessary_debug_formatting)]

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use metaharness_codex::{LaunchContext, LaunchPlan, hook_program_path, plan_launch};
use metaharness_protocol::{CredentialSource, DecisionMode, Kind, RunSpec};

fn plan(root: &Path) -> LaunchPlan {
    let mut spec = RunSpec::new(Kind::Codex);
    spec.credentials = CredentialSource::None;
    spec.decisions = DecisionMode::Observe;
    spec.model = Some("fixture-model".to_owned());
    spec.model_endpoint = Some("http://127.0.0.1:9".to_owned());
    spec.prompt = Some("go".to_owned());
    let context = LaunchContext {
        context_files: Vec::new(),
        scratch_root: root.to_path_buf(),
        cwd: root.join("work"),
        credentials_file: None,
        inherited_env: BTreeMap::from([
            ("HOME".to_owned(), "/operator".to_owned()),
            ("LANG".to_owned(), "C.UTF-8".to_owned()),
        ]),
        memory_ancestors: Vec::new(),
        inputs_digest: None,
        plugins: Vec::new(),
        loopback: None,
        config_layers: Vec::new(),
    };
    plan_launch(&spec, &context).unwrap_or_else(|refusal| panic!("{root:?}: {refusal}"))
}

fn sh_words(script: &str) -> Vec<String> {
    let output = Command::new("/bin/sh")
        .arg("-c")
        .arg(script)
        .env_clear()
        .env("PATH", "/usr/bin:/bin")
        .output()
        .expect("sh runs");
    assert!(
        output.status.success(),
        "{script:?}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout)
        .expect("UTF-8 words")
        .split_terminator('\0')
        .map(str::to_owned)
        .collect()
}

fn awkward_roots() -> Vec<String> {
    let mut roots: Vec<String> = (0x01u32..0x20)
        .chain([0x7f])
        .chain(0x80..0xa0)
        .map(|code| {
            let character = char::from_u32(code).expect("a scalar");
            format!("/s/a{character}b/.tmpXXXXXX")
        })
        .collect();
    roots.extend(
        [
            "/s/\"\"\"/.tmp",
            "/s/'''/.tmp",
            "/s/\\/.tmp",
            "/s/\\\\\"\\u0041/.tmp",
            "/s/\\n\\t/.tmp",
            "/s/\u{1F600}\u{10FFFF}\u{FEFF}\u{2028}\u{2029}/.tmp",
            "/s/a = \"b\"\n[hooks]\ncommand = 'x'/.tmp",
            "/s/it's \"q\" $HOME `true` a b\nc/.tmp",
        ]
        .map(str::to_owned),
    );
    roots
}

#[test]
fn adversary_w5u2_config_round_trips_every_awkward_root() {
    for root in awkward_roots() {
        let root = PathBuf::from(root);
        let plan = plan(&root);
        let document: toml::Table = plan
            .config
            .parse()
            .unwrap_or_else(|error| panic!("{root:?}: {error}\n{}", plan.config));
        let handlers = document["hooks"]["PreToolUse"]
            .as_array()
            .expect("PreToolUse groups");
        assert_eq!(handlers.len(), 1, "{root:?}");
        let inner = handlers[0]["hooks"].as_array().expect("handlers");
        assert_eq!(inner.len(), 1, "{root:?}");
        let parsed = inner[0]["command"].as_str().expect("a command");
        let planned = plan.hook["hooks"][0]["command"]
            .as_str()
            .expect("a command");
        assert_eq!(parsed, planned, "{root:?}");
        let expected = hook_program_path(&root).to_str().expect("UTF-8").to_owned();
        assert_eq!(
            sh_words(&format!("printf '%s\\0' {parsed}")),
            vec![expected],
            "{root:?}"
        );
    }
}
