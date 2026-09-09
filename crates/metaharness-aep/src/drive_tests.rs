#[cfg(test)]
mod tests {
    use aep_cli::drive::on_path;
    use aep_domain::ids::ExecutionId;
    use aep_engine::{Engine, Registry};
    use aep_engine::policy::Decision;
use super::*;
use aep_domain::capability::Environment;
use aep_domain::ids::StateId;
fn config(capabilities: &[Capability]) -> ToolConfig {
        ToolConfig::new(capabilities.iter().cloned().collect())
    }
/// The execution every fixture below belongs to: the first run of task `T-1`.
    ///
    /// `'static` because two of the helpers here *return* a [`StepContext`], and a context borrows
    /// its execution for as long as it lives.
    fn driven_execution() -> &'static ExecutionId {
        static EXECUTION: std::sync::OnceLock<ExecutionId> = std::sync::OnceLock::new();
        EXECUTION.get_or_init(|| ExecutionId::new("T-1.1").expect("an execution id"))
    }
/// One `llm` step, as a step map that names the second harness would produce it.
    ///
    /// The harness is spelled as a literal rather than through a constant, deliberately: this is
    /// the string a step map author writes, and a test that read it out of the same constant the
    /// selector reads would pass whatever that constant said.
    /// A one-step map whose `llm` step names the native harness.
    fn b10x_map() -> StepMap {
        aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/b10x\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n\
             \x20       harness: b10x\n",
            None,
        )
        .expect("the map validates")
    }
fn b10x_step() -> LlmStep {
        LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: "b10x".to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        }
    }
/// A step context with nothing outstanding, for a test that is about the surface.
    fn step_context<'a>(
        tools: &'a ToolConfig,
        state: &'a StateId,
        task: &'a aep_domain::task::Task,
    ) -> StepContext<'a> {
        StepContext {
            execution: driven_execution(),
            task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state,
            index: 0,
            attempt: 1,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &[],
            reaching: &[],
            preceding_llm: None,
        }
    }
/// The task a prompt test's run is driving.
    ///
    /// `derived_from` is populated because the identity line names the artifacts, and a fixture
    /// without one would let the line pass by saying nothing.
    fn driven_task() -> aep_domain::task::Task {
        aep_schema::parse::task(
            "id: T-1\nkind: feature\nobjective: drive something\nprotocol: aep/1\n\
             profile: test.standard\nderived_from: [story:the-one-being-driven]\n",
            None,
        )
        .expect("the fixture task parses")
    }
/// The prompt the driver would build for one `llm` step.
    fn prompt_with_skills(skills: &[&str]) -> String {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: skills.iter().map(ToString::to_string).collect(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "specify".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        prompt_for(&step, &context, "/toolchain/driver/aep")
    }
/// The session's `PATH` is metaharness's constructed one, and this pre-flight resolves on it.
    ///
    /// Pinned against that crate's `child_path` by construction rather than by dependency — the
    /// arrow in `adr/0002` runs one way and metaharness is not on it. If they drift, a driven run
    /// is refused when it would have worked, or worse, started when it cannot: both are cheaper to
    /// find here than at $1 a state.
    #[test]
    fn a_session_path_matches_what_metaharness_constructs() {
        let path = session_path();
        assert!(
            path.ends_with("/usr/local/bin:/usr/bin:/bin"),
            "metaharness's BASE_PATH is the tail of the constructed PATH: {path}"
        );
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty() {
                assert!(
                    path.starts_with(&format!("{home}/.local/bin:")),
                    "and `$HOME/.local/bin` is the head, which is the only directory an operator \
                     can install into without root: {path}"
                );
            }
        assert!(
            !path.contains("target/debug"),
            "the session never sees this repository's build directory, which is the whole point: \
             {path}"
        );
        assert!(
            !path.contains(".cargo/bin"),
            "nor cargo's own install root — which is why the refusal says `--root ~/.local`: {path}"
        );
    }
/// The prompt names the state's tools, from the same value the policy refuses on.
    ///
    /// **Run `W4-3/1`, 2026-08-29.** Every session spent a turn calling a tool it did not have —
    /// and the first attempt was a `ToolSearch` for `Grep` and `Glob`, which is a session trying to
    /// *load* what it had been told existed. Nothing told it. `decide_tool` prints the admitted set
    /// in its refusal, so the surface was knowable the whole time and only reachable by being
    /// refused: an allowlist a session learns by trial is an allowlist that costs a turn per state.
    ///
    /// Rendered from `context.tools` rather than from a second list, because two lists drift and
    /// the model would then trust the wrong one.
    #[test]
    fn the_prompt_names_the_tools_the_state_admits_and_the_policy_agrees() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context, "/toolchain/driver/aep");
        let admitted = allowed_tools(&tools);
        assert!(!admitted.is_empty(), "the fixture admits something to name");
        for tool in &admitted {
            assert!(
                prompt.contains(tool.as_str()),
                "`{tool}` is admitted and the session is not told: {prompt}"
            );
        }
        assert!(
            prompt.contains("there are no others"),
            "the list is stated as closed, or it reads as a suggestion: {prompt}"
        );

        // The two must come from one source. A tool the prompt names and the policy refuses would
        // be worse than saying nothing — it would be an instruction to do what will be denied.
        for tool in &admitted {
            assert!(
                decide_tool(&context, no_scope(), tool, &serde_json::json!({})).is_ok()
                    || tool == "Bash"
                    || matches!(tool.as_str(), "Edit" | "Write" | "NotebookEdit"),
                "the prompt names `{tool}` and the policy refuses it"
            );
        }
    }
/// A step that names `b10x` is told **that** harness's tool names, not Claude Code's.
    ///
    /// The rendering half of § 4.9 point 2, in the place it is most expensive to get wrong. The
    /// decision about which capabilities admit which operations is shared and is
    /// `aep_driver::tool::tool_config`'s; only the naming table is the harness's. A prompt that
    /// named `Read`, `Edit` and `Bash` to the b10x loop would be naming six tools that do not
    /// exist in its catalogue — which is exactly the class of waste this executor was added to
    /// remove, reintroduced by the driver itself.
    ///
    /// The b10x names are `b10x_harness_tools::entry_names`', read from the loop's own catalogue:
    /// `file_read`, `file_write`, `file_edit`, `dir_list`, `search`, `run`.
    #[test]
    fn a_b10x_step_is_told_the_b10x_catalogues_names_and_never_claude_codes() {
        let prompt = prompt_for(
            &b10x_step(),
            &step_context(
                &config(&[
                    Capability::RepositoryRead,
                    Capability::RepositoryWrite,
                    Capability::CommandExecution,
                ]),
                &"implement".parse().expect("a state id"),
                &driven_task(),
            ),
            "/toolchain/driver/aep",
        );
        for named in [
            "file_read",
            "file_write",
            "file_edit",
            "dir_list",
            "search",
            "run",
        ] {
            assert!(
                prompt.contains(named),
                "`{named}` is in the b10x catalogue and this state admits it: {prompt}"
            );
        }
        for vendor in ["`Read`", "`Edit`", "`Write`", "`Glob`", "`Grep`", "`Bash`"] {
            assert!(
                !prompt.contains(vendor),
                "{vendor} is Claude Code's name and no b10x session has one: {prompt}"
            );
        }
    }
/// A state that admits reading can read at scale, and still cannot write by any route.
    ///
    /// **The gap this closes.** `repository.read` renders `Glob` and `Grep`; Claude Code 2.1.247
    /// offers neither, and its own error tells the model to *search file contents with `grep` via
    /// the Bash tool instead* — which `driven_surface` refused. So a driven session was told to do
    /// the one thing the driver denied, and run `W4-3/1` spent 19 calls discovering that and never
    /// searched anything.
    ///
    /// The widening is safe only because composition and redirection are refused before this rule
    /// is reached, so this asserts both halves: the readers are admitted, and every route from a
    /// reader to a written byte is still closed.
    #[test]
    fn a_reading_state_may_read_at_scale_and_still_cannot_write_by_any_route() {
        let _step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "find it".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        let bash = |command: &str| {
            decide_tool(
                &context,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": command }),
            )
        };

        for reading in [
            "grep -rn DriverOptions crates/",
            "rg --files crates/drive/aep-driver",
            "ls .engineering/planning",
            "cat README.md",
            "head -40 crates/drive/aep-driver/src/run.rs",
            "wc -l Cargo.toml",
        ] {
            assert!(
                bash(reading).is_ok(),
                "`{reading}` only reads and the state admits reading"
            );
        }

        // Every route from a reader to a byte on disk. The first four are refused by the
        // composition rule; the rest are programs that write with no help from a shell.
        for writing in [
            "grep -rn x crates/ > out.txt",
            "cat a.md >> b.md",
            "ls | tee out.txt",
            "cat a && rm b",
            "sed -i s/a/b/ Cargo.toml",
            "awk '{print > \"out\"}' a",
            "find . -delete",
            "xargs rm",
            "sh -c 'rm -rf x'",
            "env rm x",
        ] {
            assert!(
                bash(writing).is_err(),
                "`{writing}` reaches a write and must be refused"
            );
        }

        // And a state with no read capability gets none of them.
        let blind = config(&[Capability::CommandExecution]);
        let deaf = StepContext {
            execution: driven_execution(),
            tools: &blind,
            ..context
        };
        assert!(
            decide_tool(
                &deaf,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": "grep -r x ." })
            )
            .is_err(),
            "a state that does not admit reading does not get a reader"
        );
    }
/// The prompt states every rule the shell will refuse on, and the policy agrees with it.
    ///
    /// **Measured on run `W4-3/1`, 2026-08-29: 28 of 174 tool calls — 16% of everything the run did
    /// — were refused, and every one was a rule the session could have been told.** Eleven were a
    /// program outside the surface, ten were a composed command, four were a tool the harness does
    /// not have, three were a tool the state does not admit. They recurred in every state from the
    /// first to the last, because being refused teaches one call and the next session starts fresh.
    ///
    /// This asserts the prompt names both shell rules, and — the part that matters — that the
    /// *examples it gives* are genuinely refused by `driven_surface`. A prompt that warned about a
    /// command the policy allows would train the session out of something it may do.
    #[test]
    fn the_prompt_states_the_shell_rules_the_policy_will_refuse_on() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "do the thing".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        let prompt = prompt_for(&step, &context, "/toolchain/driver/aep");

        assert!(
            prompt.contains("one simple invocation per call"),
            "the composed-command rule is stated: {prompt}"
        );
        assert!(
            prompt.contains("protocol plan artifact") && prompt.contains("protocol observe trace"),
            "and the two verb families the surface admits: {prompt}"
        );

        // Everything the prompt tells the session not to do must actually be refused. Otherwise the
        // instruction is a superstition the run pays for in capability.
        let refused = |command: &str| {
            decide_tool(
                &context,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": command }),
            )
            .is_err()
        };
        // `ls` and `cat` were on this list until the readers were admitted, and this test is
        // where that change had to be argued: what is forbidden is what *writes* or what runs a
        // program the surface never admitted, not what reads.
        for forbidden in [
            "protocol plan artifact list && protocol plan artifact graph",
            "protocol plan artifact list | head",
            "git status",
            "cargo test --workspace",
            "sed -i s/a/b/ Cargo.toml",
            "protocol --help",
        ] {
            assert!(
                refused(forbidden),
                "the prompt warns against `{forbidden}` and the policy permits it"
            );
        }
        // And the one thing it tells the session it *may* do has to work.
        assert!(
            decide_tool(
                &context,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": "protocol plan artifact list" })
            )
            .is_ok(),
            "the prompt's own example is refused by the policy"
        );
    }
