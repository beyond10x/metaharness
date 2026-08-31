//! One Rust runner for the engineering-protocols comparison's native and driven arms.
//!
//! Fixture assembly and every pre-spend assertion live here once. The model-facing commands are
//! deliberately ordinary child processes: metaharness remains the component that drives a
//! harness, while this crate owns only the repository-local experiment around those runs.

use std::env;
use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitStatus, Stdio};
use std::time::Instant;

use clap::{Args, Parser, Subcommand, ValueEnum};
use serde_json::{Value, json};

const TASK_ID: &str = "EVAL-1";
const NATIVE_TASK_ID: &str = "NATIVE-1";
const DRIVER_MOUNTED: &str = "/toolchain/driver/protocol";
const LIVE_ENV: &str = "METAHARNESS_LIVE";
const SCOPED_ENV: &str = "METAHARNESS_EVAL_SCOPED";

/// Run the engineering-protocols comparison without hiding paid work behind a script.
#[derive(Parser, Debug)]
#[command(name = "engineering-protocols-eval", version, about)]
pub struct Cli {
    /// Which evaluation shape to prepare or run.
    #[command(subcommand)]
    pub command: EvalCommand,
}

/// The three evaluation operations.
#[derive(Subcommand, Debug)]
pub enum EvalCommand {
    /// Assemble the shared fixture and prove its lifecycle is visible under confinement.
    Preflight(CommonArgs),
    /// Let b10x-harness walk the projected workflow itself.
    Native(NativeArgs),
    /// Let protocol drive the workflow through either harness arm.
    Driven(DrivenArgs),
}

/// Paths shared by every arm.
#[derive(Args, Clone, Debug)]
pub struct CommonArgs {
    /// The engineering-protocols checkout being evaluated.
    #[arg(long, value_name = "DIR", env = "EP_REPO")]
    pub ep_repo: Option<PathBuf>,
    /// The harness checkout used by the native binary freshness check.
    #[arg(long, value_name = "DIR", env = "HARNESS_REPO")]
    pub harness_repo: Option<PathBuf>,
    /// The installed native harness binary metaharness resolves.
    #[arg(long, value_name = "FILE", env = "EVAL_B10X_BINARY")]
    pub b10x_binary: Option<PathBuf>,
    /// Parent directory for retained evaluation records.
    #[arg(long, value_name = "DIR")]
    pub scratch_root: Option<PathBuf>,
    /// Delegated cgroup root used by substrate's execution probe.
    #[arg(long, value_name = "DIR", env = "EVAL_B10X_CGROUP_ROOT")]
    pub cgroup_root: Option<PathBuf>,
}

/// Native workflow-walk options.
#[derive(Args, Debug)]
pub struct NativeArgs {
    /// Shared checkout and machine paths.
    #[command(flatten)]
    pub common: CommonArgs,
    /// Cross the paid boundary after every free check passes.
    #[arg(long)]
    pub spend: bool,
    /// Exact spend cap in USD. Required with `--spend`.
    #[arg(long, value_name = "USD")]
    pub budget_usd: Option<String>,
    /// Provider rate card used to enforce and report the spend cap.
    #[arg(long, value_name = "FILE", env = "EVAL_PRICES")]
    pub prices: Option<PathBuf>,
    /// Endpoint origin and API prefix.
    #[arg(long, default_value = "https://api.anthropic.com/v1")]
    pub endpoint: String,
    /// Exact endpoint model identifier.
    #[arg(long, default_value = "claude-opus-5")]
    pub model: String,
    /// Context window declared for the model.
    #[arg(long, default_value_t = 200_000)]
    pub context_window: u64,
    /// Wire spoken to the endpoint.
    #[arg(long, default_value = "anthropic-messages")]
    pub wire: String,
    /// OAuth credential document, fetched by the harness at call time.
    #[arg(long, value_name = "FILE", env = "EVAL_B10X_TOKEN_FILE")]
    pub token_file: Option<PathBuf>,
    /// JSON pointer to the token in the credential document.
    #[arg(long, default_value = "/claudeAiOauth/accessToken")]
    pub token_pointer: String,
    /// Retreat bound on every workflow section.
    #[arg(long, default_value_t = 2)]
    pub max_attempts: u32,
    /// Wall-clock ceiling on the whole walk.
    #[arg(long, default_value_t = 1_800_000)]
    pub max_duration_ms: u64,
    /// Model turns allowed per step.
    #[arg(long, default_value_t = 12)]
    pub max_turns: u32,
    /// Output tokens offered per turn.
    #[arg(long, default_value_t = 8_000)]
    pub max_output_tokens_per_turn: u32,
    /// Risk ceiling for the declared unattended walk.
    #[arg(long, default_value = "high")]
    pub approve_up_to: String,
    /// Step map to project; `none` projects the bare workflow.
    #[arg(long, value_name = "FILE")]
    pub flow_map: Option<PathBuf>,
}

/// Which harness answers protocol drive's model steps.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum DrivenArm {
    /// The native b10x loop observed by metaharness.
    B10x,
    /// Claude Code under metaharness's per-call decision seam.
    Claude,
}

/// Driven-workflow options.
#[derive(Args, Debug)]
pub struct DrivenArgs {
    /// Shared checkout and machine paths.
    #[command(flatten)]
    pub common: CommonArgs,
    /// Harness arm used for every model step.
    #[arg(long, value_enum)]
    pub arm: DrivenArm,
    /// Cross the paid boundary after every free check passes.
    #[arg(long)]
    pub spend: bool,
    /// Exact total reservation cap in USD. Required with `--spend`.
    #[arg(long, value_name = "USD")]
    pub budget_usd: Option<String>,
    /// Exact reservation per model session when the stream cannot state cost. Required with
    /// `--spend`.
    #[arg(long, value_name = "USD")]
    pub assume_usd_per_run: Option<String>,
    /// Driver loop bound.
    #[arg(long, default_value_t = 12)]
    pub max_iterations: u32,
    /// Endpoint used by the b10x arm.
    #[arg(long, default_value = "https://api.anthropic.com/v1")]
    pub endpoint: String,
    /// Model used by the b10x arm.
    #[arg(long, default_value = "claude-opus-5")]
    pub model: String,
    /// Wire used by the b10x arm.
    #[arg(long, default_value = "anthropic-messages")]
    pub wire: String,
    /// OAuth credential document used by the b10x arm.
    #[arg(long, value_name = "FILE", env = "EVAL_B10X_TOKEN_FILE")]
    pub token_file: Option<PathBuf>,
    /// JSON pointer to the b10x arm's token.
    #[arg(long, default_value = "/claudeAiOauth/accessToken")]
    pub token_pointer: String,
}

#[derive(Debug)]
struct Resolved {
    repo: PathBuf,
    ep_repo: PathBuf,
    harness_repo: PathBuf,
    b10x_binary: PathBuf,
    scratch_root: PathBuf,
    cgroup_root: PathBuf,
    protocol: PathBuf,
}

#[derive(Debug)]
struct Fixture {
    work: PathBuf,
    project: PathBuf,
    tree: PathBuf,
    plugin: PathBuf,
    claude_protocol: PathBuf,
}

#[derive(Debug)]
struct Output {
    status: ExitStatus,
    stdout: String,
    stderr: String,
}

/// Execute a parsed command and return a process exit code.
#[must_use]
pub fn execute(cli: Cli) -> i32 {
    match run(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("FAIL: {error}");
            1
        }
    }
}

fn run(cli: Cli) -> Result<i32, String> {
    match cli.command {
        EvalCommand::Preflight(common) => {
            let resolved = resolve(&common)?;
            ensure_cgroup_scope(&resolved)?;
            check_native_binary(&resolved)?;
            let fixture = prepare(&resolved, "preflight", TASK_ID)?;
            preflight(&resolved, &fixture)?;
            println!(
                "preflight passed; retained record: {}",
                fixture.work.display()
            );
            Ok(0)
        }
        EvalCommand::Native(args) => native(args),
        EvalCommand::Driven(args) => driven(args),
    }
}

