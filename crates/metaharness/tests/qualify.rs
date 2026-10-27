//! `story:qualify-a-binary-without-a-model-call`: a binary's floor rows from its `--version`
//! alone. The vendor binaries are scripts this file writes into a temporary directory; no model,
//! no network and no credential is involved, and each script records every invocation it gets.

mod qualify {
    use std::os::unix::fs::PermissionsExt as _;
    use std::path::PathBuf;

    use metaharness::protocol::{
        CredentialSource, HermeticMode, HermeticRow, Kind, Severity, Verdict,
    };
    use metaharness::{
        ClaudeSeams, Input, ManualClock, Metaharness, Qualification, Refusal, Run, ScriptStep,
        ScriptedLog, ScriptedRunner, qualify,
    };

    /// A vendor stand-in: prints `banner` for `--version` and appends every argument list it is
    /// run with to `calls`, one line per invocation.
    struct Fake {
        _dir: tempfile::TempDir,
        program: PathBuf,
        calls: PathBuf,
    }

    impl Fake {
        fn new(name: &str, banner: &str) -> Self {
            let dir = tempfile::tempdir().expect("a directory");
            let calls = dir.path().join("calls.log");
            let program = dir.path().join(name);
            std::fs::write(
                &program,
                format!(
                    "#!/bin/sh\necho \"$*\" >> '{}'\necho '{banner}'\n",
                    calls.display()
                ),
            )
            .expect("the script");
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o700))
                .expect("executable");
            Self {
                _dir: dir,
                program,
                calls,
            }
        }

        fn program(&self) -> &str {
            self.program.to_str().expect("a utf-8 path")
        }

        /// The argument lists the script was run with, in order.
        fn calls(&self) -> Vec<String> {
            std::fs::read_to_string(&self.calls)
                .unwrap_or_default()
                .lines()
                .map(str::to_owned)
                .collect()
        }
    }

    /// Qualify the fake, retrying the one transient a freshly written script can raise when
    /// another test thread forks while it is still open (`ETXTBSY`): the script never ran, so it
    /// recorded nothing and the retry is the first call.
    fn qualified(kind: Kind, fake: &Fake, hermetic: HermeticMode) -> Qualification {
        let mut last = None;
        for _ in 0..20 {
            let found = qualify(kind, fake.program(), hermetic).expect("the fake resolves");
            if found.version.is_some() {
                return found;
            }
            last = Some(found);
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        panic!("the fake never reported a version: {last:?}");
    }

    fn row(found: &Qualification, which: HermeticRow) -> &metaharness::QualifiedRow {
        found
            .rows
            .iter()
            .find(|row| row.row.row == which)
            .unwrap_or_else(|| panic!("{} is not in {:?}", which.id(), found.rows))
    }

    #[test]
    fn an_unpinned_claude_gates_on_h11_under_strict() {
        let fake = Fake::new("claude", "2.1.311 (Claude Code)");
        let found = qualified(Kind::Claude, &fake, HermeticMode::Strict);

        assert_eq!(found.version.as_deref(), Some("2.1.311"));
        let h9 = row(&found, HermeticRow::H9);
        assert_eq!(h9.row.verdict, Verdict::Gap, "{}", h9.row.detail);
        assert!(!h9.gates, "H9 is advisory since a62: {}", h9.row.detail);
        assert!(found.strict_version_refuses);

        // The floor reads an unmeasured binary as `unk`, never `gap` (a62): nobody found out.
        // The store text for this story says `gap`; the floor this crate ships says `unk`, and
        // either one is a gating row that is not `ok`.
        let h11 = row(&found, HermeticRow::H11);
        assert_eq!(h11.row.verdict, Verdict::Unk, "{}", h11.row.detail);
        assert_eq!(h11.row.severity, Severity::Gating);
        assert!(h11.gates, "{}", h11.row.detail);
        assert!(h11.row.detail.contains("2.1.311"), "{}", h11.row.detail);
        assert!(!found.qualified());
        assert_eq!(
            found
                .failing()
                .map(|failing| failing.row.row)
                .collect::<Vec<_>>(),
            vec![HermeticRow::H11]
        );

        // The same binary does not gate under a mode that only reports.
        let reported = qualified(Kind::Claude, &fake, HermeticMode::On);
        assert!(reported.qualified());
        assert!(!row(&reported, HermeticRow::H11).gates);

        // Every probe was a `--version`, and nothing else ran the binary.
        let calls = fake.calls();
        assert_eq!(
            calls.len(),
            2,
            "one call for each of the two qualifications: {calls:?}"
        );
        assert!(calls.iter().all(|call| call == "--version"), "{calls:?}");
    }

    #[test]
    fn an_unpinned_claude_makes_exactly_one_version_call_and_no_other() {
        let fake = Fake::new("claude", "2.1.311 (Claude Code)");
        let found = qualified(Kind::Claude, &fake, HermeticMode::Strict);
        assert_eq!(found.version.as_deref(), Some("2.1.311"));
        assert_eq!(fake.calls(), vec!["--version".to_owned()]);
    }

    #[test]
    fn a_pinned_claude_is_ok() {
        for pinned in metaharness_claude::PINNED_VERSIONS {
            let fake = Fake::new("claude", &format!("{pinned} (Claude Code)"));
            let found = qualified(Kind::Claude, &fake, HermeticMode::Strict);
            assert!(found.qualified(), "{pinned}: {:?}", found.rows);
            assert!(!found.strict_version_refuses);
            for which in [HermeticRow::H9, HermeticRow::H11] {
                let checked = row(&found, which);
                assert_eq!(checked.row.verdict, Verdict::Ok, "{pinned}: {checked:?}");
                assert!(!checked.gates);
            }
        }
    }

    const RESULT: &str = r#"{"type":"result","subtype":"success","is_error":false}"#;

    /// The floor a strict Claude run over a declared scratch copy reads for an opening record on
    /// `version`: the launch's own attestation held to the record.
    fn launch_floor(version: &str) -> Vec<metaharness::protocol::RowVerdict> {
        let root = tempfile::tempdir().expect("a root");
        let copy = root.path().join("copy");
        std::fs::create_dir(&copy).expect("the copy");
        let init = format!(
            r#"{{"type":"system","subtype":"init","claude_code_version":"{version}","cwd":{},"tools":[],"mcp_servers":[],"output_style":"default","apiKeySource":"none","plugins":[]}}"#,
            serde_json::Value::from(copy.display().to_string())
        );
        let mut runner = ScriptedRunner::new(
            vec![ScriptStep::line(init), ScriptStep::line(RESULT)],
            ScriptedLog::new(),
        );
        let mut run: Run = Metaharness::new(Kind::Claude)
            .with_credentials(CredentialSource::None)
            .with_model_endpoint("http://127.0.0.1:1")
            .with_model("synthetic-never-called")
            .with_hermetic(HermeticMode::Strict)
            .with_cwd(&copy)
            .with_scratch_cwd()
            .start_with_clock(
                Input::Prompt("synthetic".into()),
                &mut runner,
                &mut ClaudeSeams,
                Box::new(ManualClock::new()),
            )
            .expect("the launch");
        run.drain().expect("drains");
        run.hermetic_floor()
    }

    #[test]
    fn the_rows_match_the_launch() {
        let mut versions: Vec<String> = metaharness_claude::PINNED_VERSIONS
            .iter()
            .map(ToString::to_string)
            .collect();
        versions.push("2.1.311".to_owned());
        for version in versions {
            let fake = Fake::new("claude", &format!("{version} (Claude Code)"));
            let found = qualified(Kind::Claude, &fake, HermeticMode::Strict);
            let floor = launch_floor(&version);
            assert_eq!(found.rows.len(), 2, "{:?}", found.rows);
            for qualifying in &found.rows {
                let launched = floor
                    .iter()
                    .find(|launched| launched.row == qualifying.row.row)
                    .expect("the launch evaluates the row");
                assert_eq!(
                    (launched.verdict, launched.severity),
                    (qualifying.row.verdict, qualifying.row.severity),
                    "{version} {}: launch {:?} against qualify {:?}",
                    launched.row.id(),
                    launched.detail,
                    qualifying.row.detail
                );
                assert_eq!(
                    qualifying.gates,
                    launched.gates(),
                    "{version} {}",
                    launched.row.id()
                );
            }
        }
    }

    #[test]
    fn a_missing_binary_is_named() {
        let program = "metaharness-qualify-test-no-such-claude";
        let Err(refusal) = qualify(Kind::Claude, program, HermeticMode::Strict) else {
            panic!("a program that resolves to nothing qualified");
        };
        let Refusal::Io { detail } = &refusal else {
            panic!("{refusal:?}");
        };
        let searched = metaharness_claude::child_path(std::env::var("HOME").ok().as_deref());
        assert!(detail.contains(program), "{detail}");
        assert!(detail.contains(&searched), "{detail}");
    }

    #[test]
    fn a_binary_with_no_readable_version_does_not_qualify() {
        let fake = Fake::new("claude", "");
        let mut found = None;
        for _ in 0..20 {
            let attempt = qualify(Kind::Claude, fake.program(), HermeticMode::Strict).unwrap();
            if !fake.calls().is_empty() {
                found = Some(attempt);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        let found = found.expect("the script ran");
        assert_eq!(found.version, None);
        assert!(!found.qualified(), "{:?}", found.rows);
        assert!(found.strict_version_refuses);
        assert_eq!(row(&found, HermeticRow::H9).row.verdict, Verdict::Unk);
        assert_eq!(row(&found, HermeticRow::H11).row.verdict, Verdict::Unk);
    }

    #[test]
    fn codex_rows_match_its_pins() {
        for pinned in metaharness_codex::PINNED_VERSIONS {
            let fake = Fake::new("codex", &format!("codex-cli {pinned}"));
            let found = qualified(Kind::Codex, &fake, HermeticMode::Strict);
            let h9 = row(&found, HermeticRow::H9);
            assert_eq!(h9.row.verdict, Verdict::Ok, "{pinned}: {}", h9.row.detail);
            assert!(!found.strict_version_refuses);
            assert!(found.qualified());
        }

        let fake = Fake::new("codex", "codex-cli 0.164.0");
        let found = qualified(Kind::Codex, &fake, HermeticMode::Strict);
        let h9 = row(&found, HermeticRow::H9);
        assert_eq!(h9.row.verdict, Verdict::Gap, "{}", h9.row.detail);
        assert_eq!(h9.row.severity, Severity::Advisory);
        assert!(!h9.gates, "H9 gates through --strict-version only");
        assert!(found.strict_version_refuses);
        assert!(found.qualified());
        assert!(
            found.rows.iter().all(|row| row.row.row != HermeticRow::H11),
            "Codex records no memory canary, so no version-keyed H11: {:?}",
            found.rows
        );
    }
}