/// The session is told which task the run drives, before it is told anything else.
    ///
    /// **Run `W4-3/1`, 2026-08-28, is why.** The map's `receive` prompt says *read the task under
    /// `.engineering/`* — a map is written once and driven many times, so it cannot say more. By
    /// then that directory held three task documents from three runs. The session read `task.yaml`,
    /// which is `W4-1`, described a different objective entirely, found the intake for it already
    /// in the store and reported that its work was done. It created nothing. The engine's cursor
    /// said `W4-3` throughout, and the next state went further wrong: 62 mentions of the wrong
    /// story against 10 of the right one.
    ///
    /// Nothing was violated — the guards held, the store was untouched, every transition was the
    /// engine's. The run was simply about something else than its own audit trail said, which is
    /// worse than a run that fails, because everything downstream is *about* something and nothing
    /// says what.
    ///
    /// The identity leads the prompt, so it is the subject of every sentence after it, and it names
    /// artifacts rather than a path — a path has to be read correctly, an id is what the store
    /// answers to.
    #[test]
    fn the_prompt_names_the_task_the_run_drives_before_the_maps_own_words() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "Read the task under `.engineering/` and record what is asked for.".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "receive".parse().expect("a state id");
        let requirements: Vec<String> = Vec::new();
        let reaching: Vec<String> = Vec::new();
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context, "/toolchain/driver/aep");
        let identity = prompt
            .split("Read the task under")
            .next()
            .expect("the map's own prompt follows the identity");
        assert!(
            identity.contains("`T-1`"),
            "the run's task is named before the step's own words: {prompt}"
        );
        assert!(
            identity.contains("story:the-one-being-driven"),
            "and so is what it is derived from, because that is what the store answers to: {prompt}"
        );
        assert!(
            identity.contains("belongs to another run"),
            "and the other task documents are ruled out by name, which is the whole defect: {prompt}"
        );
        assert!(
            prompt.starts_with("This run drives task"),
            "it leads, so it is the subject of every sentence after it: {prompt}"
        );
    }
/// What the step is trying to reach reaches the step, under a heading of its own.
    ///
    /// Run `W4-1/1` spent $8.36 in `establish_verifiers` writing checks the guard out of that state
    /// then refused, because the prompt carried `Evaluation::requirements` — what must hold *while
    /// in* the state — and never `Evaluation::transitions[].requirements`, which is what the state
    /// is trying to reach. The two lines are asserted apart rather than together: a prompt that
    /// merged them would tell a step that its outgoing guard is already in force here, which is a
    /// different instruction.
    #[test]
    fn an_unmet_outgoing_guard_is_named_in_the_prompt_under_the_reaching_heading() {
        let step = LlmStep {
            context: Vec::new(),
            scope: Vec::new(),
            description: None,
            harness: LlmStep::DEFAULT_HARNESS.to_owned(),
            skills: Vec::new(),
            prompt: "write the checks".to_owned(),
        };
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "establish_verifiers".parse().expect("a state id");
        let requirements = vec!["✓ artifact story (any) [state establish_verifiers]".to_owned()];
        let reaching = vec![
            "-> implement: guard: test.exists".to_owned(),
            "-> implement: ✗ test.first_result == failed [principle test-driven]".to_owned(),
        ];
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 0,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("/runs/W4-1/1"),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };

        let prompt = prompt_for(&step, &context, "/toolchain/driver/aep");
        let (held, reached) = prompt
            .split_once("What this state is trying to reach")
            .expect("the reaching lines are under their own heading");
        assert!(
            held.contains("artifact story (any)"),
            "what must hold here stays under its own heading: {prompt}"
        );
        for line in &reaching {
            assert!(
                reached.contains(line.as_str()),
                "`{line}` is what the state is trying to reach and belongs in the prompt: {prompt}"
            );
            assert!(
                !held.contains(line.as_str()),
                "`{line}` guards the way out and must not read as a rule in force here: {prompt}"
            );
        }
    }
/// A step map's `skills:` list is a request to the model, not a command-line flag.
    ///
    /// The skill reaches the session by being asked for, and the `Skill` tool answers; nothing
    /// about the invocation carries it, which is what keeps a skill list from becoming a second
    /// tool surface.
    #[test]
    fn a_steps_skills_are_asked_for_in_the_prompt() {
        let prompt = prompt_with_skills(&["planning"]);
        assert!(
            prompt.contains("Load the `planning` skill"),
            "the step's skill has to be asked for somewhere: {prompt}"
        );
    }
/// The fixture task, borrowed for a context that outlives this call.
    ///
    /// Leaked rather than threaded through every caller: it is one small value per test binary,
    /// and the alternative is a lifetime parameter on two helpers that exist to shorten tests.
    fn task_ref() -> &'static aep_domain::task::Task {
        Box::leak(Box::new(driven_task()))
    }
/// One context for the policy tests.
    fn policy_context<'a>(state: &'a StateId, tools: &'a ToolConfig) -> StepContext<'a> {
        StepContext {
            execution: driven_execution(),
            task: task_ref(),
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state,
            index: 0,
            attempt: 1,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements: &[],
            reaching: &[],
            preceding_llm: None,
        }
    }
/// A step whose map declared no `scope:` at all, which restricts nothing.
    ///
    /// The default for every test that is about a different layer. It is deliberately *not* an
    /// allow-everything scope: an undeclared scope and a scope that allows are different documents
    /// and the seam answers them differently, and a helper that blurred the two would hide which.
    fn no_scope() -> WriteSurface<'static> {
        WriteSurface {
            scope: &[],
            root: Path::new("/repo"),
        }
    }
/// The retired `driven-surface.sh`, case for case: the grant is held to one simple
    // recorded-under-this-name: retained flat-alias compatibility assertion.
    /// `protocol artifact|trace` invocation, and a state with no shell says so by name.
    #[test]
    fn the_shell_surface_is_one_simple_protocol_invocation() {
        let state: StateId = "implement".parse().expect("a state id");
        let shell = config(&[Capability::CommandExecution]);
        let context = policy_context(&state, &shell);
        let bash = |command: &str| {
            decide_tool(
                &context,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": command }),
            )
        };

        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(bash("protocol artifact list").is_ok());
        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(bash("protocol trace check t.jsonl").is_ok());
        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(bash("/usr/local/bin/protocol artifact list").is_ok());

        assert!(
            // recorded-under-this-name: retained flat-alias compatibility assertion.
            bash("protocol artifact list | tee out").is_err(),
            "composition"
        );
        assert!(
            // recorded-under-this-name: retained flat-alias compatibility assertion.
            bash("protocol artifact list; rm -rf /").is_err(),
            "chaining"
        );
        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(bash("protocol artifact list > out").is_err(), "redirection");
        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(bash("protocol artifact $(cat x)").is_err(), "substitution");
        assert!(bash("cargo test").is_err(), "another program");
        assert!(bash("protocol drive run").is_err(), "another verb");
        assert!(bash("").is_err(), "an empty command");

        let no_shell = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &no_shell);
        let refusal = decide_tool(
            &context,
            no_scope(),
            "Bash",
            // recorded-under-this-name: retained flat-alias compatibility assertion.
            &serde_json::json!({ "command": "protocol artifact list" }),
        )
        .expect_err("no shell in this state");
        assert!(
            refusal.contains("does not admit `command.execute`"),
            "{refusal}"
        );
    }
/// The same surface at the grouped spelling, which is what the step maps now tell a model.
    ///
    /// The CLI's first level became the four area names, and `drivers/development/checks.yaml`
    /// asks a driven step to write through `aep plan artifact new`. A surface that matched on the
    /// second word would have refused every one of those calls — the model would have been told to
    /// run a command the driver then blocked, in the one state whose whole job is writing the plan.
    /// Nothing else in the gate reaches this pairing: the step map is a document and this is a
    /// pattern, and the two only meet in a live driven run.
    #[test]
    fn the_shell_surface_admits_the_grouped_spelling_of_the_same_two_verbs() {
        let state: StateId = "implement".parse().expect("a state id");
        let shell = config(&[Capability::CommandExecution]);
        let context = policy_context(&state, &shell);
        let bash = |command: &str| {
            decide_tool(
                &context,
                no_scope(),
                "Bash",
                &serde_json::json!({ "command": command }),
            )
        };

        assert!(bash("protocol plan artifact list").is_ok());
        assert!(bash("protocol plan artifact new story x --title t").is_ok());
        assert!(bash("protocol observe trace check t.jsonl").is_ok());
        assert!(bash("/usr/local/bin/protocol plan artifact list").is_ok());

        // The area word is skipped, not blessed: what follows it still has to be one of the two.
        let refusal = bash("protocol plan serve").expect_err("`serve` is outside the surface");
        // recorded-under-this-name: retained flat-alias compatibility assertion.
        assert!(refusal.contains("`protocol serve`"), "{refusal}");
        assert!(
            bash("protocol govern validate --root .").is_err(),
            "an area word does not admit the verbs under it"
        );
        assert!(
            bash("protocol drive run").is_err(),
            "`drive` is an area name as well as the verb it always was, and neither admits `run`"
        );
    }
/// A planning document's frontmatter is the CLI's: an edit may not cross the closing `---`.
    ///
    /// **The half that stayed in code, and the fixture exists to make it load-bearing.** The
    /// step's declared scope answers `partial-only` for the store here, so the declaration
    /// *admits* a targeted edit and the fence is the only thing left that can refuse one. Under
    /// `drivers/development/default.yaml`'s own `denied` the scope refuses first and this test
    /// would pass with the fence rule deleted — which is why the admitted edit is asserted before
    /// the refused ones.
    #[test]
    fn the_planning_stores_frontmatter_is_the_clis() {
        let state: StateId = "implement".parse().expect("a state id");
        let writing = config(&[Capability::RepositoryWrite, Capability::RepositoryRead]);
        let context = policy_context(&state, &writing);
        let scope = vec![
            ScopeRule {
                paths: vec![".engineering/planning/**".to_owned()],
                write: WriteScope::PartialOnly,
            },
            ScopeRule {
                paths: vec!["**".to_owned()],
                write: WriteScope::Allowed,
            },
        ];
        let surface = WriteSurface {
            scope: &scope,
            root: Path::new("/repo"),
        };
        let store_file = "/repo/.engineering/planning/story/one.md";
        let edit = |old: &str, new: &str| {
            decide_tool(
                &context,
                surface,
                "Edit",
                &serde_json::json!({ "file_path": store_file, "old_string": old, "new_string": new }),
            )
        };

        // The state the rule is load-bearing in: the declaration says a body edit here is fine.
        assert!(
            edit("a body sentence", "a better body sentence").is_ok(),
            "the declared scope admits a targeted edit under the store, so anything refused below \
             is refused by the fence rule and not by the scope"
        );

        let quoted = edit("---", "-- -").expect_err("an edit that quotes the fence is refused");
        assert!(
            quoted.contains("crosses the `---` frontmatter fence"),
            "{quoted}"
        );
        assert!(
            quoted.contains("old_string"),
            "and names the field it read it in: {quoted}"
        );
        let padded = edit("  ---  ", "x").expect_err("a padded fence line is still the fence");
        assert!(padded.contains("frontmatter fence"), "{padded}");
        let written = edit("a body sentence", "---\nstatus: done\n---\nprose")
            .expect_err("an edit that writes a fence is refused too");
        assert!(
            written.contains("new_string"),
            "the replacement text is read as well as the quoted one: {written}"
        );

        // Content, not path: the same three dashes outside the store are three dashes.
        let elsewhere = decide_tool(
            &context,
            surface,
            "Edit",
            &serde_json::json!({
                "file_path": "/repo/docs/design/a.md",
                "old_string": "---",
                "new_string": "***",
            }),
        );
        assert!(
            elsewhere.is_ok(),
            "a horizontal rule in a design document is not a store fence"
        );
    }
