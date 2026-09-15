//! Concrete governed model execution, hosted above AEP.
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command as Process, ExitCode, Stdio};

use aep_domain::action::{
    Action, ActionRequest, CommandExecute, NetworkIntent, NetworkRequest, RepositoryRead,
    RepositoryWrite,
};
use aep_domain::capability::{Audience, Capability};
use aep_domain::ids::StateId;
use aep_driver::executor::{LlmStepExecutor, StepAuthorizer, StepContext, StepOutcome};
use aep_driver::tool::TOOL_CANDIDATES;
use aep_driver_spec::map::{LlmStep, ScopeRule, Step, StepMap, WriteScope};
use aep_driver_spec::tool::ToolConfig;
use anyhow::{Context, Result, bail};
use clap::{Args, Subcommand};
// The one glob matcher in the workspace, and the one a step map's `scope:` is decided with. Taken
// from `trace-domain` rather than written again here for the reason `AGENTS.md` gives about a
// second copy of a rule: two matchers would disagree about `*` the first time either was touched,
// and this one is already property-tested against the paths the design writes
// (`crates/observe/trace-domain/src/matcher.rs`).
use aep_cli::drive::{
    Answer, CommandOnlyHost, ExecutionHost, Inputs, Launch, Moment, PreparedExecution,
    READ_ONLY_PROGRAMS, TRANSCRIPTS, TransitionArgs, WriteSurface, answer, declared_write,
    driven_surface, engine_refusal, has_llm_steps, llm_step_count, protocol_command_steps,
    session_env, store_integrity_at, transcript_path, write_scope_word,
};

/// Reservations made against a driven run's model-session ceiling.
const SPEND_FILE: &str = "spend.json";

/// The explicit opt-in shared with `metaharness aep drive eval run` for paid model processes.
const METAHARNESS_LIVE_ENV: &str = "METAHARNESS_LIVE";

/// What a `harness: b10x` step needs and a step map cannot say.
///
/// **Three facts about the machine, not about the work.** `metaharness run b10x` refuses a launch
/// that names no endpoint and no model rather than defaulting either — *"a default here would aim
/// an evaluation arm at somebody's production API the first time the flag was forgotten"* — and
/// the b10x loop holds no vendor login of its own, so the credential is a file or a variable
/// somebody named. None of that belongs in a step map: a map is pinned, committed and driven on
/// more than one machine, and an endpoint written into one would be the same URL for all of them.
///
/// They are flags rather than environment variables for the reason the launch record exists at
/// all: a run has to be able to say what it was started with, and a variable that was exported in
/// one shell is not a fact anybody can read back afterwards. They persist into `launch.json` with
/// everything else, so a `resume` re-reads them instead of being told again.
#[derive(Debug, Clone, Default, Args, serde::Serialize, serde::Deserialize)]
pub struct B10xOptions {
    /// The endpoint a `harness: b10x` step's loop is pointed at, as the gateway's root URL.
    #[arg(long = "b10x-endpoint", value_name = "BASE_URL")]
    #[serde(default)]
    endpoint: Option<String>,
    /// The planning executable built from the AEP source this runner pins.
    #[arg(long = "aep-binary", value_name = "FILE")]
    #[serde(default)]
    aep_binary: Option<PathBuf>,
    /// The model that endpoint serves. The loop picks none of its own.
    #[arg(long = "b10x-model", value_name = "MODEL")]
    #[serde(default)]
    model: Option<String>,
    /// Point the loop at `OPENAI_API_KEY` instead of launching it with no credential at all.
    ///
    /// Off by default, because a gateway that authenticates nobody is the case a driven b10x run
    /// starts in and a credential nobody asked to send is one that travels by accident. With it,
    /// metaharness refuses the launch by name when the variable is not in this process's
    /// environment rather than starting a run that will fail at its first request.
    #[arg(long = "b10x-api-key")]
    #[serde(default)]
    api_key: bool,
    /// Point a `harness: claude-code` step at the same gateway, so the two arms differ by harness.
    ///
    /// **This is what makes a harness comparison a comparison.** With one arm on a vendor's own
    /// model and the other on whatever a gateway serves, a difference in waste is a difference in
    /// two things at once, and no scorer can separate them afterwards. Pointed here, both arms
    /// speak to the same endpoint — Claude Code reaches Anthropic messages at `{root}/v1/messages`
    /// and the b10x loop the Responses wire at `{root}/v1/responses`, which is the generic model
    /// adapter's whole point.
    ///
    /// metaharness requires `--credentials none` alongside an endpoint, because a child pointed at
    /// a foreign endpoint must hold no operator credential; the driver passes it rather than making
    /// the caller remember.
    #[arg(long = "claude-endpoint", value_name = "BASE_URL")]
    #[serde(default)]
    claude_endpoint: Option<String>,
    /// The model that endpoint serves, for the Claude Code arm.
    #[arg(long = "claude-model", value_name = "MODEL")]
    #[serde(default)]
    claude_model: Option<String>,
    /// A delegated cgroup subtree, so a confined `b10x` step may execute.
    ///
    /// **Without it the arm cannot attempt a test-first task, and metaharness says why:** *"a run
    /// that may not execute its suite cannot see a test fail before writing the code, so it will
    /// not write the code."* Substrate publishes no `run` entry without a subtree to start a
    /// process in, so the catalogue behind the loop's three verbs stays read-only and the arm can
    /// read a repository and change nothing in it.
    ///
    /// Passing it also turns on `--substrate-embedded`, because the two are one decision: confining
    /// a workspace and being allowed to execute inside it are the same intent, and a run given one
    /// without the other is a run that can write and not test, or test and not write. Embedded
    /// rather than a daemon socket, on metaharness's own advice — *"right for a run on the
    /// operator's own machine, wrong for anything multi-tenant"* — which is what a driven dogfood
    /// run is.
    ///
    /// The subtree must be delegated to this user and hold `cpu`, `memory` and `pids`. On a systemd
    /// machine that is `/sys/fs/cgroup/user.slice/user-$(id -u).slice/user@$(id -u).service`.
    #[arg(long = "b10x-cgroup-root", value_name = "DIR")]
    #[serde(default)]
    cgroup_root: Option<PathBuf>,
    /// Which model API the loop speaks under `--b10x-endpoint`.
    ///
    /// The loop reaches two different endpoints under one root — `openai-responses` at
    /// `{root}/responses` and `anthropic-messages` at `{root}/messages` — and infers neither from
    /// the URL. Left unset, the loop keeps its own default, which is the Responses wire.
    #[arg(long = "b10x-wire", value_name = "WIRE")]
    #[serde(default)]
    wire: Option<String>,
    /// A file holding a subscription token for the b10x arm, instead of an API key.
    ///
    /// **This is what lets the native arm run against a model with a window the protocol fits
    /// in.** With only `--b10x-api-key` the arm could reach a gateway and whatever that gateway
    /// served; run `b10x-32k` died at turn 37 on `maximum context length is 32768 tokens` with
    /// the state half finished, which is a fact about the endpoint and not about the harness, and
    /// no scorer can tell the two apart afterwards.
    ///
    /// Named and never read here: the path travels into an argv and the token enters neither this
    /// process nor metaharness.
    #[arg(long = "b10x-oauth-token-file", value_name = "FILE")]
    #[serde(default)]
    oauth_token_file: Option<PathBuf>,
    /// A JSON pointer to the token inside that file, when the file is a JSON document.
    #[arg(long = "b10x-oauth-token-pointer", value_name = "POINTER")]
    #[serde(default)]
    oauth_token_pointer: Option<String>,
}

impl B10xOptions {
    /// The gateway a Claude Code step is pointed at, when both halves were given.
    ///
    /// Both or neither: an endpoint with no model reaches a gateway and asks it for nothing, and a
    /// model with no endpoint is a word with nowhere to go. metaharness refuses each on its own,
    /// and a driver that passed one and not the other would turn a flag mistake into a launch
    /// refusal three states into a paid run.
    fn claude_gateway(&self) -> Option<(&str, &str)> {
        match (&self.claude_endpoint, &self.claude_model) {
            (Some(endpoint), Some(model)) => Some((endpoint.as_str(), model.as_str())),
            _ => None,
        }
    }
}

/// The immutable cost terms one launch declares and every resume inherits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct SpendTerms {
    /// Maximum total reservation.
    cap_micro_usd: u64,
    /// Charge reserved before each metaharness spawn.
    assumed_micro_usd_per_run: u64,
}

/// The append-in-effect spend state persisted before a model process is spawned.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct SpendState {
    format: String,
    spent_micro_usd: u64,
    launches: u64,
}

/// A cost ceiling held by the executor that performs the paid effect.
#[derive(Debug)]
struct SpendBudget {
    terms: SpendTerms,
    state: SpendState,
    path: PathBuf,
}

/// Why reserving the next session did not produce authority to spawn it.
#[derive(Debug)]
enum ReserveError {
    /// The declared bound would be crossed; retrying cannot change this.
    Exhausted(String),
    /// The reservation could not be made durable, so no paid effect is allowed.
    Persist(String),
}

impl SpendBudget {
    /// Starts a new empty ledger beside the run's launch record.
    fn start(run_directory: &Path, terms: SpendTerms) -> Result<Self> {
        let budget = Self {
            terms,
            state: SpendState {
                format: "aep.drive-spend/1".to_owned(),
                ..SpendState::default()
            },
            path: run_directory.join(SPEND_FILE),
        };
        budget.persist().map_err(anyhow::Error::msg)?;
        Ok(budget)
    }

    /// Reopens the reservations a prior invocation made.
    fn resume(run_directory: &Path, terms: SpendTerms) -> Result<Self> {
        let path = run_directory.join(SPEND_FILE);
        let text = fs::read_to_string(&path)
            .with_context(|| format!("reading the spend ledger at {}", path.display()))?;
        let state: SpendState = serde_json::from_str(&text)
            .with_context(|| format!("reading the spend ledger at {}", path.display()))?;
        if state.format != "aep.drive-spend/1" {
            bail!(
                "{} claims spend-ledger format `{}`, not `aep.drive-spend/1`",
                path.display(),
                state.format
            );
        }
        let expected_spent = state
            .launches
            .checked_mul(terms.assumed_micro_usd_per_run)
            .context("the spend ledger's launch count is too large to account for exactly")?;
        if expected_spent != state.spent_micro_usd {
            bail!(
                "{} records {} launch(es) at {} each but says {} was reserved; the spend ledger is inconsistent",
                path.display(),
                state.launches,
                aep_cli::money::dollars(terms.assumed_micro_usd_per_run),
                aep_cli::money::dollars(state.spent_micro_usd),
            );
        }
        Ok(Self { terms, state, path })
    }

    /// Reserves one assumed charge and persists it before the caller may spawn.
    fn reserve(&mut self) -> std::result::Result<(), ReserveError> {
        let next = self.terms.assumed_micro_usd_per_run;
        let Some(after) = self.state.spent_micro_usd.checked_add(next) else {
            return Err(ReserveError::Exhausted(self.exhausted_line(next)));
        };
        if after > self.terms.cap_micro_usd {
            return Err(ReserveError::Exhausted(self.exhausted_line(next)));
        }
        let Some(launches) = self.state.launches.checked_add(1) else {
            return Err(ReserveError::Persist(
                "the spend ledger cannot count another launch exactly; no process was spawned"
                    .to_owned(),
            ));
        };
        self.state.spent_micro_usd = after;
        self.state.launches = launches;
        self.persist().map_err(ReserveError::Persist)
    }

    /// The durable accounting line a run report retains on a spend stop.
    fn exhausted_line(&self, next: u64) -> String {
        format!(
            "the model-session cost ceiling stopped the run before launch {}: {} already \
             reserved plus the next assumed charge {} would exceed the cap {}",
            self.state.launches.saturating_add(1),
            aep_cli::money::dollars(self.state.spent_micro_usd),
            aep_cli::money::dollars(next),
            aep_cli::money::dollars(self.terms.cap_micro_usd),
        )
    }

    /// Atomically publishes the next reservation.
    fn persist(&self) -> std::result::Result<(), String> {
        let next = self.path.with_extension("json.next");
        let rendered = serde_json::to_string_pretty(&self.state)
            .map_err(|error| format!("cannot render the spend reservation: {error}"))?;
        fs::write(&next, rendered + "\n")
            .map_err(|error| format!("cannot write {}: {error}", next.display()))?;
        fs::rename(&next, &self.path).map_err(|error| {
            format!(
                "cannot publish the spend reservation from {} to {}: {error}",
                next.display(),
                self.path.display()
            )
        })
    }
}

/// The three things that touch the world.
struct CliExecutors {
    /// Where a command step runs.
    working_directory: PathBuf,
    /// Where transcripts and logs go.
    run_directory: PathBuf,
    /// The plugins every `llm` step's session loads — and with them, the hooks.
    plugin_dirs: Vec<PathBuf>,
    /// The workflow the run resolved to, for the frame the `metaharness` executor writes.
    workflow_id: String,
    /// Its pinned major version, as the step map states it.
    workflow_version: String,
    /// What a `harness: b10x` step needs from this machine, and nothing else reads.
    b10x: B10xOptions,
    /// The one non-human actor whose recorded approval may answer an `operator` step, so the
    /// pause can say who may answer it.
    /// The run-level reservation ledger, present exactly when this map can spawn a model.
    spend: Option<SpendBudget>,
}

