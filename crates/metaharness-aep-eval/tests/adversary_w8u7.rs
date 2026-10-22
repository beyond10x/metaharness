//! Adversary pass, wave 8 unit 7 (`story:aep-eval-native-arm-removed`).
//!
//! Two surfaces. The removed-verb source scan is private to `src/lib.rs`'s test module, so the
//! scan cases compile a verbatim copy of its items, extracted from the file at test time, and
//! feed it hand-built sources: whatever the scan does here, the gate's scan does. The confined
//! preflight is driven through the built `aep-eval preflight` with `/bin/sh` stand-ins for the
//! harness, the driver, `cargo` and `git`; the stand-in harness talks to the real scripted
//! endpoint with `curl`. No model, no vendor binary, no credential, no network beyond 127.0.0.1.
//!
//! The removed verbs never appear here as quoted literals: they are assembled at run time, as the
//! scan's own test does, so this file is not itself an offence.

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Writing an executable while another thread forks leaves a copy of the write descriptor in the
/// child until it execs, and the exec of that script then fails with `ETXTBSY`. Every case here
/// writes scripts or a binary and runs them, so they run one at a time.
static SERIAL: Mutex<()> = Mutex::new(());

fn serial() -> MutexGuard<'static, ()> {
    SERIAL.lock().unwrap_or_else(PoisonError::into_inner)
}

fn repository() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("the repository root")
        .to_path_buf()
}

fn work_root() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("adversary_w8u7")
}

fn removed_verbs() -> [String; 2] {
    [["work", "flow"].concat(), ["eve", "nts"].concat()]
}

fn quoted(verb: &str) -> String {
    format!("\"{verb}\"")
}

// ---------------------------------------------------------------------------------------------
// The source scan, compiled from its own text.
// ---------------------------------------------------------------------------------------------

/// One four-space-indented item of `lib.rs`'s test module, dedented: a one-line `const`, or a
/// `fn` up to the first line that is exactly its closing brace.
fn extract_item(source: &str, opening: &str) -> String {
    let mut lines = source.lines().skip_while(|line| !line.starts_with(opening));
    let first = lines
        .next()
        .unwrap_or_else(|| panic!("lib.rs has no item opening `{opening}`"));
    let mut item = format!("{}\n", first.strip_prefix("    ").unwrap_or(first));
    if first.trim_end().ends_with(';') {
        return item;
    }
    for line in lines {
        item.push_str(line.strip_prefix("    ").unwrap_or(line));
        item.push('\n');
        if line == "    }" {
            return item;
        }
    }
    panic!("`{opening}` never closes");
}

/// The scan's binary: the five items `harness_verb_offences` is made of, byte for byte, and a
/// `main` that prints each offence for one relative path and one source file.
fn scan_binary() -> &'static Path {
    static BINARY: OnceLock<PathBuf> = OnceLock::new();
    BINARY.get_or_init(|| {
        let library = fs::read_to_string(repository().join("crates/metaharness-aep-eval/src/lib.rs"))
            .expect("read lib.rs");
        let mut program = String::from("use std::path::Path;\n\n");
        for opening in [
            "    const HARNESS_BINARY_MARKERS",
            "    const HARNESS_ARGV_RENDERER",
            "    fn removed_harness_verb_literals",
            "    fn command_span_end",
            "    fn harness_verb_offences",
        ] {
            program.push_str(&extract_item(&library, opening));
            program.push('\n');
        }
        program.push_str(
            "fn main() {\n    let arguments: Vec<String> = std::env::args().collect();\n    let text = std::fs::read_to_string(&arguments[2]).expect(\"case source\");\n    for offence in harness_verb_offences(Path::new(&arguments[1]), &text) {\n        println!(\"{offence}\");\n    }\n}\n",
        );
        let directory = work_root().join("scan");
        fs::create_dir_all(&directory).expect("create the scan directory");
        let source = directory.join("scan_copy.rs");
        fs::write(&source, program).expect("write the scan copy");
        let binary = directory.join("scan_copy");
        let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
        let built = Command::new(rustc)
            .arg("--edition")
            .arg("2024")
            .arg("--crate-name")
            .arg("scan_copy")
            .arg("-o")
            .arg(&binary)
            .arg(&source)
            .output()
            .expect("start rustc");
        assert!(
            built.status.success(),
            "the extracted scan does not compile: {}",
            String::from_utf8_lossy(&built.stderr)
        );
        binary
    })
}