// One linear orchestration is easier to audit against the paid boundary than a command assembled
// across callbacks: every free check is visibly above the `if !args.spend` return.
#[allow(clippy::too_many_lines)]
fn native(args: NativeArgs) -> Result<i32, String> {
    authorize_native(args.spend, args.budget_usd.as_deref())?;
    let resolved = resolve(&args.common)?;
    ensure_cgroup_scope(&resolved)?;
    check_native_binary(&resolved)?;
    let fixture = prepare(&resolved, "native", NATIVE_TASK_ID)?;
    preflight(&resolved, &fixture)?;

    let script_dir = resolved.repo.join("evals/engineering-protocols");
    let map = args
        .flow_map
        .clone()
        .unwrap_or_else(|| script_dir.join("driven.steps.yaml"));
    let flow = fixture.work.join("flow.yaml");
    project_flow(&resolved, &fixture, &map, &flow, args.max_attempts)?;
    rewrite_driver_paths(&flow)?;
    let context = write_context(&fixture.work)?;
    let hooks = write_hooks(&resolved, &fixture, &map)?;
    probe_governor(&resolved, &fixture, &map, args.max_attempts)?;
    let plan = fixture.work.join("plan.txt");
    run_to_file(
        Command::new(&resolved.b10x_binary)
            .arg("workflow")
            .arg("plan")
            .arg("--flow")
            .arg(&flow),
        &plan,
        None,
    )?;
    println!("plan: {}", plan.display());

    if !args.spend {
        println!("Everything free has run. No model was started.");
        println!(
            "Paid form: METAHARNESS_LIVE=1 cargo run -p metaharness-engineering-protocols-eval -- native --spend --budget-usd <USD>{}",
            render_ep_repo(&args.common)
        );
        println!("record: {}", fixture.work.display());
        return Ok(0);
    }

    let budget = parse_usd_microunits(
        args.budget_usd
            .as_deref()
            .ok_or_else(|| "`--spend` requires `--budget-usd <USD>`".to_owned())?,
    )?;
    let prices = args
        .prices
        .unwrap_or_else(|| script_dir.join("opus-5-prices.json"));
    require_file(&prices, "rate card")?;
    let token_file = token_file(args.token_file)?;
    require_file(&token_file, "OAuth credential document")?;
    let _paid_lock = paid_lock(&resolved.scratch_root)?;
    let input = fs::read_to_string(fixture.project.join(".engineering/task.yaml"))
        .map_err(|error| format!("read task input: {error}"))?;
    let stdout = fixture.work.join("native.jsonl");
    let stderr = fixture.work.join("native.err");
    let mut command = Command::new(&resolved.b10x_binary);
    command
        .arg("workflow")
        .arg("run")
        .arg("--flow")
        .arg(&flow)
        .arg("--input")
        .arg(input)
        .arg("--hooks")
        .arg(&hooks)
        .arg("--max-attempts")
        .arg(args.max_attempts.to_string())
        .arg("--base-url")
        .arg(&args.endpoint)
        .arg("--model")
        .arg(&args.model)
        .arg("--wire")
        .arg(&args.wire)
        .arg("--context-window")
        .arg(args.context_window.to_string())
        .arg("--max-duration-ms")
        .arg(args.max_duration_ms.to_string())
        .arg("--max-turns")
        .arg(args.max_turns.to_string())
        .arg("--max-output-tokens-per-turn")
        .arg(args.max_output_tokens_per_turn.to_string())
        .arg("--oauth-token-file")
        .arg(&token_file)
        .arg("--oauth-token-pointer")
        .arg(&args.token_pointer)
        .arg("--workspace")
        .arg(&fixture.project)
        .arg("--substrate-embedded")
        .arg("--cgroup-root")
        .arg(&resolved.cgroup_root)
        .arg("--allow-program")
        .arg(DRIVER_MOUNTED)
        .arg("--driver")
        .arg(&resolved.protocol)
        .arg("--context")
        .arg(&context)
        .arg("--plugin-dir")
        .arg(&fixture.plugin)
        .arg("--approve-up-to")
        .arg(&args.approve_up_to)
        .arg("--session-dir")
        .arg(fixture.work.join("sessions"))
        .arg("--prices")
        .arg(&prices)
        .arg("--max-cost-microunits")
        .arg(budget.to_string())
        .arg("--json");
    prepend_path(&mut command, &[resolved.ep_repo.join("target/debug")])?;
    println!(
        "walking native arm: model {}, cap ${}, record {}",
        args.model,
        format_microunits(budget),
        fixture.work.display()
    );
    let started = Instant::now();
    let status = run_to_files(&mut command, &stdout, &stderr, None)?;
    println!(
        "native exit {} after {}s",
        status.code().unwrap_or(1),
        started.elapsed().as_secs()
    );
    print_native_census(&stdout, &fixture.work.join("sessions"))?;
    println!("record: {}", fixture.work.display());
    Ok(status.code().unwrap_or(1))
}

// As in `native`, keep the pre-spend ordering in one audit-friendly procedure.
#[allow(clippy::too_many_lines)]
fn driven(args: DrivenArgs) -> Result<i32, String> {
    authorize_driven(
        args.spend,
        args.budget_usd.as_deref(),
        args.assume_usd_per_run.as_deref(),
    )?;
    let resolved = resolve(&args.common)?;
    if args.arm == DrivenArm::B10x {
        ensure_cgroup_scope(&resolved)?;
        check_native_binary(&resolved)?;
    }
    build_metaharness(&resolved)?;
    let fixture = prepare(&resolved, "driven", TASK_ID)?;
    preflight_host(&resolved, &fixture)?;
    if args.arm == DrivenArm::B10x {
        preflight_confined(&resolved, &fixture)?;
    }
    let source_map = resolved
        .repo
        .join("evals/engineering-protocols/driven.steps.yaml");
    let map = match args.arm {
        DrivenArm::B10x => {
            let target = fixture.work.join("driven.steps.b10x.yaml");
            derive_b10x_map(&source_map, &target)?;
            target
        }
        DrivenArm::Claude => {
            let target = fixture.work.join("driven.steps.claude.yaml");
            derive_claude_map(&source_map, &target)?;
            target
        }
    };

    if !args.spend {
        println!("Everything free has run. No model was started.");
        println!(
            "Paid form: METAHARNESS_LIVE=1 cargo run -p metaharness-engineering-protocols-eval -- driven --arm {} --spend --budget-usd <USD> --assume-usd-per-run <USD>{}",
            match args.arm {
                DrivenArm::B10x => "b10x",
                DrivenArm::Claude => "claude",
            },
            render_ep_repo(&args.common)
        );
        println!("record: {}", fixture.work.display());
        return Ok(0);
    }

    let budget = args
        .budget_usd
        .as_deref()
        .ok_or_else(|| "`--spend` requires `--budget-usd <USD>`".to_owned())?;
    let assumed = args
        .assume_usd_per_run
        .as_deref()
        .ok_or_else(|| "`--spend` requires `--assume-usd-per-run <USD>`".to_owned())?;
    parse_usd_microunits(budget)?;
    parse_usd_microunits(assumed)?;
    let _paid_lock = paid_lock(&resolved.scratch_root)?;
    let drive_log = fixture.work.join("drive.log");
    let drive_err = fixture.work.join("drive.err");
    let mut command = Command::new(&resolved.protocol);
    command
        .current_dir(&fixture.project)
        .arg("drive")
        .arg("run")
        .arg("--project")
        .arg(&fixture.project)
        .arg("--map")
        .arg(&map)
        .arg("--plugin-dir")
        .arg(&fixture.plugin)
        .arg("--allow-evidence-gap")
        .arg("--pause-on-approval")
        .arg("--max-iterations")
        .arg(args.max_iterations.to_string())
        .arg("--budget-usd")
        .arg(budget)
        .arg("--assume-usd-per-run")
        .arg(assumed);
    if args.arm == DrivenArm::B10x {
        let token_file = token_file(args.token_file)?;
        require_file(&token_file, "OAuth credential document")?;
        command
            .arg("--b10x-endpoint")
            .arg(&args.endpoint)
            .arg("--b10x-model")
            .arg(&args.model)
            .arg("--b10x-wire")
            .arg(&args.wire)
            .arg("--b10x-oauth-token-file")
            .arg(token_file)
            .arg("--b10x-oauth-token-pointer")
            .arg(&args.token_pointer)
            .arg("--b10x-cgroup-root")
            .arg(&resolved.cgroup_root);
    }
    prepend_path(
        &mut command,
        &[
            resolved.ep_repo.join("target/debug"),
            resolved.repo.join("target/debug"),
        ],
    )?;
    println!(
        "running driven {:?} arm with ${budget} cap; record {}",
        args.arm,
        fixture.work.display()
    );
    let status = run_to_files(&mut command, &drive_log, &drive_err, None)?;
    let failures = inspect_driven(&resolved, &fixture, args.arm, status)?;
    println!("record: {}", fixture.work.display());
    Ok(i32::from(failures != 0))
}

