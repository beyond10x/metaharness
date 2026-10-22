//! Adversary cases for the first-user documents (wave 8, unit 5).
//!
//! Each case reads a published page or the design document and holds one claim to the value the
//! binary or the crate has. The first two are red at the unit's tree: a page states an exit the
//! run does not owe, and the design's CLI section names fewer verbs than the binary carries. The
//! rest are green and fail under a page edit that `website_claims.rs` lets through: a count
//! wrapped across two lines, a count above twenty written as a word, the status page's contract
//! count, and the refusal code the frame page names.
//!
//! No process is spawned beyond what the free conformance vectors spawn, and no model is called.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

use clap::{CommandFactory as _, Parser as _, ValueEnum as _};
use metaharness::protocol::{CommandSupport, HermeticMode, Kind};
use metaharness_cli::{Cli, Verb};

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

fn published() -> BTreeMap<&'static str, &'static str> {
    metaharness_docs::pages().collect()
}

fn page(key: &str) -> &'static str {
    published()
        .get(key)
        .copied()
        .unwrap_or_else(|| panic!("{key} is not a published page"))
}

fn kinds() -> Vec<Kind> {
    Kind::value_variants().to_vec()
}

fn vector_count(kind: Kind) -> usize {
    metaharness::conformance_vectors(kind)
        .expect("every kind has vectors")
        .len()
}

/// A command line as a shell would split it: whitespace outside double quotes separates words,
/// and the quotes themselves are dropped. `$PWD` stands for a directory and becomes `.`.
fn shell_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quoted = false;
    let mut started = false;
    for c in line.chars() {
        match c {
            '"' => {
                quoted = !quoted;
                started = true;
            }
            c if c.is_whitespace() && !quoted => {
                if started {
                    words.push(std::mem::take(&mut current));
                    started = false;
                }
            }
            c => {
                current.push(c);
                started = true;
            }
        }
    }
    if started {
        words.push(current);
    }
    words
        .into_iter()
        .map(|word| word.replace("$PWD", "."))
        .collect()
}

/// The `metaharness run …` command a line carries: inside a backtick span, or the whole line when
/// it is a line of a fenced block.
fn run_command(line: &str) -> Option<String> {
    let trimmed = line.trim();
    if trimmed.starts_with("metaharness run ") {
        return Some(trimmed.to_string());
    }
    line.split('`')
        .skip(1)
        .step_by(2)
        .find(|span| span.starts_with("metaharness run "))
        .map(ToString::to_string)
}

/// A line that says the process exits on the hermetic floor's verdict, links unwrapped.
fn claims_a_floor_exit(line: &str) -> bool {
    let mut text = String::new();
    let mut rest = line;
    while let Some(open) = rest.find("](") {
        text.push_str(&rest[..open]);
        rest = rest[open + 2..]
            .find(')')
            .map_or("", |close| &rest[open + 2 + close + 1..]);
    }
    text.push_str(rest);
    let text = text.replace('[', "").to_ascii_lowercase();
    text.contains("exits on the") && text.contains("floor") && text.contains("verdict")
}

// ------------------------------------------------------------------ red at the unit's tree