/// What the gate's scan reports for `text` standing at `relative`.
fn scan(relative: &str, label: &str, text: &str) -> Vec<String> {
    let binary = scan_binary();
    let case = work_root().join("scan").join(format!("{label}.rs.txt"));
    fs::write(&case, text).expect("write the case source");
    let output = Command::new(binary)
        .arg(relative)
        .arg(&case)
        .output()
        .expect("run the scan copy");
    assert!(output.status.success(), "the scan copy failed: {output:?}");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect()
}

/// The form the scan's own test proves it sees. Run beside every evasion so a red evasion is the
/// scan missing a shape, not the extraction failing.
fn assert_the_scan_sees_the_direct_form(verb: &str) {
    let direct = format!(
        "fn plan(resolved: &Resolved) {{\n    Command::new(&resolved.b10x_binary).arg({}).arg(\"run\");\n}}\n",
        quoted(verb)
    );
    let offences = scan("crates/x/src/lib.rs", &format!("direct-{verb}"), &direct);
    assert_eq!(offences.len(), 1, "control: {offences:?}");
}

/// `crates/metaharness/src/doctor.rs` spawns the harness binary today (`flag_surface`, the
/// `run --help` read), on a `Command::new(&program)` whose line names no marker. Swap its verb for
/// a removed one, in memory, and the scan must notice: it is the one harness spawn in the
/// workspace outside the adapter and the eval.
#[test]
fn the_scan_sees_a_removed_verb_at_the_doctors_harness_spawn() {
    let _serial = serial();
    let doctor = fs::read_to_string(repository().join("crates/metaharness/src/doctor.rs"))
        .expect("read doctor.rs");
    let spawn = ".args([\"run\", \"--help\"])";
    assert!(
        doctor.contains(spawn) && doctor.contains("resolve_on(\"b10x-harness\""),
        "doctor.rs no longer reads the harness's run help this way"
    );
    assert!(
        scan("crates/metaharness/src/doctor.rs", "doctor-as-is", &doctor).is_empty(),
        "doctor.rs as checked in is clean"
    );
    for verb in removed_verbs() {
        assert_the_scan_sees_the_direct_form(&verb);
        let mutated = doctor.replace(spawn, &format!(".args([{}, \"--help\"])", quoted(&verb)));
        let offences = scan(
            "crates/metaharness/src/doctor.rs",
            &format!("doctor-{verb}"),
            &mutated,
        );
        assert!(
            !offences.is_empty(),
            "doctor.rs passes {} to the harness binary it resolved as `b10x-harness`, and the scan reports nothing",
            quoted(&verb)
        );
    }
}

/// The verb named by a constant, the way this crate names every other fixed argument.
#[test]
fn the_scan_sees_a_removed_verb_held_in_a_constant() {
    let _serial = serial();
    for verb in removed_verbs() {
        assert_the_scan_sees_the_direct_form(&verb);
        let source = format!(
            "const PLAN_VERB: &str = {};\n\nfn plan(resolved: &Resolved) {{\n    Command::new(&resolved.b10x_binary).arg(PLAN_VERB).arg(\"run\");\n}}\n",
            quoted(&verb)
        );
        let offences = scan("crates/x/src/lib.rs", &format!("constant-{verb}"), &source);
        assert!(
            !offences.is_empty(),
            "a constant holding {} handed to the harness binary is not reported",
            quoted(&verb)
        );
    }
}

/// The shape `confined_probe_command` has: one function builds the harness `Command`, its caller
/// adds arguments. The scan's span stops at the caller's `fn` line.
#[test]
fn the_scan_sees_a_removed_verb_added_by_the_builders_caller() {
    let _serial = serial();
    for verb in removed_verbs() {
        assert_the_scan_sees_the_direct_form(&verb);
        let source = format!(
            "fn harness(resolved: &Resolved) -> Command {{\n    Command::new(&resolved.b10x_binary)\n}}\n\nfn plan(resolved: &Resolved) {{\n    let mut command = harness(resolved);\n    command.arg({}).arg(\"run\");\n}}\n",
            quoted(&verb)
        );
        let offences = scan("crates/x/src/lib.rs", &format!("builder-{verb}"), &source);
        assert!(
            !offences.is_empty(),
            "{} added to a harness Command built by another function is not reported",
            quoted(&verb)
        );
    }
}