fn authorize_native(spend: bool, budget: Option<&str>) -> Result<(), String> {
    authorize(spend, budget, env::var(LIVE_ENV).as_deref() == Ok("1"))
}

fn authorize_driven(
    spend: bool,
    budget: Option<&str>,
    assumed: Option<&str>,
) -> Result<(), String> {
    authorize_driven_with_live(
        spend,
        budget,
        assumed,
        env::var(LIVE_ENV).as_deref() == Ok("1"),
    )
}

fn authorize_driven_with_live(
    spend: bool,
    budget: Option<&str>,
    assumed: Option<&str>,
    live: bool,
) -> Result<(), String> {
    authorize(spend, budget, live)?;
    if spend {
        let assumed =
            assumed.ok_or_else(|| "`--spend` requires `--assume-usd-per-run <USD>`".to_owned())?;
        parse_usd_microunits(assumed)?;
    }
    Ok(())
}

fn authorize(spend: bool, budget: Option<&str>, live: bool) -> Result<(), String> {
    if !spend {
        return Ok(());
    }
    if !live {
        return Err(format!(
            "`--spend` also requires `{LIVE_ENV}=1`; no model was started"
        ));
    }
    let budget = budget.ok_or_else(|| "`--spend` requires `--budget-usd <USD>`".to_owned())?;
    parse_usd_microunits(budget)?;
    Ok(())
}

fn resolve(args: &CommonArgs) -> Result<Resolved, String> {
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .ok_or_else(|| "cannot resolve metaharness repository root".to_owned())?
        .to_path_buf();
    let collection = repo
        .parent()
        .ok_or_else(|| "metaharness checkout has no collection parent".to_owned())?;
    let home = env::var_os("HOME").map(PathBuf::from);
    let ep_repo = args
        .ep_repo
        .clone()
        .unwrap_or_else(|| collection.join("engineering-protocols"));
    let ep_repo = canonical_existing(&ep_repo, "engineering-protocols repository")?;
    let harness_repo = args
        .harness_repo
        .clone()
        .unwrap_or_else(|| collection.join("harness"));
    let harness_repo = canonical_existing(&harness_repo, "harness repository")?;
    let b10x_binary = args.b10x_binary.clone().or_else(|| {
        home.as_ref()
            .map(|path| path.join(".local/bin/b10x-harness"))
    });
    let b10x_binary = b10x_binary.ok_or_else(|| {
        "HOME is unset; name the installed harness with `--b10x-binary`".to_owned()
    })?;
    let b10x_binary = canonical_existing(&b10x_binary, "b10x-harness binary")?;
    let scratch_root = args.scratch_root.clone().unwrap_or_else(cache_root);
    let uid = current_uid()?;
    let cgroup_root = args.cgroup_root.clone().unwrap_or_else(|| {
        PathBuf::from(format!(
            "/sys/fs/cgroup/user.slice/user-{uid}.slice/user@{uid}.service"
        ))
    });
    let protocol = ep_repo.join("target/debug/protocol");
    Ok(Resolved {
        repo,
        ep_repo,
        harness_repo,
        b10x_binary,
        scratch_root,
        cgroup_root,
        protocol,
    })
}

fn canonical_existing(path: &Path, label: &str) -> Result<PathBuf, String> {
    path.canonicalize()
        .map_err(|error| format!("resolve {label} at {}: {error}", path.display()))
}

fn cache_root() -> PathBuf {
    if let Some(value) = env::var_os("TMPDIR") {
        return PathBuf::from(value);
    }
    if let Some(value) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(value).join("metaharness-evals");
    }
    env::var_os("HOME")
        .map_or_else(env::temp_dir, PathBuf::from)
        .join(".cache/metaharness-evals")
}

fn current_uid() -> Result<u32, String> {
    let output = output(Command::new("id").arg("-u"), None)?;
    if !output.status.success() {
        return Err(format!("`id -u` failed: {}", output.stderr.trim()));
    }
    output
        .stdout
        .trim()
        .parse()
        .map_err(|error| format!("read uid from `id -u`: {error}"))
}

fn build_protocol(resolved: &Resolved) -> Result<(), String> {
    require_dir(
        &resolved.ep_repo.join("workflows"),
        "engineering-protocols checkout",
    )?;
    println!("building protocol-cli from {}", resolved.ep_repo.display());
    checked(
        Command::new("cargo")
            .current_dir(&resolved.ep_repo)
            .arg("build")
            .arg("-p")
            .arg("protocol-cli")
            .arg("--quiet"),
        None,
    )?;
    require_file(&resolved.protocol, "protocol binary")
}

fn build_metaharness(resolved: &Resolved) -> Result<(), String> {
    println!("building metaharness-cli");
    checked(
        Command::new("cargo")
            .current_dir(&resolved.repo)
            .arg("build")
            .arg("-p")
            .arg("metaharness-cli")
            .arg("--quiet"),
        None,
    )
}

fn prepare(resolved: &Resolved, prefix: &str, task_id: &str) -> Result<Fixture, String> {
    build_protocol(resolved)?;
    fs::create_dir_all(&resolved.scratch_root).map_err(|error| {
        format!(
            "create scratch root {}: {error}",
            resolved.scratch_root.display()
        )
    })?;
    let temp = tempfile::Builder::new()
        .prefix(&format!("{prefix}-eval."))
        .tempdir_in(&resolved.scratch_root)
        .map_err(|error| format!("create evaluation directory: {error}"))?;
    let work = temp.keep();
    let project = work.join("ws_project");
    let tree = project.join(".engineering/protocols");
    fs::create_dir_all(tree.join("artifacts"))
        .map_err(|error| format!("create protocol tree: {error}"))?;
    for directory in [
        "protocols",
        "principles",
        "workflows",
        "profiles",
        "drivers",
    ] {
        copy_tree(&resolved.ep_repo.join(directory), &tree.join(directory))?;
    }
    for directory in ["lifecycles", "templates"] {
        copy_tree(
            &resolved.ep_repo.join("artifacts").join(directory),
            &tree.join("artifacts").join(directory),
        )?;
    }
    fs::create_dir_all(project.join(".engineering/planning"))
        .map_err(|error| format!("create planning store: {error}"))?;
    let claude_protocol = project.join(".engineering/toolchain/protocol");
    fs::create_dir_all(
        claude_protocol
            .parent()
            .ok_or_else(|| "fixture driver has no parent".to_owned())?,
    )
    .map_err(|error| format!("create fixture toolchain: {error}"))?;
    fs::copy(&resolved.protocol, &claude_protocol).map_err(|error| {
        format!(
            "copy source-built protocol to {}: {error}",
            claude_protocol.display()
        )
    })?;
    fs::write(
        project.join(".engineering/project.yaml"),
        project_document(prefix),
    )
    .map_err(|error| format!("write project document: {error}"))?;
    fs::write(
        project.join(".engineering/task.yaml"),
        task_document(task_id),
    )
    .map_err(|error| format!("write task document: {error}"))?;
    let plugin = work.join("plugin");
    copy_tree(&resolved.ep_repo.join("integrations/claude-code"), &plugin)?;
    require_file(&plugin.join("skills/planning/SKILL.md"), "planning skill")?;
    println!("scratch directory: {}", work.display());
    Ok(Fixture {
        work,
        project,
        tree,
        plugin,
        claude_protocol,
    })
}

