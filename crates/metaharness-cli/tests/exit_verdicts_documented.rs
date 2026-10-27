//! The CLI reference's § *Exit codes* says what an audited run prints, line for line (design
//! amendment a48).
//!
//! An audited run that exits `3` may have a harness child that exited `0`: the child ended
//! cleanly and the floor could not tell whether every control held. The report names the child's
//! exit, the control verdict and the audit verdict on three lines of their own, so a caller does
//! not have to read the vendor's record to tell the three apart. Each test below renders those
//! lines from a real `--hermetic strict` audited run over a declared scratch copy, whose scripted
//! child's runner states what it measured or measures nothing, and holds
//! `website/docs/reference/cli.md` to them. No process is started and no model is called.

use std::fs;
use std::path::{Path, PathBuf};

use metaharness::protocol::{CredentialSource, HermeticMode, Kind, ProcessTermination};
use metaharness::{
    FakeAuditor, HarnessProcess, Input, LaunchPlanView, Metaharness, ProcessPoll, ProcessRunner,
    ScriptStep, ScriptedLog, ScriptedProcess, ScriptedSeams,
};

const PAGE: &str = "website/docs/reference/cli.md";

/// What the opening record says, which is what selects the gating rows' verdicts.
#[derive(Debug, Clone, Copy)]
enum Opening {
    /// Every field the floor reads, on the adapter's pin: every gating row is `ok`.
    Complete,
    /// No plugin list and no MCP server list: H1a and H5 are `unk`.
    SilentOnPluginsAndMcp,
    /// A harness version off the pin: H9 is `gap` and advisory, and H11 is `unk` because no
    /// memory canary is recorded for that version (amendment a62).
    OffThePin,
    /// A plugin nobody declared and the binary does not compile in: H1a is `gap`.
    UndeclaredPlugin,
}

/// A scripted child whose runner says what it measured, or measures nothing at all.
struct Measured {
    lines: Vec<String>,
    measured: Option<ProcessTermination>,
}

struct MeasuredProcess(ScriptedProcess, Option<ProcessTermination>);

impl HarnessProcess for MeasuredProcess {
    fn next_line(&mut self) -> std::io::Result<Option<String>> {
        self.0.next_line()
    }
    fn poll_line(&mut self) -> std::io::Result<ProcessPoll> {
        self.0.poll_line()
    }
    fn write_line(&mut self, line: &str) -> std::io::Result<()> {
        self.0.write_line(line)
    }
    fn kill(&mut self) -> std::io::Result<()> {
        self.0.kill()
    }
    fn wait(&mut self) -> std::io::Result<Option<i32>> {
        self.0.wait()
    }
    fn termination(&self) -> ProcessTermination {
        self.1.unwrap_or_else(|| self.0.termination())
    }
    fn stderr(&self) -> String {
        self.0.stderr()
    }
}

impl ProcessRunner for Measured {
    fn requires_executable(&self) -> bool {
        false
    }
    fn start(&mut self, _: &LaunchPlanView<'_>) -> std::io::Result<Box<dyn HarnessProcess>> {
        Ok(Box::new(MeasuredProcess(
            ScriptedProcess::new(
                self.lines.iter().map(ScriptStep::line).collect(),
                ScriptedLog::new(),
            ),
            self.measured,
        )))
    }
}

/// One audited strict Claude run over a declared scratch copy: the report's last three lines and
/// the exit code the binary would return.
fn audited(opening: Opening, measured: Option<ProcessTermination>) -> (Vec<String>, i32) {
    let root = tempfile::tempdir().expect("a scratch directory");
    let copy = root.path().join("copy");
    fs::create_dir(&copy).expect("the copy is made");
    let canonical = fs::canonicalize(&copy).expect("the copy resolves");
    let pinned = metaharness::capabilities(Kind::Claude)
        .expect("claude has capabilities")
        .versions_pinned;
    let mut record = serde_json::json!({
        "emit": "session.started",
        "harness_version": pinned[0],
        "output_style": "default",
        "plugins": [],
        "mcp_servers": [],
        "credential_source": "none",
        "cwd": canonical.display().to_string(),
    });
    match opening {
        Opening::Complete => {}
        Opening::SilentOnPluginsAndMcp => {
            let fields = record.as_object_mut().expect("an object");
            fields.remove("plugins");
            fields.remove("mcp_servers");
        }
        Opening::OffThePin => record["harness_version"] = "0.0.0-off-the-pin".into(),
        Opening::UndeclaredPlugin => record["plugins"] = serde_json::json!(["an-ambient-plugin"]),
    }
    let lines = vec![
        record.to_string(),
        r#"{"emit":"session.ended","is_error":false,"subtype":"success"}"#.to_owned(),
    ];
    let mut run = Metaharness::new(Kind::Claude)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_cwd(&copy)
        .with_scratch_cwd()
        .start_with(
            Input::Prompt("a measured child".to_owned()),
            &mut Measured { lines, measured },
            &mut ScriptedSeams,
        )
        .expect("the run starts");
    run.drain().expect("the run drains");
    let report = run
        .audit(&mut FakeAuditor::missing())
        .expect("the floor always runs");
    let rendered = report.render();
    let all: Vec<&str> = rendered.lines().collect();
    let tail = all[all.len().saturating_sub(3)..]
        .iter()
        .map(|line| (*line).to_owned())
        .collect();
    (tail, run.exit(Some(&report)).code())
}

