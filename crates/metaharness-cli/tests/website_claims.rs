//! Every count and every release behaviour the first-user pages state, checked against the source
//! it describes.
//!
//! A first user follows the published pages in order: build, install, run. A page that states a
//! number the code no longer has, a verb set the binary does not carry, or a gate step the
//! Taskfile does not run is a step the user cannot take. Each test below reads the page as
//! published — the page set is `metaharness-docs`' own allowlist, [`metaharness_docs::pages`] —
//! and holds it to the value it names: `conformance_vectors(kind).len()`, `Cli::command()`, the
//! adapters' capability descriptors, `Taskfile.yml`, `spec/ess-inputs.yaml` and `Cargo.toml`.
//! No process is spawned beyond what the free conformance vectors spawn, and no model is called.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use clap::{CommandFactory as _, Parser as _, ValueEnum as _};
use metaharness::protocol::{CommandSupport, DecisionMode, Kind, RefusalCode};
use metaharness_cli::{Cli, execute};

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the workspace root resolves")
}

fn read(relative: &str) -> String {
    let path = workspace_root().join(relative);
    fs::read_to_string(&path).unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
}

/// Every published page, by its route key.
fn published() -> BTreeMap<&'static str, &'static str> {
    metaharness_docs::pages().collect()
}

fn page(key: &str) -> &'static str {
    published()
        .get(key)
        .copied()
        .unwrap_or_else(|| panic!("{key} is not a published page"))
}

/// Every kind the binary accepts, from the type clap parses it with.
fn kinds() -> Vec<Kind> {
    Kind::value_variants().to_vec()
}

/// The lines of a section: from the heading line that equals `heading` up to the next heading of
/// the same or a higher level.
fn section<'a>(markdown: &'a str, heading: &str) -> Vec<&'a str> {
    let level = heading.chars().take_while(|c| *c == '#').count();
    let mut lines = markdown.lines();
    assert!(
        lines.by_ref().any(|line| line.trim_end() == heading),
        "no heading {heading:?}"
    );
    lines
        .take_while(|line| {
            let hashes = line.chars().take_while(|c| *c == '#').count();
            !(hashes > 0 && hashes <= level && line[hashes..].starts_with(' '))
        })
        .collect()
}

/// The first fenced code block in `lines`, without its fences.
fn first_block<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines[fence_end(lines)..]
        .iter()
        .take_while(|line| !line.starts_with("```"))
        .copied()
        .collect()
}

/// The index of the line after the first fence in `lines`.
fn fence_end(lines: &[&str]) -> usize {
    lines
        .iter()
        .position(|line| line.starts_with("```"))
        .expect("a fenced block")
        + 1
}