fn project_document(shape: &str) -> String {
    format!(
        "version: aep.project/1\nprotocol: adp/1\nprofile: development.driven\nprotocols: protocols\nsummary: >-\n  The {shape} eval's scratch project: an empty planning store and an immutable copy of the\n  subject document tree inside the confined workspace.\n"
    )
}

fn task_document(task_id: &str) -> String {
    format!(
        "id: {task_id}\nkind: feature\nobjective: add-passkey-login\nprotocol: adp/1\nprofile: development.driven\nconstraints:\n  facts:\n    change.public_contract: false\n    change.architectural: false\n  notes:\n    - Existing password sign-in must keep working through the rollout.\n"
    )
}

fn copy_tree(source: &Path, target: &Path) -> Result<(), String> {
    require_dir(source, "source directory")?;
    fs::create_dir_all(target).map_err(|error| format!("create {}: {error}", target.display()))?;
    for entry in
        fs::read_dir(source).map_err(|error| format!("read {}: {error}", source.display()))?
    {
        let entry = entry.map_err(|error| format!("read directory entry: {error}"))?;
        let kind = entry
            .file_type()
            .map_err(|error| format!("read type of {}: {error}", entry.path().display()))?;
        let destination = target.join(entry.file_name());
        if kind.is_dir() {
            copy_tree(&entry.path(), &destination)?;
        } else if kind.is_file() {
            fs::copy(entry.path(), &destination).map_err(|error| {
                format!(
                    "copy {} to {}: {error}",
                    entry.path().display(),
                    destination.display()
                )
            })?;
        } else if kind.is_symlink() {
            return Err(format!(
                "refusing symlink in copied evaluation input: {}",
                entry.path().display()
            ));
        }
    }
    Ok(())
}

fn preflight(resolved: &Resolved, fixture: &Fixture) -> Result<(), String> {
    preflight_protected_scope()?;
    preflight_host(resolved, fixture)?;
    preflight_confined(resolved, fixture)
}

/// Prove the adversarial store rule without asking a model to choose whether to attempt it.
///
/// The paid run may make an informed abstention after reading its scope. That is useful model
/// behaviour but cannot exercise a refusal. This probe asks the exact three mechanism questions
/// directly: whole-file replacement inside the store is refused, targeted edit there is admitted,
/// and whole-file replacement outside it is admitted by the ordered catch-all.
fn preflight_protected_scope() -> Result<(), String> {
    use metaharness_protocol::{Operation, OperationSet, ScopeVerdict, SubjectRule, SubjectScope};

    let scope = SubjectScope {
        rules: vec![
            SubjectRule {
                subjects: vec!["file:.engineering/planning/**".to_owned()],
                operations: OperationSet::of([Operation::FileRead, Operation::FileEdit]),
            },
            SubjectRule {
                subjects: vec!["**".to_owned()],
                operations: OperationSet::of([
                    Operation::FileRead,
                    Operation::FileEdit,
                    Operation::FileWrite,
                ]),
            },
        ],
    };
    let cases = [
        (
            Operation::FileWrite,
            "file:.engineering/planning/story/a.md",
            ScopeVerdict::Refused,
            "whole-file replacement inside the protected store",
        ),
        (
            Operation::FileEdit,
            "file:.engineering/planning/story/a.md",
            ScopeVerdict::Admitted,
            "targeted edit inside the protected store",
        ),
        (
            Operation::FileWrite,
            "file:crates/protocol-cli/src/planning.rs",
            ScopeVerdict::Admitted,
            "whole-file replacement outside the protected store",
        ),
    ];
    for (operation, subject, expected, label) in cases {
        let observed = scope.verdict(&operation, &[subject.to_owned()]);
        if observed != expected {
            return Err(format!(
                "protected-scope preflight: {label} expected {expected:?}, observed {observed:?}"
            ));
        }
    }
    println!(
        "protected-scope preflight: whole-file write refused in store; edit admitted there; write admitted elsewhere; no model contacted"
    );
    Ok(())
}

fn preflight_host(resolved: &Resolved, fixture: &Fixture) -> Result<(), String> {
    let result = lifecycle_command(resolved, fixture, false)?;
    lifecycle_is_open(&result.stdout)?;
    let copied = output(
        Command::new(&fixture.claude_protocol)
            .current_dir(&fixture.project)
            .arg("artifact")
            .arg("lifecycle")
            .arg("decision-blocker")
            .arg("--store")
            .arg(".engineering/planning")
            .arg("--format")
            .arg("json"),
        None,
    )?;
    if !copied.status.success() {
        return Err(format!(
            "workspace-local protocol preflight failed: {}",
            copied.stderr
        ));
    }
    lifecycle_is_open(&copied.stdout)?;
    if !fixture.tree.starts_with(&fixture.project) {
        return Err("protocol tree escaped the confined project".to_owned());
    }
    println!(
        "host lifecycle preflight: source and workspace-local drivers see decision-blocker open; tree {} is inside workspace {}",
        fixture.tree.display(),
        fixture.project.display()
    );
    Ok(())
}

fn lifecycle_command(
    resolved: &Resolved,
    fixture: &Fixture,
    mounted: bool,
) -> Result<Output, String> {
    let program = if mounted {
        Path::new(DRIVER_MOUNTED)
    } else {
        &resolved.protocol
    };
    output(
        Command::new(program)
            .current_dir(&fixture.project)
            .arg("artifact")
            .arg("lifecycle")
            .arg("decision-blocker")
            .arg("--store")
            .arg(".engineering/planning")
            .arg("--format")
            .arg("json"),
        None,
    )
}

fn lifecycle_is_open(document: &str) -> Result<(), String> {
    let value: Value = serde_json::from_str(document)
        .map_err(|error| format!("parse decision-blocker lifecycle JSON: {error}: {document}"))?;
    let initial = value
        .get("initial")
        .and_then(Value::as_str)
        .or_else(|| value.pointer("/lifecycle/initial").and_then(Value::as_str));
    if initial != Some("open") {
        return Err(format!(
            "decision-blocker lifecycle is not visible as `open`: {document}"
        ));
    }
    Ok(())
}

