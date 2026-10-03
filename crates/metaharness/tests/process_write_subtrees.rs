//! Public builder admission and production native argv, with an injected process only.
use metaharness::protocol::{CredentialSource, DecisionMode, Kind, RunSpec};
use metaharness::{Input, Metaharness, ScriptedLog, ScriptedRunner};

fn spec() -> RunSpec {
    let mut spec = RunSpec::new(Kind::B10x);
    spec.credentials = CredentialSource::None;
    spec.decisions = DecisionMode::Observe;
    spec.model = Some("fixture".to_owned());
    spec.model_endpoint = Some("http://fixture.invalid".to_owned());
    spec
}

fn launch(builder: Metaharness) -> Vec<String> {
    let root = tempfile::tempdir().unwrap();
    let cwd = root.path().join("work");
    std::fs::create_dir(&cwd).unwrap();
    let log = ScriptedLog::new();
    let mut runner = ScriptedRunner::of_lines(Vec::<String>::new(), log.clone());
    let mut seams = metaharness_b10x::B10xSeams::new(None, None, None);
    let _run = builder
        .with_cwd(cwd)
        .start_with(Input::Prompt("fixture".to_owned()), &mut runner, &mut seams)
        .unwrap();
    assert_eq!(log.spawns(), 1);
    log.launched().pop().unwrap()
}

#[test]
fn builder_and_run_spec_forward_only_explicit_process_directories() {
    for daemon in [false, true] {
        let mut spec = spec();
        if daemon {
            spec.substrate = Some("/owned-fixture/socket".into());
        } else {
            spec.substrate_embedded = true;
        }
        spec.write_scope = vec!["**=allowed".to_owned()];
        let base = launch(Metaharness::from_spec(spec.clone()));
        assert!(!base.iter().any(|arg| arg == "--process-write-subtree"));
        let builder = launch(
            Metaharness::from_spec(spec.clone())
                .with_process_write_subtree("target")
                .with_process_write_subtree("generated"),
        );
        spec.process_write_subtree = vec!["target".to_owned(), "generated".to_owned()];
        let direct = launch(Metaharness::from_spec(spec));
        for argv in [builder, direct] {
            let paths: Vec<&str> = argv
                .windows(2)
                .filter(|p| p[0] == "--process-write-subtree")
                .map(|p| p[1].as_str())
                .collect();
            assert_eq!(paths, ["target", "generated"]);
        }
    }
}

#[test]
fn declaration_refuses_unsupported_adapters_absent_confinement_and_invalid_paths_before_spawn() {
    for kind in [Kind::Claude, Kind::Codex, Kind::B10x] {
        let mut spec = spec();
        spec.kind = kind;
        spec.process_write_subtree = vec!["target".to_owned()];
        let refusal = metaharness::check_spec(&spec).expect_err("unsupported declaration");
        assert!(
            refusal.to_string().contains("--process-write-subtree"),
            "{refusal}"
        );
    }
    for paths in [
        vec!["../escape"],
        vec!["target", "target/debug"],
        vec!["target/**"],
    ] {
        let mut spec = spec();
        spec.substrate_embedded = true;
        spec.process_write_subtree = paths.into_iter().map(str::to_owned).collect();
        let log = ScriptedLog::new();
        let mut runner = ScriptedRunner::of_lines(Vec::<String>::new(), log.clone());
        let mut seams = metaharness_b10x::B10xSeams::new(None, None, None);
        let result = Metaharness::from_spec(spec).start_with(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
        );
        assert!(result.is_err());
        assert_eq!(log.spawns(), 0);
    }
}