/// The cells of each row of the first table in `lines`, the header row first and the separator
/// row left out.
fn table_rows(lines: &[&str]) -> Vec<Vec<String>> {
    let start = lines
        .iter()
        .position(|line| line.trim().starts_with('|'))
        .unwrap_or(lines.len());
    lines[start..]
        .iter()
        .map(|line| line.trim())
        .take_while(|line| line.starts_with('|'))
        .filter(|line| !line.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')))
        .map(|line| {
            line.trim_matches('|')
                .split(" | ")
                .map(|cell| cell.trim().to_string())
                .collect()
        })
        .collect()
}

/// What a backticked cell says, without its backticks.
fn unquoted(cell: &str) -> &str {
    cell.trim().trim_matches('`')
}

/// A count written as digits or as a word from zero to twenty.
fn number(word: &str) -> Option<usize> {
    const WORDS: [&str; 21] = [
        "zero",
        "one",
        "two",
        "three",
        "four",
        "five",
        "six",
        "seven",
        "eight",
        "nine",
        "ten",
        "eleven",
        "twelve",
        "thirteen",
        "fourteen",
        "fifteen",
        "sixteen",
        "seventeen",
        "eighteen",
        "nineteen",
        "twenty",
    ];
    let word = word.to_ascii_lowercase();
    word.parse()
        .ok()
        .or_else(|| WORDS.iter().position(|w| *w == word))
}

/// Words with Markdown emphasis, code ticks and trailing punctuation taken off.
fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

/// A word that names what `conformance` counts: its vectors, or the `checked` of a `--contract`
/// run, which is the same number (`contract_result` in `crates/metaharness/src/vectors.rs`).
fn is_vector_noun(word: &str) -> bool {
    word.starts_with("vector") || word == "check" || word == "checks"
}

/// The number stated right before the first word `noun` accepts (within three words), if the
/// text states one.
fn count_before(text: &str, noun: impl Fn(&str) -> bool) -> Option<usize> {
    let words = words(text);
    let at = words.iter().position(|word| noun(word))?;
    words[at.saturating_sub(3)..at]
        .iter()
        .rev()
        .find_map(|word| number(word))
}

// ------------------------------------------------------------------ the page set

/// The page count the documentation check prints is `PAGES.len()`, so the allowlist has to be
/// every page under `website/docs/` and nothing else.
#[test]
fn every_page_under_website_docs_is_published_and_every_published_page_exists() {
    fn markdown_under(directory: &Path, root: &Path, found: &mut BTreeSet<String>) {
        for entry in fs::read_dir(directory).expect("a readable directory") {
            let path = entry.expect("an entry").path();
            if path.is_dir() {
                markdown_under(&path, root, found);
            } else if path.extension().is_some_and(|e| e == "md") {
                let key = path
                    .strip_prefix(root)
                    .expect("under the root")
                    .with_extension("");
                found.insert(key.to_string_lossy().into_owned());
            }
        }
    }
    let docs = workspace_root().join("website/docs");
    let mut on_disk = BTreeSet::new();
    markdown_under(&docs, &docs, &mut on_disk);
    let allowlisted: BTreeSet<String> = published().keys().map(ToString::to_string).collect();
    assert_eq!(allowlisted, on_disk);
    assert_eq!(metaharness_docs::pages().count(), on_disk.len());
}

// ------------------------------------------------------------------ conformance vector counts

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct VectorClaim {
    page: &'static str,
    line: usize,
    kind: &'static str,
    count: usize,
    /// "N of which run a real process", on the codex claims.
    real_process: Option<usize>,
}

fn vector_claims(key: &'static str, markdown: &str) -> Vec<VectorClaim> {
    let mut claims = Vec::new();
    for (index, line) in markdown.lines().enumerate() {
        for kind in kinds() {
            let needle = format!("conformance {}", kind.as_str());
            let mut rest = line;
            while let Some(at) = rest.find(&needle) {
                let after = &rest[at + needle.len()..];
                let end = after.find("conformance ").unwrap_or(after.len());
                let claim = &after[..end];
                if let Some(count) = count_before(claim, is_vector_noun) {
                    let real_process = claim
                        .find(" of which")
                        .and_then(|of| words(&claim[..of]).last().and_then(|word| number(word)));
                    claims.push(VectorClaim {
                        page: key,
                        line: index + 1,
                        kind: kind.as_str(),
                        count,
                        real_process,
                    });
                }
                rest = after;
            }
        }
    }
    claims
}

/// Every "N vectors" a published page states for `conformance <kind>` is the number that verb
/// runs, and codex's "N of which run a real process" is the number of its spawn vectors.
#[test]
fn every_vector_count_on_a_page_is_the_number_conformance_runs() {
    let runs: BTreeMap<&str, usize> = kinds()
        .into_iter()
        .map(|kind| {
            let vectors = metaharness::conformance_vectors(kind).expect("every kind has vectors");
            (kind.as_str(), vectors.len())
        })
        .collect();
    let codex_real_process = metaharness::codex_spawn_vectors().len();

    let mut found = BTreeSet::new();
    let mut wrong = Vec::new();
    for (key, markdown) in published() {
        for claim in vector_claims(key, markdown) {
            found.insert((claim.page, claim.kind));
            if claim.count != runs[claim.kind] {
                wrong.push(format!(
                    "{}.md:{} says `conformance {}` runs {}; it runs {}",
                    claim.page, claim.line, claim.kind, claim.count, runs[claim.kind]
                ));
            }
            if let Some(stated) = claim.real_process {
                if claim.kind != "codex" {
                    wrong.push(format!(
                        "{}.md:{} states a real-process share for `conformance {}`; only codex's is checked",
                        claim.page, claim.line, claim.kind
                    ));
                } else if stated != codex_real_process {
                    wrong.push(format!(
                        "{}.md:{} says {stated} codex vectors run a real process; {codex_real_process} do",
                        claim.page, claim.line
                    ));
                }
            }
        }
    }
    assert!(wrong.is_empty(), "{wrong:#?}");

    // The claims a first user reads. A rewording that the reader above cannot parse loses its
    // check, so the set of claims found is held to the set the pages are meant to carry.
    let expected: BTreeSet<(&str, &str)> = [
        ("quickstart", "claude"),
        ("quickstart", "codex"),
        ("quickstart", "b10x"),
        ("status", "claude"),
        ("status", "codex"),
        ("status", "b10x"),
        ("harnesses/claude", "claude"),
        ("harnesses/codex", "codex"),
        ("harnesses/b10x", "b10x"),
    ]
    .into_iter()
    .collect();
    assert_eq!(found, expected);
}

/// The class, not the instance: no published page states a number of vectors that the claim
/// reader above did not read and check.
#[test]
fn no_page_states_a_vector_count_the_check_does_not_read() {
    let mut unread = Vec::new();
    for (key, markdown) in published() {
        let checked: BTreeSet<(usize, usize)> = vector_claims(key, markdown)
            .into_iter()
            .map(|claim| (claim.line, claim.count))
            .collect();
        for (index, line) in markdown.lines().enumerate() {
            let words = words(line);
            for (at, word) in words.iter().enumerate() {
                if !is_vector_noun(word) {
                    continue;
                }
                let stated = words[at.saturating_sub(3)..at]
                    .iter()
                    .rev()
                    .find_map(|word| number(word));
                if let Some(stated) = stated
                    && !checked.contains(&(index + 1, stated))
                {
                    unread.push(format!("{key}.md:{}: {}", index + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        unread.is_empty(),
        "state a vector count as `conformance <kind>` … N vectors, or not at all: {unread:#?}"
    );
}

// ------------------------------------------------------------------ the verbs

/// `reference/cli.md` lists every verb the binary has and no other, and the number it states is
/// the number of rows (D9). `mcp-serve` is listed and says a client starts it.
#[test]
fn the_cli_reference_lists_exactly_the_verbs_the_binary_carries() {
    let binary: BTreeSet<String> = Cli::command()
        .get_subcommands()
        .map(|verb| verb.get_name().to_string())
        .collect();

    let markdown = page("reference/cli");
    let intro: Vec<&str> = markdown
        .lines()
        .take_while(|line| !line.starts_with("## "))
        .collect();
    let rows = table_rows(&intro);
    let listed: BTreeSet<String> = rows[1..]
        .iter()
        .map(|cells| {
            unquoted(&cells[0])
                .split_whitespace()
                .next()
                .expect("a verb")
                .to_string()
        })
        .collect();
    assert_eq!(listed, binary, "the verb table against `Cli::command()`");

    let stated = count_before(&intro.join("\n"), |word| word == "verbs")
        .expect("the page states a verb count");
    assert_eq!(
        stated,
        binary.len(),
        "the verb count against `Cli::command()`"
    );

    let mcp_serve = rows
        .iter()
        .find(|cells| unquoted(&cells[0]).starts_with("mcp-serve"))
        .expect("an mcp-serve row");
    assert!(
        mcp_serve[1].contains("client"),
        "the mcp-serve row says who starts it: {mcp_serve:?}"
    );
}

/// The crate's own exit-code table (`crates/metaharness-cli/src/lib.rs`, design § 9.4) and the
/// CLI reference's say the same thing for every code.
#[test]
fn the_cli_reference_exit_codes_mean_what_the_crate_says_they_mean() {
    // A sentence on the page and a table cell in the crate differ in a capital and a full stop and
    // in nothing else.
    fn sentence(meaning: &str) -> String {
        let meaning = meaning.trim().trim_end_matches('.');
        let mut chars = meaning.chars();
        let first = chars.next().expect("a meaning").to_lowercase();
        format!("{first}{}", chars.as_str())
    }

    let crate_doc: BTreeMap<String, String> = read("crates/metaharness-cli/src/lib.rs")
        .lines()
        .filter_map(|line| line.strip_prefix("//! | `"))
        .filter_map(|row| {
            let (code, rest) = row.split_once("` | ")?;
            Some((
                code.to_string(),
                rest.trim_end_matches(" |").trim().to_string(),
            ))
        })
        .collect();
    assert_eq!(crate_doc.len(), 4, "{crate_doc:#?}");

    let exit_codes = section(page("reference/cli"), "## Exit codes");
    let rows = table_rows(&exit_codes);
    let page_rows: BTreeMap<String, String> = rows[1..]
        .iter()
        .map(|cells| (unquoted(&cells[0]).to_string(), sentence(&cells[1])))
        .collect();
    let crate_doc: BTreeMap<String, String> = crate_doc
        .into_iter()
        .map(|(code, meaning)| (code, sentence(&meaning)))
        .collect();
    assert_eq!(page_rows, crate_doc);
}

// ------------------------------------------------------------------ build, install, gate

/// The quickstart installs the binary before any step calls it, by a command that installs the
/// crate whose binary is `metaharness`.
#[test]
fn the_quickstart_installs_the_binary_before_any_step_calls_it() {
    const INSTALL: &str = "cargo install --path crates/metaharness-cli --locked";
    let manifest = read("crates/metaharness-cli/Cargo.toml");
    let binary = manifest
        .split("[[bin]]")
        .nth(1)
        .and_then(|bin| bin.lines().find_map(|line| line.strip_prefix("name = ")))
        .expect("the crate declares a binary");
    assert_eq!(binary, "\"metaharness\"");

    let lines: Vec<&str> = page("quickstart").lines().map(str::trim).collect();
    let installed = lines
        .iter()
        .position(|line| *line == INSTALL)
        .expect("the quickstart carries the install command");
    let first_call = lines
        .iter()
        .position(|line| line.starts_with("metaharness ") || line.contains("=1 metaharness "))
        .expect("the quickstart calls the binary");
    assert!(
        installed < first_call,
        "quickstart line {} calls `metaharness` before line {} installs it",
        first_call + 1,
        installed + 1
    );
}

/// The quickstart's run that asks for the workspace's crates names the workspace: without
/// `--cwd` the child starts in an empty scratch directory with nothing to list.
#[test]
fn the_quickstart_run_that_lists_the_workspace_names_it() {
    let run = page("quickstart")
        .lines()
        .find(|line| line.starts_with("metaharness run") && line.contains("list the crates"))
        .expect("the listing run");
    assert!(run.contains(" --cwd "), "{run}");
}

/// The quickstart's gate block is the Taskfile's `check` task, command for command, and the
/// prerequisites it names are the versions the repository declares.
#[test]
fn the_quickstart_gate_is_the_taskfile_check_task_and_its_prerequisites() {
    let taskfile = read("Taskfile.yml");
    let mut lines = taskfile.lines();
    assert!(lines.by_ref().any(|line| line == "  check:"));
    assert!(lines.by_ref().any(|line| line == "    cmds:"));
    let steps: Vec<&str> = lines
        .map_while(|line| line.strip_prefix("      - "))
        .collect();
    assert_ne!(steps, Vec::<&str>::new(), "Taskfile.yml check has no cmds");

    let gate = section(page("quickstart"), "## The gate");
    assert_eq!(first_block(&gate), ["task check"]);
    let opened = &gate[fence_end(&gate)..];
    let after_first = &opened[fence_end(opened)..];
    assert_eq!(first_block(after_first), steps);

    let ess = read("spec/ess-inputs.yaml");
    let ess = ess
        .lines()
        .find_map(|line| line.strip_prefix("requires: ess "))
        .expect("the specification names its ESS");
    let rust = read("Cargo.toml");
    let rust = rust
        .lines()
        .find_map(|line| line.strip_prefix("rust-version = "))
        .expect("the workspace declares its rust-version")
        .trim_matches('"')
        .to_string();
    let text = gate.join(" ");
    for named in [
        format!("ESS {ess}"),
        format!("Rust {rust} or newer"),
        "go-task".to_string(),
    ] {
        assert!(text.contains(&named), "the gate section names {named}");
    }
}

/// A paragraph of a published page that names `task check` and a step it runs is a second
/// statement of the gate; outside the quickstart's gate section none may exist. Lines starting
/// with `task check` are read by `no_page_summarises_the_gate_beside_task_check`.
#[test]
fn no_page_summarises_the_gate_in_prose() {
    let taskfile = read("Taskfile.yml");
    let mut lines = taskfile.lines();
    assert!(lines.by_ref().any(|line| line == "    cmds:"));
    let programs: BTreeSet<String> = lines
        .map_while(|line| line.strip_prefix("      - "))
        .map(|step| {
            step.split_whitespace()
                .take(2)
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect();
    let mut summaries = Vec::new();
    for (key, markdown) in published() {
        let gate = if key == "quickstart" {
            section(markdown, "## The gate").join("\n")
        } else {
            String::new()
        };
        let prose = markdown.replace(&gate, "");
        for paragraph in prose.split("\n\n") {
            let flat = paragraph.split_whitespace().collect::<Vec<_>>().join(" ");
            if !flat.contains("task check") {
                continue;
            }
            for program in &programs {
                if flat.contains(program.as_str()) {
                    summaries.push(format!("{key}.md: names `{program}` beside `task check`"));
                }
            }
        }
    }
    assert!(summaries.is_empty(), "{summaries:#?}");
}

/// The site's own README states the number of documentation pages the builder publishes.
#[test]
fn the_website_readme_counts_the_pages_the_builder_publishes() {
    let readme = read("website/README.md");
    let flat = readme.split_whitespace().collect::<Vec<_>>().join(" ");
    let at = flat
        .find(" documentation pages")
        .expect("website/README.md names its documentation pages");
    let stated = flat[..at]
        .rsplit(' ')
        .next()
        .and_then(number)
        .expect("website/README.md states a page count");
    assert_eq!(
        stated,
        metaharness_docs::pages().count(),
        "website/README.md"
    );
}

/// A page that summarises the gate beside `task check` drifts the first time a step moves; the
/// steps are stated once, as the Taskfile's, on the quickstart.
#[test]
fn no_page_summarises_the_gate_beside_task_check() {
    let mut summaries = Vec::new();
    for (key, markdown) in published() {
        for (index, line) in markdown.lines().enumerate() {
            if line.starts_with("task check") && line.trim_end() != "task check" {
                summaries.push(format!("{key}.md:{}: {line}", index + 1));
            }
        }
    }
    assert!(summaries.is_empty(), "{summaries:#?}");
}

// ------------------------------------------------------------------ release behaviour

/// The control seam's mode table is each adapter's declared decision mode, which is also what
/// plans and what is refused (design § 8.4 O4).
#[test]
fn the_control_seam_mode_table_is_each_adapters_declared_mode() {
    let lines = section(
        page("control-seam"),
        "### Which adapter delivers which mode",
    );
    let rows = table_rows(&lines);
    let header: Vec<&str> = rows[0][1..].iter().map(|cell| unquoted(cell)).collect();
    let page_kinds: BTreeSet<&str> = header.iter().copied().collect();
    let all: BTreeSet<&str> = kinds().iter().map(Kind::as_str).collect();
    assert_eq!(page_kinds, all);

    let mut modes = BTreeSet::new();
    for cells in &rows[1..] {
        let mode = DecisionMode::ALL
            .into_iter()
            .find(|mode| mode.as_str() == unquoted(&cells[0]))
            .unwrap_or_else(|| panic!("{} is not a decision mode", cells[0]));
        modes.insert(mode.as_str());
        for (kind, cell) in header.iter().zip(&cells[1..]) {
            let kind = kinds()
                .into_iter()
                .find(|k| k.as_str() == *kind)
                .expect("a kind");
            let declared = metaharness::capabilities(kind)
                .expect("every kind has an adapter")
                .decision_mode(mode);
            let declared = serde_json::to_value(declared).expect("serializes");
            assert_eq!(
                unquoted(cell),
                declared.as_str().expect("a word"),
                "`--decisions {}` on {}",
                mode.as_str(),
                kind.as_str()
            );
        }
    }
    let every: BTreeSet<&str> = DecisionMode::ALL.iter().map(DecisionMode::as_str).collect();
    assert_eq!(modes, every);
}

/// The frame page says what the release does with a frame after launch (D10): `frame.set` is
/// refused, on every adapter.
#[test]
fn the_frame_page_says_frame_set_is_refused_and_every_adapter_refuses_it() {
    for kind in kinds() {
        let support = metaharness::capabilities(kind)
            .expect("every kind has an adapter")
            .support("frame.set");
        assert!(
            matches!(support, CommandSupport::Refused(_)),
            "{} honours frame.set; the frame page says no adapter does",
            kind.as_str()
        );
    }
    let text = page("frames")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        text.contains("`frame.set` is refused on every adapter"),
        "frames.md does not say frame.set is refused on every adapter"
    );
    assert!(
        !text.contains("Presented every turn"),
        "frames.md still says a frame is presented every turn"
    );
    let malformed = serde_json::to_value(RefusalCode::Malformed).expect("serialises");
    let malformed = malformed.as_str().expect("a word");
    assert!(
        text.contains(&format!("`frame.set` is refused `{malformed}` before")),
        "frames.md does not say a malformed frame.set is refused {malformed} first"
    );
}

/// The status page lists as refused exactly the verbs that refuse: `audit` exits 2 naming what it
/// waits for, and `project` writes its document and exits 0.
#[test]
fn the_status_page_refuses_only_what_the_binary_refuses() {
    let scratch = tempfile::tempdir().expect("a scratch directory");
    let events = workspace_root().join("evals/aep/runs/decomposer-clean.events.jsonl");
    let out = scratch.path().join("run.trace-ir.json");
    let argv: BTreeMap<&str, Vec<String>> = BTreeMap::from([
        (
            "project",
            vec![
                "metaharness".into(),
                "project".into(),
                events.display().to_string(),
                "--out".into(),
                out.display().to_string(),
            ],
        ),
        (
            "audit",
            vec![
                "metaharness".into(),
                "audit".into(),
                "--transcript".into(),
                "t.jsonl".into(),
            ],
        ),
    ]);
    let exit: BTreeMap<&str, i32> = argv
        .iter()
        .map(|(verb, argv)| {
            (
                *verb,
                execute(Cli::try_parse_from(argv).expect("the command line parses")),
            )
        })
        .collect();
    assert_eq!(exit["project"], 0);
    assert_eq!(exit["audit"], 2);

    let refused = section(page("status"), "## Not built — and refusing by name");
    let verbs: BTreeSet<String> = table_rows(&refused)[1..]
        .iter()
        .filter_map(|cells| {
            let quoted = cells[0].split('`').nth(1)?;
            quoted
                .strip_prefix("metaharness ")
                .map(|verb| verb.split_whitespace().next().unwrap_or(verb).to_string())
        })
        .collect();
    let refusing: BTreeSet<String> = exit
        .iter()
        .filter(|(_, code)| **code == 2)
        .map(|(verb, _)| (*verb).to_string())
        .collect();
    assert_eq!(verbs, refusing);
}