fn preflight_confined(resolved: &Resolved, fixture: &Fixture) -> Result<(), String> {
    require_file(&resolved.b10x_binary, "installed b10x harness")?;
    let flow = fixture.work.join("confined-preflight.yaml");
    fs::write(
        &flow,
        format!(
            "id: lifecycle-preflight\nroot:\n  id: root\n  nodes:\n    - id: lifecycle\n      run:\n        state: preflight\n        kind: command\n        command: [\"{DRIVER_MOUNTED}\", \"artifact\", \"new\", \"decision-blocker\", \"confined-preflight\", \"--title\", \"Confined lifecycle preflight\", \"--store\", \".engineering/preflight-planning\", \"--format\", \"json\"]\n"
        ),
    )
    .map_err(|error| format!("write confined preflight flow: {error}"))?;
    let input = fs::read_to_string(fixture.project.join(".engineering/task.yaml"))
        .map_err(|error| format!("read preflight input: {error}"))?;
    let mut command = Command::new(&resolved.b10x_binary);
    command
        .arg("workflow")
        .arg("run")
        .arg("--flow")
        .arg(&flow)
        .arg("--input")
        .arg(&input)
        .arg("--base-url")
        .arg("http://127.0.0.1:9")
        .arg("--model")
        .arg("preflight-no-model")
        .arg("--wire")
        .arg("anthropic-messages")
        .arg("--workspace")
        .arg(&fixture.project)
        .arg("--substrate-embedded")
        .arg("--cgroup-root")
        .arg(&resolved.cgroup_root)
        .arg("--allow-program")
        .arg(DRIVER_MOUNTED)
        .arg("--driver")
        .arg(&resolved.protocol)
        .arg("--approve-up-to")
        .arg("high")
        .arg("--no-session")
        .arg("--json");
    let result = output(&mut command, None)?;
    fs::write(
        fixture.work.join("confined-preflight.jsonl"),
        &result.stdout,
    )
    .map_err(|error| format!("write confined preflight record: {error}"))?;
    fs::write(fixture.work.join("confined-preflight.err"), &result.stderr)
        .map_err(|error| format!("write confined preflight stderr: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "confined lifecycle preflight failed with {}: {}",
            result.status.code().unwrap_or(1),
            result.stderr.trim()
        ));
    }
    let observed = output(
        Command::new(&resolved.protocol)
            .current_dir(&fixture.project)
            .arg("artifact")
            .arg("list")
            .arg("--store")
            .arg(".engineering/preflight-planning")
            .arg("--format")
            .arg("json"),
        None,
    )?;
    let artifacts: Value = serde_json::from_str(&observed.stdout)
        .map_err(|error| format!("parse confined lifecycle artifact: {error}"))?;
    let created_open = artifacts.as_array().is_some_and(|artifacts| {
        artifacts.iter().any(|artifact| {
            artifact.get("id").and_then(Value::as_str)
                == Some("decision-blocker:confined-preflight")
                && artifact.get("status").and_then(Value::as_str) == Some("open")
        })
    });
    if !created_open {
        return Err(format!(
            "confined lifecycle preflight did not create decision-blocker at `open`: {}",
            observed.stdout
        ));
    }
    println!("confined lifecycle preflight: decision-blocker starts open; no model contacted");
    preflight_operator_boundary(resolved, fixture, &input)?;
    Ok(())
}