const CHILDREN: [(Option<ProcessTermination>, &str); 4] = [
    (
        Some(ProcessTermination::Exited { code: 0 }),
        "child exit: 0",
    ),
    (
        Some(ProcessTermination::Exited { code: 7 }),
        "child exit: 7",
    ),
    (
        Some(ProcessTermination::Signaled { signal: 9 }),
        "child exit: signal 9",
    ),
    (None, "child exit: unknown"),
];

const OPENINGS: [(Opening, &str, &str, i32); 4] = [
    (
        Opening::Complete,
        "control verdict: ok",
        "audit verdict: ok (exit 0)",
        0,
    ),
    (
        Opening::SilentOnPluginsAndMcp,
        "control verdict: unk",
        "audit verdict: no verdict (exit 3)",
        3,
    ),
    // story:off-pin-vendor-version-is-advisory: off the pin is no longer a gap (a62).
    (
        Opening::OffThePin,
        "control verdict: unk",
        "audit verdict: no verdict (exit 3)",
        3,
    ),
    (
        Opening::UndeclaredPlugin,
        "control verdict: gap",
        "audit verdict: gap (exit 1)",
        1,
    ),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

/// The page's § *Exit codes*, from its heading to the next second-level heading.
fn exit_codes_section() -> String {
    let page = fs::read_to_string(workspace_root().join(PAGE))
        .unwrap_or_else(|error| panic!("reading {PAGE}: {error}"));
    let start = page
        .find("\n## Exit codes\n")
        .unwrap_or_else(|| panic!("{PAGE} has no `## Exit codes` section"));
    let section = &page[start + 1..];
    let end = section[1..]
        .find("\n## ")
        .map_or(section.len(), |offset| offset + 1);
    section[..end].to_owned()
}

/// Every fenced `text` block in the section, its lines joined by newlines.
fn text_blocks(section: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut open: Option<Vec<&str>> = None;
    for line in section.lines() {
        let fence = line.trim();
        if let Some(block) = open.as_mut() {
            if fence == "```" {
                blocks.push(block.join("\n"));
                open = None;
            } else {
                block.push(line);
            }
        } else if fence == "```text" {
            open = Some(Vec::new());
        }
    }
    blocks
}

/// The table row whose first cell is the code.
fn row_for(section: &str, code: i32) -> String {
    let start = format!("| `{code}` |");
    section
        .lines()
        .find(|line| line.starts_with(&start))
        .unwrap_or_else(|| panic!("{PAGE}'s exit-code table has no row for `{code}`"))
        .to_owned()
}

/// A line with its measured number replaced by the placeholder the page uses for it.
fn shape(line: &str) -> String {
    match line.strip_prefix("child exit: ") {
        Some(rest) if rest.parse::<i32>().is_ok() => "child exit: <N>".to_owned(),
        Some(rest)
            if rest
                .strip_prefix("signal ")
                .is_some_and(|signal| signal.parse::<i32>().is_ok()) =>
        {
            "child exit: signal <N>".to_owned()
        }
        _ => line.to_owned(),
    }
}

/// The child's exit decides no verdict, and the exit code is the audit verdict's, whatever the
/// child did. An audit with an unknown gating row exits `3` beside a child that exited `0`.
#[test]
fn the_report_names_the_childs_exit_the_control_verdict_and_the_audit_verdict_apart() {
    for (opening, control, audit, code) in OPENINGS {
        for (measured, child) in CHILDREN {
            let (lines, exit) = audited(opening, measured);
            assert_eq!(
                lines,
                [child, control, audit],
                "story:off-pin-vendor-version-is-advisory: {opening:?}, {measured:?}"
            );
            assert_eq!(
                exit, code,
                "story:off-pin-vendor-version-is-advisory: {opening:?}, {measured:?}"
            );
        }
    }
}

/// The page shows the case it explains, byte for byte: a child that exited `0`, gating rows
/// nobody could read, and exit `3`. Its row for `3` says the child may have exited `0`.
#[test]
fn the_cli_reference_shows_what_a_child_that_exited_zero_beside_an_unknown_control_prints() {
    let (lines, exit) = audited(
        Opening::SilentOnPluginsAndMcp,
        Some(ProcessTermination::Exited { code: 0 }),
    );
    assert_eq!(exit, 3, "an audit with an unknown gating row exits nonzero");
    let section = exit_codes_section();
    let blocks = text_blocks(&section);
    assert!(
        blocks.contains(&lines.join("\n")),
        "{PAGE} § Exit codes shows no `text` block equal to the rendered lines:\n{}\n\nblocks: \
         {blocks:#?}",
        lines.join("\n")
    );
    let three = row_for(&section, 3);
    assert!(
        three.contains("`child exit: 0`"),
        "{PAGE}'s row for `3` does not say the child may have exited `0`: {three}"
    );
}

/// Every line shape the report prints is named on the page, and each audit verdict on the row of
/// the code it exits with.
#[test]
fn the_cli_reference_names_every_verdict_line_the_report_prints() {
    let section = exit_codes_section();
    let mut missing = Vec::new();
    for (opening, _, _, code) in OPENINGS {
        for (measured, _) in CHILDREN {
            let (lines, _) = audited(opening, measured);
            for line in &lines {
                let named = format!("`{}`", shape(line));
                if !section.contains(&named) {
                    missing.push(named);
                }
            }
            let audit = format!("`{}`", lines[2]);
            if !row_for(&section, code).contains(&audit) {
                missing.push(format!("{audit} on the row for `{code}`"));
            }
        }
    }
    missing.sort();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "{PAGE} § Exit codes does not name: {missing:#?}"
    );
}
