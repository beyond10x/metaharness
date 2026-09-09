//! Live evaluation; recorded evidence and its interpretation remain in AEP.
use aep_cli::eval::{
    Arm, Case, EVENTS_SUFFIX, Harness, METAHARNESS_BIN_ENV, METAHARNESS_BINARY,
    METAHARNESS_LIVE_ENV, Plan, RunArgs, RunRefusal, TOOL_MISSING_EXIT, declared_plugins, ingest,
    ingest_recorded, launched_elsewhere, refused_run, require_plugin_treatment, select_cases,
    stream_for, write_products,
};
use aep_cli::money::{dollars, micro_usd};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

// --- spawning ---------------------------------------------------------------------------------------

/// The binary, where it is installed.
///
/// A lookup and never a spawn, on `aep_cli::drive::on_path`'s reasoning: running the tool to find out
/// whether it exists is a side effect in a pre-flight.
fn tool() -> Option<String> {
    match std::env::var_os(METAHARNESS_BIN_ENV) {
        Some(path) => Path::new(&path)
            .is_file()
            .then(|| PathBuf::from(path).display().to_string()),
        None => std::env::current_exe()
            .ok()
            .map(|path| path.display().to_string()),
    }
}

/// Where the binary was looked for, for the refusal that says it is not there.
fn looked_for() -> String {
    match std::env::var_os(METAHARNESS_BIN_ENV) {
        Some(named) => format!(
            "`{METAHARNESS_BIN_ENV}` names {}, which is not a file",
            PathBuf::from(named).display()
        ),
        None => format!("nothing on PATH is named `{METAHARNESS_BINARY}`"),
    }
}

/// Whether the environment permits spending money.
fn live() -> bool {
    std::env::var(METAHARNESS_LIVE_ENV).is_ok_and(|value| value == "1")
}

/// The prompt one arm gives one case.
///
/// **Arm `raw` gets the instructions and arm `plugin` does not**, and that is the experiment rather
/// than an omission. Arm a is *text and hope*: the workflow's committed instruction document,
/// rendered by `protocol govern workflow instruct`, in front of the task. Arm b's treatment **is** the
/// plugin — the skills and agents it installs are what are supposed to carry the workflow — so
/// giving it the instructions too would measure a and b at once and attribute the result to b.
fn prompt_for(plan: &Plan, instructions: &Path) -> Result<String> {
    if plan.arm != Arm::Raw {
        return Ok(plan.case.task.clone());
    }
    let document = instructions.join(format!("{}.md", plan.case.workflow));
    let rendered = std::fs::read_to_string(&document).map_err(|_| {
        refused_run(
            &plan.case.expectations,
            &[RunRefusal::InstructionsMissing {
                workflow: plan.case.workflow.clone(),
                expected: document.display().to_string(),
            }],
        )
    })?;
    Ok(format!("{rendered}\n---\n\n{}", plan.case.task))
}

/// The `metaharness run` invocation for one arm of one case.
///
/// `--decisions observe` is what makes arms a and b comparable with each other and with arm c:
/// every run is spawned by the same instrument into the same hermetic scratch home with the same
/// recording, and only the treatment varies. The mode allows everything and records everything —
/// nothing here decides a tool call, which is arm c's whole difference and `protocol drive`'s job.
/// The base of the `PATH` metaharness constructs for the session it spawns.
const CHILD_BASE_PATH: &str = "/usr/local/bin:/usr/bin:/bin";

/// The `PATH` the spawned session gets: `$HOME/.local/bin` in front of [`CHILD_BASE_PATH`], as both
/// metaharness vendor adapters build it (`metaharness-claude/src/launch.rs`, `child_path`), and never
/// this process's own. Written out here rather than asked of metaharness, because the runner has to
/// look where the child will look before it pays for the child to look there.
fn child_path_for(home: Option<&str>) -> String {
    match home {
        Some(home) if !home.is_empty() => format!("{home}/.local/bin:{CHILD_BASE_PATH}"),
        _ => CHILD_BASE_PATH.to_owned(),
    }
}