/// The harness path bound to a local first, so the constructor line names no marker.
#[test]
fn the_scan_sees_a_removed_verb_on_a_harness_path_bound_to_a_local() {
    let _serial = serial();
    for verb in removed_verbs() {
        assert_the_scan_sees_the_direct_form(&verb);
        let source = format!(
            "fn plan(resolved: &Resolved) {{\n    let harness = &resolved.b10x_binary;\n    Command::new(harness).arg({}).arg(\"run\");\n}}\n",
            quoted(&verb)
        );
        let offences = scan("crates/x/src/lib.rs", &format!("binding-{verb}"), &source);
        assert!(
            !offences.is_empty(),
            "{} on a harness Command built from a local binding is not reported",
            quoted(&verb)
        );
    }
}

/// The harness run through a shell, its verb inside the command string.
#[test]
fn the_scan_sees_a_removed_verb_inside_a_shell_command_string() {
    let _serial = serial();
    for verb in removed_verbs() {
        assert_the_scan_sees_the_direct_form(&verb);
        let source = format!(
            "fn plan() {{\n    Command::new(\"/bin/sh\").arg(\"-c\").arg(\"b10x-harness {verb} run --flow flow.json\");\n}}\n"
        );
        let offences = scan("crates/x/src/lib.rs", &format!("shell-{verb}"), &source);
        assert!(
            !offences.is_empty(),
            "`b10x-harness {verb}` in a shell string is not reported"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// The confined preflight, end to end, with stand-ins.
// ---------------------------------------------------------------------------------------------

/// What the stand-in harness does once `aep-eval` starts it with `run`.
struct Behaviour {
    /// Request bodies sent by `POST` to `{base-url}/responses`, in order, by file name in the case dir.
    requests: &'static [&'static str],
    /// The index in `requests` before which the staged driver creates the blocker, if it does.
    create_before: Option<usize>,
    /// Extra `curl` header, if any.
    header: Option<&'static str>,
    exit: i32,
}

const FIRST: &str = r#"{"model":"preflight-scripted","tools":[{"type":"function","name":"run"}],"input":[{"role":"user","content":"go"}],"stream":true}"#;
const SECOND: &str = r#"{"model":"preflight-scripted","tools":[{"type":"function","name":"run"}],"input":[{"role":"user","content":"go"},{"type":"function_call","call_id":"call_confined_preflight","name":"run","arguments":"{}"},{"type":"function_call_output","call_id":"call_confined_preflight","output":"{\"exit\":0}"}],"stream":true}"#;
const REFUSED: &str = r#"{"model":"preflight-scripted","tools":[{"type":"function","name":"run"}],"input":[{"role":"user","content":"go"},{"type":"function_call","call_id":"call_confined_preflight","name":"run","arguments":"{}"},{"type":"function_call_output","call_id":"call_confined_preflight","output":"refused: above the approval ceiling"}],"stream":true}"#;
const OTHER_CALL: &str = r#"{"model":"preflight-scripted","tools":[{"type":"function","name":"run"}],"input":[{"role":"user","content":"go"},{"type":"function_call_output","call_id":"call_somebody_else","output":"{\"exit\":0}"}],"stream":true}"#;

fn write_script(path: &Path, body: &str) {
    fs::write(path, body).unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    fs::set_permissions(path, fs::Permissions::from_mode(0o755))
        .expect("make the script executable");
}

const DRIVER: &str = r#"#!/bin/sh
store=""
previous=""
for argument in "$@"; do
  if [ "$previous" = "--store" ]; then store="$argument"; fi
  previous="$argument"
done
case "$1:$2" in
  artifact:lifecycle) echo '{"initial":"open"}' ;;
  artifact:new) mkdir -p "$store" && : > "$store/confined-preflight.created" && echo '{}' ;;
  artifact:list)
    if [ -f "$store/confined-preflight.created" ]; then
      echo '[{"id":"decision-blocker:confined-preflight","status":"LISTED"}]'
    else
      echo '[]'
    fi ;;
  *) echo "stand-in driver: unexpected $*" >&2; exit 2 ;;
