//! Rust executable fixtures exercise both real spawn runners, without a vendor or model.
use metaharness::protocol::{CredentialSource, DecisionMode, Event, Kind, ProcessTermination};
use metaharness::{
    CodexSpawnRunner, HarnessProcess, Input, LaunchPlanView, ManualClock, Metaharness,
    ProcessRunner, Run, ScriptedSeams, SpawnRunner,
};

#[test]
fn native_child() {
    use std::io::Write;
    let Ok(mode) = std::env::var("METAHARNESS_TEST_NATIVE_EXIT") else {
        return;
    };
    let failed = mode == "terminal-error";
    let record = format!("{{\"emit\":\"session.ended\",\"is_error\":{failed}}}\n");
    if let Some(path) = std::env::var_os("METAHARNESS_TEST_RECORD_FILE") {
        let path = std::path::PathBuf::from(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, record).unwrap();
    } else {
        print!("{record}");
        std::io::stdout().flush().unwrap();
    }
    if mode == "signal" {
        loop {
            std::thread::park();
        }
    }
    std::process::exit(if failed || mode == "wait-error" {
        0
    } else {
        mode.parse().unwrap()
    });
}

struct NativeFixture<'a> {
    mode: &'a str,
    codex: bool,
}
impl ProcessRunner for NativeFixture<'_> {
    fn requires_executable(&self) -> bool {
        false
    }
    fn start(&mut self, plan: &LaunchPlanView<'_>) -> std::io::Result<Box<dyn HarnessProcess>> {
        let program = std::env::current_exe()?;
        let args = vec![
            "--exact".to_owned(),
            "native_child".to_owned(),
            "--nocapture".to_owned(),
        ];
        let mut env = plan.env.clone();
        env.insert(
            "METAHARNESS_TEST_NATIVE_EXIT".to_owned(),
            self.mode.to_owned(),
        );
        if self.codex {
            // Exercise the runner's file transport and cached try_wait status. The contents
            // are neutral ScriptedSeams records, not an alternative vendor decoder.
            let path = std::path::Path::new(env.get("CODEX_HOME").unwrap())
                .join("sessions/rollout-fixture.jsonl");
            env.insert(
                "METAHARNESS_TEST_RECORD_FILE".to_owned(),
                path.display().to_string(),
            );
        }
        let selected = LaunchPlanView {
            program: program.to_str().unwrap(),
            args: &args,
            env: &env,
            ..*plan
        };
        let inner = if self.codex {
            CodexSpawnRunner::new().start(&selected)?
        } else {
            SpawnRunner::new().start(&selected)?
        };
        Ok(Box::new(FixtureProcess {
            inner,
            kill_on_record: self.mode == "signal",
            fail_wait: self.mode == "wait-error",
        }))
    }
}
struct FixtureProcess {
    inner: Box<dyn HarnessProcess>,
    kill_on_record: bool,
    fail_wait: bool,
}
impl HarnessProcess for FixtureProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        let line = self.inner.next_line()?;
        if self.kill_on_record
            && line
                .as_ref()
                .is_some_and(|line| line.contains("session.ended"))
        {
            self.inner.kill()?;
            self.kill_on_record = false;
        }
        Ok(line)
    }
    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        self.inner.write_line(line)
    }
    fn kill(&mut self) -> std::io::Result<()> {
        self.inner.kill()
    }
    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        let status = self.inner.wait()?; // Always reap even in the fault-injection case.
        if self.fail_wait {
            Err(std::io::Error::other("planted unavailable status"))
        } else {
            Ok(status)
        }
    }
    fn termination(&self) -> ProcessTermination {
        self.inner.termination()
    }
}
fn run(mode: &str, codex: bool) -> Run {
    let mut run = Metaharness::new(if codex { Kind::Codex } else { Kind::Claude })
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut NativeFixture { mode, codex },
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .unwrap();
    run.drain().unwrap();
    assert_eq!(
        run.events()
            .iter()
            .filter(|e| matches!(e, Event::StreamClosed { .. }))
            .count(),
        1
    );
    assert!(run.next_event().unwrap().is_none());
    run
}
#[test]
fn actual_child_exit_is_bound_to_the_final_closure() {
    for codex in [false, true] {
        for code in [0, 7] {
            let run = run(&code.to_string(), codex);
            let events = run.events();
            let last = serde_json::to_value(events.last().unwrap()).unwrap();
            assert_eq!(
                last["process"],
                serde_json::json!({"kind":"exited","code":code})
            );
            assert_eq!(last["events"], events.len() - 1);
            assert_eq!(last["reason"], "completed");
            assert_eq!(
                run.exit(None).code(),
                0,
                "transport is distinct from native success"
            );
        }
    }
}
#[test]
fn zero_native_exit_cannot_repair_terminal_failure() {
    for codex in [false, true] {
        let run = run("terminal-error", codex);
        let last = serde_json::to_value(run.events().last().unwrap()).unwrap();
        assert_eq!(
            last["process"],
            serde_json::json!({"kind":"exited","code":0})
        );
        assert_eq!(last["reason"], "error");
        assert_ne!(run.exit(None).code(), 0);
    }
}
#[cfg(unix)]
#[test]
fn killed_actual_child_retains_signal_and_a_single_closure() {
    for codex in [false, true] {
        let run = run("signal", codex);
        let last = serde_json::to_value(run.events().last().unwrap()).unwrap();
        assert_eq!(
            last["process"],
            serde_json::json!({"kind":"signaled","signal":9})
        );
    }
}
#[test]
fn unavailable_wait_status_is_unknown_even_after_valid_terminal_output() {
    for codex in [false, true] {
        let run = run("wait-error", codex);
        assert!(matches!(
            run.events().last(),
            Some(Event::StreamClosed {
                process: ProcessTermination::Unknown,
                ..
            })
        ));
    }
}
#[test]
fn old_closures_and_synthetic_runs_have_unknown_native_status() {
    let old: Event = serde_json::from_value(
        serde_json::json!({"event":"stream.closed","events":1,"reason":"completed","run_id":"old"}),
    )
    .unwrap();
    assert!(matches!(
        old,
        Event::StreamClosed {
            process: ProcessTermination::Unknown,
            ..
        }
    ));
    let mut runner = metaharness::ScriptedRunner::new(vec![], metaharness::ScriptedLog::new());
    let mut run = Metaharness::new(Kind::Claude)
        .with_decisions(DecisionMode::Observe)
        .start_with_clock(
            Input::Prompt("fixture".into()),
            &mut runner,
            &mut ScriptedSeams,
            Box::new(ManualClock::new()),
        )
        .unwrap();
    run.drain().unwrap();
    assert!(matches!(
        run.events().last(),
        Some(Event::StreamClosed {
            process: ProcessTermination::Unknown,
            ..
        })
    ));
    assert_ne!(run.exit(None).code(), 0);
}