/// A whole-file store rewrite is refused from the **declaration**, not from a function.
    ///
    /// **This is the acceptance of `story:retire-store-integrity-paths`.** The rule used to be a
    /// Rust function written in one vendor's tool names, which every other arm walked straight
    /// past; it is now the step map's `scope:`, which a person can read and both arms are held to.
    /// So the test reads the committed `drivers/development/default.yaml` rather than a fixture: a
    /// fixture would keep passing on the day the map lost its declaration, which is exactly the
    /// failure this story exists to make impossible.
    #[test]
    fn a_whole_file_store_rewrite_is_refused_by_the_committed_maps_declaration() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/aep")
            .canonicalize()
            .expect("the workspace root exists");
        let path = repository.join("drivers/development/default.yaml");
        let text = fs::read_to_string(&path).expect("the committed step map is readable");
        let map = aep_schema::parse::step_map(&text, Some(&path.display().to_string()))
            .expect("the committed step map validates");
        let state: StateId = "implement".parse().expect("a state id");
        let Some(Step::Llm(step)) = map
            .states
            .get(&state)
            .expect("the committed map drives `implement`")
            .steps
            .first()
        else {
            panic!("`implement`'s first step is the `llm` one this test is about");
        };

        // The declaration this test is about, asserted before what it decides. Without this the
        // test would pass on a map that had stopped saying anything about the store.
        assert!(
            step.scope
                .iter()
                .any(|rule| rule.write == WriteScope::Denied
                    && rule
                        .paths
                        .iter()
                        .any(|glob| glob == ".engineering/planning/**")),
            "the committed map still declares the planning store denied to this step's file \
             writers: {:?}",
            step.scope
        );

        let writing = config(&[Capability::RepositoryWrite, Capability::RepositoryRead]);
        let context = policy_context(&state, &writing);
        let surface = WriteSurface {
            scope: &step.scope,
            root: Path::new("/repo"),
        };
        let store_file = "/repo/.engineering/planning/story/one.md";

        let whole = decide_tool(
            &context,
            surface,
            "Write",
            &serde_json::json!({ "file_path": store_file, "content": "---\nid: x\n---\n" }),
        )
        .expect_err("a whole-file rewrite of an artifact is refused");
        assert!(
            whole.contains("declared write scope answers `denied`"),
            "and the refusal says the declaration is what refused it: {whole}"
        );
        assert!(
            whole.contains(".engineering/planning/**"),
            "naming the rule that matched, so the map is where a reader goes: {whole}"
        );
        assert!(
            whole.contains("protocol plan artifact"),
            "and what to use instead, spelled as the step maps now spell it: {whole}"
        );

        let notebook = decide_tool(
            &context,
            surface,
            "NotebookEdit",
            &serde_json::json!({ "notebook_path": store_file }),
        );
        assert!(
            notebook.is_err(),
            "the other whole-file writer is read from its own argument name and refused too"
        );
        assert!(
            decide_tool(
                &context,
                surface,
                "Edit",
                &serde_json::json!({
                    "file_path": store_file,
                    "old_string": "a body sentence",
                    "new_string": "another",
                }),
            )
            .is_err(),
            "`denied` is denied to every writer, not only the whole-file ones"
        );

        // The same declaration's other two rules, so the test is about a scope being *read* and
        // not about one path being special-cased.
        assert!(
            decide_tool(
                &context,
                surface,
                "Write",
                &serde_json::json!({ "file_path": "/repo/crates/govern/aep-domain/src/lib.rs", "content": "x" }),
            )
            .is_ok(),
            "the map allows `crates/**` to this step"
        );
        let outside = decide_tool(
            &context,
            surface,
            "Write",
            &serde_json::json!({ "file_path": "/repo/target/debug/x", "content": "x" }),
        )
        .expect_err("the catch-all denies what nobody named");
        assert!(outside.contains("`**`"), "{outside}");
    }
/// The allowlist that used to ride on `--allowedTools`, now a decision with a reason: a tool
    /// no admitted capability renders to is denied naming the state's actual surface.
    #[test]
    fn a_tool_outside_the_states_surface_is_denied_with_the_surface_named() {
        let state: StateId = "specify".parse().expect("a state id");
        let reading = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &reading);

        assert!(decide_tool(&context, no_scope(), "Read", &serde_json::json!({})).is_ok());
        assert!(decide_tool(&context, no_scope(), "Skill", &serde_json::json!({})).is_ok());
        let refusal = decide_tool(
            &context,
            no_scope(),
            "Edit",
            &serde_json::json!({ "file_path": "/x" }),
        )
        .expect_err("no write capability in this state");
        assert!(
            refusal.contains("not offered in state `specify`"),
            "{refusal}"
        );
        assert!(
            decide_tool(&context, no_scope(), "Task", &serde_json::json!({})).is_err(),
            "a subagent is never offered"
        );
    }
/// One `StepContext` for the frame tests, with the engine's lines present.
    fn metaharness_context<'a>(
        state: &'a StateId,
        tools: &'a ToolConfig,
        requirements: &'a [String],
        reaching: &'a [String],
    ) -> StepContext<'a> {
        StepContext {
            execution: driven_execution(),
            task: task_ref(),
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state,
            index: 2,
            attempt: 3,
            tools,
            run_directory: Path::new("/runs/T-1/1"),
            requirements,
            reaching,
            preceding_llm: None,
        }
    }
#[test]
    fn the_metaharness_operations_mirror_the_allowed_tools_decisions() {
        let reading = config(&[Capability::RepositoryRead]);
        assert_eq!(
            metaharness_operations(&reading),
            ["dir.list", "file.read", "search", "skill.load"]
        );
        assert!(!metaharness_operations(&reading).contains(&"shell"));

        let shell = config(&[Capability::CommandExecution]);
        assert!(metaharness_operations(&shell).contains(&"shell"));

        let everything = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
            Capability::NetworkRead(Audience::Any),
        ]);
        assert!(
            !metaharness_operations(&everything).contains(&"subagent.spawn"),
            "a subagent's tool set is derived by nothing in these decisions"
        );
    }
/// Gap register `:40`. The document the driver writes has to be one `protocol observe trace check`
    /// can actually read, or it is a file nobody consumes that looks like an audit.
    ///
    /// Read back through `trace_domain::raw::read_spec` — the same door the CLI uses — rather than
    /// eyeballed as JSON.
    #[test]
    fn the_refusal_specification_is_a_specification_the_checker_reads() {
        let state: aep_domain::ids::StateId = "implement".parse().expect("a state id");
        let read_only = ToolConfig::new([Capability::RepositoryRead].into_iter().collect());
        let document =
            refusal_specification(&state, 0, &read_only).expect("a read-only state refuses things");
        let text = serde_json::to_string(&document).expect("renders");

        let spec = trace_domain::raw::read_spec(&text)
            .expect("the driver must write a specification the checker can read");

        let refused: Vec<&str> = spec
            .expectations
            .iter()
            .map(|expectation| expectation.id.as_str())
            .collect();
        assert!(
            refused.contains(&"refused-file-write"),
            "a read-only state must refuse writing: {refused:?}"
        );
        assert!(
            refused.contains(&"refused-shell"),
            "and a shell: {refused:?}"
        );
        assert!(
            !refused.iter().any(|id| id.ends_with("file-read")),
            "and must not refuse what it admitted: {refused:?}"
        );
        assert!(
            !refused.iter().any(|id| id.ends_with("skill-load")),
            "skills are always offered, so refusing them would be a row that can only fail: \
             {refused:?}"
        );
    }
/// The complement is computed from the one table, so the two cannot drift.
    #[test]
    fn admitted_and_refused_operations_partition_the_vocabulary() {
        for config in [
            ToolConfig::default(),
            ToolConfig::new([Capability::RepositoryRead].into_iter().collect()),
            ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect()),
        ] {
            let admitted = metaharness_operations(&config);
            let refused = refused_operations(&config);
            let mut together: Vec<&str> = admitted.iter().chain(refused.iter()).copied().collect();
            together.sort_unstable();
            let mut all = every_operation();
            all.sort_unstable();
            assert_eq!(
                together, all,
                "every operation is either admitted or refused, and never both or neither"
            );
        }
    }
/// A fully permissive state writes no specification, and that is `trace-spec/1`'s rule rather
    /// than a shortcut.
    ///
    /// The format refuses a specification with no expectations — *"a report with no content reads
    /// exactly like a report with no gaps"* — which is the same argument for not writing one. What
    /// keeps absence readable is the **frame**: it is written unconditionally, so a frame with no
    /// refusal file beside it means this state was admitted everything, and no frame at all means
    /// the step never ran.
    #[test]
    fn a_state_that_admits_everything_writes_no_specification() {
        let state: aep_domain::ids::StateId = "implement".parse().expect("a state id");
        let everything = ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect());
        assert!(refusal_specification(&state, 0, &everything).is_none());

        // And the empty document would indeed have been refused, so this is the format's rule and
        // not a preference.
        let empty = serde_json::json!({
            "format": "trace-spec/1",
            "id": "driver/implement-0",
            "expectations": [],
        });
        assert!(
            trace_domain::raw::read_spec(&empty.to_string()).is_err(),
            "an empty specification judges nothing and must not be writable"
        );
    }
/// The seal is the metaharness § 5.5 rule, reproduced here without its crates: SHA-256 over
    /// the compact key-sorted serialization with `digest` and `format` absent. A document this
    /// test passes is a document metaharness's parser accepts byte-for-byte; one it fails is a
    /// run refused before a cent is spent.
    #[test]
    fn the_frame_document_is_sealed_by_the_rule_metaharness_verifies() {
        let tools = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements = vec!["the suite is red before the implementation".to_owned()];
        let reaching = vec!["to verify: the suite is green".to_owned()];
        let context = metaharness_context(&state, &tools, &requirements, &reaching);

        let frame = metaharness_frame(&context, &[], "development/default", "1");
        assert_eq!(frame["format"], METAHARNESS_FRAME_FORMAT);

        let mut unsealed = frame.clone();
        let object = unsealed.as_object_mut().expect("an object");
        let stated = object.remove("digest").expect("a digest");
        object.remove("format");
        let recomputed = {
            use sha2::{Digest as _, Sha256};
            let bytes = serde_json::to_vec(&unsealed).expect("serialises");
            let mut hasher = Sha256::new();
            hasher.update(&bytes);
            hasher
                .finalize()
                .iter()
                .fold(String::new(), |mut output, byte| {
                    use std::fmt::Write as _;
                    write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
                    output
                })
        };
        assert_eq!(stated, serde_json::Value::String(recomputed));
    }