impl CliExecutors {
    /// Builds the executors for one run.
    fn new(
        working_directory: PathBuf,
        run_directory: PathBuf,
        plugin_dirs: Vec<PathBuf>,
        workflow_id: String,
        workflow_version: String,
        b10x: B10xOptions,
    ) -> Self {
        Self {
            working_directory,
            run_directory,
            plugin_dirs,
            workflow_id,
            workflow_version,
            b10x,
            spend: None,
        }
    }

    /// Attaches the run-level model-session ceiling after its ledger is durable.
    fn with_spend(mut self, spend: Option<SpendBudget>) -> Self {
        self.spend = spend;
        self
    }

    /// The step's sealed frame document, written beside the transcript it governs.
    /// Writes the hook file a native step's loop consults, and answers with its path.
    ///
    /// **This is the native arm's half of the content rule.** The vendor arm's calls come back
    /// through the metaharness seam and reach `store_integrity` in this process; the native loop
    /// decides in-process and consults programs, so the same rule is declared here as a program to
    /// spawn — `protocol drive hook`, this binary by the path `driven_programs` already names,
    /// calling the same `store_integrity_at`.
    ///
    /// Scoped to `file_edit` alone, because the fence rule is about the text an edit quotes.
    /// `file_write` replaces a whole file, which is the question the step map's `scope:` answers
    /// and `--write-scope` carries to this loop's own tools — asking a hook about it as well would
    /// be a second copy of that rule. The hook itself proceeds for any other entry, so the two
    /// agree rather than one relying on the other. `run` is deliberately absent: what a program may
    /// *be* is decided by the allowlist before the run starts, which is the stronger answer and the
    /// one already made.
    fn write_hooks_document(transcripts: &Path) -> Result<PathBuf, String> {
        let binary = std::env::current_exe()
            .map_err(|error| format!("cannot name this binary for the hook file: {error}"))?;
        let document = serde_json::json!({
            "version": 1,
            "hooks": [{
                "on": "before-call",
                "tools": ["file_edit"],
                "command": [binary.display().to_string(), "aep", "drive", "hook"],
            }],
        });
        let path = transcripts.join("hooks.json");
        let rendered = serde_json::to_string_pretty(&document)
            .map_err(|error| format!("cannot render the hook file: {error}"))?;
        fs::write(&path, rendered)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        Ok(path)
    }

    fn write_frame_document(
        &self,
        context: &StepContext<'_>,
        scope: &[ScopeRule],
        transcripts: &Path,
    ) -> Result<PathBuf, String> {
        let frame = metaharness_frame(context, scope, &self.workflow_id, &self.workflow_version);
        let path = transcripts.join(format!(
            "{}-{}-{}.frame.json",
            context.state, context.index, context.attempt
        ));
        let document = frame_document(&frame)?;
        fs::write(&path, document)
            .map_err(|error| format!("cannot write {}: {error}", path.display()))?;

        // Beside the frame, when this state refused anything. `protocol observe trace check` reads it as
        // it reads any specification.
        if let Some(refusals) = refusal_specification(context.state, context.index, context.tools) {
            let refusals_path = transcripts.join(format!(
                "{}-{}-{}.refused.json",
                context.state, context.index, context.attempt
            ));
            let rendered = serde_json::to_string_pretty(&refusals)
                .map_err(|error| format!("cannot render the refusal specification: {error}"))?;
            fs::write(&refusals_path, rendered)
                .map_err(|error| format!("cannot write {}: {error}", refusals_path.display()))?;
        }
        Ok(path)
    }

    /// The `metaharness run` invocation for one step, on whichever arm the step named.
    ///
    /// The frame document is written for **both** harnesses and passed to only one. See
    /// [`b10x_argv`]: metaharness refuses a b10x launch that carries a frame, because a frame is
    /// enforced through a decision channel that loop does not have. What the file is on that arm
    /// is the record of what the step was, in the same neutral vocabulary, beside the same refusal
    /// specification and the same `metaharness.event/1` transcript — which is what makes the two
    /// arms comparable at all.
    fn argv_for(
        &self,
        harness: Harness,
        step: &LlmStep,
        frame_file: &Path,
        prompt: &str,
        context: &StepContext<'_>,
        hooks: Option<&Path>,
    ) -> Vec<String> {
        match harness {
            // **The actor is derived here, beside the argv it belongs to.** `session_env` sets the
            // same value on every child this process spawns, which reaches a `command` step because
            // that is our child; an `llm` step's model is behind metaharness, which constructs its
            // child's environment rather than inheriting ours, so it has to be *said*. `None` when
            // the execution id has no actor spelling — the same silence `session_env` keeps, and
            // declaring a mangled name would be worse than declaring nothing.
            Harness::ClaudeCode => metaharness_argv(
                frame_file,
                &self.working_directory,
                &self.plugin_dirs,
                prompt,
                self.b10x.claude_gateway(),
                aep_driver::attest::session_actor(context.execution)
                    .map(|actor| actor.to_string())
                    .as_deref(),
            ),
            Harness::B10x => b10x_argv(
                &self.b10x,
                &self.working_directory,
                &step.scope,
                &step.context,
                prompt,
                context.tools,
                OperatorFiles {
                    hooks,
                    plugin_dirs: &self.plugin_dirs,
                },
            ),
        }
    }

    /// The one `llm` executor: the vendor is driven through the metaharness seam, in ask mode.
    ///
    /// The step's surface travels twice, deliberately (F9's "both halves"): the sealed
    /// `metaharness.frame/1` document pins what the step *is*, and this process answers every
    /// `tool.requested` event at decision time through [`decide_tool`] — the two retired shell
    /// hooks, ported, plus the per-state allowlist that used to ride on `--allowedTools` — and then
    /// through the **engine**, which is what `authorize` is. The decisions and denials arrive as
    /// `tool.decided` events in the event stream this executor writes as the transcript, never in a
    /// side-channel log a forgotten flag can silence: run `W4-2` lost all eight of its post-fix
    /// sessions to exactly that, a resume that dropped `--plugin-dir` and ran unenforced while
    /// looking clean.
    fn run_llm_metaharness(
        &mut self,
        harness: Harness,
        step: &LlmStep,
        context: &StepContext<'_>,
        authorize: StepAuthorizer<'_>,
    ) -> StepOutcome {
        let transcripts = self.run_directory.join(TRANSCRIPTS);
        if let Err(error) = fs::create_dir_all(&transcripts) {
            return StepOutcome::NoVerdict {
                reason: format!(
                    "cannot write transcripts to {}: {error}",
                    transcripts.display()
                ),
            };
        }
        let transcript = transcript_path(
            self.run_directory.as_path(),
            context.state,
            context.index,
            context.attempt,
        );

        let frame_file = match self.write_frame_document(context, &step.scope, &transcripts) {
            Ok(path) => path,
            Err(reason) => return StepOutcome::NoVerdict { reason },
        };

        // The native arm's content rule travels as a file; the vendor arm answers in this process
        // and needs none. Failing to write it refuses the step rather than running without it.
        let hooks = match harness {
            Harness::B10x => match Self::write_hooks_document(&transcripts) {
                Ok(path) => Some(path),
                Err(reason) => return StepOutcome::NoVerdict { reason },
            },
            Harness::ClaudeCode => None,
        };
        let argv = self.argv_for(
            harness,
            step,
            &frame_file,
            &prompt_for(step, context, &staged_driver(&self.b10x)),
            context,
            hooks.as_deref(),
        );
        // The last action before the paid effect. Persist first: if this process dies after the
        // child starts, a resume must not regain authority that was already handed to a session.
        let Some(spend) = self.spend.as_mut() else {
            return StepOutcome::BudgetExhausted {
                reason: "this map reached an `llm` step without a model-session cost ceiling; no \
                         metaharness process was spawned"
                    .to_owned(),
            };
        };
        match spend.reserve() {
            Ok(()) => {}
            Err(ReserveError::Exhausted(reason)) => {
                return StepOutcome::BudgetExhausted { reason };
            }
            Err(ReserveError::Persist(reason)) => {
                return StepOutcome::NoVerdict { reason };
            }
        }
        // No `current_dir`: the working directory travels as `--cwd` and metaharness spawns the
        // vendor there itself, with a constructed environment nothing here needs to reach into.
        //
        // `session_env` is set on **this** process all the same, and its own doc says how far it
        // gets: metaharness `env_clear()`s and rebuilds its child's environment from a fixed
        // allowlist, so the actor reaches metaharness and not the model's shell. Set here rather
        // than omitted because this is the launch that declares who the session is, and the day
        // the other side admits a variable this is the line that already says it.
        let binary = std::env::current_exe().unwrap_or_else(|_| PathBuf::from(&argv[0]));
        let spawned = Process::new(binary)
            .args(&argv[1..])
            .envs(session_env(context.execution))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();
        let mut child = match spawned {
            Ok(child) => child,
            Err(error) => {
                return StepOutcome::NoVerdict {
                    reason: format!("`{}` could not be run: {error}", argv.join(" ")),
                };
            }
        };
        let mut commands = child.stdin.take().expect("stdin was piped");
        let events = child.stdout.take().expect("stdout was piped");
        let stderr = child.stderr.take().expect("stderr was piped");
        // Drained on its own thread: a child blocked writing a full stderr pipe while this loop
        // blocks reading stdout is a deadlock, not a slow run.
        let stderr_thread = std::thread::spawn(move || {
            let mut text = String::new();
            let _ = std::io::Read::read_to_string(&mut std::io::BufReader::new(stderr), &mut text);
            text
        });

        let mut transcript_file = match fs::File::create(&transcript) {
            Ok(file) => file,
            Err(error) => {
                let _ = child.kill();
                return StepOutcome::NoVerdict {
                    reason: format!("cannot write {}: {error}", transcript.display()),
                };
            }
        };
        // **The declaration, handed to the seam.** The same `scope:` the native arm receives as
        // `--write-scope` decides this arm's writes too, so the rule a run is held to is the one a
        // person reads in the step map rather than one written into a policy function.
        let adjudication = answer_events(
            harness,
            context,
            WriteSurface {
                scope: &step.scope,
                root: &self.working_directory,
            },
            events,
            &mut commands,
            &mut transcript_file,
            authorize,
        );
        drop(commands);
        outln!("{}", adjudication.line(harness, context.state));
        let status = child.wait();
        let stderr_text = stderr_thread.join().unwrap_or_default();

        metaharness_outcome(status, &stderr_text, &transcript)
    }
}

/// What the step made of the harness having stopped.
///
/// Split out of [`CliExecutors::run_llm_metaharness`] so the spawn and the verdict are readable
/// apart;
/// the mapping is the whole reason a non-zero exit is not simply a panic.
///
/// A failed exit carries the **last three lines of stderr, not the first**: a harness that dies
/// says why at the end, and a head would quote its banner. The transcript path is named either
/// way, because the reader's next move is to open it rather than to re-run.
fn metaharness_outcome(
    status: std::io::Result<std::process::ExitStatus>,
    stderr_text: &str,
    transcript: &Path,
) -> StepOutcome {
    match status {
        Ok(status) if status.success() => {
            // An `llm` step never carries evidence, and the type is what makes that true.
            // What the model achieved that is checkable is observed by the command step
            // after it.
            StepOutcome::Nothing
        }
        Ok(status) => {
            let tail: String = stderr_text
                .lines()
                .rev()
                .take(3)
                .collect::<Vec<_>>()
                .join(" | ");
            StepOutcome::NoVerdict {
                reason: format!(
                    "metaharness exited {}; {}the event stream is at {}",
                    status
                        .code()
                        .map_or_else(|| "on a signal".to_owned(), |code| code.to_string()),
                    if tail.is_empty() {
                        String::new()
                    } else {
                        format!("it said: {tail}; ")
                    },
                    transcript.display()
                ),
            }
        }
        Err(error) => StepOutcome::NoVerdict {
            reason: format!("waiting on metaharness failed: {error}"),
        },
    }
}

impl LlmStepExecutor for CliExecutors {
    fn run_llm(
        &mut self,
        step: &LlmStep,
        context: &StepContext<'_>,
        authorize: StepAuthorizer<'_>,
    ) -> StepOutcome {
        // The seam § 4.9 point 3 names, and the reason it is a name rather than a trait: a
        // second harness is a second executor selected by this string. Since
        // `epic:metaharness-migration` there is no bare-argv path left to select — every name here
        // reaches a `metaharness run` invocation, because a second way to launch a session is a
        // second policy to forget. `claude-code` names the vendor, `metaharness` is the name the
        // executor first landed under, and `b10x` is the loop this org owns.
        let Some(harness) = Harness::named(&step.harness) else {
            return StepOutcome::NoVerdict {
                reason: format!(
                    "the step names harness `{}`, and this build invokes {}",
                    step.harness,
                    Harness::NAMES.map(|name| format!("`{name}`")).join(", ")
                ),
            };
        };
        self.run_llm_metaharness(harness, step, context, authorize)
    }
}