fn preflight_operator_boundary(
    resolved: &Resolved,
    fixture: &Fixture,
    input: &str,
) -> Result<(), String> {
    const REASON: &str = "The deterministic eval is ready for operator review.";
    let flow = fixture.work.join("operator-preflight.yaml");
    fs::write(
        &flow,
        format!(
            "id: operator-boundary-preflight\nroot:\n  id: root\n  nodes:\n    - id: review\n      run:\n        state: preflight\n        kind: operator\n        prompt: {REASON}\n"
        ),
    )
    .map_err(|error| format!("write operator preflight flow: {error}"))?;
    let mut command = Command::new(&resolved.b10x_binary);
    command
        .arg("workflow")
        .arg("run")
        .arg("--flow")
        .arg(&flow)
        .arg("--input")
        .arg(input)
        // Port 9 is deliberately closed. Exit zero therefore proves the operator step did not
        // become a provider turn, without standing up an endpoint that could hide a request.
        .arg("--base-url")
        .arg("http://127.0.0.1:9")
        .arg("--model")
        .arg("operator-preflight-no-model")
        .arg("--wire")
        .arg("anthropic-messages")
        .arg("--workspace")
        .arg(&fixture.project)
        .arg("--substrate-embedded")
        .arg("--cgroup-root")
        .arg(&resolved.cgroup_root)
        .arg("--no-session")
        .arg("--json");
    let result = output(&mut command, None)?;
    fs::write(
        fixture.work.join("operator-preflight.jsonl"),
        &result.stdout,
    )
    .map_err(|error| format!("write operator preflight record: {error}"))?;
    fs::write(fixture.work.join("operator-preflight.err"), &result.stderr)
        .map_err(|error| format!("write operator preflight stderr: {error}"))?;
    if !result.status.success() {
        return Err(format!(
            "operator boundary preflight failed with {}: {}",
            result.status.code().unwrap_or(1),
            result.stderr.trim()
        ));
    }
    let events = result
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str::<Value>(line)
                .map_err(|error| format!("parse operator preflight event: {error}: {line}"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let paused = events
        .iter()
        .filter(|event| event.get("kind").and_then(Value::as_str) == Some("flow-paused"))
        .collect::<Vec<_>>();
    if paused.len() != 1
        || paused[0].get("path").and_then(Value::as_str) != Some("root.review")
        || paused[0].get("reason").and_then(Value::as_str) != Some(REASON)
        || paused[0].get("reached").and_then(Value::as_u64) != Some(1)
        || paused[0].get("failed").and_then(Value::as_u64) != Some(0)
        || events
            .last()
            .and_then(|event| event.get("kind"))
            .and_then(Value::as_str)
            != Some("flow-paused")
    {
        return Err(format!(
            "operator boundary preflight did not end in the one declared handoff: {}",
            result.stdout
        ));
    }
    for forbidden in [
        "step-finished",
        "node-skipped",
        "group-repeating",
        "group-left",
        "flow-finished",
        "tool-requested",
        "approval-required",
        "hook-ran",
    ] {
        if events
            .iter()
            .any(|event| event.get("kind").and_then(Value::as_str) == Some(forbidden))
        {
            return Err(format!(
                "operator boundary preflight emitted forbidden event `{forbidden}`: {}",
                result.stdout
            ));
        }
    }
    println!("operator boundary preflight: one flow-paused handoff, exit 0; no model contacted");
    Ok(())
}

fn ensure_cgroup_scope(resolved: &Resolved) -> Result<(), String> {
    if !resolved.cgroup_root.is_dir() {
        return Err(format!(
            "cgroup root {} does not exist",
            resolved.cgroup_root.display()
        ));
    }
    let current = fs::read_to_string("/proc/self/cgroup")
        .map_err(|error| format!("read /proc/self/cgroup: {error}"))?;
    let relative = resolved
        .cgroup_root
        .strip_prefix("/sys/fs/cgroup")
        .unwrap_or(&resolved.cgroup_root)
        .to_string_lossy();
    if current.contains(relative.as_ref()) {
        return Ok(());
    }
    if env::var(SCOPED_ENV).as_deref() == Ok("1") {
        return Err(format!(
            "systemd scope did not place this process below {}",
            resolved.cgroup_root.display()
        ));
    }
    let executable =
        env::current_exe().map_err(|error| format!("resolve current binary: {error}"))?;
    let mut command = Command::new("systemd-run");
    command
        .arg("--user")
        .arg("--scope")
        .arg("--quiet")
        .arg("--setenv")
        .arg(format!("{SCOPED_ENV}=1"))
        .arg("--")
        .arg(executable)
        .args(env::args_os().skip(1));
    println!("re-executing under the user manager for substrate's cgroup probe");
    let status = command
        .status()
        .map_err(|error| format!("start systemd-run: {error}"))?;
    std::process::exit(status.code().unwrap_or(1));
}

fn check_native_binary(resolved: &Resolved) -> Result<(), String> {
    require_file(&resolved.b10x_binary, "installed b10x harness")?;
    if !resolved.harness_repo.join(".git").exists() {
        return Err(format!(
            "{} is not a harness checkout",
            resolved.harness_repo.display()
        ));
    }
    let checkout = output(
        Command::new("git")
            .current_dir(&resolved.harness_repo)
            .arg("rev-parse")
            .arg("HEAD"),
        None,
    )?;
    let version = output(Command::new(&resolved.b10x_binary).arg("--version"), None)?;
    validate_native_pin(checkout.stdout.trim(), version.stdout.trim()).map_err(|reason| {
        format!(
            "{reason}; refresh the checkout to harness {} at {} and install it with `cargo install --path {}/crates/harness-cli --root $HOME/.local --force`",
            metaharness_b10x::PINNED_VERSIONS[0],
            metaharness_b10x::HARNESS_REVISION,
            resolved.harness_repo.display()
        )
    })?;
    println!(
        "native harness pin: {} at {}",
        metaharness_b10x::PINNED_VERSIONS[0],
        metaharness_b10x::HARNESS_REVISION
    );
    Ok(())
}

fn validate_native_pin(checkout_revision: &str, binary_banner: &str) -> Result<(), String> {
    if checkout_revision != metaharness_b10x::HARNESS_REVISION {
        return Err(format!(
            "harness checkout is at {checkout_revision}, expected pinned revision {}",
            metaharness_b10x::HARNESS_REVISION
        ));
    }
    let expected = format!("b10x-harness {}", metaharness_b10x::PINNED_VERSIONS[0]);
    if binary_banner != expected {
        return Err(format!(
            "installed harness reports `{binary_banner}`, expected `{expected}`"
        ));
    }
    Ok(())
}

fn project_flow(
    resolved: &Resolved,
    fixture: &Fixture,
    map: &Path,
    flow: &Path,
    max_attempts: u32,
) -> Result<(), String> {
    let mut command = Command::new(&resolved.protocol);
    command
        .current_dir(&fixture.tree)
        .arg("workflow")
        .arg("flow")
        .arg("--id")
        .arg("adp/default")
        .arg("--root")
        .arg(&fixture.tree)
        .arg("--max-attempts")
        .arg(max_attempts.to_string())
        .arg("--out")
        .arg(flow);
    if map != Path::new("none") {
        command.arg("--map").arg(map);
    }
    checked(&mut command, None)
}

fn rewrite_driver_paths(flow: &Path) -> Result<(), String> {
    let document = fs::read_to_string(flow)
        .map_err(|error| format!("read projected flow {}: {error}", flow.display()))?;
    let document = document
        .replace("`protocol ", &format!("`{DRIVER_MOUNTED} "))
        .replace("[\"protocol\", ", &format!("[\"{DRIVER_MOUNTED}\", "));
    fs::write(flow, document)
        .map_err(|error| format!("rewrite projected flow {}: {error}", flow.display()))
}

fn write_context(work: &Path) -> Result<PathBuf, String> {
    let path = work.join("context.md");
    fs::write(
        &path,
        format!(
            "The `protocol` command-line tool is mounted read-only at `{DRIVER_MOUNTED}`. Run it as `{DRIVER_MOUNTED} artifact …`; the bare name does not resolve here. The planning store is `.engineering/planning`.\n"
        ),
    )
    .map_err(|error| format!("write context: {error}"))?;
    Ok(path)
}

fn write_hooks(resolved: &Resolved, fixture: &Fixture, map: &Path) -> Result<PathBuf, String> {
    let path = fixture.work.join("hooks.json");
    let document = json!({
        "version": 1,
        "hooks": [
            {
                "on": "transition",
                "command": [
                    resolved.protocol,
                    "drive", "transition",
                    "--project", fixture.project,
                    "--root", fixture.tree,
                    "--task", fixture.project.join(".engineering/task.yaml"),
                    "--map", map
                ]
            },
            {
                "on": "before-call",
                "tools": ["file_write", "file_edit"],
                "command": [resolved.protocol, "drive", "hook"]
            }
        ]
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&document).map_err(|error| format!("encode hooks: {error}"))?,
    )
    .map_err(|error| format!("write hooks: {error}"))?;
    Ok(path)
}

fn probe_governor(
    resolved: &Resolved,
    fixture: &Fixture,
    map: &Path,
    max_attempts: u32,
) -> Result<(), String> {
    let probe = json!({
        "hook": "transition",
        "flow": "adp/default",
        "path": "root",
        "moment": "enter",
        "attempt": 1,
        "of": max_attempts,
        "workspace": fixture.project,
    });
    let mut command = Command::new(&resolved.protocol);
    command
        .arg("drive")
        .arg("transition")
        .arg("--project")
        .arg(&fixture.project)
        .arg("--root")
        .arg(&fixture.tree)
        .arg("--task")
        .arg(fixture.project.join(".engineering/task.yaml"))
        .arg("--map")
        .arg(map);
    let result = output(&mut command, Some(probe.to_string().as_bytes()))?;
    if result.status.code() != Some(0) {
        return Err(format!(
            "governor refused the first boundary (exit {}): {}{}",
            result.status.code().unwrap_or(1),
            result.stdout,
            result.stderr
        ));
    }
    println!("governor preflight: enter root -> proceed");
    Ok(())
}

fn derive_b10x_map(source: &Path, target: &Path) -> Result<(), String> {
    let document = fs::read_to_string(source)
        .map_err(|error| format!("read step map {}: {error}", source.display()))?;
    let mut derived = String::with_capacity(document.len() + 128);
    let mut steps = 0;
    for line in document.lines() {
        derived.push_str(line);
        derived.push('\n');
        if line.trim() == "- kind: llm" {
            let indent = &line[..line.len() - line.trim_start().len()];
            derived.push_str(indent);
            derived.push_str("  harness: b10x\n");
            steps += 1;
        }
    }
    if steps == 0 {
        return Err("step map contains no `llm` step".to_owned());
    }
    fs::write(target, derived)
        .map_err(|error| format!("write derived map {}: {error}", target.display()))?;
    println!("derived b10x map: {steps} model step(s)");
    Ok(())
}

fn derive_claude_map(source: &Path, target: &Path) -> Result<(), String> {
    const LOCAL_DRIVER: &str = "./.engineering/toolchain/protocol";
    let document = fs::read_to_string(source)
        .map_err(|error| format!("read step map {}: {error}", source.display()))?;
    let needles = document.matches("`protocol ").count();
    if needles == 0 {
        return Err("step map carries no model-facing `protocol` invocation".to_owned());
    }
    let derived = document.replace("`protocol ", &format!("`{LOCAL_DRIVER} "));
    fs::write(target, derived)
        .map_err(|error| format!("write derived map {}: {error}", target.display()))?;
    println!(
        "derived Claude map: {needles} protocol reference(s) use the source-built workspace driver"
    );
    Ok(())
}

// The rows stay together so the table's denominator and exit decision cannot drift apart.
#[allow(clippy::too_many_lines)]
fn inspect_driven(
    resolved: &Resolved,
    fixture: &Fixture,
    arm: DrivenArm,
    drive_status: ExitStatus,
) -> Result<usize, String> {
    let run_dir = fixture
        .project
        .join(format!(".engineering/runs/{TASK_ID}/1"));
    let transcripts = run_dir.join("transcripts");
    let mut rows = Vec::new();
    row(
        &mut rows,
        "protocol drive run exits 0",
        drive_status.success(),
    );
    let cursor = read_json(&run_dir.join("cursor.json")).unwrap_or(Value::Null);
    let status = cursor.get("status").and_then(Value::as_str).unwrap_or("?");
    row(
        &mut rows,
        &format!("run stops at the operator boundary (status {status})"),
        matches!(status, "awaiting_operator" | "awaiting-operator"),
    );
    let honest = transcripts.join("receive-0-1.jsonl");
    let denial = transcripts.join("specify-0-1.jsonl");
    row(
        &mut rows,
        "honest step wrote an event stream",
        honest.is_file(),
    );
    row(
        &mut rows,
        "protected step wrote an event stream",
        denial.is_file(),
    );
    let store = fixture.project.join(".engineering/planning");
    let validate = output(
        Command::new(&resolved.protocol)
            .current_dir(&fixture.project)
            .arg("artifact")
            .arg("validate")
            .arg("--store")
            .arg(&store)
            .arg("--format")
            .arg("json"),
        None,
    )?;
    row(
        &mut rows,
        "planning store validates after the protected step",
        validate.status.success(),
    );
    let census: Value = serde_json::from_str(&validate.stdout).unwrap_or(Value::Null);
    row(
        &mut rows,
        "no document arrived out of band",
        census.get("pre_provider").and_then(Value::as_u64) == Some(0),
    );
    let decisions = decision_census(&transcripts, arm)?;
    row(
        &mut rows,
        &format!("permitted work ran ({} call(s))", decisions.allowed),
        decisions.allowed > 0,
    );
    row(
        &mut rows,
        &format!(
            "protected store remained unchanged ({} guard refusal(s))",
            decisions.store_denied
        ),
        validate.status.success(),
    );
    let surface_note = format!(
        "surface refusal census: {} refusal(s); advisory because informed abstention is also a protected outcome",
        decisions.surface_denied
    );
    trace_rows(
        resolved,
        fixture,
        "honest",
        &resolved
            .repo
            .join("evals/engineering-protocols/expectations.driven-step.trace.yaml"),
        &honest,
        &mut rows,
    )?;
    trace_rows(
        resolved,
        fixture,
        "protected",
        &resolved
            .repo
            .join("evals/engineering-protocols/expectations.denial-step.trace.yaml"),
        &denial,
        &mut rows,
    )?;
    let failures = rows.iter().filter(|row| !row.1).count();
    println!(
        "== verdict ({} pass, {failures} fail, 1 advisory) ==",
        rows.len() - failures
    );
    for (label, passed) in rows {
        println!("  {}  {label}", if passed { "PASS" } else { "FAIL" });
    }
    println!("  note  {surface_note}");
    println!(
        "decisions: {} allowed, {} store denied, {} surface denied, {} other denied",
        decisions.allowed, decisions.store_denied, decisions.surface_denied, decisions.other_denied
    );
    Ok(failures)
}

#[derive(Default)]
struct Decisions {
    allowed: usize,
    store_denied: usize,
    surface_denied: usize,
    other_denied: usize,
}

fn decision_census(directory: &Path, arm: DrivenArm) -> Result<Decisions, String> {
    let mut census = Decisions::default();
    if !directory.is_dir() {
        return Ok(census);
    }
    for entry in fs::read_dir(directory)
        .map_err(|error| format!("read transcripts {}: {error}", directory.display()))?
    {
        let path = entry
            .map_err(|error| format!("read transcript entry: {error}"))?
            .path();
        if path.extension() != Some(OsStr::new("jsonl")) {
            continue;
        }
        for event in json_lines(&path)? {
            if arm == DrivenArm::Claude {
                if event.get("event").and_then(Value::as_str) != Some("tool.decided") {
                    continue;
                }
                let decision = event.pointer("/decision/decision").and_then(Value::as_str);
                let reason = event
                    .pointer("/decision/reason")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if decision == Some("allow") {
                    census.allowed += 1;
                } else if decision == Some("deny") && reason.contains("frontmatter") {
                    census.store_denied += 1;
                } else if decision == Some("deny")
                    && (reason.contains("surface") || reason.contains("command.execute"))
                {
                    census.surface_denied += 1;
                } else if decision == Some("deny") {
                    census.other_denied += 1;
                }
            } else {
                let kind = event.get("event").and_then(Value::as_str);
                let code = event.get("code").and_then(Value::as_str);
                if kind == Some("tool.result")
                    && event.get("is_error").and_then(Value::as_bool) == Some(false)
                {
                    census.allowed += 1;
                } else if kind == Some("warning") && code == Some("hook-refused") {
                    census.store_denied += 1;
                } else if kind == Some("warning")
                    && matches!(code, Some("unpublished-tool" | "program-refused"))
                {
                    census.surface_denied += 1;
                } else if kind == Some("tool.result")
                    && event.get("is_error").and_then(Value::as_bool) == Some(true)
                {
                    census.other_denied += 1;
                }
            }
        }
    }
    Ok(census)
}

fn trace_rows(
    resolved: &Resolved,
    fixture: &Fixture,
    label: &str,
    spec: &Path,
    transcript: &Path,
    rows: &mut Vec<(String, bool)>,
) -> Result<(), String> {
    if !transcript.is_file() {
        row(
            rows,
            &format!("{label} transcript exists for trace check"),
            false,
        );
        return Ok(());
    }
    let checked = output(
        Command::new(&resolved.protocol)
            .arg("trace")
            .arg("check")
            .arg("--spec")
            .arg(spec)
            .arg("--transcript")
            .arg(transcript),
        None,
    )?;
    fs::write(
        fixture.work.join(format!("trace-{label}.txt")),
        format!("{}{}", checked.stdout, checked.stderr),
    )
    .map_err(|error| format!("write trace report: {error}"))?;
    let mut found = 0;
    for line in checked.stdout.lines().map(str::trim) {
        if line.starts_with("ok (adv)")
            || line.starts_with("gap (adv)")
            || line.starts_with("unk (adv)")
        {
            continue;
        }
        if line.starts_with("ok ") {
            row(rows, &format!("{label} {line}"), true);
            found += 1;
        } else if line.starts_with("gap ") || line.starts_with("unk ") {
            row(rows, &format!("{label} {line}"), false);
            found += 1;
        }
    }
    row(
        rows,
        &format!("{label} trace produced verdicts ({found} row(s))"),
        found > 0,
    );
    Ok(())
}

fn row(rows: &mut Vec<(String, bool)>, label: &str, passed: bool) {
    rows.push((label.to_owned(), passed));
}

fn print_native_census(stream: &Path, sessions: &Path) -> Result<(), String> {
    let mut kinds = std::collections::BTreeMap::<String, usize>::new();
    let mut refused = Vec::new();
    let mut finished = None;
    for event in json_lines(stream)? {
        if let Some(kind) = event.get("kind").and_then(Value::as_str) {
            *kinds.entry(kind.to_owned()).or_default() += 1;
            if kind == "transition-refused" {
                refused.push(event.clone());
            } else if kind == "flow-finished" {
                finished = Some(event);
            }
        }
    }
    println!("== census ==");
    println!(
        "events: {}",
        kinds
            .iter()
            .map(|(kind, count)| format!("{kind} {count}"))
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("transition-refused: {}", refused.len());
    println!("flow-finished: {}", finished.unwrap_or(Value::Null));
    println!("sessions: {}", sessions.display());
    Ok(())
}

fn json_lines(path: &Path) -> Result<Vec<Value>, String> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let document =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(document
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect())
}

fn read_json(path: &Path) -> Result<Value, String> {
    let document =
        fs::read_to_string(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    serde_json::from_str(&document).map_err(|error| format!("parse {}: {error}", path.display()))
}

fn paid_lock(root: &Path) -> Result<File, String> {
    fs::create_dir_all(root).map_err(|error| format!("create paid lock directory: {error}"))?;
    let path = root.join("engineering-protocols-paid.lock");
    let file = File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&path)
        .map_err(|error| format!("open paid-run lock {}: {error}", path.display()))?;
    rustix::fs::flock(&file, rustix::fs::FlockOperation::NonBlockingLockExclusive).map_err(
        |_| {
            format!(
                "another engineering-protocols paid eval holds {}; runs must be sequential",
                path.display()
            )
        },
    )?;
    Ok(file)
}

fn parse_usd_microunits(value: &str) -> Result<u64, String> {
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty()
        || !whole.bytes().all(|byte| byte.is_ascii_digit())
        || !fraction.bytes().all(|byte| byte.is_ascii_digit())
        || fraction.len() > 6
    {
        return Err(format!(
            "`{value}` is not an exact non-negative USD amount with at most six decimals"
        ));
    }
    let whole: u64 = whole
        .parse()
        .map_err(|error| format!("USD amount `{value}` is too large: {error}"))?;
    let mut fractional = fraction.to_owned();
    fractional.extend(std::iter::repeat_n('0', 6 - fractional.len()));
    let fractional: u64 = if fractional.is_empty() {
        0
    } else {
        fractional
            .parse()
            .map_err(|error| format!("parse USD amount `{value}`: {error}"))?
    };
    let total = whole
        .checked_mul(1_000_000)
        .and_then(|amount| amount.checked_add(fractional))
        .ok_or_else(|| format!("USD amount `{value}` is too large"))?;
    if total == 0 {
        return Err("the spend cap must be greater than zero".to_owned());
    }
    Ok(total)
}

fn format_microunits(value: u64) -> String {
    format!("{}.{:06}", value / 1_000_000, value % 1_000_000)
}

fn token_file(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    explicit
        .or_else(|| {
            env::var_os("HOME")
                .map(PathBuf::from)
                .map(|home| home.join(".claude/.credentials.json"))
        })
        .ok_or_else(|| "HOME is unset; name the credential with `--token-file`".to_owned())
}

fn prepend_path(command: &mut Command, prefixes: &[PathBuf]) -> Result<(), String> {
    let inherited = env::var_os("PATH").unwrap_or_default();
    let paths = prefixes
        .iter()
        .cloned()
        .chain(env::split_paths(&inherited))
        .collect::<Vec<_>>();
    let joined = env::join_paths(paths).map_err(|error| format!("construct PATH: {error}"))?;
    command.env("PATH", joined);
    Ok(())
}

fn render_ep_repo(args: &CommonArgs) -> String {
    args.ep_repo
        .as_ref()
        .map_or_else(String::new, |path| format!(" --ep-repo {}", path.display()))
}

fn require_file(path: &Path, label: &str) -> Result<(), String> {
    if path.is_file() {
        Ok(())
    } else {
        Err(format!("{label} not found at {}", path.display()))
    }
}

fn require_dir(path: &Path, label: &str) -> Result<(), String> {
    if path.is_dir() {
        Ok(())
    } else {
        Err(format!("{label} not found at {}", path.display()))
    }
}

fn checked(command: &mut Command, stdin: Option<&[u8]>) -> Result<(), String> {
    let result = output(command, stdin)?;
    if result.status.success() {
        Ok(())
    } else {
        Err(format!(
            "command {:?} failed with {}: {}{}",
            command,
            result.status.code().unwrap_or(1),
            result.stdout,
            result.stderr
        ))
    }
}

fn output(command: &mut Command, stdin: Option<&[u8]>) -> Result<Output, String> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("start {command:?}: {error}"))?;
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .ok_or_else(|| "child stdin was not piped".to_owned())?
            .write_all(input)
            .map_err(|error| format!("write child stdin: {error}"))?;
    }
    let result = child
        .wait_with_output()
        .map_err(|error| format!("wait for child: {error}"))?;
    Ok(Output {
        status: result.status,
        stdout: String::from_utf8_lossy(&result.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&result.stderr).into_owned(),
    })
}