esac
"#;

/// The stand-in driver, listing the blocker it created at `listed`.
fn driver_script(listed: &str) -> String {
    DRIVER.replace("LISTED", listed)
}

fn harness_script(case: &Path, behaviour: &Behaviour) -> String {
    let header = behaviour
        .header
        .map_or_else(String::new, |header| format!("-H '{header}' "));
    let mut steps = String::new();
    for (index, request) in behaviour.requests.iter().enumerate() {
        if behaviour.create_before == Some(index) {
            steps.push_str(
                "(cd \"$workspace\" && \"$driver\" artifact new decision-blocker confined-preflight --title t --store .engineering/preflight-planning/store --format json > /dev/null) || exit 8\n",
            );
        }
        steps.push_str("post ");
        steps.push_str(&index.to_string());
        steps.push_str(" \"$case/");
        steps.push_str(request);
        steps.push_str("\" || exit 9\n");
    }
    if behaviour.create_before == Some(behaviour.requests.len()) {
        steps.push_str(
            "(cd \"$workspace\" && \"$driver\" artifact new decision-blocker confined-preflight --title t --store .engineering/preflight-planning/store --format json > /dev/null) || exit 8\n",
        );
    }
    format!(
        r#"#!/bin/sh
set -u
case="{case}"
if [ "${{1:-}}" = "--version" ]; then echo "b10x-harness {version}"; exit 0; fi
printf '%s\n' "$@" > "$case/harness.argv"
printf '%s\n' "${{XDG_CONFIG_HOME:-unset}}" > "$case/harness.xdg"
base=""; workspace=""; driver=""
while [ $# -gt 0 ]; do
  case "$1" in
    --base-url) base="$2"; shift 2 ;;
    --workspace) workspace="$2"; shift 2 ;;
    --driver) driver="$2"; shift 2 ;;
    *) shift ;;
  esac
done
[ -d "$workspace/.engineering/preflight-planning/store" ] && : > "$case/store-existed"
post() {{
  /usr/bin/curl -sS --max-time 20 -o "$case/response.$1" -H 'Content-Type: application/json' {header}--data-binary "@$2" "$base/responses"
}}
{steps}exit {exit}
"#,
        case = case.display(),
        version = metaharness_b10x::PINNED_VERSIONS[0],
        exit = behaviour.exit,
    )
}