/// What the session is told about its own surface, in the harness's own words.
///
/// Split out of [`prompt_for`] because it is the one paragraph that is genuinely per-harness, and
/// because the two arms differ in *what kind of thing* bounds them: one is refused by a seam, the
/// other is never offered the tool at all. Rendered from the same [`ToolConfig`] the policy reads,
/// so the prompt and the enforcement cannot disagree — two lists that could drift would be worse
/// than one list nobody has, because the model would trust the wrong one.
fn surface_lines(harness: Harness, tools: &ToolConfig, driver: &str) -> String {
    let mut lines = String::new();
    let offered = harness.tools(tools);
    if !offered.is_empty() {
        lines.push_str("\nThe tools this state admits, and there are no others: ");
        lines.push_str(&offered.join(", "));
        lines.push_str(if harness.adjudicates() {
            ".\nA call outside that set is refused by the driver before it runs. Do not search for \
             a tool that is not on the list — it is not hidden, it does not exist here.\n"
        } else {
            // Not *refused* — **absent**. Telling an observed loop that a call will be refused
            // would describe a seam it does not have, and a session told to expect a refusal that
            // never comes learns nothing from the silence.
            ".\nThat list is what this **state** admits, and there is no seam behind it: a tool \
             outside it is not published to you at all, so there is nothing to search for and \
             nothing that will refuse you. What you were actually **given** is the part of it \
             this machine can confine, which may be less — a write or an exec entry appears only \
             where the workspace is confined. Work from the tools you have, and do not reach for \
             one that is named here and absent from your surface: it is not hidden, this machine \
             could not publish it.\n"
        });
    }
    // **The shell's two rules, stated rather than discovered.** `driven_surface` refuses on both,
    // and a session not told either learns them by being refused: measured over run `W4-3/1`, 21 of
    // 174 calls — 12% of everything the run did — were one of these two, in every state, from the
    // first to the last. They are the cheapest possible thing to say and the most expensive thing
    // to find out.
    //
    // The b10x arm needs one of the two and not the other, and the difference is the point: its
    // `run` entry takes an argv **list**, so there is no string for a `&&` to appear in and a
    // composed command is not a thing that can be written. The program restriction still has to be
    // stated, because a declared program set is only cheap to obey when it is known.
    if tools.shell_offered() && !harness.adjudicates() {
        // **The path, not the name.** The CLI is not on this sandbox's `PATH` and is not at the
        // path it occupies on the host: it is mounted read-only at one place, and a step told to
        // reach the store "through `protocol`" and given no spelling that resolves will hand-write
        // the store instead — which is exactly what EVAL-1/1 did, twice, for two different reasons.
        let _ = write!(
            lines,
            "\n`run` takes an argv **list** and starts one program. Nothing is composed, \
             redirected or substituted — there is no shell here to do it with — and the only \
             program it will start is `{driver}`, and only `{driver} plan \
             artifact …` and `{driver} observe trace …` — the older `artifact …` and \
             `trace …` spellings, without the area word, reach the same commands. That is the \
             whole path and it is not on `PATH`; the \
             bare name `protocol` does not resolve here. Building and testing are `command` steps \
             the driver runs itself, so that their records carry a verifier's provenance instead \
             of yours.\n",
        );
    } else if tools.shell_offered() {
        lines.push_str(
            "\n`Bash` runs **one simple invocation per call**. No `&&`, no `|`, no `;`, no `$(…)`, \
             no redirect — a composed command is refused whole, so two things you want are two \
             calls.\n\
             It runs `protocol plan artifact …` and `protocol observe trace …` \
             and the readers `grep`, `rg`, \
             `ls`, `cat`, `head`, `tail` and `wc` — those only because nothing here can redirect \
             their output into a file. Not `git`, not `cargo`, not `sed`, not `awk`, not `find`, \
             not `xargs`, not `protocol --help`. \
             Building and testing are `command` steps the driver runs itself, so that their records \
             carry a verifier's provenance instead of yours — running them here would produce \
             nothing the engine can admit.\n",
        );
    } else {
        lines.push_str(
            "\nThis state holds **no shell**. Anything a suite must observe is run by the driver as \
             a `command` step, recorded with a verifier's provenance rather than yours.\n",
        );
    }
    lines
}

/// The prompt one `llm` step is given.
///
/// Assembled from the step map's own prompt and the state's requirement lines, each of which names
/// the document that asked for it. Everything an `llm` step knows is either in a file or in this
/// string — which is the property that makes a step's input a function of persisted state, and
/// therefore the property the narrow replay claim rests on.
///
/// **The harness is read off the step rather than passed in**, so the prompt and the tool set the
/// step will actually be given cannot be rendered from two different tables. A step map that names
/// `b10x` and a prompt naming `Bash` would be an instruction to reach for a tool that does not
/// exist in that loop's catalogue — a whole turn spent, per session, learning what the driver
/// already knew. A name this build does not invoke falls back to the default rendering; nothing
/// runs on that path, because [`Harness::named`] has already refused the step.
fn prompt_for(step: &LlmStep, context: &StepContext<'_>, driver: &str) -> String {
    let harness = Harness::named(&step.harness).unwrap_or(Harness::ClaudeCode);
    let mut prompt = String::new();
    // **Which task this run is driving, before anything the map says.** A step map is written once
    // and driven many times, so its prompt can only say *the task under `.engineering/`* — and a
    // repository that has driven more than one run has several sitting there. Run `W4-3/1` read
    // `task.yaml`, which is `W4-1`, and reported that the intake it had been asked for already
    // existed; the cursor said `W4-3` the whole time. The engine knew and the model did not.
    //
    // It leads rather than follows the step's own prompt because it is the subject of every
    // sentence after it, and it names the artifacts rather than a path: a path has to be read
    // correctly, and an id is what the store answers to.
    prompt.push_str("This run drives task `");
    prompt.push_str(context.task.id.as_str());
    prompt.push_str("` — objective `");
    prompt.push_str(context.task.objective.summary.as_str());
    prompt.push('`');
    let derived = &context.task.artifacts.derived_from;
    if !derived.is_empty() {
        prompt.push_str(", derived from ");
        for (position, artifact) in derived.iter().enumerate() {
            if position > 0 {
                prompt.push_str(", ");
            }
            prompt.push('`');
            prompt.push_str(&artifact.to_string());
            prompt.push('`');
        }
    }
    prompt.push_str(
        ". Any other task document in this tree belongs to another run and is not yours to read.\n\n",
    );
    prompt.push_str(&step.prompt);
    // The skills the step names, in the prompt rather than on the command line. `--agents` takes a
    // JSON object of *agent definitions* and is not a skill selector; a step map's `skills:` list
    // reaches the session by being asked for, and the `Skill` tool — a named exemption in the tool
    // table, because loading instructions takes no action — is what answers.
    if !step.skills.is_empty() {
        prompt.push_str("\n\nLoad ");
        for (position, skill) in step.skills.iter().enumerate() {
            if position > 0 {
                prompt.push_str(" and ");
            }
            prompt.push_str("the `");
            prompt.push_str(skill);
            prompt.push('`');
        }
        // Named without a tool on a harness that has no skill mechanism: the b10x catalogue has no
        // entry for `skill.load`, so instructing it to use one would be instructing it to reach
        // for something the loop cannot publish.
        prompt.push_str(match (step.skills.len() == 1, harness.adjudicates()) {
            (true, true) => " skill before you act, with the `Skill` tool.\n",
            (false, true) => " skills before you act, with the `Skill` tool.\n",
            (true, false) => " skill before you act.\n",
            (false, false) => " skills before you act.\n",
        });
    }
    prompt.push_str("\n\nYou are in workflow state `");
    prompt.push_str(context.state.as_str());
    prompt.push_str("`.\n");
    if !context.requirements.is_empty() {
        prompt.push_str("\nWhat must hold here, one line per requirement:\n");
        for line in context.requirements {
            prompt.push_str("  ");
            prompt.push_str(line);
            prompt.push('\n');
        }
    }
    // The other half of the same question, and the half no step was ever told: what the state is
    // trying to *reach*. Under its own heading rather than merged into the list above, because the
    // two are different obligations — one is owed while here, the other is owed before the run may
    // leave — and a step that cannot tell them apart cannot tell which one it is being refused on.
    if !context.reaching.is_empty() {
        prompt.push_str(
            "\nWhat this state is trying to reach, one line per requirement that does not hold yet \
             on the way out:\n",
        );
        for line in context.reaching {
            prompt.push_str("  ");
            prompt.push_str(line);
            prompt.push('\n');
        }
    }
    // **The surface, stated rather than discovered.** `decide_tool` refuses a call outside this set
    // and prints exactly this list in the refusal — so a session that is not told it up front
    // learns its own surface by being refused, one wasted turn at a time. Run `W4-3/1` spent a turn
    // per session doing precisely that, and its first attempt was a `ToolSearch` for `Grep` and
    // `Glob`: it was not guessing, it was trying to *load* what nothing had told it it did not have.
    //
    // Rendered from `context.tools`, the same value `decide_tool` reads, so the prompt and the
    // policy cannot disagree. Two lists that could drift would be worse than one list nobody has:
    // the model would trust the wrong one.
    prompt.push_str(&surface_lines(harness, context.tools, driver));
    prompt.push_str(
        "\nYou cannot submit evidence, and nothing you say is evidence. What you achieve is \
         observed by the verifier the driver runs after this step.\n",
    );
    prompt
}

/// The rendering half of adapter point 2: the *decision* about which capabilities admit which
/// actions is the protocol's and is shared; only this table is Claude Code's. Three entries are not
/// functions of a capability and each is decided rather than left to an implementer — a shell is
/// offered only with `command.execute`, `Skill` is a named exemption, and `Task` is never offered,
/// because a subagent's tool set is derived by nothing in these decisions and would be a route
/// around the per-state allowlist.
fn allowed_tools(config: &ToolConfig) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        tools.extend(["Read", "Glob", "Grep"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::RepositoryWrite) {
        tools.extend(["Edit", "Write", "NotebookEdit"].map(ToOwned::to_owned));
    }
    // `network.read:private`, not the wildcard: `TOOL_CANDIDATES` asks the strictest audience
    // question because neither table can tell which audience a URL will reach.
    if config.admits(&Capability::NetworkRead(Audience::Private)) {
        tools.extend(["WebFetch", "WebSearch"].map(ToOwned::to_owned));
    }
    if config.shell_offered() {
        tools.push("Bash".to_owned());
    }
    if config.skills_offered() {
        tools.push("Skill".to_owned());
    }
    tools.sort();
    tools.dedup();
    tools
}

/// One vendor tool call as the `ActionRequest` the engine decides on, or nothing when no honest one
/// exists.
///
/// **The reverse direction of [`allowed_tools`], and it lives beside it for that reason** — the
/// answer to `story:metaharness-executor`'s open question. `allowed_tools` renders *capability →
/// tool names*; this renders *one call → the action it is*. Neither decides anything: the protocol
/// owns which capability an action needs (`Action::required_capability`), and a table here that
/// tried to be clever would be a second, weaker policy.
///
/// | tool | action | capability it therefore needs |
/// |---|---|---|
/// | `Read` | `repository.read` of the named file | `repository.read` |
/// | `Glob`, `Grep` | `repository.read` of the searched directory | `repository.read` |
/// | `Edit`, `Write` | `repository.write` of the named file | `repository.write` |
/// | `NotebookEdit` | `repository.write` of the named notebook | `repository.write` |
/// | `Bash` | `command.execute` of the program and its arguments | `command.execute` |
/// | `WebFetch` | a reading network request to the named URL | `network.read` |
///
/// **Two offered tools deliberately return `None`, and the engine is not consulted about them:**
///
/// * `Skill` — it loads instructions and takes no action. It is a named exemption in
///   [`allowed_tools`] for the same reason, and everything it *causes* is a subsequent, governed
///   call that arrives here on its own.
/// * `WebSearch` — a search names no URL, and a `NetworkRequest` carrying a query string in its
///   `url` field would state a destination nobody requested. The capability layer still gates it:
///   the tool is only offered when `network.read` is admitted.
///
/// Everything else — `Task` above all — never reaches this function, because [`decide_tool`] has
/// already refused a tool the state does not offer.
///
/// One disagreement is worth naming rather than discovering: [`allowed_tools`] offers `Read`,
/// `Glob` and `Grep` when **either** `repository.read` **or** `artifact.read` is admitted, and this
/// renders all three as a repository read. A state admitting only `artifact.read` therefore has the
/// engine refuse what the rendering table offered — and the engine wins, which is the right way
/// round: reading a file is a repository read whatever tool asked for it.
fn action_for(tool: &str, input: &serde_json::Value) -> Option<ActionRequest> {
    /// Every path a payload names under `keys`, in the order the keys are given.
    fn paths(input: &serde_json::Value, keys: &[&str]) -> Vec<String> {
        keys.iter()
            .filter_map(|key| input[*key].as_str())
            .map(ToOwned::to_owned)
            .collect()
    }

    let action = match tool {
        "Read" => Action::RepositoryRead(RepositoryRead {
            paths: paths(input, &["file_path"]),
        }),
        // A search with no `path` is a search of the working directory, which is what it is
        // recorded as rather than as a read of nothing.
        "Glob" | "Grep" => Action::RepositoryRead(RepositoryRead {
            paths: match paths(input, &["path"]) {
                empty if empty.is_empty() => vec![".".to_owned()],
                named => named,
            },
        }),
        "Edit" | "Write" => Action::RepositoryWrite(RepositoryWrite {
            paths: paths(input, &["file_path"]),
            intent: None,
        }),
        "NotebookEdit" => Action::RepositoryWrite(RepositoryWrite {
            paths: paths(input, &["notebook_path"]),
            intent: None,
        }),
        // Splitting on whitespace is honest **here and only here**: `driven_surface` has already
        // refused anything that composes, redirects or substitutes, so what is left is one simple
        // invocation and its arguments.
        "Bash" => {
            let mut words = input["command"]
                .as_str()
                .unwrap_or_default()
                .split_whitespace();
            Action::CommandExecute(CommandExecute {
                program: words.next().unwrap_or_default().to_owned(),
                args: words.map(ToOwned::to_owned).collect(),
            })
        }
        "WebFetch" => Action::NetworkRequest(NetworkRequest {
            url: input["url"].as_str().unwrap_or_default().to_owned(),
            intent: NetworkIntent::Read,
        }),
        _ => return None,
    };
    Some(ActionRequest::new(action))
}