/// A page that says a run exits on the hermetic floor's verdict names a command that owes one.
///
/// The CLI evaluates the floor only when the run asks for an audit: `--audit`, or `--hermetic
/// strict` (`spec_wants_audit`, `crates/metaharness/src/audit.rs`). A bare `--hermetic` run
/// returns `run.exit(None)` (`verdict` in `crates/metaharness-cli/src/lib.rs`), which reads the
/// stream's completeness and no floor row, and `hermetic.md` says so: "bare `--hermetic` without
/// `--audit` owes no verdict and exits 0".
#[test]
fn a_page_that_says_a_run_exits_on_the_floor_names_a_run_that_owes_one() {
    let mut found = 0;
    let mut wrong = Vec::new();
    for (key, markdown) in published() {
        let lines: Vec<&str> = markdown.lines().collect();
        for (index, line) in lines.iter().enumerate() {
            if !claims_a_floor_exit(line) {
                continue;
            }
            let Some(command) = lines[..=index].iter().rev().find_map(|l| run_command(l)) else {
                wrong.push(format!(
                    "{key}.md:{}: a floor exit with no run command before it",
                    index + 1
                ));
                continue;
            };
            found += 1;
            let mut argv = vec!["metaharness".to_string()];
            argv.extend(shell_words(&command).into_iter().skip(1));
            let cli = Cli::try_parse_from(&argv)
                .unwrap_or_else(|error| panic!("{key}.md: `{command}` does not parse: {error}"));
            let Verb::Run(run) = cli.command else {
                panic!("{key}.md: `{command}` is not a run");
            };
            let owes = run.spec.audit || run.spec.hermetic == HermeticMode::Strict;
            if !owes {
                wrong.push(format!(
                    "{key}.md:{}: says the process exits on the floor's verdict, but `{command}` \
                     asks for no audit (hermetic {:?}, audit {}), so no floor is evaluated",
                    index + 1,
                    run.spec.hermetic,
                    run.spec.audit
                ));
            }
        }
    }
    assert!(
        found > 0,
        "no page states a floor exit; the reader found nothing"
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The design's CLI section (§ 9.2) names every verb the binary carries.
///
/// `tests/anti_drift.rs` holds the surface to eight verbs under the name
/// `every_verb_the_design_names_is_on_the_surface`, and the changelog says the eight are the ones
/// "the design names"; invariant 7 makes the design document the source the site states. The
/// published CLI reference lists eight.
#[test]
fn the_design_cli_section_names_every_verb_the_binary_carries() {
    let design = read("docs/design/metaharness-protocol-v0.1.md");
    let mut lines = design.lines();
    assert!(
        lines.by_ref().any(|line| line.trim_end() == "### 9.2 CLI"),
        "the design has no § 9.2 CLI"
    );
    assert!(
        lines.by_ref().any(|line| line.starts_with("```")),
        "§ 9.2 has no fenced block"
    );
    let block: Vec<&str> = lines.take_while(|line| !line.starts_with("```")).collect();
    let named: BTreeSet<String> = block
        .iter()
        .filter_map(|line| line.strip_prefix("metaharness "))
        .filter_map(|rest| rest.split_whitespace().next())
        .map(ToString::to_string)
        .collect();
    let carried: BTreeSet<String> = Cli::command()
        .get_subcommands()
        .map(|verb| verb.get_name().to_string())
        .collect();
    let missing: Vec<&String> = carried.difference(&named).collect();
    let extra: Vec<&String> = named.difference(&carried).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "design § 9.2 names {named:?}; the binary carries {carried:?}; \
         missing from the design: {missing:?}; not on the binary: {extra:?}"
    );
}

// ------------------------------------------------------------------ green; catch page edits

/// A count written as digits, or as an English number word up to ninety-nine
/// (`twenty-four`, `thirty`).
fn number(word: &str) -> Option<usize> {
    const UNITS: [&str; 20] = [
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
    ];
    const TENS: [&str; 8] = [
        "twenty", "thirty", "forty", "fifty", "sixty", "seventy", "eighty", "ninety",
    ];
    let word = word.to_ascii_lowercase();
    if let Ok(n) = word.parse() {
        return Some(n);
    }
    if let Some(n) = UNITS.iter().position(|w| *w == word) {
        return Some(n);
    }
    let (tens, unit) = word.split_once('-').unwrap_or((word.as_str(), ""));
    let tens = TENS.iter().position(|w| *w == tens)? * 10 + 20;
    if unit.is_empty() {
        return Some(tens);
    }
    let unit = UNITS[1..10].iter().position(|w| *w == unit)? + 1;
    Some(tens + unit)
}

fn words(text: &str) -> Vec<String> {
    text.split_whitespace()
        .map(|word| {
            word.trim_matches(|c: char| !c.is_ascii_alphanumeric() && c != '-')
                .to_string()
        })
        .filter(|word| !word.is_empty())
        .collect()
}

fn is_vector_noun(word: &str) -> bool {
    word.starts_with("vector") || word == "check" || word == "checks"
}

/// The prose of a page as paragraphs, each joined onto one line, fenced blocks kept line by line
/// (a command and its comment are one claim; two lines of a block are not).
fn paragraphs(markdown: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fenced = false;
    for line in markdown.lines() {
        if line.trim_start().starts_with("```") {
            if !current.is_empty() {
                out.push(current.join(" "));
                current.clear();
            }
            fenced = !fenced;
            continue;
        }
        if fenced {
            out.push(line.to_string());
        } else if line.trim().is_empty() || line.trim_start().starts_with('|') {
            if !current.is_empty() {
                out.push(current.join(" "));
                current.clear();
            }
            if !line.trim().is_empty() {
                out.push(line.to_string());
            }
        } else {
            current.push(line);
        }
    }
    if !current.is_empty() {
        out.push(current.join(" "));
    }
    out
}

/// Every number a paragraph states right before a vector noun (within three words), with the
/// kind whose `conformance <kind>` most recently precedes it in the paragraph.
fn stated_counts(paragraph: &str) -> Vec<(Option<&'static str>, usize, String)> {
    let words = words(paragraph);
    let mut kind: Option<&'static str> = None;
    let mut out = Vec::new();
    for (at, word) in words.iter().enumerate() {
        if word == "conformance"
            && let Some(next) = words.get(at + 1)
            && let Some(k) = kinds().into_iter().find(|k| k.as_str() == next)
        {
            kind = Some(k.as_str());
        }
        if is_vector_noun(word)
            && let Some(n) = words[at.saturating_sub(3)..at]
                .iter()
                .rev()
                .find_map(|w| number(w))
        {
            out.push((kind, n, words[at.saturating_sub(3)..=at].join(" ")));
        }
    }
    out
}

/// Every vector count a page states, read paragraph by paragraph so a count wrapped onto the
/// next line is read, is the number `conformance <kind>` runs; a count with no `conformance
/// <kind>` before it in its paragraph is a count nothing checks.
#[test]
fn every_vector_count_read_across_wrapped_lines_is_the_number_conformance_runs() {
    let runs: BTreeMap<&str, usize> = kinds()
        .into_iter()
        .map(|kind| (kind.as_str(), vector_count(kind)))
        .collect();
    let mut wrong = Vec::new();
    let mut read = 0;
    for (key, markdown) in published() {
        for paragraph in paragraphs(markdown) {
            for (kind, stated, context) in stated_counts(&paragraph) {
                read += 1;
                match kind {
                    Some(kind) if stated == runs[kind] => {}
                    Some(kind) => wrong.push(format!(
                        "{key}.md: \"{context}\" for `conformance {kind}` says {stated}; it runs {}",
                        runs[kind]
                    )),
                    None => wrong.push(format!(
                        "{key}.md: \"{context}\" states a vector count with no `conformance <kind>` \
                         before it"
                    )),
                }
            }
        }
    }
    assert!(
        read >= 9,
        "the reader found {read} vector counts; the pages carry at least 9"
    );
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The status page's `(`checked: N`)` for b10x's contract is the number `conformance b10x
/// --contract` checks, which is the number of its vectors (`contract_result`,
/// `crates/metaharness/src/vectors.rs`).
#[test]
fn the_status_page_contract_count_is_the_number_conformance_checks() {
    let mut read = 0;
    let mut wrong = Vec::new();
    for (index, line) in page("status").lines().enumerate() {
        let mut rest = line;
        let mut consumed = 0;
        while let Some(at) = rest.find("`checked: ") {
            let value = &rest[at + "`checked: ".len()..];
            let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
            let start = consumed + at;
            consumed = start + "`checked: ".len();
            rest = &line[consumed..];
            if digits.is_empty() || !value[digits.len()..].starts_with('`') {
                continue;
            }
            let before = &line[..start];
            let kind = kinds()
                .into_iter()
                .filter_map(|kind| before.rfind(kind.as_str()).map(|at| (at, kind)))
                .max_by_key(|(at, _)| *at)
                .map(|(_, kind)| kind);
            let Some(kind) = kind else {
                wrong.push(format!(
                    "status.md:{}: `checked: {digits}` names no adapter before it",
                    index + 1
                ));
                continue;
            };
            read += 1;
            let stated: usize = digits.parse().expect("digits");
            if stated != vector_count(kind) {
                wrong.push(format!(
                    "status.md:{}: `checked: {stated}` for {}; `conformance {} --contract` checks {}",
                    index + 1,
                    kind.as_str(),
                    kind.as_str(),
                    vector_count(kind)
                ));
            }
        }
    }
    assert!(read > 0, "the status page states no `checked: N`");
    assert!(wrong.is_empty(), "{wrong:#?}");
}

/// The refusal code the frame page names for `frame.set` is the code each adapter refuses it
/// with, in its wire spelling.
#[test]
fn the_frame_page_names_the_code_each_adapter_refuses_frame_set_with() {
    let text = page("frames")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    let marker = "`frame.set` is refused on every adapter with `";
    let at = text
        .find(marker)
        .expect("frames.md names the code frame.set is refused with");
    let named = text[at + marker.len()..]
        .split('`')
        .next()
        .expect("a closing backtick");
    for kind in kinds() {
        let support = metaharness::capabilities(kind)
            .expect("every kind has an adapter")
            .support("frame.set");
        let CommandSupport::Refused(code) = support else {
            panic!("{} honours frame.set", kind.as_str());
        };
        let wire = serde_json::to_value(code).expect("a refusal code serialises");
        assert_eq!(
            wire.as_str(),
            Some(named),
            "frames.md names {named}; {} refuses frame.set with {wire}",
            kind.as_str()
        );
    }
}