struct Run {
    case: PathBuf,
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

/// The binary under attack, or a mutated build of it named by `ADV_W8U7_AEP_EVAL`.
fn aep_eval() -> PathBuf {
    std::env::var_os("ADV_W8U7_AEP_EVAL").map_or_else(
        || PathBuf::from(env!("CARGO_BIN_EXE_aep-eval")),
        PathBuf::from,
    )
}

fn preflight(label: &str, behaviour: &Behaviour) -> Run {
    preflight_listing(label, behaviour, "open")
}

/// [`preflight`], with the stand-in driver listing the created blocker at `listed`.
fn preflight_listing(label: &str, behaviour: &Behaviour, listed: &str) -> Run {
    let case = work_root().join("preflight").join(label);
    if case.exists() {
        fs::remove_dir_all(&case).expect("clear the previous case");
    }
    let standins = case.join("bin");
    let aep = case.join("aep");
    let plugins = case.join("agentplugins");
    let harness_repo = case.join("harness");
    let scratch = case.join("scratch");
    let home = case.join("home");
    for directory in [&standins, &plugins, &scratch, &home] {
        fs::create_dir_all(directory).expect("create a case directory");
    }
    for directory in [
        "workflows",
        "protocols",
        "principles",
        "profiles",
        "drivers",
        "artifacts/lifecycles",
        "artifacts/templates",
        "target/debug",
    ] {
        fs::create_dir_all(aep.join(directory)).expect("create the stand-in AEP checkout");
    }
    fs::create_dir_all(harness_repo.join(".git")).expect("create the stand-in harness checkout");
    let skill = plugins.join("plugins/aep-plan/skills/planning/SKILL.md");
    fs::create_dir_all(skill.parent().expect("skill parent")).expect("create the plugin");
    fs::write(&skill, "stand-in\n").expect("write the skill");
    for (name, body) in [
        ("first.json", FIRST),
        ("second.json", SECOND),
        ("refused.json", REFUSED),
        ("other-call.json", OTHER_CALL),
    ] {
        fs::write(case.join(name), body).expect("write a request body");
    }
    write_script(&standins.join("cargo"), "#!/bin/sh\nexit 0\n");
    write_script(
        &standins.join("git"),
        &format!("#!/bin/sh\necho {}\n", metaharness_b10x::HARNESS_REVISION),
    );
    write_script(&aep.join("target/debug/aep"), &driver_script(listed));
    let harness = standins.join("stand-in-harness");
    write_script(&harness, &harness_script(&case, behaviour));

    let output = Command::new(aep_eval())
        .env_clear()
        .env("PATH", format!("{}:/usr/bin:/bin", standins.display()))
        .env("HOME", &home)
        .arg("preflight")
        .arg("--aep-repo")
        .arg(&aep)
        .arg("--agentplugins-repo")
        .arg(&plugins)
        .arg("--harness-repo")
        .arg(&harness_repo)
        .arg("--b10x-binary")
        .arg(&harness)
        .arg("--scratch-root")
        .arg(&scratch)
        .arg("--cgroup-root")
        .arg("/sys/fs/cgroup")
        .output()
        .expect("start aep-eval");
    Run {
        case,
        code: output.status.code(),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

fn assert_refused(run: &Run, reason: &str) {
    assert_eq!(
        run.code,
        Some(1),
        "the preflight passed; stdout:\n{}\nstderr:\n{}",
        run.stdout,
        run.stderr
    );
    assert!(
        run.stderr.contains(reason),
        "refused for another reason than `{reason}`:\n{}",
        run.stderr
    );
}

const HONEST: Behaviour = Behaviour {
    requests: &["first.json", "second.json"],
    create_before: Some(1),
    header: None,
    exit: 0,
};

/// The control: a harness that holds the scripted conversation and whose call creates the blocker
/// passes, with the confinement argv it was given and the store already in place.
#[test]
fn an_honest_stand_in_harness_passes_the_confined_preflight() {
    let _serial = serial();
    let run = preflight("honest", &HONEST);
    assert_eq!(
        run.code,
        Some(0),
        "stdout:\n{}\nstderr:\n{}",
        run.stdout,
        run.stderr
    );
    assert!(
        run.stdout.contains("confined lifecycle preflight:"),
        "{}",
        run.stdout
    );
    let argv = fs::read_to_string(run.case.join("harness.argv")).expect("the harness argv");
    let argv: Vec<&str> = argv.lines().collect();
    let pair = |flag: &str| {
        argv.iter()
            .position(|argument| *argument == flag)
            .and_then(|at| argv.get(at + 1).copied())
    };
    assert_eq!(argv.first().copied(), Some("run"), "{argv:?}");
    assert_eq!(pair("--max-duration-ms"), Some("120000"), "{argv:?}");
    assert_eq!(
        pair("--process-write-subtree"),
        Some(".engineering/preflight-planning"),
        "{argv:?}"
    );
    assert_eq!(
        pair("--allow-program"),
        Some("/toolchain/driver/aep"),
        "{argv:?}"
    );
    assert_eq!(pair("--wire"), Some("openai-responses"), "{argv:?}");
    assert!(
        pair("--base-url")
            .is_some_and(|url| url.starts_with("http://127.0.0.1:") && url.ends_with("/v1")),
        "{argv:?}"
    );
    assert!(argv.contains(&"--substrate-embedded"), "{argv:?}");
    assert!(argv.contains(&"--no-session"), "{argv:?}");
    assert!(
        !argv
            .iter()
            .any(|argument| argument.contains("token") || argument.contains("credential")),
        "{argv:?}"
    );
    let config = fs::read_to_string(run.case.join("harness.xdg")).expect("the harness config");
    let config = PathBuf::from(config.trim());
    assert!(
        config.ends_with("confined-preflight-config"),
        "{}",
        config.display()
    );
    assert!(
        config.starts_with(run.case.join("scratch")),
        "{}",
        config.display()
    );
    assert!(
        fs::read_dir(&config)
            .expect("read the config home")
            .next()
            .is_none(),
        "the config home is not empty"
    );
    assert!(
        run.case.join("store-existed").exists(),
        "the store was not made before the run"
    );
    let call = fs::read_to_string(run.case.join("response.0")).expect("the first response");
    for needle in [
        "\"name\":\"run\"",
        "call_confined_preflight",
        "decision-blocker",
        "confined-preflight",
        ".engineering/preflight-planning/store",
        "/toolchain/driver/aep",
    ] {
        assert!(
            call.contains(needle),
            "the scripted call lacks {needle}: {call}"
        );
    }
    let answer = fs::read_to_string(run.case.join("response.1")).expect("the second response");
    assert!(answer.contains("output_text"), "{answer}");
}

/// A harness that never calls the endpoint and exits 0 is not a confined run.
#[test]
fn a_harness_that_never_calls_the_endpoint_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "silent",
        &Behaviour {
            requests: &[],
            create_before: Some(0),
            header: None,
            exit: 0,
        },
    );
    assert_refused(&run, "sent no request");
}

/// A bearer token on the wire fails the preflight, whatever else went right.
#[test]
fn an_authorization_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "authorization",
        &Behaviour {
            header: Some("Authorization: Bearer stand-in"),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// So does the Anthropic-style key header.
#[test]
fn an_x_api_key_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "x-api-key",
        &Behaviour {
            header: Some("X-Api-Key: stand-in"),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// And the Azure-style one.
#[test]
fn an_api_key_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "api-key",
        &Behaviour {
            header: Some("api-key: stand-in"),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// A proxy credential is a credential on the wire too; the header name is matched whatever its
/// case.
#[test]
fn a_proxy_authorization_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "proxy-authorization",
        &Behaviour {
            header: Some("PROXY-AUTHORIZATION: Basic c3RhbmQtaW46aW4="),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// A session cookie authenticates as much as a bearer token does.
#[test]
fn a_cookie_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "cookie",
        &Behaviour {
            header: Some("cookie: session=stand-in"),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// And the Gemini-style key header.
#[test]
fn an_x_goog_api_key_header_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "x-goog-api-key",
        &Behaviour {
            header: Some("X-Goog-Api-Key: stand-in"),
            ..HONEST
        },
    );
    assert_refused(&run, "authorization header");
}

/// The approver refused the call: the result comes back, the run completes, no blocker exists.
#[test]
fn a_refused_tool_call_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "refused",
        &Behaviour {
            requests: &["first.json", "refused.json"],
            create_before: None,
            header: None,
            exit: 0,
        },
    );
    assert_refused(&run, "did not create decision-blocker");
}

/// A run that held the conversation and exited nonzero is a failed run.
#[test]
fn a_nonzero_harness_exit_fails_the_preflight() {
    let _serial = serial();
    let run = preflight("nonzero", &Behaviour { exit: 3, ..HONEST });
    assert_refused(&run, "failed with 3");
}

/// A result for some other call id is not the scripted call's result.
#[test]
fn a_result_for_another_call_id_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "other-call",
        &Behaviour {
            requests: &["first.json", "other-call.json"],
            create_before: Some(1),
            header: None,
            exit: 0,
        },
    );
    assert_refused(&run, "never returned the scripted `run` call's result");
}

/// A request after the final message is outside the script.
#[test]
fn a_request_after_the_final_message_fails_the_preflight() {
    let _serial = serial();
    let run = preflight(
        "third",
        &Behaviour {
            requests: &["first.json", "second.json", "second.json"],
            create_before: Some(1),
            header: None,
            exit: 0,
        },
    );
    assert_refused(&run, "expected exactly two requests");
}

/// The blocker exists after the run but the driver lists it at another status than the
/// lifecycle's initial one: the call did not create it at `open`, whatever else went right.
#[test]
fn a_blocker_listed_at_another_status_fails_the_preflight() {
    let _serial = serial();
    for listed in ["accepted", "closed", "Open"] {
        let run = preflight_listing(&format!("listed-{listed}"), &HONEST, listed);
        assert_refused(&run, "did not create decision-blocker at `open`");
    }
}