/// The session loop: every event line into the transcript, every decision back down stdin.
///
/// A free function of its streams so the executor stays under its own roof: nothing here knows a
/// process, only a reader of event lines, a writer of command lines, and the engine.
///
/// # Two layers, in this order, and the reason it is this one
///
/// 1. **[`decide_tool`]** — the ported hooks and the per-state allowlist. It runs first because it
///    is the only layer that sees a call's *arguments*: `protocol plan artifact list | tee out` and
///    `protocol plan artifact list` need the same capability and are not the same act, and no
///    `ActionRequest` can express the difference.
/// 2. **the engine** — [`action_for`] renders the call as an `ActionRequest` and `authorize`
///    decides. Asked only about calls layer 1 admitted, so a refusal is attributed to the layer
///    that took it rather than to both, and **the engine's deny wins**: the two layers read the
///    same effective policy, so a disagreement means the rendering table is looser than the
///    protocol, and the protocol is what governs.
///
/// Every reason names its layer, because the event stream is where a person finds out who refused.
fn answer_events(
    harness: Harness,
    context: &StepContext<'_>,
    surface: WriteSurface<'_>,
    events: impl std::io::Read,
    commands: &mut impl std::io::Write,
    transcript: &mut impl std::io::Write,
    authorize: StepAuthorizer<'_>,
) -> Adjudication {
    let mut tally = Adjudication::default();
    for line in std::io::BufRead::lines(std::io::BufReader::new(events)) {
        let Ok(line) = line else { break };
        let _ = transcript
            .write_all(line.as_bytes())
            .and_then(|()| transcript.write_all(b"\n"));
        let Ok(event) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        // **The driver audits its own claim.** The per-state tool set is the primary enforcement
        // mechanism, and the standard this repository sets for itself is that an enforcement
        // mechanism nobody audits is a claim. Until now nothing compared the set the driver
        // *renders* against the set the session was actually given — so `tool_config` named `Glob`
        // and `Grep` to a Claude Code that offers neither, and every session of run `W4-3/1` spent
        // a turn finding that out for itself.
        //
        // Reported and never fatal: a harness offering *more* than the state admits is normal and
        // is what `decide_tool` is for; a harness offering *less* is a rendering this repository
        // owns and should fix. Only the second is printed.
        if event["event"] == "session.started" {
            // **Which list answers *can this session do it* depends on the harness, and reading the
            // wrong one is how an audit cries wolf.** A vendor harness publishes one tool per act,
            // so `offered_tools` is the answer. A loop that publishes three verbs over a catalogue
            // — `tool_search`, `tool_describe`, `tool_invoke` — answers in `available_operations`
            // instead, and comparing a rendered catalogue against those three verbs reported every
            // single entry as missing: run `b10x-2623331` was told it lacked `file_read`,
            // `file_write`, `file_edit`, `dir_list` and `search` while its own record published all
            // five. An audit that fires on a session that has everything it needs is worse than no
            // audit, because the next true one is read as noise.
            //
            // Operations first, tools second, union never: the two are different vocabularies, and
            // a name present in one is not absent from the other.
            let published: Vec<&str> = event["available_operations"]
                .as_array()
                .or_else(|| event["offered_tools"].as_array())
                .map(|listed| listed.iter().filter_map(|v| v.as_str()).collect())
                .unwrap_or_default();
            if !published.is_empty() {
                let present = published;
                let missing: Vec<String> = harness
                    .operations_or_tools(context.tools)
                    .into_iter()
                    // The shell is the one entry a harness may legitimately hold back: Claude Code
                    // does not list `Bash` among its offered tools, and the b10x loop publishes
                    // `run` only where the machine can confine an exec.
                    .filter(|named| {
                        named != "Bash"
                            && named != "run"
                            && named != "command.execute"
                            && !present.contains(&named.as_str())
                    })
                    .collect();
                if !missing.is_empty() {
                    outln!(
                        "note: state `{}` admits {} the session was not offered — {}. The step map \
                         and this harness disagree about the tool set; the model will be refused by \
                         the vendor rather than by the policy, and the turn is spent either way.",
                        context.state,
                        if missing.len() == 1 {
                            "a tool"
                        } else {
                            "tools"
                        },
                        missing.join(", ")
                    );
                }
            }
        }
        if event["event"] == "tool.requested" {
            tally.requested += 1;
        }
        // **`decision_required: false` is a fact and not a silence.** The b10x adapter sets it on
        // every call, beside `Seam::None`, because nothing on that loop adjudicates — so the
        // driver counts what it saw and answers nothing. Writing a `tool.decide` here would be
        // this process claiming a decision the wire says nobody made.
        if event["event"] == "tool.requested" && event["decision_required"] == true {
            tally.asked += 1;
            let call_id = event["call_id"].as_str().unwrap_or_default();
            let name = event["name"].as_str().unwrap_or_default();
            let deny = |reason: String| serde_json::json!({ "decision": "deny", "reason": reason });
            let decision = match decide_tool(context, surface, name, &event["input"]) {
                Err(reason) => deny(format!("the driver's per-call policy refuses: {reason}")),
                // Nothing renders this call as an action — `Skill` and `WebSearch` are the two, and
                // [`action_for`] says why — so the engine is not consulted and the policy's allow
                // stands. Inventing a request would put an act nobody performed in the engine's
                // record, which is invariant 7's failure one layer up.
                Ok(()) => match action_for(name, &event["input"]) {
                    None => serde_json::json!({ "decision": "allow" }),
                    Some(request) => {
                        let verdict = authorize(&request);
                        if verdict.is_allowed() {
                            serde_json::json!({ "decision": "allow" })
                        } else {
                            deny(engine_refusal(&verdict))
                        }
                    }
                },
            };
            if decision["decision"] == "deny" {
                tally.denied += 1;
            }
            let command = serde_json::json!({
                "format": "metaharness.command/1",
                "id": format!("decide-{call_id}"),
                "command": "tool.decide",
                "call_id": call_id,
                "decision": decision,
            });
            // A write that fails means the child is gone; the caller's wait reports how.
            if commands
                .write_all(format!("{command}\n").as_bytes())
                .and_then(|()| commands.flush())
                .is_err()
            {
                break;
            }
        }
    }
    tally
}

/// What the driver was actually asked while one session ran.
///
/// # Why three counts and not one
///
/// A denial count on its own is only readable when something was asking. The claude arm answers
/// every call, so `denied: 0` there genuinely means *nothing this session did was refused by the
/// driver*. The b10x arm answers nothing — `Seam::None`, `decision_required: false` on every
/// `tool.requested` — so `denied: 0` there means *nobody asked*, and a report that printed the
/// same words for both would be reporting an adjudication that never happened. Two runs compared
/// on that number would be compared on an artefact of the instrument.
///
/// So [`Self::requested`] is what the session did, [`Self::asked`] is how much of it reached this
/// process at all, and [`Self::denied`] is what this process refused. `asked == 0` is the state
/// [`Self::line`] refuses to describe as a clean run.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Adjudication {
    /// `tool.requested` events seen, whether or not one asked anything.
    requested: u32,
    /// Those that put a decision to this process.
    asked: u32,
    /// Those this process refused, by its own policy or by the engine's.
    denied: u32,
}

impl Adjudication {
    /// The line the run prints about one session's calls.
    ///
    /// A harness that does not adjudicate gets a sentence naming what bounded the run instead of a
    /// count of what this process refused, because the count is `0` for a reason that has nothing
    /// to do with the session's behaviour.
    fn line(self, harness: Harness, state: &StateId) -> String {
        if harness.adjudicates() {
            return format!(
                "note: state `{state}` put {} tool call(s) to the driver and {} were refused.",
                self.asked, self.denied
            );
        }
        format!(
            "note: state `{state}` observed {} tool call(s) and adjudicated none of them — the \
             `{}` loop publishes a toolset computed from what the machine can confine, so a tool \
             outside it does not exist rather than being refused. Nothing here says the run was \
             not refused anything; it says nobody asked this process. What the loop itself \
             refused is in the `{}` transcript beside this run, and the refusal specification for \
             this step is what checks it.",
            self.requested,
            harness.kind(),
            METAHARNESS_EVENT_FORMAT
        )
    }
}

/// The event stream both harnesses write, and the one both are checked from.
const METAHARNESS_EVENT_FORMAT: &str = "metaharness.event/1";

/// The per-call policy: the retired shell hooks, in the driver's own process.
///
/// This is the § 10.1 shape the hooks existed to approximate: the layer that sees a call's
/// *arguments* is the embedder, in Rust, and its verdict reaches the child through the
/// metaharness seam before the call runs. Three checks, first refusal wins, every reason written
/// for the model to act on rather than as a wall:
///
/// 1. **the driven surface** (`Bash`): one `protocol plan artifact` or `protocol observe trace` invocation — no
///    pipes, no redirection, no substitution — and no shell at all in a state that does not
///    admit `command.execute`;
/// 2. **the per-state allowlist**: the tool must render from a capability this state admits,
///    which is what `--allowedTools` used to carry (and can no longer, because a bare
///    `--allowedTools` entry auto-approves the whole tool before any seam is consulted);
/// 3. **the step's declared write scope** (`Edit`/`Write`/`NotebookEdit`): where this step may
///    write and how much of a file it may replace, read off the step map's `scope:` rather than
///    written here — the same declaration the native loop is handed as `--write-scope`;
/// 4. **store integrity** (`Edit`): an edit's text may not cross a planning document's closing
///    `---`. A question about **content**, which is the one thing no scope can answer.
fn decide_tool(
    context: &StepContext<'_>,
    surface: WriteSurface<'_>,
    tool: &str,
    input: &serde_json::Value,
) -> Result<(), String> {
    if tool == "Bash" {
        return driven_surface(context, input);
    }
    let offered = allowed_tools(context.tools);
    if !offered.iter().any(|name| name == tool) {
        return Err(format!(
            "`{tool}` is not offered in state `{}`; this state's tools are: {}",
            context.state,
            offered.join(", ")
        ));
    }
    match tool {
        "Edit" | "Write" | "NotebookEdit" => {
            declared_write(surface, tool, input)?;
            store_integrity(tool, input)
        }
        _ => Ok(()),
    }
}

/// A planning document's frontmatter is the CLI's: an edit may not cross the closing `---`.
///
/// Answers one `before-call` consultation from the native loop's hook port, with the **content**
/// tier and only that. Where the loop may write at all, and whether it may replace a whole file,
/// is declared in the step map's `scope:` and travels to that arm as `--write-scope`, which its own
/// tools enforce before this program is ever spawned.
///
/// # Why this exists at all
///
/// The two arms enforce the same decision at different moments. The vendor arm's calls come back
/// through the metaharness seam and are answered by [`decide_tool`] inside this process. The native
/// loop makes its decisions in-process and consults **programs** — so the same rule has to be
/// runnable as one, and this is it. It calls [`store_integrity_at`] rather than restating the rule,
/// because a second copy of a rule is a second rule and they diverge on the day one is edited.
///
/// # The entry names differ and the rule does not
///
/// The loop names the *invoked entry* — `file_edit` — where the vendor names a tool: `Edit`. It
/// spells the arguments differently too: `path`, `old` and `new` against `file_path`,
/// `old_string` and `new_string`. Both spellings are read here, because a rule that guessed one
/// of them is a rule that silently allows everything on the other arm.
///
/// # Fail closed is the loop's rule, not ours
///
/// Anything unreadable here exits non-zero without `2`, which the loop's port records as
/// `Failed` — *fail closed* before a call. So a malformed document refuses the call rather than
/// letting it through, and this function does not have to decide that for itself.
fn hook() -> ExitCode {
    let mut document = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut document).is_err() {
        eprintln!("the hook document could not be read from stdin");
        return ExitCode::from(1);
    }
    let Ok(document) = serde_json::from_str::<serde_json::Value>(&document) else {
        eprintln!("the hook document is not JSON");
        return ExitCode::from(1);
    };
    // Only `before-call` can refuse anything; every other point proceeds. Stated rather than
    // assumed, so a file that ever declares this program at `after-call` does not silently block.
    if document["hook"].as_str() != Some("before-call") {
        return ExitCode::SUCCESS;
    }
    let entry = document["entry"].as_str().unwrap_or_default();
    // A hook declared for entries this rule says nothing about proceeds. The `tools` list in the
    // hooks file is what scopes it; this is the belt.
    //
    // **`file_write` is deliberately not here.** It replaces a whole file, which is a question of
    // granularity and path — the one the step map's `scope:` answers and `--write-scope` carries to
    // this loop's own tools. Asking this program about it too would be a second copy of that rule,
    // in the place least likely to be read.
    if entry != "file_edit" {
        return ExitCode::SUCCESS;
    }
    // **The loop's own spelling.** `file_edit` takes `path`, `old` and `new`; `file_path`,
    // `old_string` and `new_string` are the vendor's words, and reading only those answered `None`
    // for every call once, which this rule then read as "no planning file involved" and allowed.
    // Both spellings are read rather than a pretence that only one can arrive.
    let arguments = &document["call"]["arguments"];
    let field = |loop_word: &str, vendor_word: &str| -> &str {
        arguments[loop_word]
            .as_str()
            .or_else(|| arguments[vendor_word].as_str())
            .unwrap_or_default()
    };
    let target = field("path", "file_path");
    let edits = [
        ("old", field("old", "old_string")),
        ("new", field("new", "new_string")),
    ];
    match store_integrity_at(target, &edits) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            outln!("{}", serde_json::json!({ "reason": reason }));
            ExitCode::from(2)
        }
    }
}