/// The engine's lines travel verbatim, on the same rule as the prompt: the frame is the only
    /// place they exist for the seam, and a summary here would be the only summary.
    #[test]
    fn the_frame_carries_the_engines_lines_and_the_steps_coordinates() {
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "specify".parse().expect("a state id");
        let requirements = vec!["an approved specification exists".to_owned()];
        let reaching = vec!["to implement: the suite is red".to_owned()];
        let context = metaharness_context(&state, &tools, &requirements, &reaching);

        let frame = metaharness_frame(&context, &[], "development/default", "1");
        assert_eq!(frame["node"]["id"], "specify");
        assert_eq!(frame["step"]["index"], 2);
        assert_eq!(frame["step"]["attempt"], 3);
        assert_eq!(frame["workflow"]["version"], "1");
        assert_eq!(
            frame["obligations"][0]["text"],
            "an approved specification exists"
        );
        assert_eq!(
            frame["reaching"][0]["text"],
            "to implement: the suite is red"
        );
        assert_eq!(frame["handoff"]["handoff"], "none");
        let operations: Vec<&str> = frame["operations"]
            .as_array()
            .expect("a list")
            .iter()
            .map(|entry| entry["op"].as_str().expect("a name"))
            .collect();
        assert_eq!(
            operations,
            ["dir.list", "file.read", "search", "skill.load"]
        );
    }
/// The vendor arm receives the map's write scope in the frame rather than in a second flag.
    ///
    /// The b10x arm compiles these same words to `--write-scope`; this is the other half of the
    /// comparison. Ordered rules and their granularities must survive the translation exactly,
    /// while the `file:` prefix prevents a catch-all file rule from governing `proc:` subjects.
    #[test]
    fn the_frame_compiles_the_steps_write_scope_into_ordered_subject_rules() {
        let tools = config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);
        let state: StateId = "implement".parse().expect("a state id");
        let context = metaharness_context(&state, &tools, &[], &[]);
        let scope = vec![
            ScopeRule {
                paths: vec![".engineering/planning/**".to_owned()],
                write: WriteScope::PartialOnly,
            },
            ScopeRule {
                paths: vec!["crates/**".to_owned()],
                write: WriteScope::Allowed,
            },
            ScopeRule {
                paths: vec!["**".to_owned()],
                write: WriteScope::Denied,
            },
        ];

        let frame = metaharness_frame(&context, &scope, "development/default", "1");
        assert_eq!(
            frame["subjects"],
            serde_json::json!({
                "rules": [
                    {
                        "subjects": ["file:.engineering/planning/**"],
                        "operations": [
                            {"op": "file.edit"},
                            {"op": "file.read"},
                        ],
                    },
                    {
                        "subjects": ["file:crates/**"],
                        "operations": [
                            {"op": "file.edit"},
                            {"op": "file.read"},
                            {"op": "file.write"},
                        ],
                    },
                    {
                        "subjects": ["file:**"],
                        "operations": [{"op": "file.read"}],
                    },
                ],
            })
        );
        assert!(
            frame["subjects"]["rules"][2]["subjects"][0]
                .as_str()
                .is_some_and(|pattern| !pattern.starts_with("proc:")),
            "the write scope does not become an execution scope"
        );
    }