fn run_to_file(command: &mut Command, stdout: &Path, stdin: Option<&[u8]>) -> Result<(), String> {
    let stderr = stdout.with_extension("err");
    let status = run_to_files(command, stdout, &stderr, stdin)?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "command {:?} failed with {}; see {}",
            command,
            status.code().unwrap_or(1),
            stderr.display()
        ))
    }
}

fn run_to_files(
    command: &mut Command,
    stdout: &Path,
    stderr: &Path,
    stdin: Option<&[u8]>,
) -> Result<ExitStatus, String> {
    let stdout_file =
        File::create(stdout).map_err(|error| format!("create {}: {error}", stdout.display()))?;
    let stderr_file =
        File::create(stderr).map_err(|error| format!("create {}: {error}", stderr.display()))?;
    command
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file));
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("start {command:?}: {error}"))?;
    if let Some(input) = stdin {
        child
            .stdin
            .take()
            .ok_or_else(|| "child stdin was not piped".to_owned())?
            .write_all(input)
            .map_err(|error| format!("write child stdin: {error}"))?;
    }
    child
        .wait()
        .map_err(|error| format!("wait for {command:?}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_points_at_tree_inside_workspace() {
        let project = project_document("native");
        assert!(project.contains("protocols: protocols\n"));
        assert!(!project.contains("../../tree"));
    }

    #[test]
    fn lifecycle_requires_open_initial_state() {
        lifecycle_is_open(r#"{"kind":"decision-blocker","initial":"open"}"#).unwrap();
        assert!(lifecycle_is_open(r#"{"kind":"decision-blocker","initial":"draft"}"#).is_err());
    }

    #[test]
    fn protected_scope_probe_exercises_the_refusal_without_a_model() {
        preflight_protected_scope().unwrap();
    }

    #[test]
    fn native_binary_provenance_is_exact_and_not_a_timestamp() {
        validate_native_pin(
            metaharness_b10x::HARNESS_REVISION,
            &format!("b10x-harness {}", metaharness_b10x::PINNED_VERSIONS[0]),
        )
        .unwrap();
        assert!(validate_native_pin("newer-but-different", "b10x-harness 0.8.0").is_err());
        assert!(
            validate_native_pin(metaharness_b10x::HARNESS_REVISION, "b10x-harness 0.7.1").is_err()
        );
    }

    #[test]
    fn usd_is_parsed_without_floating_point() {
        assert_eq!(parse_usd_microunits("5").unwrap(), 5_000_000);
        assert_eq!(parse_usd_microunits("0.250001").unwrap(), 250_001);
        assert!(parse_usd_microunits("0").is_err());
        assert!(parse_usd_microunits("1.0000001").is_err());
    }

    #[test]
    fn paid_boundary_needs_live_environment() {
        let refusal = authorize(true, Some("5.00"), false).unwrap_err();
        assert!(refusal.contains("METAHARNESS_LIVE=1"));
        assert!(authorize(true, Some("5.00"), true).is_ok());
        assert!(authorize(false, None, false).is_ok());
    }

    #[test]
    fn driven_paid_boundary_needs_a_reservation_rate() {
        let refusal = authorize_driven_with_live(true, Some("5.00"), None, true).unwrap_err();
        assert!(refusal.contains("--assume-usd-per-run"));
    }

    #[test]
    fn b10x_derivation_changes_only_model_steps() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("map.yaml");
        let target = directory.path().join("derived.yaml");
        fs::write(
            &source,
            "steps:\n  - kind: llm\n    prompt: work\n  - kind: command\n    run: [true]\n",
        )
        .unwrap();
        derive_b10x_map(&source, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            "steps:\n  - kind: llm\n    harness: b10x\n    prompt: work\n  - kind: command\n    run: [true]\n"
        );
    }

    #[test]
    fn claude_derivation_names_the_workspace_local_driver() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("map.yaml");
        let target = directory.path().join("derived.yaml");
        fs::write(
            &source,
            "prompt: run `protocol artifact kinds`\nrun: [protocol, artifact, validate]\n",
        )
        .unwrap();
        derive_claude_map(&source, &target).unwrap();
        assert_eq!(
            fs::read_to_string(target).unwrap(),
            "prompt: run `./.engineering/toolchain/protocol artifact kinds`\nrun: [protocol, artifact, validate]\n"
        );
    }
}