/// What the loop asks at a section boundary, as this verb reads it.
///
/// The document is the harness's (`b10x-harness` design 0003 § 3): `path` is the flow node,
/// `moment` is `enter` or `leave`, `failed` says whether a left section came out failed. Everything
/// else the loop sends — `attempt`, `of`, `handoff`, `workspace` — is recorded by the loop and is
/// not what the engine decides on.
#[derive(Debug, serde::Deserialize)]
struct TransitionConsultation {
    hook: String,
    #[serde(default)]
    path: String,
    #[serde(default)]
    moment: String,
    #[serde(default)]
    failed: bool,
}

/// `protocol drive transition`
///
/// Exit `0` proceeds; exit `2` refuses with `{"reason": …}` on stdout; anything else is a verb that
/// could not answer, which the loop reads **fail closed** — a governor that could not answer did
/// not say yes.
fn transition(args: &TransitionArgs) -> ExitCode {
    let mut document = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut document).is_err() {
        eprintln!("the transition document could not be read from stdin");
        return ExitCode::from(1);
    }
    let consultation: TransitionConsultation = match serde_json::from_str(&document) {
        Ok(consultation) => consultation,
        Err(error) => {
            eprintln!("the transition document is not the loop's JSON: {error}");
            return ExitCode::from(1);
        }
    };
    // Only `transition` is answered here; a file that declares this program at another point
    // proceeds, said out loud rather than assumed, so it cannot silently block a call.
    if consultation.hook != "transition" {
        return ExitCode::SUCCESS;
    }
    let moment = match consultation.moment.as_str() {
        "enter" => Moment::Enter,
        "leave" => Moment::Leave,
        other => {
            eprintln!("`moment` is `{other}`; this verb answers `enter` and `leave`");
            return ExitCode::from(1);
        }
    };
    // A section that came out failed is already failed; the refusal is the loop's record and the
    // engine has nothing to add (design 0003 § 3, third row).
    if moment == Moment::Leave && consultation.failed {
        return ExitCode::SUCCESS;
    }

    match answer(args, &consultation.path, moment) {
        Ok(Answer::Proceed) => ExitCode::SUCCESS,
        Ok(Answer::Refuse(reason)) => {
            outln!("{}", serde_json::json!({ "reason": reason }));
            ExitCode::from(2)
        }
        Err(error) => {
            eprintln!("the governor could not answer: {error:#}");
            ExitCode::from(1)
        }
    }
}

fn store_integrity(tool: &str, input: &serde_json::Value) -> Result<(), String> {
    let target = match tool {
        "NotebookEdit" => input["notebook_path"].as_str().unwrap_or_default(),
        _ => input["file_path"].as_str().unwrap_or_default(),
    };
    store_integrity_at(
        target,
        &[
            (
                "old_string",
                input["old_string"].as_str().unwrap_or_default(),
            ),
            (
                "new_string",
                input["new_string"].as_str().unwrap_or_default(),
            ),
        ],
    )
}

/// The harness name that selects the metaharness executor.
const METAHARNESS_HARNESS: &str = "metaharness";

/// The harness name that selects the b10x loop.
const B10X_HARNESS: &str = "b10x";

/// The binary every `llm` step is spawned through.
const METAHARNESS_BINARY: &str = "metaharness";

/// The loop `metaharness run b10x` spawns, which has to be installed separately.
const B10X_BINARY: &str = "b10x-harness";

/// Which harness an `llm` step is spawned through, and the only place the two differ.
///
/// **§ 4.9 point 3's seam, with a second implementation in it at last.** The design says a second
/// harness is *a second free function chosen by this name*, not a trait added before there is
/// anything to design one against; `story:shell-echo-harness` proved the shape with a fake
/// executor and this is the first real one. What varies between the two is exactly three things —
/// the `metaharness run` kind, the naming table a shared capability decision renders into, and
/// whether the seam puts a decision to this process at all — and they are enumerated here so a
/// third harness has to answer the same three questions rather than discover them.
///
/// What deliberately does **not** vary: [`metaharness_operations`], which is the neutral
/// vocabulary the frame and the refusal specification are written in, and
/// `aep_driver::tool::tool_config`, which decides what a capability admits. A harness that decided
/// for itself could quietly re-admit a shell the state never granted, which is the one thing point
/// 2 exists to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Harness {
    /// Claude Code, driven through `metaharness run claude` in ask mode: every call is put to this
    /// process and answered before it runs.
    ClaudeCode,
    /// The b10x loop, spawned through `metaharness run b10x` and **observed**.
    ///
    /// It adjudicates nothing, by design and not by omission: the loop's published toolset is
    /// computed from what the machine can confine, so a tool outside the surface does not exist
    /// rather than being refused, and a seam that adjudicated its calls would put the driven arm's
    /// treatment back on top of the arm that exists to measure its absence.
    B10x,
}

impl Harness {
    /// The harness a step's `harness:` field names, or nothing this build can invoke.
    fn named(name: &str) -> Option<Self> {
        match name {
            // `claude-code` names the vendor and `metaharness` is the name the executor first
            // landed under; both reach the same invocation.
            LlmStep::DEFAULT_HARNESS | METAHARNESS_HARNESS => Some(Self::ClaudeCode),
            B10X_HARNESS => Some(Self::B10x),
            _ => None,
        }
    }

    /// Every name this build invokes, for a refusal that lists them rather than hinting.
    const NAMES: [&'static str; 3] = [LlmStep::DEFAULT_HARNESS, METAHARNESS_HARNESS, B10X_HARNESS];

    /// The `metaharness run` kind.
    fn kind(self) -> &'static str {
        match self {
            Self::ClaudeCode => "claude",
            Self::B10x => B10X_HARNESS,
        }
    }

    /// This harness's own names for an admitted capability set.
    ///
    /// The rendering half of § 4.9 point 2. Both arms read the same [`ToolConfig`] and neither
    /// decides anything about it.
    fn tools(self, config: &ToolConfig) -> Vec<String> {
        match self {
            Self::ClaudeCode => allowed_tools(config),
            Self::B10x => b10x_tools(config),
        }
    }

    /// The names to audit a session's own published list against.
    ///
    /// **Two harnesses answer *what can this session do* in two vocabularies, and the audit has to
    /// ask in the one the answer is written in.** Claude Code publishes one tool per act, so its
    /// tool names are the question. The b10x loop publishes three verbs over a catalogue and states
    /// its reach as `available_operations` in the neutral scheme — so the tool names would compare
    /// a catalogue against `tool_search`, `tool_describe`, `tool_invoke` and report every entry
    /// missing, which is what run `b10x-2623331` was told while its record published all five.
    fn operations_or_tools(self, config: &ToolConfig) -> Vec<String> {
        match self {
            Self::ClaudeCode => allowed_tools(config),
            Self::B10x => b10x_operations(config),
        }
    }

    /// Whether this harness's seam puts a decision to the driver before a call runs.
    ///
    /// `false` for b10x, and the run report has to say so in those words rather than reporting a
    /// denial count of zero: *nobody asked me* and *nothing was refused* are different findings
    /// and only one of them is about the run.
    fn adjudicates(self) -> bool {
        matches!(self, Self::ClaudeCode)
    }
}

/// The programs a driven step may start, shared by both arms.
///
/// One decision, two enforcements: the vendor arm refuses anything outside this set at the call
/// (`driven_surface`), and the native arm is handed it as `--allow-program` so the loop never
/// publishes a `run` that could start anything else. The second is the stronger of the two — a
/// program not on the list has no tool to reach it — which is the whole argument for that loop.
fn driven_programs(config: &ToolConfig) -> Vec<String> {
    // **The driver is not declared here at all any more, and that is the correction.**
    //
    // Two spellings were tried and both failed, for two different reasons that looked the same
    // from inside the sandbox. The bare name failed because the confined exec has its own `PATH`.
    // The absolute host path then failed because the sandbox is not this filesystem: it binds
    // `/usr`, `/bin`, `/lib`, `/lib64` and the workspace, and nothing else, so a path outside
    // those is not there to run. The comment that used to sit here blamed `PATH` for both, which
    // is why the second spelling was expected to work.
    //
    // Measured twice. On EVAL-1/1 at 8783e3c the bare name took `127` three times and the session
    // hand-wrote the store's frontmatter with `file_write`, omitting `id`, leaving the store
    // unparseable. On EVAL-1/1 at 3d8ac3b the absolute path was allow-listed, admitted, and still
    // found nothing: the session said so in its own words — *"the `protocol` binary ... does not
    // exist in the accessible filesystem"* — and the run ended with zero artifacts.
    //
    // An allow-list decides what a `run` may **name**; only a mount decides what the sandbox
    // **contains**. So the driver travels as `--driver` instead ([`b10x_argv`]), which stages the
    // one file, mounts it read-only, and adds its mounted path to the loop's own allow-list. One
    // declaration, made where it can be honoured.
    let mut programs = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        programs.extend(READ_ONLY_PROGRAMS.iter().map(|name| (*name).to_owned()));
    }
    programs
}

/// Match substrate's staging of the canonical binary under its original file name.
fn staged_driver(options: &B10xOptions) -> String {
    let name = options
        .aep_binary
        .as_deref()
        .and_then(Path::file_name)
        .unwrap_or_else(|| std::ffi::OsStr::new("aep"));
    format!("/toolchain/driver/{}", name.to_string_lossy())
}

/// The neutral operations an admitted capability set reaches, as `available_operations` spells them.
///
/// The same shared decision as every other rendering, in the vocabulary the b10x adapter answers
/// in. `command.execute` is deliberately absent from the audit's view of it: the loop publishes
/// `run` only where the machine can confine an exec, which is a fact about the machine and not a
/// disagreement about the map.
fn b10x_operations(config: &ToolConfig) -> Vec<String> {
    let mut operations: Vec<String> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        operations.extend(["file.read", "dir.list", "search"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::RepositoryWrite) {
        operations.extend(["file.write", "file.edit"].map(ToOwned::to_owned));
    }
    operations
}

/// The b10x loop's tool names for an admitted capability set.
///
/// The second naming table, and the reason § 4.9 point 2 puts the *decision* somewhere else: this
/// function reads the same [`ToolConfig`] [`allowed_tools`] reads and renames its answer. Nothing
/// here consults a capability the shared decision did not already admit.
///
/// The names are `b10x-harness-tools`' own catalogue entries — `file_read`, `file_write`,
/// `file_edit`, `dir_list`, `search`, `run` — read from `entry_names()` there rather than invented
/// here. Three neutral operations the Claude Code table renders have **no entry at all** in that
/// catalogue and are therefore rendered by nothing:
///
/// | operation | Claude Code | b10x |
/// |---|---|---|
/// | `web.read` | `WebFetch`, `WebSearch` | *no entry* — the loop has no web tool |
/// | `skill.load` | `Skill` | *no entry* — the loop has no skill mechanism |
/// | `subagent.spawn` | never offered | *no entry*, and never offered either |
///
/// A capability this table cannot render is **not** silently downgraded: `network.read` stays
/// admitted by the policy and the session simply has no tool for it, which the session-start audit
/// in [`answer_events`] reports against the same list. Rendering it as something else would be the
/// second, weaker policy point 2 forbids.
fn b10x_tools(config: &ToolConfig) -> Vec<String> {
    let mut tools: Vec<String> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        tools.extend(["dir_list", "file_read", "search"].map(ToOwned::to_owned));
    }
    if config.admits(&Capability::RepositoryWrite) {
        tools.extend(["file_edit", "file_write"].map(ToOwned::to_owned));
    }
    if config.shell_offered() {
        // `run` and not a shell: the entry takes an argv list, composes nothing and starts only a
        // declared program. See [`b10x_argv`] for why the declaration travels on the launch.
        tools.push("run".to_owned());
    }
    tools.sort();
    tools.dedup();
    tools
}

/// Validates the paid-run opt-in and exact cost terms before a run id or lock exists.
fn spend_terms(
    map: &StepMap,
    budget_usd: Option<&str>,
    assume_usd_per_run: Option<&str>,
) -> Result<Option<SpendTerms>> {
    spend_terms_with_live(
        map,
        budget_usd,
        assume_usd_per_run,
        std::env::var(METAHARNESS_LIVE_ENV).as_deref() == Ok("1"),
    )
}

/// The spend pre-flight with its ambient opt-in already read, so its rules are unit-testable.
fn spend_terms_with_live(
    map: &StepMap,
    budget_usd: Option<&str>,
    assume_usd_per_run: Option<&str>,
    live: bool,
) -> Result<Option<SpendTerms>> {
    if !has_llm_steps(map) {
        return Ok(None);
    }
    if !live {
        bail!(
            "this map has {} `llm` step(s), and `{METAHARNESS_LIVE_ENV}=1` is not in this \
             environment. A model session can cost money; opt in explicitly before `protocol \
             drive run` may allocate a run or spawn metaharness",
            llm_step_count(map)
        );
    }
    let budget = budget_usd.context(
        "this map can spawn a model and no `--budget-usd <USD>` was given; a paid run with no \
         outer cost cap is refused before allocation",
    )?;
    let assumed = assume_usd_per_run.context(
        "this map can spawn a model and no `--assume-usd-per-run <USD>` was given; the driver \
         cannot reserve the next launch against its cap without an operator-declared charge",
    )?;
    let cap_micro_usd = aep_cli::money::micro_usd(budget)?;
    let assumed_micro_usd_per_run = aep_cli::money::micro_usd(assumed)?;
    if cap_micro_usd == 0 {
        bail!("`--budget-usd` must be greater than zero for a map that can spawn a model");
    }
    if assumed_micro_usd_per_run == 0 {
        bail!("`--assume-usd-per-run` must be greater than zero");
    }
    if assumed_micro_usd_per_run > cap_micro_usd {
        bail!(
            "the first assumed charge {} already exceeds the run cap {}; no run was allocated",
            aep_cli::money::dollars(assumed_micro_usd_per_run),
            aep_cli::money::dollars(cap_micro_usd)
        );
    }
    Ok(Some(SpendTerms {
        cap_micro_usd,
        assumed_micro_usd_per_run,
    }))
}