/// A native step is told which programs it may start, or its loop publishes no `run` at all.
    ///
    /// **Run `b10x-2991520`, 2026-08-29: 30 `tool_search` calls, 28 of them distinct**, hunting for
    /// `run`, `exec`, `shell`, `spawn`, `execute`, `argv` and `program`. The step it was given
    /// records something in the planning store, whose only route is the `protocol` CLI, and nothing
    /// in its catalogue could start a process — `harness-tools` withholds `run` outright when no
    /// allowlist was supplied (`programs.is_none()`). The loop was right and the driver had not
    /// told it anything.
    ///
    /// The list is the same decision `driven_surface` enforces on the vendor arm, rendered rather
    /// than re-decided. The native rendering is the stronger of the two: a program not on it has no
    /// tool to reach it, where the vendor arm refuses the call after the model has spent the turn.
    #[test]
    fn a_native_step_is_told_which_programs_it_may_start() {
        let executing = config(&[Capability::RepositoryRead, Capability::CommandExecution]);
        let options = B10xOptions {
            aep_binary: Some(PathBuf::from("/selected/bin/aep")),
            ..B10xOptions::default()
        };
        let argv = b10x_argv(
            &options,
            Path::new("/home/op/.cache/ws_run"),
            &[],
            &[],
            "do the thing",
            &executing,
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        let allowed: Vec<&String> = argv
            .windows(2)
            .filter(|pair| pair[0] == "--allow-program")
            .map(|pair| &pair[1])
            .collect();
        // **The CLI by a path, and never by a bare name the confined `PATH` cannot resolve.**
        // Asserting `== "protocol"` is what this said before, and it passed while run EVAL-1/1
        // spent four turns taking `127` from a word this list had said yes to. A declared program
        // that cannot be found is admitted and then fails at exec; one that is not declared is
        // refused here, by name, listing the set — which is the only form of this answer that tells
        // the model the spelling that works.
        // **The CLI is not on this list at all, and that is the fix.** Two spellings were tried
        // here and both failed from inside the sandbox: the bare name, because the confined exec
        // has its own `PATH`; then the absolute host path, because the sandbox binds `/usr`,
        // `/bin`, `/lib`, `/lib64` and the workspace and is not this filesystem. An allow-list
        // decides what a `run` may name; only a mount decides what the sandbox contains. So the
        // driver travels as `--driver`, and the loop allow-lists the mounted path itself.
        let cli: Vec<&&String> = allowed
            .iter()
            .filter(|name| !READ_ONLY_PROGRAMS.contains(&name.as_str()))
            .collect();
        assert!(
            cli.is_empty(),
            "the CLI is not declared as a program: a path this sandbox does not hold is admitted \
             and then dies at `ENOENT`, which reads as a wrong command rather than a missing \
             file — EVAL-1/1 took that twice and hand-wrote the store both times: {argv:?}"
        );
        let driver = argv
            .iter()
            .position(|word| word == "--driver")
            .expect("the driver travels as a mount");
        assert!(
            Path::new(&argv[driver + 1]).is_absolute(),
            "and it is staged from a real path on this host: {argv:?}"
        );
        // The two sides are separate binaries, so the path the instructions quote and the path the
        // loop mounts at are pinned together here rather than by a comment asking a reader to keep
        // them level.
        assert_eq!(
            staged_driver(&options), "/toolchain/driver/aep",
            "the loop mounts a declared driver at `/toolchain/driver`; a step told any other path \
             is told one that does not resolve"
        );
        for reader in READ_ONLY_PROGRAMS {
            assert!(
                allowed.iter().any(|name| name.as_str() == *reader),
                "`{reader}` is admitted on the vendor arm and must be admitted here: {allowed:?}"
            );
        }

        // A state with no `command.execute` is told nothing, so the loop publishes no `run` — the
        // absence is the enforcement rather than a refusal after the fact.
        let reading_only = config(&[Capability::RepositoryRead]);
        let quiet = b10x_argv(
            &B10xOptions::default(),
            Path::new("/home/op/.cache/ws_run"),
            &[],
            &[],
            "do the thing",
            &reading_only,
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        assert!(
            !quiet.iter().any(|word| word == "--allow-program"),
            "a state that admits no execution names no program: {quiet:?}"
        );
    }

    #[test]
    fn the_native_prompt_names_the_selected_staged_binary() {
        let executing = config(&[Capability::CommandExecution]);
        for name in ["aep", "protocol", "aep-pinned"] {
            let options = B10xOptions {
                aep_binary: Some(PathBuf::from(format!("/selected/bin/{name}"))),
                ..B10xOptions::default()
            };
            let mounted = staged_driver(&options);
            let prompt = surface_lines(Harness::B10x, &executing, &mounted);
            assert!(prompt.contains(&format!("\x60{mounted} plan artifact")));
            let argv = b10x_argv(
                &options, Path::new("/workspace"), &[], &[], &prompt, &executing,
                OperatorFiles { hooks: None, plugin_dirs: &[] },
            );
            assert!(argv.windows(2).any(|p| p == ["--driver", &format!("/selected/bin/{name}")]));
        }
    }
/// The audit asks each harness in the vocabulary that harness answers in.
    ///
    /// **Run `b10x-2623331`, 2026-08-29.** Its `session.started` published
    /// `available_operations: [file.read, dir.list, search, file.write, file.edit]` — everything the
    /// state admitted — and the audit told it, per state, that it was missing every one of them. It
    /// had compared a rendered catalogue against `offered_tools`, which on that loop is only
    /// `tool_search`, `tool_describe` and `tool_invoke`. An audit that fires on a session holding
    /// exactly what it needs is worse than none: the next true one is read as noise.
    #[test]
    fn the_tool_audit_reads_the_list_each_harness_answers_in() {
        let reading_and_writing =
            config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);

        // What the b10x loop actually published in that run, and what the audit must compare to.
        let published = ["file.read", "dir.list", "search", "file.write", "file.edit"];
        let asked = Harness::B10x.operations_or_tools(&reading_and_writing);
        for operation in &published {
            assert!(
                asked.iter().any(|name| name == operation),
                "`{operation}` was published and the audit does not ask about it: {asked:?}"
            );
        }
        for name in &asked {
            assert!(
                published.contains(&name.as_str()),
                "the audit asks about `{name}`, which that loop never publishes — this is the false \
                 alarm the run was given"
            );
        }

        // The vendor arm is unchanged: one tool per act, so its tool names are the question.
        let claude = Harness::ClaudeCode.operations_or_tools(&reading_and_writing);
        assert!(
            claude.iter().any(|name| name == "Read"),
            "Claude Code answers in tool names: {claude:?}"
        );
        assert!(
            !claude.iter().any(|name| name.contains('.')),
            "and never in the neutral operation scheme, which would compare two vocabularies: \
             {claude:?}"
        );
    }
/// A confined workspace publishes the tools the arm needs, and an ordinary one says why not.
    ///
    /// The native arm could read a repository and change nothing in it, so a comparison against it
    /// measured an arm that could not attempt the work. Substrate represents a workspace only when
    /// its directory name starts with `ws_`, and publishes `run` only with a delegated subtree —
    /// metaharness states the consequence plainly: *a run that may not execute its suite cannot see
    /// a test fail before writing the code, so it will not write the code.*
    ///
    /// The two travel together on purpose. An arm given confinement without execution can write and
    /// not test; given execution without confinement it is refused at launch.
    #[test]
    fn a_confined_workspace_gets_the_flags_that_let_the_arm_write_and_an_ordinary_one_does_not() {
        let with_subtree = B10xOptions {
            endpoint: Some("http://127.0.0.1:18080".to_owned()),
            model: Some("qwen3.8-27b".to_owned()),
            cgroup_root: Some(PathBuf::from("/sys/fs/cgroup/u")),
            ..B10xOptions::default()
        };
        let confined = b10x_argv(
            &with_subtree,
            Path::new("/home/op/.cache/ws_run"),
            &[],
            &[],
            "do the thing",
            &config(&[Capability::RepositoryRead, Capability::CommandExecution]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        let joined = confined.join(" ");
        assert!(
            joined.contains("--substrate-embedded"),
            "an adoptable workspace with a subtree is confined: {joined}"
        );
        assert!(
            joined.contains("--cgroup-root /sys/fs/cgroup/u"),
            "and may execute, or it cannot see a test fail: {joined}"
        );

        // An ordinary checkout: asking would be a launch refusal, so nothing is asked.
        let ordinary = b10x_argv(
            &with_subtree,
            Path::new("/home/op/aep"),
            &[],
            &[],
            "do the thing",
            &config(&[Capability::RepositoryRead, Capability::CommandExecution]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        assert!(
            !ordinary.join(" ").contains("--substrate"),
            "substrate adopts no workspace here and asking would refuse the launch"
        );
        assert!(
            b10x_read_only_note(
                &b10x_map(),
                Path::new("/home/op/aep"),
                &with_subtree
            )
            .is_some_and(|note| note.contains("does not")),
            "and the operator is told which half is missing"
        );

        // Adoptable but no subtree: confined and unable to run its suite, which is its own note.
        let no_subtree = B10xOptions {
            cgroup_root: None,
            ..with_subtree.clone()
        };
        assert!(
            b10x_read_only_note(
                &b10x_map(),
                Path::new("/home/op/.cache/ws_run"),
                &no_subtree
            )
            .is_some_and(|note| note.contains("no `--b10x-cgroup-root`")),
            "the other half, named as the other half"
        );

        // And when both hold, the note does not fire at all: a warning that cries wolf is one a
        // reader learns to skip.
        assert!(
            b10x_read_only_note(
                &b10x_map(),
                Path::new("/home/op/.cache/ws_run"),
                &with_subtree
            )
            .is_none(),
            "everything the note warns about is satisfied, so it says nothing"
        );
    }
/// Both arms can be pointed at one gateway, which is what makes a harness comparison one.
    ///
    /// With one arm on a vendor's own model and the other on whatever a gateway serves, a
    /// difference in waste is a difference in two things at once and no scorer can separate them
    /// afterwards. The endpoint and the model travel together — an endpoint with no model reaches a
    /// gateway and asks it for nothing — and `--credentials none` travels with them, because
    /// metaharness refuses a child that holds an operator credential while pointed somewhere
    /// foreign.
    #[test]
    fn a_claude_step_can_be_pointed_at_the_same_gateway_as_the_native_loop() {
        let both = B10xOptions {
            claude_endpoint: Some("http://127.0.0.1:18080".to_owned()),
            claude_model: Some("qwen3.8-27b".to_owned()),
            ..B10xOptions::default()
        };
        let argv = metaharness_argv(
            Path::new("/runs/T-1/1/frame.json"),
            Path::new("/repo"),
            &[],
            "do the thing",
            both.claude_gateway(),
            None,
        );
        let joined = argv.join(" ");
        assert!(
            joined.contains("--model-endpoint http://127.0.0.1:18080"),
            "the gateway reaches the argv: {joined}"
        );
        assert!(
            joined.contains("--model qwen3.8-27b"),
            "and so does the model it serves: {joined}"
        );
        assert!(
            joined.contains("--credentials none"),
            "and the credential rule travels with them rather than being remembered: {joined}"
        );

        // Half a gateway is no gateway: metaharness refuses each alone, and a driver that passed
        // one would turn a flag mistake into a launch refusal states into a paid run.
        for half in [
            B10xOptions {
                claude_endpoint: Some("http://127.0.0.1:18080".to_owned()),
                ..B10xOptions::default()
            },
            B10xOptions {
                claude_model: Some("qwen3.8-27b".to_owned()),
                ..B10xOptions::default()
            },
        ] {
            assert!(
                half.claude_gateway().is_none(),
                "an endpoint with no model, or a model with nowhere to go, is not a gateway"
            );
        }

        // And with neither, the argv is what it has always been.
        let plain = metaharness_argv(
            Path::new("/runs/T-1/1/frame.json"),
            Path::new("/repo"),
            &[],
            "do the thing",
            None,
            None,
        );
        assert!(
            !plain.join(" ").contains("--model-endpoint"),
            "a run that named no gateway is pointed at none"
        );
    }
#[test]
    fn the_metaharness_argv_drives_the_seam_with_the_declared_directory_and_frame() {
        let argv = metaharness_argv(
            Path::new("/runs/T-1/1/transcripts/implement-2-3.frame.json"),
            Path::new("/operator/repo"),
            &[PathBuf::from("/plugins/claude-code")],
            "do the thing",
            None,
            Some("agent:T-1.1"),
        );
        assert_eq!(argv[0], "metaharness");
        assert_eq!(argv[1], "run");
        assert_eq!(argv[2], "claude");
        // **Who the session writes as, declared across the boundary.** `session_env` reaches a
        // `command` step because that is this process's own child; an `llm` step's model is behind
        // metaharness, which constructs its child's environment rather than inheriting ours. This
        // is the flag that closes it, and it is passed rather than exported for the same reason
        // metaharness keeps an allowlist: a variable the surrounding shell can set is not
        // provenance. Without it a driven session's `artifact move` journals as `human:$USER` and
        // the store cannot tell an agent's write from a person's.
        assert!(
            argv.windows(2)
                .any(|pair| pair[0] == "--actor" && pair[1] == "agent:T-1.1"),
            "the session's actor travels to the harness that will not inherit it: {argv:?}"
        );
        let has = |flag: &str, value: &str| {
            argv.windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value)
        };
        assert!(has("--cwd", "/operator/repo"));
        assert!(has(
            "--frame",
            "/runs/T-1/1/transcripts/implement-2-3.frame.json"
        ));
        assert!(has("--decisions", "ask"));
        assert!(has("-p", "do the thing"));
        assert!(has("--plugin-dir", "/plugins/claude-code"));
        assert!(argv.contains(&"--hermetic".to_owned()));
    }
/// The b10x argv is the launch that loop refuses least, and it carries no frame.
    ///
    /// **Three of these assertions are about what is absent, and the absences are the design.**
    /// `metaharness run b10x --frame …` is refused before a model is reached — a frame's
    /// enforcement rides on `tool.decide`, which the b10x adapter refuses because nothing on that
    /// loop ever asks. `--decisions observe` is present because that loop supplies observation
    /// without a decision channel; leaving the default `frame` in place would claim a control seam
    /// the adapter does not have. `--substrate-embedded` is absent because substrate adopts a
    /// workspace only when its directory name starts with `ws_`, and a governed tree is the
    /// operator's repository: asking would turn every driven b10x step into a launch refusal.
    ///
    /// What is present instead is the surface travelling as the two spec fields that exist for a
    /// harness with no seam — the step's `scope:` as `--write-scope` in the order it was written,
    /// and its `context:` as `--context`.
    #[test]
    fn the_b10x_argv_carries_the_scope_and_never_the_frame_that_loop_would_refuse() {
        let mut step = b10x_step();
        step.scope = vec![
            ScopeRule {
                paths: vec![".engineering/planning/**".to_owned()],
                write: WriteScope::PartialOnly,
            },
            ScopeRule {
                paths: vec!["**".to_owned()],
                write: WriteScope::Denied,
            },
        ];
        step.context = vec!["AGENTS.md".to_owned()];
        let options = B10xOptions {
            endpoint: Some("http://127.0.0.1:8080".to_owned()),
            model: Some("a-model".to_owned()),
            api_key: false,
            ..B10xOptions::default()
        };

        let argv = b10x_argv(
            &options,
            Path::new("/operator/repo"),
            &step.scope,
            &step.context,
            "do the thing",
            &config(&[Capability::RepositoryRead, Capability::CommandExecution]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        assert_eq!(argv[0], "metaharness");
        assert_eq!(argv[1], "run");
        assert_eq!(argv[2], "b10x");
        let has = |flag: &str, value: &str| {
            argv.windows(2)
                .any(|pair| pair[0] == flag && pair[1] == value)
        };
        assert!(has("--cwd", "/operator/repo"));
        assert!(has("--model-endpoint", "http://127.0.0.1:8080"));
        assert!(has("--model", "a-model"));
        assert!(has("--credentials", "none"), "{argv:?}");
        assert!(has("--decisions", "observe"), "{argv:?}");
        assert!(has("-p", "do the thing"));
        assert!(argv.contains(&"--hermetic".to_owned()));

        // Ordered, first match wins, so the map's own order is the argv's order.
        let scopes: Vec<&String> = argv
            .windows(2)
            .filter(|pair| pair[0] == "--write-scope")
            .map(|pair| &pair[1])
            .collect();
        assert_eq!(
            scopes,
            [".engineering/planning/**=partial-only", "**=denied"]
        );
        assert!(has("--context", "AGENTS.md"));

        for refused in ["--frame", "--substrate-embedded", "--plugin-dir"] {
            assert!(
                !argv.iter().any(|word| word == refused),
                "`{refused}` is either refused by the b10x adapter or means nothing to it: {argv:?}"
            );
        }

        // The credential is a choice and not a silence: `operator-login` is the flag's default and
        // names nothing on this loop, which refuses it rather than launching unauthenticated.
        let authenticated = b10x_argv(
            &B10xOptions {
                api_key: true,
                ..options
            },
            Path::new("/operator/repo"),
            &[],
            &[],
            "do the thing",
            &config(&[Capability::RepositoryRead]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        assert!(authenticated
            .windows(2)
            .any(|pair| pair[0] == "--credentials" && pair[1] == "api-key"));
    }
/// The native arm reaches a subscription model, and the token stays out of both processes.
    ///
    /// Without this the arm could only be pointed at a gateway, and the gateway on hand served a
    /// 32k window: run `b10x-32k` died at turn 37 on `maximum context length is 32768 tokens`
    /// mid-state. A run that fails on the endpoint's window measures the endpoint, not the
    /// harness, so a comparison drawn from it says nothing about either arm.
    #[test]
    fn a_subscription_source_and_a_dialect_reach_metaharness_as_flags_and_the_token_does_not() {
        let argv = b10x_argv(
            &B10xOptions {
                endpoint: Some("https://api.anthropic.com/v1".to_owned()),
                model: Some("claude-haiku-4-5-20251001".to_owned()),
                wire: Some("anthropic-messages".to_owned()),
                oauth_token_file: Some(PathBuf::from("/operator/.store.json")),
                oauth_token_pointer: Some("/claudeAiOauth/accessToken".to_owned()),
                ..B10xOptions::default()
            },
            Path::new("/operator/repo"),
            &[],
            &[],
            "do the thing",
            &config(&[Capability::RepositoryRead]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        let after = |flag: &str| {
            argv.windows(2)
                .find(|pair| pair[0] == flag)
                .map(|pair| pair[1].clone())
        };
        assert_eq!(after("--model-wire").as_deref(), Some("anthropic-messages"));
        assert_eq!(
            after("--subscription-token-file").as_deref(),
            Some("/operator/.store.json")
        );
        assert_eq!(
            after("--subscription-token-pointer").as_deref(),
            Some("/claudeAiOauth/accessToken")
        );
        // `none` is what a subscription run declares: the token is the loop's to read, so there is
        // nothing for metaharness to copy and nothing of the operator's in the child's home.
        assert_eq!(after("--credentials").as_deref(), Some("none"));
        // The pointer never travels without the source it points into.
        let sourceless = b10x_argv(
            &B10xOptions {
                oauth_token_pointer: Some("/claudeAiOauth/accessToken".to_owned()),
                ..B10xOptions::default()
            },
            Path::new("/operator/repo"),
            &[],
            &[],
            "do the thing",
            &config(&[Capability::RepositoryRead]),
            OperatorFiles {
                hooks: None,
                plugin_dirs: &[],
            },
        );
        assert!(
            !sourceless
                .iter()
                .any(|word| word == "--subscription-token-pointer"),
            "{sourceless:?}"
        );
    }
/// One capability decision, two naming tables, and no second decision anywhere.
    ///
    /// § 4.9 point 2's load-bearing assertion, driven from `aep_driver::tool::tool_config` rather
    /// than from a hand-built [`ToolConfig`] so the shared half is genuinely the shared function.
    /// A second harness that re-decided could quietly re-admit a shell the state never granted,
    /// which is what makes this the guard rather than the name comparison.
    #[test]
    fn the_shared_tool_decision_renders_into_two_vocabularies_and_is_taken_once() {
        use aep_domain::capability::CapabilityPolicy;
        use aep_driver::tool::tool_config;

        let everything = tool_config(&CapabilityPolicy::allowing(TOOL_CANDIDATES.iter().cloned()));
        assert!(
            !everything.is_empty(),
            "the widest policy admits something, or every assertion below is vacuous"
        );
        let b10x = b10x_tools(&everything);
        let claude = allowed_tools(&everything);
        assert!(!b10x.is_empty() && !claude.is_empty());
        for named in &b10x {
            assert!(
                !claude.contains(named),
                "`{named}` is in both tables, so one of them is not a rendering of its own harness"
            );
        }
        // The three entries § 4.9 point 2 decides rather than leaves to an implementer.
        assert!(
            !everything.subagents_offered() && !b10x.iter().any(|named| named.contains("agent")),
            "no subagent spawner is ever rendered, whatever is admitted"
        );
        for (capabilities, admitted) in [
            (vec![Capability::RepositoryRead], false),
            (
                vec![Capability::RepositoryRead, Capability::CommandExecution],
                true,
            ),
        ] {
            let config = tool_config(&CapabilityPolicy::allowing(capabilities.clone()));
            assert_eq!(
                b10x_tools(&config).contains(&"run".to_owned()),
                admitted,
                "the exec entry is offered iff `command.execute` is admitted, and \
                 {capabilities:?} admits it: {admitted}"
            );
        }
        // `web.read` and `skill.load` have no entry in that loop's catalogue. Not downgraded to
        // something else and not silently dropped from the shared decision: the capability stays
        // admitted and the session simply has no tool, which the session-start audit reports.
        let networked = tool_config(&CapabilityPolicy::allowing([
            Capability::RepositoryRead,
            Capability::NetworkRead(Audience::Any),
        ]));
        assert!(
            networked.admits(&Capability::NetworkRead(Audience::Private)),
            "the shared decision still admits it"
        );
        assert_eq!(
            b10x_tools(&networked),
            ["dir_list", "file_read", "search"],
            "and this table renders nothing for it, because the catalogue has no entry"
        );
    }
/// An observed session's calls are counted and never reported as an adjudication.
    ///
    /// **The failure this exists to make impossible.** The b10x adapter sets
    /// `decision_required: false` on every `tool.requested` and `Seam::None` beside it, so a
    /// driver that folded that stream into the claude arm's report would print *0 refused* — and
    /// two arms compared on that number would be compared on an artefact of the instrument rather
    /// than on what the runs did. What is refused on this arm is refused by the toolset, before a
    /// call exists to be counted.
    ///
    /// Both directions are asserted, because a report that said *nobody asked* on every run would
    /// be exactly as wrong in the other direction.
    #[test]
    fn an_observed_session_is_counted_and_never_reported_as_a_clean_adjudication() {
        let tools = config(&[Capability::RepositoryRead]);
        let state: StateId = "specify".parse().expect("a state id");
        let task = driven_task();
        let context = step_context(&tools, &state, &task);
        let observed = format!(
            "{}\n{}\n",
            serde_json::json!({
                "format": METAHARNESS_EVENT_FORMAT,
                "event": "tool.requested",
                "decision_required": false,
                "seam": "none",
                "call_id": "call-1",
                "name": "file_read",
                "input": { "path": "AGENTS.md" },
            }),
            serde_json::json!({
                "format": METAHARNESS_EVENT_FORMAT,
                "event": "tool.requested",
                "decision_required": false,
                "seam": "none",
                "call_id": "call-2",
                "name": "search",
                "input": { "path": "." },
            })
        );

        let mut commands: Vec<u8> = Vec::new();
        let mut transcript: Vec<u8> = Vec::new();
        let mut authorize = |_: &ActionRequest| -> Decision {
            panic!("an observed stream asks the engine nothing, because nobody asked the driver")
        };
        let tally = answer_events(
            Harness::B10x,
            &context,
            no_scope(),
            observed.as_bytes(),
            &mut commands,
            &mut transcript,
            &mut authorize,
        );

        assert_eq!(
            tally,
            Adjudication {
                requested: 2,
                asked: 0,
                denied: 0
            }
        );
        assert!(
            commands.is_empty(),
            "answering a call nobody put would be this process claiming a decision the wire says \
             nobody made: {}",
            String::from_utf8_lossy(&commands)
        );
        assert_eq!(
            String::from_utf8_lossy(&transcript),
            observed,
            "every event line reaches the transcript, decided or not"
        );

        let line = tally.line(Harness::B10x, &state);
        assert!(
            line.contains("observed 2 tool call(s) and adjudicated none"),
            "the count of what happened is reported: {line}"
        );
        assert!(
            line.contains("nobody asked this process"),
            "and it is distinguished from nothing having been refused: {line}"
        );
        assert!(
            !line.contains("were refused"),
            "a denial count here would read as a verdict about the run: {line}"
        );

        // The other direction: an arm that does adjudicate reports the counts, because there the
        // zero genuinely means nothing was refused.
        let adjudicated = Adjudication {
            requested: 5,
            asked: 5,
            denied: 1,
        }
        .line(Harness::ClaudeCode, &state);
        assert!(
            adjudicated.contains("put 5 tool call(s) to the driver and 1 were refused"),
            "{adjudicated}"
        );
    }
/// A map naming a harness this machine cannot spawn is refused before a run id or a lock.
    ///
    /// The same shape as the two pre-flights beside it, and the same argument: this is decidable
    /// from the map and the filesystem, and discovering it at the first `llm` step means a run
    /// directory, an id, the store lock and a snapshot for a `NoVerdict` about something that was
    /// never run.
    ///
    /// Asserted on the two checks that need no process — a map with no b10x step is silent, and a
    /// map with one that declares no endpoint is refused naming the flag. The two spawning checks
    /// above them are not exercised here: a unit test that shelled out to whatever
    /// `{METAHARNESS_BINARY}` this machine happens to hold would report on the machine.
    #[test]
    fn a_map_naming_the_b10x_harness_is_refused_when_the_run_cannot_say_where_to_point_it() {
        let map = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/b10x\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n\
             \x20       harness: b10x\n",
            None,
        )
        .expect("the map validates");
        assert_eq!(b10x_step_count(&map), 1, "the fixture reaches the rule");

        let claude = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/llm\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do it\n",
            None,
        )
        .expect("the map validates");
        assert!(
            b10x_preflight(&claude, &B10xOptions::default()).is_none(),
            "a map with no b10x step is not this pre-flight's business, whatever is installed"
        );

        // The two checks that read only the arguments, and they answer the same on every machine
        // — which is the reason they run first.
        let refusal = b10x_preflight(&map, &B10xOptions::default()).expect("refused");
        assert!(
            refusal.contains("--b10x-endpoint"),
            "the refusal names the flag that answers it: {refusal}"
        );
        let refusal = b10x_preflight(
            &map,
            &B10xOptions {
                endpoint: Some("http://127.0.0.1:8080".to_owned()),
                ..B10xOptions::default()
            },
        )
        .expect("refused");
        assert!(refusal.contains("--b10x-model"), "{refusal}");

        // With both declared, what is left is about this machine, so the assertion is about which
        // answer it gave rather than about which one it should have.
        let declared = B10xOptions {
            endpoint: Some("http://127.0.0.1:8080".to_owned()),
            model: Some("a-model".to_owned()),
            api_key: false,
            ..B10xOptions::default()
        };
        let installed = session_path()
            .split(':')
            .any(|directory| Path::new(directory).join(B10X_BINARY).is_file());
        match b10x_preflight(&map, &declared) {
            None => assert!(
                installed && metaharness_knows(B10X_HARNESS),
                "the only reason to admit a b10x map is that both halves are installed"
            ),
            Some(refusal) => assert!(
                refusal.contains(B10X_BINARY) || refusal.contains("does not publish an adapter"),
                "an install predating the adapter is named as that rather than as a missing \
                 binary: {refusal}"
            ),
        }
    }
/// The name of the committed cross-repository golden, under this crate's `fixtures/`.
    const GOLDEN: &str = "metaharness-frame-canonical.json";
/// The one frame the golden is minted from, and the reason it can be committed at all.
    ///
    /// Nothing here reads a clock, an environment variable or anything off this machine, so the
    /// document is byte-identical wherever it is minted — a golden that varied with its producer
    /// would pin the producer and not the format. The `run_directory` a [`StepContext`] carries
    /// never reaches the frame, which is why the fixture holds no path at all; the workflow id and
    /// the state are document names this repository publishes, and the two lines are the engine's
    /// own vocabulary. There is deliberately nothing account-level in it: this file is public and
    /// is read by a repository that is not.
    ///
    /// The capability set is the widest a driven state gets, so the golden carries seven of the ten
    /// parameterless operations rather than a corner of the vocabulary.
    fn canonical_frame() -> serde_json::Value {
        let tools = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
        ]);
        let state: StateId = "implement".parse().expect("a state id");
        let requirements = vec!["the suite is red before the implementation".to_owned()];
        let reaching = vec!["to verify: the suite is green".to_owned()];
        let task = driven_task();
        let context = StepContext {
            execution: driven_execution(),
            task: &task,
            task_document: Some(Path::new("/projects/repo/task.yaml")),
            state: &state,
            index: 2,
            attempt: 1,
            tools: &tools,
            run_directory: Path::new("."),
            requirements: &requirements,
            reaching: &reaching,
            preceding_llm: None,
        };
        metaharness_frame(&context, &[], "development/default", "1")
    }
/// Compares `produced` against the committed golden, or writes it when there is none.
    ///
    /// Written-when-absent and then failing, on `aep-render`'s rule: a regeneration is a reviewable
    /// diff and never a silent overwrite. There is deliberately **no** environment variable that
    /// accepts whatever the minter now produces — this file is another repository's input, and a
    /// golden that rewrites itself pins nothing on either side of the seam.
    fn golden(produced: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(GOLDEN);
        let Ok(committed) = fs::read_to_string(&path) else {
            fs::create_dir_all(path.parent().expect("the golden has a directory"))
                .expect("the fixture directory is writable");
            fs::write(&path, produced).expect("the golden is writable");
            panic!(
                "no golden at {}; it has been written — review it and run again",
                path.display()
            );
        };
        if committed == produced {
            return;
        }
        let differs = committed
            .lines()
            .zip(produced.lines())
            .enumerate()
            .find(|(_, (want, got))| want != got)
            .map_or_else(
                || {
                    (
                        committed.lines().count().min(produced.lines().count()) + 1,
                        "<end of file>".to_owned(),
                        "<more lines>".to_owned(),
                    )
                },
                |(index, (want, got))| (index + 1, want.to_owned(), got.to_owned()),
            );
        let (line, want, got) = differs;
        panic!(
            "{} differs at line {line}\n  committed: {want}\n  produced:  {got}\n\
             delete the file and re-run to accept the new document — and say so in the story, \
             because metaharness replays these bytes",
            path.display()
        );
    }
/// The golden is the bytes the driver writes, not a hand-typed copy of them.
    ///
    /// It is minted through [`metaharness_frame`] and rendered through [`frame_document`], which is
    /// the path `write_frame_document` takes; only the `fs::write` is missing. A fixture assembled
    /// any other way would drift from the driver in silence, and the contract test that reads it
    /// (`tests/metaharness_frame_contract.rs`) would then be certifying a document nothing sends.
    #[test]
    fn the_committed_golden_is_the_document_the_driver_would_write() {
        let document = frame_document(&canonical_frame()).expect("the frame renders");
        golden(&document);
    }
/// Two mints of the same step agree byte for byte, or the golden could not be committed and the
    /// digest could not be cited across the process boundary it is only ever cited across.
    #[test]
    fn two_mints_of_the_same_step_are_the_same_document() {
        assert_eq!(
            frame_document(&canonical_frame()).expect("the frame renders"),
            frame_document(&canonical_frame()).expect("the frame renders")
        );
    }
/// A protocol declaring more than the profile below grants, so a capability can be *known* and
    /// still not be granted — which is the state a `NotGranted` decision needs.
    const AUTHORIZE_PROTOCOL: &str = r"
id: aep
version: 1
title: Test protocol
capabilities: [repository.read, repository.write, command.execute, tests.execute]
evidence_kinds: [test_result, diff, approval]
verifiers: [test-runner, compiler, human-approval]
artifact_kinds: [story]
phases: [implementation]
observables:
  - 'task.**'
  - 'tests.**'
  - 'diff.**'
  - 'artifact.**'
  - 'evidence.**'
  - 'state.**'
  - 'workflow.**'
  - 'approvals.**'
";
const AUTHORIZE_WORKFLOW: &str = r"
id: test/linear
version: 1
title: Linear
initial: implement
states:
  implement:
    title: Implement
    phases: [implementation]
  complete:
    title: Complete
    terminal: true
    phases: [implementation]
transitions:
  - from: implement
    to: complete
    when: diff.exists
";
/// The profile that makes the fixture load-bearing: it grants `repository.read` and **not**
    /// `repository.write`, so a state whose rendered surface offers `Edit` is a state where the two
    /// layers disagree and the engine is the one that refuses.
    const AUTHORIZE_PROFILE: &str = r"
id: test.reading
title: Reading only
protocol: aep/1
workflow: test/linear
capabilities:
  allow: [repository.read]
completion:
  - diff.exists
";
const AUTHORIZE_TASK: &str = r"
id: T-1
kind: feature
objective: drive something
protocol: aep/1
profile: test.reading
";
/// An engine over those documents, and an execution of that task in `implement`.
    fn authorizing_execution() -> (Engine, aep_engine::execution::Execution) {
        use aep_engine::ProtocolEngine as _;
        let mut registry = Registry::new();
        registry
            .insert_protocol(
                aep_schema::parse::protocol(AUTHORIZE_PROTOCOL, None).expect("the protocol parses"),
            )
            .expect("the protocol is unique");
        registry
            .insert_workflow(
                aep_schema::parse::workflow(AUTHORIZE_WORKFLOW, None).expect("the workflow parses"),
            )
            .expect("the workflow is unique");
        registry
            .insert_profile(
                aep_schema::parse::profile(AUTHORIZE_PROFILE, None).expect("the profile parses"),
            )
            .expect("the profile is unique");
        let engine = Engine::new(registry);
        let execution = engine
            .initialize(aep_schema::parse::task(AUTHORIZE_TASK, None).expect("the task parses"))
            .expect("the task resolves");
        (engine, execution)
    }
/// One `tool.requested` event line of the shape metaharness writes in ask mode.
    fn requested(tool: &str, input: &serde_json::Value) -> String {
        format!(
            "{}\n",
            serde_json::json!({
                "format": "metaharness.event/1",
                "event": "tool.requested",
                "decision_required": true,
                "call_id": "call-1",
                "name": tool,
                "input": input,
            })
        )
    }
/// Runs one scripted call through the whole seam and returns the decision written back down
    /// stdin — the same object metaharness reads, never a summary of it.
    fn decide_through_the_seam(
        context: &StepContext<'_>,
        surface: WriteSurface<'_>,
        engine: &Engine,
        execution: &mut aep_engine::execution::Execution,
        tool: &str,
        input: &serde_json::Value,
    ) -> serde_json::Value {
        use aep_engine::ProtocolEngine as _;
        let mut commands: Vec<u8> = Vec::new();
        let mut transcript: Vec<u8> = Vec::new();
        {
            let mut authorize =
                |request: &ActionRequest| engine.authorize(&mut *execution, request);
            answer_events(
                Harness::ClaudeCode,
                context,
                surface,
                requested(tool, input).as_bytes(),
                &mut commands,
                &mut transcript,
                &mut authorize,
            );
        }
        assert!(
            !transcript.is_empty(),
            "every event line reaches the transcript, decided or not"
        );
        serde_json::from_slice(&commands).expect("one `tool.decide` command line")
    }
/// Every event the execution recorded, by name.
    fn event_names(execution: &aep_engine::execution::Execution) -> Vec<String> {
        execution
            .events()
            .iter()
            .map(|envelope| envelope.event.name().to_owned())
            .collect()
    }
/// The gap the guide called *"a decision is in the run's record, not yet in the engine's"*,
    /// closed: the engine refuses the call **and** the refusal is in the execution's own events.
    ///
    /// The fixture reaches the state where the rule is load-bearing before asserting the outcome —
    /// the policy layer is asserted to *allow* this call first, because a test where both layers
    /// refuse would pass whether or not the engine was ever asked.
    #[test]
    fn a_call_the_engine_refuses_is_denied_and_the_refusal_is_in_the_executions_event_record() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let writing = config(&[Capability::RepositoryRead, Capability::RepositoryWrite]);
        let context = policy_context(&state, &writing);
        let input = serde_json::json!({
            "file_path": "/repo/src/lib.rs",
            "old_string": "a",
            "new_string": "b",
        });
        assert!(
            decide_tool(&context, no_scope(), "Edit", &input).is_ok(),
            "the policy layer admits this call, so the engine is the layer under test"
        );

        let command = decide_through_the_seam(
            &context,
            no_scope(),
            &engine,
            &mut execution,
            "Edit",
            &input,
        );

        assert_eq!(command["command"], "tool.decide");
        assert_eq!(command["call_id"], "call-1");
        assert_eq!(command["decision"]["decision"], "deny");
        let reason = command["decision"]["reason"]
            .as_str()
            .expect("a denial says why");
        assert!(
            reason.contains("the engine refuses this call"),
            "the reason names the layer that refused: {reason}"
        );
        assert!(
            reason.contains("repository.write") && reason.contains("not_granted"),
            "and carries the engine's own words: {reason}"
        );

        let denied = execution
            .events()
            .iter()
            .find(|envelope| envelope.event.name() == "action_denied")
            .expect(
                "the refusal is in the execution's event record, which is what authorize is for",
            );
        let json = serde_json::to_value(&denied.event).expect("the event serialises");
        assert_eq!(json["capability"], "repository.write");
        assert_eq!(json["decision"], "not_granted");
        assert!(
            event_names(&execution).contains(&"action_requested".to_owned()),
            "the request is recorded beside the refusal: {:?}",
            event_names(&execution)
        );
    }
/// Policy first, and a call it refuses never reaches the engine.
    ///
    /// The order matters in both directions: the argument-level rules are the only layer that can
    /// tell `protocol plan artifact list` from `cargo test`, and an engine asked about a call the driver
    /// already refused would record an action nobody was allowed to attempt.
    #[test]
    fn a_call_the_policy_refuses_is_attributed_to_the_policy_and_never_reaches_the_engine() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let shell = config(&[Capability::CommandExecution]);
        let context = policy_context(&state, &shell);
        let input = serde_json::json!({ "command": "cargo test" });

        let command = decide_through_the_seam(
            &context,
            no_scope(),
            &engine,
            &mut execution,
            "Bash",
            &input,
        );

        assert_eq!(command["decision"]["decision"], "deny");
        let reason = command["decision"]["reason"]
            .as_str()
            .expect("a denial says why");
        assert!(
            reason.contains("the driver's per-call policy refuses"),
            "the reason names the layer that refused: {reason}"
        );
        assert!(
            !event_names(&execution).contains(&"action_requested".to_owned()),
            "a call the policy refused is not an action the engine was asked about: {:?}",
            event_names(&execution)
        );
    }
/// The `None` arm of the table, exercised: a `Skill` load is admitted by the policy and the
    /// engine is not consulted, because no `ActionRequest` describes loading instructions.
    #[test]
    fn a_skill_load_is_admitted_without_the_engine_being_asked_to_invent_an_action() {
        let (engine, mut execution) = authorizing_execution();
        let state: StateId = "implement".parse().expect("a state id");
        let reading = config(&[Capability::RepositoryRead]);
        let context = policy_context(&state, &reading);
        let input = serde_json::json!({ "skill": "planning" });

        let command = decide_through_the_seam(
            &context,
            no_scope(),
            &engine,
            &mut execution,
            "Skill",
            &input,
        );

        assert_eq!(command["decision"]["decision"], "allow");
        assert!(
            !event_names(&execution).contains(&"action_requested".to_owned()),
            "loading instructions is not an action, and the record must not claim one: {:?}",
            event_names(&execution)
        );
    }
/// The table itself: which tool is which action, and what each therefore needs.
    ///
    /// Asserted as capabilities rather than as variants, because the capability is the only thing
    /// the engine decides on — and asserted on the *payload* too, so a request that reached the
    /// record naming the wrong file would fail here rather than mislead an audit.
    #[test]
    fn each_offered_tool_renders_as_the_action_it_is_and_two_render_as_none() {
        let needs = |tool: &str, input: serde_json::Value| {
            action_for(tool, &input).map(|request| request.required_capability().to_string())
        };
        let read = serde_json::json!({ "file_path": "/repo/src/lib.rs" });
        assert_eq!(needs("Read", read.clone()), Some("repository.read".into()));
        assert_eq!(
            needs("Grep", serde_json::json!({ "pattern": "fn main" })),
            Some("repository.read".into()),
            "a search with no path is a search of the working directory"
        );
        assert_eq!(needs("Edit", read.clone()), Some("repository.write".into()));
        assert_eq!(needs("Write", read), Some("repository.write".into()));
        assert_eq!(
            needs(
                "NotebookEdit",
                serde_json::json!({ "notebook_path": "/n.ipynb" })
            ),
            Some("repository.write".into())
        );
        assert_eq!(
            needs(
                "Bash",
                serde_json::json!({ "command": "protocol plan artifact list" })
            ),
            Some("command.execute".into())
        );
        assert_eq!(
            needs(
                "WebFetch",
                serde_json::json!({ "url": "https://example.test/" })
            ),
            Some("network.read".into())
        );

        assert!(
            action_for("Skill", &serde_json::json!({ "skill": "planning" })).is_none(),
            "loading instructions takes no action"
        );
        assert!(
            action_for("WebSearch", &serde_json::json!({ "query": "aep" })).is_none(),
            "a search names no URL, and a request stating one nobody asked for is a fiction"
        );

        let request = action_for(
            "Bash",
            &serde_json::json!({ "command": "protocol plan artifact list --kind story" }),
        )
        .expect("a shell call renders");
        assert_eq!(
            request.action.summary(),
            "run `protocol plan artifact list --kind story`",
            "what the engine records is the call that was made"
        );
        let request = action_for("Read", &serde_json::json!({ "file_path": "/repo/x.rs" }))
            .expect("a read renders");
        assert_eq!(request.action.summary(), "read /repo/x.rs");
    }
/// The launch-time refusal, and the one case it must not fire in.
    ///
    /// A map of `command` steps drives on a machine with no metaharness and no vendor, so the check
    /// is scoped to maps that would spawn one. `PATH` is not manipulated here — the assertion is
    /// about what is checked, and the refusal's text is what an operator has to act on.
    #[test]
    fn a_map_with_an_llm_step_is_refused_at_launch_when_the_seams_binary_is_missing() {
        let commands_only = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/commands\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: command\n        run: [\"true\"]\n",
            None,
        )
        .expect("the map validates");
        assert!(
            metaharness_preflight(&commands_only).is_none(),
            "a map that spawns no session needs no seam binary, whatever is on PATH"
        );

        let with_llm = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/llm\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: llm\n        prompt: do the thing\n",
            None,
        )
        .expect("the map validates");
        match metaharness_preflight(&with_llm) {
            None => assert!(
                on_path(METAHARNESS_BINARY),
                "the only reason to allow an `llm` map is that the binary is installed"
            ),
            Some(refusal) => {
                assert!(
                    refusal.contains("cargo install --path crates/metaharness-cli"),
                    "a refusal answers the question it creates: {refusal}"
                );
                assert!(
                    refusal.contains("not on PATH"),
                    "and says what it found: {refusal}"
                );
            }
        }
    }
#[test]
    fn paid_maps_require_exact_explicit_terms_and_command_maps_do_not() {
        let commands_only = aep_schema::parse::step_map(
            "format: aep.driver-steps/1\nid: test/commands-budget\nworkflow: test/linear/1\n\
             states:\n  implement:\n    steps:\n      - kind: command\n        run: [\"true\"]\n",
            None,
        )
        .expect("the command map validates");
        assert_eq!(
            spend_terms_with_live(&commands_only, None, None, false)
                .expect("a command-only map is free"),
            None
        );

        let map = b10x_map();
        let not_live = spend_terms_with_live(&map, Some("5"), Some("0.5"), false)
            .expect_err("the live opt-in is required")
            .to_string();
        assert!(not_live.contains("METAHARNESS_LIVE=1"), "{not_live}");

        let no_cap = spend_terms_with_live(&map, None, Some("0.5"), true)
            .expect_err("the outer cap is required")
            .to_string();
        assert!(no_cap.contains("--budget-usd"), "{no_cap}");
        let no_assumption = spend_terms_with_live(&map, Some("5"), None, true)
            .expect_err("the launch charge is required")
            .to_string();
        assert!(
            no_assumption.contains("--assume-usd-per-run"),
            "{no_assumption}"
        );

        for (cap, assumed) in [("0", "0.5"), ("5", "0"), ("5", "0.0000001")] {
            assert!(
                spend_terms_with_live(&map, Some(cap), Some(assumed), true).is_err(),
                "cap `{cap}` and assumed charge `{assumed}` cannot authorize a launch"
            );
        }
    }
#[test]
    fn the_next_assumed_charge_is_persisted_only_when_it_fits() {
        let root = std::env::temp_dir().join(format!(
            "protocol-drive-spend-budget-{}",
            std::process::id()
        ));
        fs::remove_dir_all(&root).ok();
        fs::create_dir_all(&root).expect("the spend fixture is writable");
        let terms = SpendTerms {
            cap_micro_usd: 1_000_000,
            assumed_micro_usd_per_run: 400_000,
        };
        let mut budget = SpendBudget::start(&root, terms).expect("the ledger starts");

        budget.reserve().expect("the first launch fits");
        budget.reserve().expect("the second launch fits");
        let refusal = match budget.reserve() {
            Err(ReserveError::Exhausted(reason)) => reason,
            Err(ReserveError::Persist(reason)) => panic!("the fixture should persist: {reason}"),
            Ok(()) => panic!("the third launch crosses the cap"),
        };
        assert!(refusal.contains("$0.800000"), "{refusal}");
        assert!(refusal.contains("$0.400000"), "{refusal}");
        assert!(refusal.contains("$1.000000"), "{refusal}");

        let persisted: serde_json::Value = serde_json::from_slice(
            &fs::read(root.join(SPEND_FILE)).expect("the ledger remains readable"),
        )
        .expect("the ledger is JSON");
        assert_eq!(persisted["spent_micro_usd"], 800_000);
        assert_eq!(persisted["launches"], 2);

        fs::write(
            root.join(SPEND_FILE),
            "{\"format\":\"aep.drive-spend/1\",\"spent_micro_usd\":1,\"launches\":2}\n",
        )
        .expect("the fixture can plant an inconsistent ledger");
        let inconsistent = SpendBudget::resume(&root, terms)
            .expect_err("a ledger whose two exact counts disagree")
            .to_string();
        assert!(inconsistent.contains("inconsistent"), "{inconsistent}");
    }
#[test]
    fn a_resume_inherits_or_narrows_its_cap_and_never_widens_it() {
        let map = b10x_map();
        let original = SpendTerms {
            cap_micro_usd: 5_000_000,
            assumed_micro_usd_per_run: 500_000,
        };
        assert_eq!(
            resumed_spend_terms_with_live(&map, Some(original), None, true)
                .expect("the remembered terms are sufficient"),
            Some(original)
        );
        assert_eq!(
            resumed_spend_terms_with_live(&map, Some(original), Some("3.5"), true)
                .expect("a lower cap narrows authority"),
            Some(SpendTerms {
                cap_micro_usd: 3_500_000,
                ..original
            })
        );

        let widened = resumed_spend_terms_with_live(&map, Some(original), Some("6"), true)
            .expect_err("a resume cannot acquire more authority")
            .to_string();
        assert!(widened.contains("not raise"), "{widened}");
        let legacy = resumed_spend_terms_with_live(&map, None, Some("5"), true)
            .expect_err("a cap cannot bound sessions that already ran")
            .to_string();
        assert!(legacy.contains("predates"), "{legacy}");
        let malformed = SpendTerms {
            cap_micro_usd: 5_000_000,
            assumed_micro_usd_per_run: 0,
        };
        let malformed = resumed_spend_terms_with_live(&map, Some(malformed), None, true)
            .expect_err("remembered zero must not authorize unlimited free launches")
            .to_string();
        assert!(malformed.contains("invalid"), "{malformed}");
    }
#[test]
    fn the_rendering_offers_a_shell_only_when_the_capability_is_admitted() {
        let reading = config(&[Capability::RepositoryRead]);
        assert_eq!(allowed_tools(&reading), ["Glob", "Grep", "Read", "Skill"]);
        assert!(!allowed_tools(&reading).contains(&"Bash".to_owned()));

        let shell = config(&[Capability::CommandExecution]);
        assert!(allowed_tools(&shell).contains(&"Bash".to_owned()));
    }
#[test]
    fn a_subagent_spawner_is_never_rendered_whatever_is_admitted() {
        let everything = config(&[
            Capability::RepositoryRead,
            Capability::RepositoryWrite,
            Capability::CommandExecution,
            Capability::NetworkRead(Audience::Any),
            Capability::Deploy(Environment::Production),
        ]);
        assert!(
            !allowed_tools(&everything).contains(&"Task".to_owned()),
            "a subagent's tool set is derived by nothing in D1-D6, so it is a route around the \
             per-state allowlist"
        );
    }
/// The committed step map compiles into an exact argv, with nobody naming a flag.
    ///
    /// **This is the acceptance bullet of `story:compile-scope-into-a-run` that had no test.**
    /// Every other argv test here builds its own step, so all of them would still pass if
    /// `drivers/development/default.yaml` lost its `scope:` tomorrow: the declaration and the
    /// compile were only ever checked against each other through a fixture. This one reads the
    /// committed file, takes the step `receive` declares, and asserts the whole vector the driver
    /// would launch, in order.
    ///
    /// It goes through [`CliExecutors::argv_for`] rather than calling [`b10x_argv`] directly, which
    /// closes the other half of the same bullet: nothing else asserted that the caller hands the
    /// compile the **step's own** `scope` and `context` rather than something it assembled.
    ///
    /// **The tool config admits reading and not execution, deliberately.** With
    /// [`Capability::CommandExecution`] the argv gains `--allow-program` naming this process's own
    /// absolute path ([`driven_programs`], from `std::env::current_exe`), which is a different
    /// string in every checkout and under every runner — so an *exact* assertion could only be
    /// written by computing it the same way, and would then assert nothing. What this bullet is
    /// about is the scope and the context, and both are here in full.
    ///
    /// The declared context file is asserted to exist in this checkout, which catches a map naming
    /// a file a later commit moved. It is **not** the run-time refusal for an absent context file:
    /// that one belongs to the loop (`harness-cli` reads the declared files and refuses the launch)
    /// and is recorded as still untested on the story.
    #[test]
    fn the_committed_step_map_compiles_into_the_exact_argv_a_native_run_is_launched_with() {
        let repository = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures/aep")
            .canonicalize()
            .expect("the workspace root exists");
        let path = repository.join("drivers/development/default.yaml");
        let text = fs::read_to_string(&path).expect("the committed step map is readable");
        let map = aep_schema::parse::step_map(&text, Some(&path.display().to_string()))
            .expect("the committed step map validates");

        let state: StateId = "receive".parse().expect("a state id");
        let Some(Step::Llm(step)) = map
            .states
            .get(&state)
            .expect("the committed map drives `receive`")
            .steps
            .first()
        else {
            panic!("`receive`'s first step is the `llm` one this test is about");
        };

        // The argv is built from the committed declaration, so assert the declaration is still the
        // one this test was written against before asserting what it renders as.
        assert!(
            step.context.is_empty(),
            "the committed map gives `receive` no context file: the planning skill it used to hand \
             over eagerly now arrives through the plugin, one `skill` call away instead of billed \
             on every turn of a stateless loop"
        );
        assert_eq!(
            step.scope.len(),
            3,
            "three rules, the last of them the catch-all validation requires"
        );

        let executors = CliExecutors::new(
            PathBuf::from("/operator/repo"),
            PathBuf::from("/runs/T-1/1"),
            // The same plugin reaches this arm through the loop's own plugin reader.
            vec![PathBuf::from("/plugins/claude-code")],
            "adp/default".to_owned(),
            "1".to_owned(),
            B10xOptions {
                endpoint: Some("http://127.0.0.1:8080".to_owned()),
                model: Some("a-model".to_owned()),
                api_key: false,
                ..B10xOptions::default()
            },
        );
        let tools = config(&[Capability::RepositoryRead]);
        let task = driven_task();
        let context = step_context(&tools, &state, &task);

        let argv = executors.argv_for(
            Harness::B10x,
            step,
            Path::new("/runs/T-1/1/transcripts/receive-0-1.frame.json"),
            "do the thing",
            &context,
            None,
        );

        assert_eq!(
            argv,
            vec![
                "metaharness",
                "run",
                "b10x",
                "--hermetic",
                "--decisions",
                "observe",
                "--cwd",
                "/operator/repo",
                "--model-endpoint",
                "http://127.0.0.1:8080",
                "--model",
                "a-model",
                "--credentials",
                "none",
                "--write-scope",
                ".engineering/planning/**=denied",
                "--write-scope",
                "crates/**=allowed",
                "--write-scope",
                "docs/**=allowed",
                "--write-scope",
                "conformance/**=allowed",
                "--write-scope",
                "drivers/**=allowed",
                "--write-scope",
                "**=denied",
                // **The plugin, and no `--context` beside it.** The loop reads the skills half
                // of the vendor's on-disk format, so the step is offered the same library the
                // vendor arm is rather than having to find the CLI's own `skill load` verb for
                // itself — and the map no longer hands it the same document eagerly on every turn
                // as well, which is what it did while there was no other route.
                "--plugin-dir",
                "/plugins/claude-code",
                "-p",
                "do the thing",
            ],
            "the committed map's `receive` step, compiled: its six `--write-scope` rules in the \
             order the document writes them, then its one `--context` file, and no frame"
        );
    }
}