/// The first executable named `program` on `path`, in `path`'s own order — a hand-rolled walk, so
/// that it is exactly the child's resolution and not a resolver with opinions of its own.
fn resolve_on_path(program: &str, path: &str) -> Option<PathBuf> {
    path.split(':')
        .filter(|dir| !dir.is_empty())
        .map(|dir| Path::new(dir).join(program))
        .find(|candidate| is_executable(candidate))
}

#[cfg(unix)]
fn is_executable(candidate: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt as _;
    std::fs::metadata(candidate)
        .is_ok_and(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(candidate: &Path) -> bool {
    candidate.is_file()
}

/// The version out of what a binary printed for `--version`: `protocol 0.44.0` → `0.44.0`. The first
/// token that starts with a digit, because the product name in front of it is prose.
fn version_token(reported: &str) -> Option<String> {
    reported
        .split_whitespace()
        .find(|token| token.starts_with(|c: char| c.is_ascii_digit()))
        .map(ToOwned::to_owned)
}

/// Before a live spawn: the `aep` the session will run is this one, and `ess` is there if a case
/// needs it.
///
/// The session's `PATH` is constructed by metaharness and does not include wherever this binary was
/// launched from, so the two can disagree without anything saying so. On 2026-09-03 they did — a
/// 0.40.1 in `~/.local/bin` beside the 0.44.0 that launched — and the golden path spent $10.96
/// before stopping at `aep doctor: unrecognized subcommand`. A mismatch is refused; an absence is a
/// warning, since a case may not run `aep` at all and the child's own failure is then cheap.
fn preflight_child_path(cases: &[Case], out: &Path) -> Result<()> {
    let child_path = child_path_for(std::env::var("HOME").ok().as_deref());
    let refusals = child_path_refusals(cases, &child_path);
    if refusals.is_empty() {
        Ok(())
    } else {
        Err(refused_run(out, &refusals))
    }
}

/// Every fault of one child `PATH`, in the order an operator would fix them.
///
/// **Accumulated and not returned one at a time** (invariant 3: validation accumulates), for the
/// reason [`declared_plugins`] gives 400 lines above: these are independent defects of one machine.
/// `EVAL-RUN-017` masked `EVAL-RUN-018` while this returned the first — a stale `aep` in
/// `~/.local/bin` and no `ess` beside it are one afternoon's fix and were two live round trips to
/// find out about.
///
/// Takes the `PATH` rather than reading `HOME` itself, so a test can point it at a tree it built
/// instead of at the developer's own `~/.local/bin`.
fn child_path_refusals(cases: &[Case], child_path: &str) -> Vec<RunRefusal> {
    let mut refusals = Vec::new();
    match resolve_on_path("aep", child_path) {
        // An absence is a warning and not a refusal: a case may not run `aep` at all, and the
        // child's own failure is then cheap. A *mismatch* is a run that pays before it finds out.
        None => eprintln!(
            "warning: no `aep` on the child's PATH ({child_path}); a case whose task runs it will \
             fail inside the session. `task install` in the aep checkout writes one to ~/.local/bin"
        ),
        Some(child) => {
            let reported = std::process::Command::new(&child)
                .arg("--version")
                .output()
                .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
                .unwrap_or_default();
            let found =
                version_token(&reported).unwrap_or_else(|| format!("unreadable: {reported:?}"));
            let own = aep_cli::VERSION;
            if found != own {
                refusals.push(RunRefusal::ChildAepMismatch {
                    child: child.display().to_string(),
                    found,
                    own: own.to_owned(),
                    child_path: child_path.to_owned(),
                });
            }
        }
    }
    if let Some(case) = cases.iter().find(|case| case.needs_ess)
        && resolve_on_path("ess", child_path).is_none()
    {
        refusals.push(RunRefusal::ChildEssMissing {
            case: case.id.clone(),
            child_path: child_path.to_owned(),
        });
    }
    refusals
}

/// Micro-dollars as the plain decimal a vendor flag takes: `12500000` → `12.500000`. Not
/// [`aep_cli::money::dollars`], which is for a person and carries the sign.
fn usd_plain(micro: u64) -> String {
    format!("{}.{:06}", micro / 1_000_000, micro % 1_000_000)
}

fn spawn_argv(
    plan: &Plan,
    binary: &str,
    working_directory: &Path,
    prompt: &str,
    plugin_directory: Option<&Path>,
    model: Option<&str>,
    max_budget_usd: Option<&str>,
) -> Vec<String> {
    let mut argv = vec![
        binary.to_owned(),
        "run".to_owned(),
        plan.harness.as_str().to_owned(),
        "--hermetic".to_owned(),
        "--cwd".to_owned(),
        working_directory.display().to_string(),
        "--decisions".to_owned(),
        "observe".to_owned(),
    ];
    // What is left of `--budget-usd`, handed to the session as the vendor's own stop, and placed
    // with the run options rather than after the prompt: every reader of this argv that takes the
    // prompt as the tail keeps working. Claude Code only, on `--model`'s reasoning — metaharness
    // 0.6.1 refuses the flag for codex by name, and a flag accepted and dropped would be a cap that
    // exists on paper. Before this the cap was checked between runs and nowhere else: a receipt, and
    // one golden-path run stated $10.96 against 5.
    if plan.harness == Harness::Claude
        && let Some(cap) = max_budget_usd
    {
        argv.push("--max-budget-usd".to_owned());
        argv.push(cap.to_owned());
    }
    argv.push("-p".to_owned());
    argv.push(prompt.to_owned());
    // Before the treatment, and on every arm: the model is the *condition* a phase holds fixed
    // across its arms, so an argv where it moved with the arm would be an experiment varying two
    // things. Absent where the operator named none, which is metaharness's default and this
    // runner's until 0.44.0 — so an invocation that does not pin one keeps the argv it had.
    if let Some(model) = model {
        argv.push("--model".to_owned());
        argv.push(model.to_owned());
    }
    if plan.arm == Arm::Plugin
        && let Some(directory) = plugin_directory
    {
        argv.push("--plugin-dir".to_owned());
        argv.push(directory.display().to_string());
    }
    // Forwarded verbatim, once per declaration, and after `--plugin-dir` rather than instead of it:
    // metaharness 0.5.0 loads a marketplace plugin through the scratch config home and a directory
    // through the vendor's own flag, so the two combine and the attestation lists both. Nothing is
    // resolved, normalised or deduplicated here — a runner that rewrote a pin would be forwarding
    // something other than what the operator wrote down.
    for plugin in &plan.plugins {
        argv.push("--plugin".to_owned());
        argv.push(plugin.to_string());
    }
    argv
}

/// Spawns one run and answers with the stream it wrote.
///
/// The stream is captured whatever the tool exits with, and written down before anything is read
/// out of it: a run that was paid for and then discarded because its last event was missing is the
/// worst outcome this verb has.
fn spawn(
    plan: &Plan,
    argv: &[String],
    stream_path: &Path,
    redact: bool,
    cwd: Option<&Path>,
) -> Result<Vec<u8>> {
    let spawned = std::process::Command::new(&argv[0])
        .args(&argv[1..])
        .stdin(std::process::Stdio::null())
        .output();
    let output = match spawned {
        Ok(output) => output,
        Err(error) => bail!("`{}` could not be run: {error}", argv.join(" ")),
    };

    // Redacted **before** the write and not after it, so the operator's home never reaches the
    // disk. The substitution is a pure byte transform that cannot fail, so nothing about the "write
    // the paid run down before anything is read out of it" rule is given up by doing it here.
    let stream = stream_for(output.stdout, redact, cwd);
    std::fs::write(stream_path, &stream)
        .with_context(|| format!("writing {}", stream_path.display()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let tail: String = stderr.lines().rev().take(3).collect::<Vec<_>>().join(" | ");
        return Err(refused_run(
            Path::new(&plan.name()),
            &[RunRefusal::SpawnFailed {
                status: output
                    .status
                    .code()
                    .map_or_else(|| "on a signal".to_owned(), |code| code.to_string()),
                tail,
                stream: stream_path.display().to_string(),
            }],
        ));
    }
    Ok(stream)
}

/// Run a live evaluation or ingest an existing stream.
///
/// # Errors
/// Refuses invalid inputs, missing spend authority, incompatible tools, and invalid records.
pub fn run_arm(args: &RunArgs) -> Result<ExitCode> {
    // Validated and then written through as the caller spelled it: the manifest carries the date,
    // and a date this binary cannot read is one the next reader cannot either.
    aep_cli::observation_time(Some(&args.observed_at))?;

    let plugins = declared_plugins(args)?;
    let cases = select_cases(args)?;
    std::fs::create_dir_all(&args.out)
        .with_context(|| format!("creating {}", args.out.display()))?;

    if let Some(stream) = &args.stream {
        return ingest_recorded(args, cases, stream, plugins);
    }

    // --- everything from here spends money -------------------------------------------------------

    let Some(binary) = tool() else {
        // Not an `Err`: the top-level handler renders those as `1`, and *the tool is missing* is
        // the one outcome a caller has to be able to tell from *what you passed is wrong* without
        // parsing prose. Design constant 4 — absent binary is a skip, never a red gate.
        eprintln!(
            "{}",
            RunRefusal::ToolMissing {
                looked_for: looked_for()
            }
        );
        return Ok(ExitCode::from(TOOL_MISSING_EXIT));
    };
    if !live() {
        return Err(refused_run(&args.out, &[RunRefusal::NotLive]));
    }
    launched_elsewhere(args)?;
    require_plugin_treatment(args, &plugins)?;
    let Some(budget) = &args.budget_usd else {
        return Err(refused_run(&args.out, &[RunRefusal::NoBudget]));
    };
    preflight_child_path(&cases, &args.out)?;
    let cap = micro_usd(budget)?;
    let assumed = micro_usd(&args.assume_usd_per_run)?;
    let Some(working_directory) = &args.cwd else {
        return Err(refused_run(&args.out, &[RunRefusal::NoWorkingTree]));
    };

    let total = cases.len();
    let mut spent = 0_u64;
    let mut launched = 0_usize;

    for (position, case) in cases.into_iter().enumerate() {
        // Checked **before** the spawn and against the assumed rate, because the only number
        // available before a run is the assumed one: a cap enforced after the fact is a receipt.
        if spent.saturating_add(assumed) > cap {
            outln!(
                "{}",
                RunRefusal::BudgetWouldBeExceeded {
                    spent,
                    next: assumed,
                    cap,
                    launched,
                    skipped: total - position,
                }
            );
            break;
        }

        let plan = Plan {
            case,
            arm: args.arm,
            harness: args.harness,
            plugins: plugins.clone(),
            model_requested: args.model.clone(),
        };
        let prompt = prompt_for(&plan, &args.instructions)?;
        let remaining = usd_plain(cap.saturating_sub(spent));
        let invocation = spawn_argv(
            &plan,
            &binary,
            working_directory,
            &prompt,
            args.plugin_dir.as_deref(),
            args.model.as_deref(),
            Some(&remaining),
        );
        let stream_path = args.out.join(format!("{}{EVENTS_SUFFIX}", plan.name()));

        let cwd = args.cwd.as_deref();
        let events = spawn(&plan, &invocation, &stream_path, args.redact, cwd)?;
        let products = ingest(&plan, &events, &args.observed_at, args.redact)?;
        // The stream's own stated cost, and the assumption **only** where it stated none — a cost
        // this reader could not convert never arrives here as `None`, because `ingest` refuses it
        // (`EVAL-STREAM-011`). A wire that writes `null` must not be able to spend without limit;
        // a wire that priced the run must not be charged an estimate instead.
        let (charge, source) = products
            .cost_micro_usd
            .map_or((assumed, "assumed"), |stated| (stated, "stated"));
        spent = spent.saturating_add(charge);
        write_products(&args.out, &plan, Some(&stream_path), &products)?;
        // Printed per run rather than only as a total, because the failure this line exists to make
        // visible is silent by nature: a stated cost dropped to an assumption looks exactly like a
        // cheap run, and one live Claude run at $0.797785 was charged $0.250000 before anybody
        // could see which of the two numbers the ledger was using.
        outln!("  charged:  {} ({source})", dollars(charge));
        launched += 1;
    }

    if launched == 0 {
        return Err(refused_run(
            &args.out,
            &[RunRefusal::BudgetWouldBeExceeded {
                spent,
                next: assumed,
                cap,
                launched,
                skipped: total,
            }],
        ));
    }

    outln!(
        "{launched} run(s), {} spent against a cap of {}",
        dollars(spent),
        dollars(cap)
    );
    Ok(ExitCode::SUCCESS)
}

include!("eval_tests.rs");