/// The remembered terms for a resume, optionally narrowed by this invocation.
fn resumed_spend_terms(
    map: &StepMap,
    remembered: Option<SpendTerms>,
    budget_usd: Option<&str>,
) -> Result<Option<SpendTerms>> {
    resumed_spend_terms_with_live(
        map,
        remembered,
        budget_usd,
        std::env::var(METAHARNESS_LIVE_ENV).as_deref() == Ok("1"),
    )
}

/// Resume cost terms with the ambient opt-in already read, for deterministic tests.
fn resumed_spend_terms_with_live(
    map: &StepMap,
    remembered: Option<SpendTerms>,
    budget_usd: Option<&str>,
    live: bool,
) -> Result<Option<SpendTerms>> {
    if !has_llm_steps(map) {
        return Ok(None);
    }
    if !live {
        bail!(
            "this run can spawn another model session, and `{METAHARNESS_LIVE_ENV}=1` is not in \
             this environment; no lock was taken and nothing was launched"
        );
    }
    let mut terms = remembered.context(
        "this run predates the model-session cost ceiling and remembers no budget; start a new \
         bounded run rather than resuming one that cannot account for its earlier launches",
    )?;
    if terms.cap_micro_usd == 0
        || terms.assumed_micro_usd_per_run == 0
        || terms.assumed_micro_usd_per_run > terms.cap_micro_usd
    {
        bail!(
            "this run's remembered cost terms are invalid: cap {}, assumed charge {}; no model session was launched",
            aep_cli::money::dollars(terms.cap_micro_usd),
            aep_cli::money::dollars(terms.assumed_micro_usd_per_run),
        );
    }
    if let Some(written) = budget_usd {
        let narrowed = aep_cli::money::micro_usd(written)?;
        if narrowed == 0 {
            bail!("a resumed `--budget-usd` must be greater than zero");
        }
        if narrowed > terms.cap_micro_usd {
            bail!(
                "a resume may narrow its remembered cap {}, not raise it to {}",
                aep_cli::money::dollars(terms.cap_micro_usd),
                aep_cli::money::dollars(narrowed)
            );
        }
        terms.cap_micro_usd = narrowed;
    }
    Ok(Some(terms))
}

fn metaharness_preflight(map: &StepMap) -> Option<String> {
    let llm_steps = llm_step_count(map);
    if llm_steps == 0 || aep_cli::drive::on_path(METAHARNESS_BINARY) {
        return None;
    }
    Some(format!(
        "this map has {llm_steps} `llm` step(s) and `{METAHARNESS_BINARY}` is not on PATH.\n\
         \n\
         Every `llm` step is spawned through `{METAHARNESS_BINARY} run <harness>`, whichever \
         harness the step names: on `{}` the step's surface travels as a sealed frame document \
         and this process answers every tool call the session makes. There is no path around it \
         — the bare vendor argv was retired with `epic:metaharness-migration`, because a second \
         way to launch a session is a second policy to forget.\n\
         \n\
         Install it with `cargo install --path crates/metaharness-cli` from a metaharness checkout, \
         or drive a map whose steps are all `command` and `operator` steps, which needs neither.",
        LlmStep::DEFAULT_HARNESS
    ))
}

/// Every *this machine cannot run it today* pre-flight, in the order they are answered.
///
/// Four checks, and the order is the one a person can act on: the seam's binary, then the CLI a
/// driven session reaches the store through, then everything a `harness: b10x` step needs, then
/// the binary a `command` step saying `protocol` would spawn. Each is decidable before a run id, a
/// lock, a snapshot or a model bill exists, which is the whole argument for them being here rather
/// than at the first `llm` step.
///
/// Two `PATH`s. The session checks are about the one metaharness constructs for an `llm` step; the
/// `command` check is about the **driver's** own, which is the operator's shell. That they are
/// different is why `W4-3/1`'s `command` step ran a binary four releases stale while a guard that
/// looked like it covered this was passing.
///
/// The evidence-coverage check is deliberately **not** folded in and runs before this: it is
/// decidable from the two documents alone and says *this map can never finish this plan* on every
/// machine, so a real coverage gap must not be hidden behind a binary that happens to be missing.
///
/// The read-only note is printed rather than returned, because it refuses nothing: a b10x step in
/// a state that only reads is legitimate work.
fn machine_preflights(map: &StepMap, project: &Path, b10x: &B10xOptions) -> Option<String> {
    if let Some(refusal) = metaharness_preflight(map) {
        return Some(refusal);
    }
    // Native sessions receive the selected binary as a mount; only vendor sessions use PATH.
    if map
        .states
        .values()
        .flat_map(|state| &state.steps)
        .any(|step| matches!(step, Step::Llm(step) if step.harness == "claude-code"))
        && let Some(refusal) = protocol_on_the_session_path()
    {
        return Some(refusal);
    }
    if let Some(refusal) = b10x_preflight(map, b10x) {
        return Some(refusal);
    }
    if let Some(note) = b10x_read_only_note(map, project, b10x) {
        outln!("note: {note}");
    }
    None
}

/// How many `llm` steps name the b10x loop.
fn b10x_step_count(map: &StepMap) -> usize {
    map.states
        .values()
        .flat_map(|state| state.steps.iter())
        .filter(|step| matches!(step, Step::Llm(step) if step.harness == B10X_HARNESS))
        .count()
}

/// Consult the same compiled adapter catalogue as the CLI without re-executing the host.
fn metaharness_knows(kind: &str) -> bool {
    kind == B10X_HARNESS && metaharness::capabilities(metaharness::protocol::Kind::B10x).is_ok()
}
/// The third of the launch-time pre-flights and the same argument as the two beside it: a map that
/// names `b10x` on a machine with no b10x loop, no endpoint or no model is decidable from the
/// documents and the filesystem, and finding it out at the first `llm` step costs a run id, the
/// store lock, a snapshot and — on the arms that get that far — a model bill, for a
/// [`StepOutcome::NoVerdict`] that is not unknown at all.
///
/// Four checks, first refusal wins, each naming what to install or declare. The two that are
/// decidable anywhere come first — an invocation that names no endpoint is wrong on every machine
/// — and the two about *this* machine follow, so a missing loop cannot mask a missing flag.
fn b10x_preflight(map: &StepMap, options: &B10xOptions) -> Option<String> {
    let steps = b10x_step_count(map);
    if steps == 0 {
        return None;
    }
    // **The two facts about the invocation come first, and the order is load-bearing** — the same
    // lesson `start`'s coverage check records. These are decidable on every machine; the two below
    // them say *this machine cannot run it today*. With the machine checks first, a run that named
    // no endpoint at all would read as fine wherever the loop happened to be missing, and the test
    // asserting it is refused would pass vacuously in CI.
    if options.endpoint.is_none() {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and no \
             `--b10x-endpoint` was given.\n\
             \n\
             The loop is pointed at an endpoint by its caller and has no service of its own to \
             fall back on, and metaharness refuses to default one: a default would aim a driven \
             run at somebody's production API the first time the flag was forgotten. It is a fact \
             about this machine rather than about the work, which is why the step map cannot \
             carry it.\n\
             \n\
             Pass the gateway's root URL as `--b10x-endpoint`, and the model it serves as \
             `--b10x-model`."
        ));
    }
    if options.model.is_none() {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and no \
             `--b10x-model` was given. The endpoint serves several and the loop picks none."
        ));
    }
    let path = session_path();
    let installed = path
        .split(':')
        .any(|directory| Path::new(directory).join(B10X_BINARY).is_file());
    if !installed {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and \
             `{B10X_BINARY}` is not on the `PATH` the run will give its child.\n\
             \n\
             That `PATH` is `{path}` — **constructed by metaharness, not inherited** (H3) — so a \
             loop the operator can run is not automatically one the run can. It is the same \
             constructed `PATH` the `protocol` CLI has to be installed onto, and for the same \
             reason.\n\
             \n\
             Install it where the run will find it:\n\
             \n\
                 cargo install --path crates/harness-cli --root ~/.local\n\
             \n\
             from a `beyond10x/harness` checkout, or drive this map's `llm` steps on \
             `{}` instead.",
            LlmStep::DEFAULT_HARNESS
        ));
    }
    if !metaharness_knows(B10X_HARNESS) {
        return Some(format!(
            "this map has {steps} `llm` step(s) that name harness `{B10X_HARNESS}` and the \
             installed `{METAHARNESS_BINARY}` does not publish an adapter for it.\n\
             \n\
             The adapter is compiled in, so an install predating it carries the same name and the \
             same `--version` and refuses `{B10X_HARNESS}` as an invalid argument at the first \
             step — after the run id, the lock and the snapshot. `{METAHARNESS_BINARY} \
             capabilities {B10X_HARNESS}` is the question that was asked and it did not answer.\n\
             \n\
             Reinstall it from a metaharness checkout that has the adapter:\n\
             \n\
                 cargo install --path crates/metaharness-cli --root ~/.local"
        ));
    }
    None
}

/// What a driven b10x session will not be able to do here, said before it is paid for.
///
/// **Not a refusal, because a read-only session is legitimate work.** A `specify` or a `review`
/// state that admits `repository.read` and nothing else drives perfectly well on this arm. What
/// would be wrong is a `implement` state discovering it, one turn at a time, in a session that was
/// told it had `file_write`.
///
/// The rule is metaharness's and it is a naming rule: substrate represents a workspace only when
/// its directory name starts with `ws_`, and a confined launch over a directory it cannot adopt is
/// refused rather than degraded. A driven run's working directory is the operator's repository, so
/// no driven b10x session is confined, so the loop publishes only the three reading entries — the
/// toolset is computed from what the machine can confine, and unconfined that is reading.
///
/// It is a note and not a refusal for a second reason: what a state admits is decided per state by
/// the engine at run time, and a pre-flight reading a map cannot know whether any state will reach
/// for a write.
fn b10x_read_only_note(
    map: &StepMap,
    working_directory: &Path,
    options: &B10xOptions,
) -> Option<String> {
    if b10x_step_count(map) == 0 {
        return None;
    }
    // Said only when it is true. The first version of this note was unconditional, and it told an
    // operator who had done everything right — named the worktree `ws_…`, delegated a subtree —
    // that their arm could not write. A warning that fires when the thing it warns about is not
    // happening teaches a reader to stop reading warnings.
    if options.cgroup_root.is_some() && adoptable(working_directory) {
        return None;
    }
    let why = if adoptable(working_directory) {
        "the workspace is adoptable but no `--b10x-cgroup-root` was given, so substrate publishes \
         no `run` entry and the catalogue stays read-only"
    } else {
        "substrate represents a workspace only when its directory name starts with `ws_`, and this \
         one does not — a governed tree is usually the operator's own repository. A worktree \
         created for the run can be named to be adoptable"
    };
    Some(format!(
        "a driven `{B10X_HARNESS}` session is **read-only** over {}: {why}. So `file_write`, \
         `file_edit` and `run` are not published to it, and this arm cannot attempt a task that \
         has to change a file — a run that may not execute its suite cannot see a test fail before \
         writing the code, so it will not write the code.",
        working_directory.display()
    ))
}

/// The `PATH` a driven session will actually have, which is **not** this process's.
///
/// metaharness constructs the child environment rather than inheriting it — `env_clear()` then an
/// allowlist, plus a `PATH` computed as `$HOME/.local/bin:/usr/local/bin:/usr/bin:/bin`
/// (`metaharness-claude`'s `child_path`, which that crate makes public precisely so a pre-flight
/// can resolve a binary *the way the spawn will*). So a `target/debug` on the operator's `PATH`
/// reaches this process and never the session, and exporting one before `protocol drive` changes
/// nothing about what the model can run.
///
/// Replicated here rather than depended on: this repository takes `entity-runtime` and nothing
/// else, and one shared constant across that boundary would be a dependency in the direction
/// `adr/0002` refuses. `a_session_path_matches_what_metaharness_constructs` pins the two together,
/// so a change on that side fails here rather than in a paid run.
fn session_path() -> String {
    match std::env::var("HOME") {
        Ok(home) if !home.is_empty() => {
            format!("{home}/.local/bin:/usr/local/bin:/usr/bin:/bin")
        }
        _ => "/usr/local/bin:/usr/bin:/bin".to_owned(),
    }
}

/// Refuses a run whose `llm` steps are told to use `protocol` when the session will not have it.
///
/// **Run `W4-3/1`, 2026-08-28, is why, and it cost $1.03 to find out.** The map's steps say *record
/// it in the planning store*, and the store's only route is the `protocol` CLI — the state's shell
/// recorded-under-this-name: historical W4-3/1 transcript.
/// exists for that and admits nothing else. The session ran `protocol artifact --help` and got
/// `exit 127, command not found`, four times across two states, because the constructed `PATH`
/// holds no `target/debug`. Every guard held and the run was simply unable to do its work.
///
/// It is the same shape as the metaharness pre-flight above and sits beside it for the same reason:
/// a run that cannot do its work should not own a run id, a lock and a model bill to discover that.
fn protocol_on_the_session_path() -> Option<String> {
    let path = session_path();
    let found = path.split(':').any(|directory| {
        let candidate = Path::new(directory).join("protocol");
        candidate.is_file()
    });
    if found {
        return None;
    }
    Some(format!(
        "a driven `llm` step reaches the planning store through the `protocol` CLI, and the \n\
         session's `PATH` does not hold it.\n\
         \n\
         That `PATH` is `{path}` — **constructed by metaharness, not inherited**, so exporting \n\
         `target/debug` before this command changes what *this* process can run and nothing about \n\
         what the model can. A run started anyway walks its states, is refused `protocol` by the \n\
         shell with `exit 127`, and submits nothing: run `W4-3/1` did exactly that on 2026-08-28 \n\
         for $1.03.\n\
         \n\
         Install this build where the session will find it — `--root ~/.local`, because cargo's \n\
         own default is `$CARGO_HOME/bin` and that directory is **not** on the constructed \n\
         `PATH` either:\n\
         \n\
             cargo install --path crates/edge/aep-cli --root ~/.local\n\
         \n\
         Install rather than symlink `target/debug`: a later `cargo build` replaces that binary \n\
         with a different version than the one this run is recorded against, and a run whose \n\
         evidence was produced by a build nobody can name is the defect `version-check` exists for."
    ))
}

/// The format tag the frame document carries, as the metaharness design § 5.5 spells it.
const METAHARNESS_FRAME_FORMAT: &str = "metaharness.frame/1";

/// The metaharness operations for an admitted capability set.
///
/// The same decisions as [`allowed_tools`], spelled in metaharness's § 5.2 vocabulary instead of
/// the vendor's: the protocol decides what a capability admits, both tables only render it, and
/// `subagent.spawn` is never offered for the same reason `Task` never is.
fn metaharness_operations(config: &ToolConfig) -> Vec<&'static str> {
    let mut operations: Vec<&'static str> = Vec::new();
    if config.admits(&Capability::RepositoryRead) || config.admits(&Capability::ArtifactRead) {
        operations.extend(["file.read", "dir.list", "search"]);
    }
    if config.admits(&Capability::RepositoryWrite) {
        operations.extend(["file.write", "file.edit"]);
    }
    if config.admits(&Capability::NetworkRead(Audience::Private)) {
        operations.push("web.read");
    }
    if config.shell_offered() {
        operations.push("shell");
    }
    if config.skills_offered() {
        operations.push("skill.load");
    }
    operations.sort_unstable();
    operations.dedup();
    operations
}

/// Every operation the table can render, whatever a policy admits.
///
/// Computed by asking [`metaharness_operations`] about a configuration that admits everything,
/// rather than written out again. Two lists would drift, and the one that drifts is the one nobody
/// looks at: a hand-written vocabulary missing `file.edit` would emit a specification that never
/// checks for an edit and reports green.
fn every_operation() -> Vec<&'static str> {
    metaharness_operations(&ToolConfig::new(TOOL_CANDIDATES.iter().cloned().collect()))
}

/// The operations this step's policy did **not** admit.
///
/// # What this is for, and what it is not
///
/// Gap register `:40`. Design § 4.8 row 3 promised the per-state tool set would be *audited*: the
/// allowlist at session launch, the hook over the same derived set, and an expectation kind reading
/// back what the session was actually given. `env.tool_available` shipped and then showed it reads
/// the harness's tool **inventory**, not the session's allow rules — the committed fixture was
/// launched with nine allowed tools and lists thirty-two.
///
/// The record that would settle it is the harness's to write and does not exist. This is the other
/// route the register names, and it is **strictly weaker**, which is why it says so out loud: it
/// catches a tool that was offered *and used*, and cannot see one that was offered and never
/// reached for. A refused operation that never appears in the transcript is the same evidence as an
/// operation nobody wanted. What it does close is the case that matters — a run that did something
/// its state was not allowed to do now fails a check instead of passing unexamined.
fn refused_operations(config: &ToolConfig) -> Vec<&'static str> {
    let admitted = metaharness_operations(config);
    every_operation()
        .into_iter()
        .filter(|operation| !admitted.contains(operation))
        .collect()
}

/// The step's refused operations as a `trace.spec/1` document.
///
/// One `tool.absent` row per refused operation, keyed by the **neutral operations** vocabulary and
/// never by a vendor's tool names. Naming tools here would make the specification decidable against
/// one harness and silently vacuous against every other — a row saying `tools: [Edit, Write]` selects
/// nothing at all on a harness that spells a write `workspace_write`, and reports green for it.
///
/// `severity: gate` and `on_unknown: gap`: a transcript that cannot say whether a refused
/// operation happened is not evidence that it did not. The whole point is to stop reading silence
/// as compliance.
fn refusal_specification(
    state: &aep_domain::ids::StateId,
    index: usize,
    config: &ToolConfig,
) -> Option<serde_json::Value> {
    let expectations: Vec<serde_json::Value> = refused_operations(config)
        .into_iter()
        .map(|operation| {
            serde_json::json!({
                // Dashes, not the dots the operation is spelled with: an expectation id is
                // lowercase letters, digits and dashes, and the checker refuses anything else.
                "id": format!("refused-{}", operation.replace('.', "-")),
                "statement": format!(
                    "step {index} of `{state}` was not admitted `{operation}`, \
                     so the run must not contain one"
                ),
                "severity": "gate",
                "on_unknown": "gap",
                "expect": { "tool.absent": { "operations": [operation] } },
            })
        })
        .collect();
    // `None` when nothing was refused, because `trace-spec/1` refuses a specification with no
    // expectations — *"a report with no content reads exactly like a report with no gaps"* — and
    // that rule is right and older than this. Absence is still readable: the frame document for the
    // same step is written unconditionally, so a frame with no refusal file beside it means this
    // state was admitted everything, and no frame at all means the step never ran.
    if expectations.is_empty() {
        return None;
    }
    Some(serde_json::json!({
        "format": "trace-spec/1",
        // One `/`, between a namespace and a name: `driver/<state>-<index>`.
        "id": format!("driver/{}-{index}", state.to_string().replace('.', "-")),
        "title": format!("what step {index} of `{state}` was not allowed to do"),
        "expectations": expectations,
    }))
}

/// The step as a sealed `metaharness.frame/1` document.
///
/// Built as plain JSON and sealed by the document's own rule — SHA-256, hex, over the compact
/// serialization with keys sorted at every level (`serde_json`'s default map order) and the
/// `digest` and `format` fields absent — so this binary produces byte-for-byte what metaharness
/// verifies, without linking its crates. The obligations and reaching lines are the engine's own
/// words, verbatim, on the same rule as the prompt: a summary here would be the only place the
/// summary existed.
fn metaharness_frame(
    context: &StepContext<'_>,
    scope: &[ScopeRule],
    workflow_id: &str,
    workflow_version: &str,
) -> serde_json::Value {
    let line = |text: &String| serde_json::json!({ "text": text, "asked_by": null });
    let mut frame = serde_json::json!({
        "workflow": { "id": workflow_id, "version": workflow_version },
        "node": { "id": context.state.to_string() },
        "step": {
            "workflow": workflow_id,
            "state": context.state.to_string(),
            "index": context.index,
            "attempt": context.attempt,
        },
        "prior": [],
        "obligations": context.requirements.iter().map(line).collect::<Vec<_>>(),
        "reaching": context.reaching.iter().map(line).collect::<Vec<_>>(),
        "next": [],
        "handoff": { "handoff": "none" },
        "operations": metaharness_operations(context.tools)
            .iter()
            .map(|operation| serde_json::json!({ "op": operation }))
            .collect::<Vec<_>>(),
        "entities": null,
    });
    if !scope.is_empty() {
        frame["subjects"] = metaharness_subject_scope(scope);
    }
    let digest: String = {
        use sha2::{Digest as _, Sha256};
        let bytes = serde_json::to_vec(&frame).expect("a frame value serialises");
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
    let object = frame.as_object_mut().expect("a frame is an object");
    object.insert("digest".into(), digest.into());
    object.insert("format".into(), METAHARNESS_FRAME_FORMAT.into());
    frame
}

/// Compile the map's write vocabulary into the frame's operation vocabulary.
///
/// A write scope says nothing about reading, so every rule preserves `file.read`. `allowed`
/// additionally admits both write granularities, `partial-only` admits only the targeted edit,
/// and `denied` admits neither. Paths are scheme-prefixed so a catch-all file rule cannot capture
/// a `proc:` subject and accidentally turn a write policy into an execution policy.
fn metaharness_subject_scope(scope: &[ScopeRule]) -> serde_json::Value {
    let rules = scope
        .iter()
        .flat_map(|rule| {
            let operations = match rule.write {
                WriteScope::Allowed => vec!["file.edit", "file.read", "file.write"],
                WriteScope::PartialOnly => vec!["file.edit", "file.read"],
                WriteScope::Denied => vec!["file.read"],
            };
            rule.paths.iter().map(move |path| {
                serde_json::json!({
                    "subjects": [format!("file:{path}")],
                    "operations": operations
                        .iter()
                        .map(|operation| serde_json::json!({ "op": operation }))
                        .collect::<Vec<_>>(),
                })
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({ "rules": rules })
}

/// A minted frame as the bytes that go on disk.
///
/// Pretty-printed with a trailing newline, which is exactly what metaharness's own
/// `Frame::to_document` writes, so the two producers of this file agree byte for byte and a
/// document minted here can be diffed against one minted there. Split out of the write so the
/// committed golden under `fixtures/` is *these* bytes and not a second rendering of them: a golden
/// produced by a path the driver does not take would pin the test and not the driver.
///
/// # Errors
///
/// When the frame will not serialise, which for a value built by [`metaharness_frame`] would be a
/// defect here rather than anything a caller did.
fn frame_document(frame: &serde_json::Value) -> Result<String, String> {
    let text = serde_json::to_string_pretty(frame)
        .map_err(|error| format!("the frame would not serialise: {error}"))?;
    Ok(format!("{text}\n"))
}

/// The `metaharness run claude` invocation for one step.
///
/// `--cwd` is the metaharness a6 declaration: the session works in the governed tree, and
/// metaharness attests the two hermetic rows that costs instead of claiming them. `--decisions
/// frame` makes metaharness the per-call decider from the frame's admitted set. The plugins
/// still travel for their skills; their hooks read a step context this launch does not carry and
/// no-op, which is the intended shape — one policy, one enforcer.
fn metaharness_argv(
    frame: &Path,
    working_directory: &Path,
    plugin_dirs: &[PathBuf],
    prompt: &str,
    gateway: Option<(&str, &str)>,
    actor: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        METAHARNESS_BINARY.to_owned(),
        "run".to_owned(),
        "claude".to_owned(),
        "--hermetic".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--frame".to_owned(),
        frame.display().to_string(),
        "--decisions".to_owned(),
        "ask".to_owned(),
    ];
    // **Who the session's own store writes are made as.** `session_env` above sets `AEP_ACTOR` on
    // every child this process spawns, which reaches a `command` step because that is our child —
    // and does not reach an `llm` step's model, because metaharness constructs its child's
    // environment rather than inheriting one. That was recorded as out of scope on this side and a
    // flag on the other; the flag exists now, and it is declared rather than inherited for the
    // reason metaharness's own allowlist exists: a variable that can be set by the surrounding
    // shell is not provenance. Absent when the execution id has no actor spelling, which is the
    // same silence `session_env` keeps.
    if let Some(actor) = actor {
        argv.push("--actor".to_owned());
        argv.push(actor.to_owned());
    }
    // **The same gateway both arms can be pointed at, which is what makes them comparable.**
    // Without it the harness comparison is confounded: one arm on a vendor's own model and the
    // other on whatever a gateway serves measures the two models at least as much as the two
    // harnesses, and no scorer can separate them afterwards. metaharness requires
    // `--credentials none` alongside an endpoint — a child pointed at a foreign endpoint must hold
    // no operator credential — so the two travel together or not at all.
    if let Some((endpoint, model)) = gateway {
        argv.push("--model-endpoint".to_owned());
        argv.push(endpoint.to_owned());
        argv.push("--model".to_owned());
        argv.push(model.to_owned());
        argv.push("--credentials".to_owned());
        argv.push("none".to_owned());
    }
    argv.push("-p".to_owned());
    argv.push(prompt.to_owned());
    for directory in plugin_dirs {
        argv.push("--plugin-dir".to_owned());
        argv.push(directory.display().to_string());
    }
    argv
}

/// The `metaharness run b10x` invocation for one step.
///
/// # Why this says `--decisions observe` and carries no `--frame`
///
/// The claude arm's surface travels twice — the sealed frame document *and* a per-call answer —
/// because F9 says a frame whose text reaches the model while nothing enforces it tells the model
/// *"strictly only these operations"* and makes it false. metaharness enforces that rule on its own
/// side: `required_commands` adds `tool.decide` to any spec carrying a frame, the b10x adapter
/// refuses `tool.decide` because nothing on that loop ever asks, and so **`metaharness run b10x
/// --frame …` is refused before a model is reached**. The direct-provider loop instead delivers
/// observation without a decision seam: its opening attestation and every tool-request event say
/// that no per-call decision was required. Naming `--decisions observe` here makes that weaker but
/// honest mode explicit rather than inheriting metaharness's vendor-oriented `frame` default.
///
/// The frame is still *minted and written* beside the transcript for a b10x step. It is the record
/// of what the step was, in the neutral vocabulary both arms are checked in, and the refusal
/// specification beside it is derived from the same [`ToolConfig`]. What changes is that on this
/// arm the document is evidence about the step rather than an instruction to a seam.
///
/// # What the surface travels as instead
///
/// `--write-scope` and `--context`, which are the b10x-only spec fields that exist for exactly this
/// — the loop has no seam, so *"for that kind the scope has to travel to the tools"*. Both come
/// off the step map's own `scope:` and `context:` keys, which no other executor reads.
///
/// # What is deliberately not asked for, because it cannot be had over a governed tree
///
/// No `--substrate-embedded`, no `--substrate` and no `--cgroup-root`. substrate represents a
/// workspace only when its directory name starts with `ws_`, and the working directory of a driven
/// run is the operator's repository — metaharness refuses a confined launch over a directory it
/// cannot adopt rather than degrading it. So a driven b10x session is **read-only**: the loop
/// publishes what the machine can confine, and with no confinement that is the three reading
/// entries. Asking for confinement here would turn every driven b10x step into a launch refusal;
/// not asking for it makes the limitation visible where it can be acted on, in [`b10x_preflight`]
/// before the run and in the session-start audit during it.
/// The files the operator handed this run, as one value.
///
/// Grouped for the reason `Confinement` groups its own: two more positional paths on a function
/// that already takes six is a call site nobody can read, and these two are one decision — what
/// the operator gave this step that the step did not go and find.
#[derive(Debug, Clone, Copy)]
struct OperatorFiles<'a> {
    /// The content rule consulted before every call, or none.
    hooks: Option<&'a Path>,
    /// Plugin directories whose skills the run may load by name.
    plugin_dirs: &'a [PathBuf],
}

fn b10x_argv(
    options: &B10xOptions,
    working_directory: &Path,
    scope: &[ScopeRule],
    context_files: &[String],
    prompt: &str,
    config: &ToolConfig,
    operator: OperatorFiles<'_>,
) -> Vec<String> {
    let mut argv = vec![
        METAHARNESS_BINARY.to_owned(),
        "run".to_owned(),
        B10X_HARNESS.to_owned(),
        "--hermetic".to_owned(),
        "--decisions".to_owned(),
        "observe".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--model-endpoint".to_owned(),
        options.endpoint.clone().unwrap_or_default(),
        "--model".to_owned(),
        options.model.clone().unwrap_or_default(),
        "--credentials".to_owned(),
        // `operator-login` is the flag's default and names nothing on this loop, which refuses it
        // rather than launching a run with no credential under a flag that claims one.
        if options.api_key { "api-key" } else { "none" }.to_owned(),
    ];
    // The dialect and the subscription source, when the arm was pointed at one. Both are
    // metaharness flags rather than loop flags here: the driver names what the run is, metaharness
    // renders it as the loop's argv, and the token is read by neither.
    if let Some(wire) = &options.wire {
        argv.push("--model-wire".to_owned());
        argv.push(wire.clone());
    }
    if let Some(path) = &options.oauth_token_file {
        argv.push("--subscription-token-file".to_owned());
        argv.push(path.display().to_string());
        if let Some(pointer) = &options.oauth_token_pointer {
            argv.push("--subscription-token-pointer".to_owned());
            argv.push(pointer.clone());
        }
    }
    // **Confinement and execution, or neither.** Substrate represents a workspace only when its
    // directory name starts with `ws_`, so a run over an ordinary checkout is read-only whatever
    // is asked for — and asking anyway would turn every driven step into a launch refusal. When the
    // workspace *is* adoptable and a subtree was named, both travel: `--substrate-embedded` makes
    // `file_write` and `file_edit` appear in the catalogue, and `--cgroup-root` makes `run` appear.
    // One without the other is an arm that can write and not test, or test and not write.
    if let Some(root) = options
        .cgroup_root
        .as_ref()
        .filter(|_| adoptable(working_directory))
    {
        argv.push("--substrate-embedded".to_owned());
        argv.push("--cgroup-root".to_owned());
        argv.push(root.display().to_string());
    }
    // **`run` is published only to a session that was told which programs it may start.** The loop
    // withholds it outright when no allowlist was given — `programs.is_none()` in
    // `harness-tools`' local operations — which is the same rule as everywhere else on that arm: a
    // tool outside the surface does not exist rather than being refused. Run `b10x-2991520` spent
    // 30 `tool_search` calls, 28 of them distinct, hunting for `run`, `exec`, `shell`, `spawn` and
    // `execute` because the step it was given needs the `protocol` CLI and nothing could start one.
    //
    // The list is the same decision `driven_surface` enforces on the vendor arm, rendered rather
    // than re-decided: the CLI, and the readers a state that admits `repository.read` may use.
    if config.admits(&Capability::CommandExecution) {
        for program in driven_programs(config) {
            argv.push("--allow-program".to_owned());
            argv.push(program);
        }
        // **And the driver itself travels as a mount, not as a name.** Allow-listing it by its
        // path on this host admitted the name and nothing else: the sandbox binds `/usr`, `/bin`,
        // `/lib`, `/lib64` and the workspace, so the file was never there and every call died at
        // `ENOENT`. `--driver` stages exactly this binary into a private directory, mounts it
        // read-only at `/toolchain/driver`, and adds the mounted path to the loop's own allowlist
        // — so the step's instructions can name `DRIVEN_DRIVER` and have it be true.
        //
        // Read-only is the point as much as present is: this is the binary that records the run's
        // evidence, and a run that could rewrite it has no evidence to show.
        if let Some(binary) = &options.aep_binary {
            argv.push("--driver".to_owned());
            argv.push(binary.display().to_string());
        }
    }
    // **The content-level refusal, which the write scope cannot express.** A scope answers *which
    // paths*; the store's rule is about *which fields* — a step legitimately writes under
    // `.engineering/planning`, and must not hand-edit the frontmatter the CLI owns. Without this the
    // native arm's whole enforcement is which tools exist, and `file_write` has to exist.
    if let Some(hooks) = operator.hooks {
        argv.push("--hooks".to_owned());
        argv.push(hooks.display().to_string());
    }
    for rule in scope {
        for path in &rule.paths {
            argv.push("--write-scope".to_owned());
            // `<glob>=<allowed|partial-only|denied>`, ordered, first match wins — which is why the
            // rules are pushed in the order the map wrote them and never sorted.
            argv.push(format!("{path}={}", write_scope_word(rule.write)));
        }
    }
    for file in context_files {
        argv.push("--context".to_owned());
        argv.push(file.clone());
    }
    // **The same directories the vendor arm is given.** The loop reads the skills half of the
    // vendor's on-disk plugin format, so a step here is offered the same library rather than
    // having to discover the CLI's own `skill load` verb for itself. What a dropped `--plugin-dir`
    // costs is on the record: run W4-2 lost all eight of its post-fix sessions to one, running
    // unenforced while looking clean.
    for directory in operator.plugin_dirs {
        argv.push("--plugin-dir".to_owned());
        argv.push(directory.display().to_string());
    }
    argv.push("-p".to_owned());
    argv.push(prompt.to_owned());
    argv
}

/// Whether substrate will represent this directory as a workspace.
///
/// Its rule, replicated rather than depended on: a directory name starting with `ws_`
/// (`SUBSTRATE_WORKSPACE_PREFIX` in metaharness's builder). A governed tree is usually the
/// operator's own repository and is not named that, which is why a driven `b10x` arm is read-only
/// by default and says so — and why a worktree created for a run *can* be named to be adoptable,
/// which is the whole of the arrangement.
///
/// A relative path has no useful file name — `.` is not `ws_anything` — so a caller who passes
/// `--project .` from inside an adoptable directory gets the read-only arm and a note saying so.
/// That is a real trap and the note is where it is caught.
fn adoptable(working_directory: &Path) -> bool {
    working_directory
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with("ws_"))
}

/// A governed run and its concrete execution options.
#[derive(Debug, Args)]
pub struct HostedRunArgs {
    /// Neutral driver options.
    #[command(flatten)]
    pub common: aep_cli::drive::RunArgs,
    /// Native and vendor launch configuration.
    #[command(flatten)]
    pub b10x: B10xOptions,
}
/// Resume an existing governed run without changing its engine contract.
#[derive(Debug, Args)]
pub struct HostedResumeArgs {
    /// Neutral resume options.
    #[command(flatten)]
    pub common: aep_cli::drive::ResumeArgs,
    /// Planning executable; defaults to the remembered launch selection.
    #[arg(long)]
    pub aep_binary: Option<PathBuf>,
}
/// Execution operations hosted above the foundation.
#[derive(Debug, Subcommand)]
pub enum DriveCommand {
    /// Run live evaluation or ingest recorded evidence.
    Eval {
        /// Evaluation operation.
        #[command(subcommand)]
        command: aep_cli::eval::EvalCommand,
    },
    /// Start a governed run.
    Run(HostedRunArgs),
    /// Continue a compatible paused run.
    Resume(HostedResumeArgs),
    /// Inspect a retained run.
    Status(aep_cli::drive::StatusArgs),
    /// Answer a native before-call hook.
    #[command(hide = true)]
    Hook,
    /// Consult the governor at a native section boundary.
    #[command(hide = true)]
    Transition(aep_cli::drive::TransitionArgs),
}
/// Execute an AEP operation through the concrete adapter.
///
/// # Errors
/// Returns the original validation, launch, or retained-run integrity refusal.
pub fn run(command: DriveCommand) -> Result<ExitCode> {
    match command {
        DriveCommand::Eval {
            command: aep_cli::eval::EvalCommand::Run(args),
        } => crate::eval::run_arm(&args),
        DriveCommand::Eval { command } => aep_cli::eval::run(command),
        DriveCommand::Run(args) => aep_cli::drive::start_with_host(
            &args.common,
            &Host {
                b10x: Some(args.b10x),
                aep_binary: None,
            },
        ),
        DriveCommand::Resume(args) => aep_cli::drive::resume_with_host(
            &args.common,
            &Host {
                b10x: None,
                aep_binary: args.aep_binary,
            },
        ),
        DriveCommand::Status(args) => aep_cli::drive::status(&args),
        DriveCommand::Hook => Ok(hook()),
        DriveCommand::Transition(args) => Ok(transition(&args)),
    }
}
struct Host {
    b10x: Option<B10xOptions>,
    aep_binary: Option<PathBuf>,
}
impl ExecutionHost for Host {
    fn resume_command(&self) -> &'static str {
        "metaharness aep drive resume"
    }

    fn prepare(
        &self,
        inputs: &Inputs,
        previous: Option<&Launch>,
        budget: Option<&str>,
        charge: Option<&str>,
    ) -> Result<PreparedExecution> {
        let mut b10x = match previous.and_then(|record| record.extra.get("b10x")) {
            Some(value) => serde_json::from_value(value.clone())
                .context("invalid remembered model launch configuration")?,
            None => self.b10x.clone().unwrap_or_default(),
        };
        if let Some(binary) = &self.aep_binary {
            b10x.aep_binary = Some(binary.clone());
        }
        if !has_llm_steps(&inputs.map)
            && b10x.aep_binary.is_none()
            && protocol_command_steps(&inputs.map) == 0
        {
            return CommandOnlyHost.prepare(inputs, previous, budget, charge);
        }
        let remembered = previous
            .and_then(|record| record.extra.get("spend"))
            .map(|value| serde_json::from_value::<Option<SpendTerms>>(value.clone()))
            .transpose()
            .context("invalid remembered spend terms")?
            .flatten();
        let terms = if previous.is_some() {
            resumed_spend_terms(&inputs.map, remembered, budget)?
        } else {
            spend_terms(&inputs.map, budget, charge)?
        };
        let binary = b10x.aep_binary.as_ref().context("pass --aep-binary pointing to the planning executable built from this runner's pinned AEP source")?;
        let binary = fs::canonicalize(binary).context("resolving the selected AEP executable")?;
        let output = Process::new(&binary)
            .arg("--version")
            .output()
            .context("checking the selected AEP executable")?;
        let banner = String::from_utf8_lossy(&output.stdout);
        if !output.status.success()
            || !banner
                .split_whitespace()
                .any(|word| word == aep_cli::VERSION)
        {
            bail!(
                "selected AEP executable must implement version {}; got {}",
                aep_cli::VERSION,
                banner.trim()
            );
        }
        b10x.aep_binary = Some(binary.clone());
        if let Some(refusal) = machine_preflights(&inputs.map, &inputs.project, &b10x) {
            bail!("{refusal}");
        }
        let mut launch_fields = std::collections::BTreeMap::new();
        launch_fields.insert("b10x".to_owned(), serde_json::to_value(&b10x)?);
        launch_fields.insert("spend".to_owned(), serde_json::to_value(terms)?);
        Ok(PreparedExecution {
            launch_fields,
            protocol_binary: Some(binary),
            executor: Box::new(move |context, resumed| {
                let spend = terms
                    .map(|terms| {
                        if resumed {
                            SpendBudget::resume(&context.run_directory, terms)
                        } else {
                            SpendBudget::start(&context.run_directory, terms)
                        }
                    })
                    .transpose()?;
                Ok(Box::new(
                    CliExecutors::new(
                        context.working_directory.clone(),
                        context.run_directory.clone(),
                        context.plugin_dirs.clone(),
                        context.workflow_id.clone(),
                        context.workflow_version.clone(),
                        b10x,
                    )
                    .with_spend(spend),
                ))
            }),
        })
    }
}

include!("drive_tests.rs");
