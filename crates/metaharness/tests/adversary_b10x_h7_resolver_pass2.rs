//! Adversary pass 2 against amendment a31 (`story:b10x-h7-from-the-loops-cwd`): the resolver
//! correction round 1 put behind H7 (`audit.rs` `resolve_without_the_reader`).
//!
//! `same_directory`'s own contract: two names are one directory when they are byte-equal, or when
//! both resolve "(every link followed, `.` and `..` removed, as `std::fs::canonicalize` does) to the
//! same path", and a name "that does not resolve (gone, or never on this machine), is compared as
//! spelled". Each case holds the production floor to `std::fs::canonicalize` (glibc `realpath`) as
//! that contract names it. Production b10x seam and floor, or the production builder over the bytes
//! `b10x-harness 0.15.1` wrote; no vendor process, no credential, no network.
#![cfg(target_os = "linux")]

use std::os::unix::ffi::OsStrExt as _;
use std::os::unix::fs::{PermissionsExt as _, symlink};

use metaharness::protocol::{
    CredentialSource, DecisionMode, HermeticAttestation, HermeticMode, HermeticRow, Kind, RunSpec,
    Seam, SeamFactory as _, TranscriptRef, Verdict,
};
use metaharness::{
    FakeAuditor, FloorInputs, Input, ManualClock, Metaharness, ScriptedLog, ScriptedRunner,
    hermetic_floor,
};
use serde_json::{Value, json};

const B10X_RECORDED_LOOP: &str = include_str!("../../metaharness-b10x/fixtures/golden/loop.jsonl");

/// H7 for a b10x opening record naming `observed`, against `planned`, through the production seam
/// and floor. No attestation: `ok` here means `same_directory` said yes, *which metaharness created*.
fn h7(observed: &str, planned: &str) -> (Verdict, String) {
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut seam = seams.build(
        TranscriptRef {
            path: None,
            digest: None,
            bytes: None,
        },
        HermeticAttestation::none(HermeticMode::Strict),
        Seam::None,
    );
    let line = json!({"kind":"started","model":"m","published_tools":[],"credential_source":"none","cwd":observed});
    let events: Vec<_> = seam
        .push_line(&line.to_string())
        .into_iter()
        .map(|emission| emission.event)
        .collect();
    let spec = RunSpec::new(Kind::B10x);
    let pins = vec![metaharness_b10x::PINNED_VERSIONS[0].to_owned()];
    let rows = hermetic_floor(
        &events,
        &FloorInputs {
            spec: &spec,
            pinned_versions: &pins,
            planned_cwd: Some(planned),
            declared_plugins: &[],
            builtin_plugins: &[],
            endpoint_key_posture: None,
            planned_mcp_servers: &[],
        },
    );
    let row = rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7 is evaluated");
    (row.verdict, row.detail.clone())
}

/// What the contract says `same_directory` answers: byte-equal, or both resolve (as
/// `std::fs::canonicalize` does) to one path. Neither name used here touches `/proc` or `/dev/fd`.
fn canonicalize_says_same(observed: &str, planned: &str) -> bool {
    observed == planned
        || matches!(
            (std::fs::canonicalize(observed), std::fs::canonicalize(planned)),
            (Ok(a), Ok(b)) if a == b
        )
}

/// **A name that passes through a file does not resolve**, and the floor reads it as the declared
/// copy. `<copy>/README.md/..` names no directory: `realpath` and the kernel refuse it with
/// `ENOTDIR`. The resolver records `README.md` as an ordinary component (it is not a link), then
/// applies `..` lexically, and arrives at `<copy>`. a31 and `same_directory` say a name that does
/// not resolve is compared as spelled, and the story says `declared` only "when it equals the
/// declared copy". Through the production builder, a strict b10x run over the 0.15.1 golden with
/// that `cwd`, under `--cwd <copy> --scratch-cwd`.
#[test]
fn a_name_through_a_file_does_not_resolve_and_is_not_the_declared_copy() {
    let root = tempfile::tempdir().expect("a scratch root");
    let copy = root.path().join("copy");
    std::fs::create_dir(&copy).expect("the embedder's copy");
    std::fs::write(copy.join("README.md"), "the copy's own file\n").expect("a file in the copy");
    let planned = std::fs::canonicalize(&copy).unwrap().display().to_string();
    let observed = format!("{planned}/README.md/..");
    let refused = std::fs::canonicalize(&observed).expect_err("a file has no `..`");
    assert_eq!(refused.raw_os_error(), Some(20), "ENOTDIR: {refused}");

    let lines: Vec<String> = B10X_RECORDED_LOOP
        .lines()
        .map(|line| match serde_json::from_str::<Value>(line) {
            Ok(mut value) if value["kind"] == "started" => {
                value["cwd"] = Value::String(observed.clone());
                value.to_string()
            }
            _ => line.to_owned(),
        })
        .collect();
    let mut runner = ScriptedRunner::of_lines(lines, ScriptedLog::new());
    let mut seams = metaharness_b10x::B10xSeams::new(
        Some(metaharness_b10x::PINNED_VERSIONS[0].to_owned()),
        None,
    );
    let mut run = Metaharness::new(Kind::B10x)
        .with_hermetic(HermeticMode::Strict)
        .with_credentials(CredentialSource::None)
        .with_decisions(DecisionMode::Observe)
        .with_model_endpoint("http://127.0.0.1:1")
        .with_model("b10x-emulated")
        .with_cwd(&copy)
        .with_scratch_cwd()
        .start_with_clock(
            Input::Prompt("fixture".to_owned()),
            &mut runner,
            &mut seams,
            Box::new(ManualClock::new()),
        )
        .expect("the run plans on b10x");
    run.drain().unwrap();
    let report = run
        .audit(&mut FakeAuditor::default())
        .expect("the floor always runs");
    let h7 = report
        .rows
        .iter()
        .find(|row| row.row == HermeticRow::H7)
        .expect("H7 is evaluated");
    assert_eq!(
        (h7.verdict, h7.detail.as_str()),
        (
            Verdict::Gap,
            format!("metaharness planned {planned} and the session ran in {observed}").as_str()
        ),
        "a name the kernel refuses (ENOTDIR) was read as the declared copy:\n{}",
        report.render()
    );
    assert_ne!(run.exit(Some(&report)).code(), 0, "{}", report.render());
}

/// The resolver agrees with `std::fs::canonicalize` on every name built from up to three
/// components over a tree of directories and links — absolute and relative targets, a link to `/`,
/// `..` after a link, a link to `.`, a loop, a dangling link, a chain, doubled slashes and trailing
/// slashes — against two spellings of the planned directory, except where `canonicalize` refuses
/// with `ENOTDIR` (a component that is a file; the case above). Plus: link chains at the 40-link
/// bound, a target whose name is not UTF-8, a directory nobody may search, and very long names.
#[test]
#[allow(clippy::too_many_lines)]
fn the_resolver_agrees_with_canonicalize_on_ordinary_names() {
    let scratch = tempfile::tempdir().expect("a scratch root");
    let t = std::fs::canonicalize(scratch.path()).unwrap();
    let ts = t.display().to_string();
    std::fs::create_dir_all(t.join("real/work/inner")).unwrap();
    std::fs::write(t.join("real/file"), "f").unwrap();
    symlink(t.join("real"), t.join("l_abs")).unwrap();
    symlink("real", t.join("l_rel")).unwrap();
    symlink("..", t.join("real/work/l_up")).unwrap();
    symlink("/", t.join("l_root")).unwrap();
    symlink("l_loop2", t.join("l_loop1")).unwrap();
    symlink("l_loop1", t.join("l_loop2")).unwrap();
    symlink("nonexistent", t.join("l_dangling")).unwrap();
    symlink("l_rel", t.join("l_chain")).unwrap();
    symlink(".", t.join("real/work/l_self")).unwrap();
    symlink("../work", t.join("real/work/l_back")).unwrap();
    symlink("real/file", t.join("l_file")).unwrap();
    symlink(
        "../../../../../../../../../../tmp/..",
        t.join("l_collapse_root"),
    )
    .unwrap();

    let segments = [
        "real",
        "work",
        "inner",
        "file",
        "l_abs",
        "l_rel",
        "l_up",
        "l_root",
        "l_loop1",
        "l_dangling",
        "l_chain",
        "l_self",
        "l_back",
        "l_file",
        "l_collapse_root",
        ".",
        "..",
        "nonexistent",
        "",
    ];
    let mut names = Vec::new();
    for a in segments {
        names.push(format!("{ts}/{a}"));
        for b in segments {
            names.push(format!("{ts}/{a}/{b}"));
            for c in segments {
                names.push(format!("{ts}/{a}/{b}/{c}"));
            }
        }
    }
    let with_trailing: Vec<String> = names.iter().map(|name| format!("{name}/")).collect();
    names.extend(with_trailing);

    let planned_spellings = [format!("{ts}/real/work"), format!("{ts}/l_rel/work")];
    let mut disagreements = Vec::new();
    let mut not_a_directory = 0;
    let mut compared = 0;
    let mut same = 0;
    for planned in &planned_spellings {
        for observed in &names {
            if std::fs::canonicalize(observed)
                .err()
                .and_then(|error| error.raw_os_error())
                == Some(20)
            {
                not_a_directory += 1;
                continue;
            }
            compared += 1;
            let expected = canonicalize_says_same(observed, planned);
            same += usize::from(expected);
            let (verdict, detail) = h7(observed, planned);
            if (verdict == Verdict::Ok) != expected {
                disagreements.push(format!(
                    "planned {planned}, recorded {observed}: canonicalize says same={expected}, \
                     floor {verdict:?} — {detail}"
                ));
            }
        }
    }
    assert!(
        not_a_directory > 0 && same > 100 && compared > 10_000,
        "the enumeration exercises both answers: {not_a_directory} ENOTDIR, {same} same of \
         {compared}"
    );

    // Link chains around the 40-link bound: `c<n>` is `n` links away from the planned directory.
    symlink(t.join("real/work"), t.join("c1")).unwrap();
    for n in 2..=45 {
        symlink(format!("c{}", n - 1), t.join(format!("c{n}"))).unwrap();
    }
    let planned = format!("{ts}/real/work");
    let mut extra: Vec<(String, String)> = (36..=45)
        .map(|n| (format!("{ts}/c{n}"), planned.clone()))
        .collect();

    // A target whose name is not UTF-8, reached through two UTF-8 spellings.
    let raw = t.join(std::ffi::OsStr::from_bytes(b"dir-\xff"));
    std::fs::create_dir(&raw).unwrap();
    symlink(std::ffi::OsStr::from_bytes(b"dir-\xff"), t.join("nu1")).unwrap();
    symlink(
        std::ffi::OsStr::from_bytes(b"./dir-\xff/../dir-\xff"),
        t.join("nu2"),
    )
    .unwrap();
    extra.push((format!("{ts}/nu1"), format!("{ts}/nu2")));

    // A directory nobody may search: what lies under it does not resolve, its parent does.
    std::fs::create_dir_all(t.join("locked/inner")).unwrap();
    std::fs::set_permissions(t.join("locked"), std::fs::Permissions::from_mode(0o000)).unwrap();
    extra.push((format!("{ts}/locked/inner"), format!("{ts}/locked/inner/.")));
    extra.push((format!("{ts}/locked/.."), ts.clone()));
    extra.push((format!("{ts}/locked/../real/work"), planned.clone()));

    // Very long names: one over PATH_MAX made only of `.`, and a long chain of `l_self` links.
    extra.push((format!("{planned}{}", "/.".repeat(2100)), planned.clone()));
    extra.push((
        format!("{planned}{}", "/l_self".repeat(30)),
        planned.clone(),
    ));
    extra.push((
        format!("{planned}{}", "/l_self".repeat(41)),
        planned.clone(),
    ));

    for (observed, planned) in &extra {
        let expected = canonicalize_says_same(observed, planned);
        let (verdict, detail) = h7(observed, planned);
        if (verdict == Verdict::Ok) != expected {
            let short = |s: &str| {
                if s.len() > 160 {
                    format!("{}…({} bytes)", &s[..160], s.len())
                } else {
                    s.to_owned()
                }
            };
            disagreements.push(format!(
                "planned {}, recorded {}: canonicalize says same={expected}, floor {verdict:?} — {}",
                short(planned),
                short(observed),
                short(&detail)
            ));
        }
    }
    std::fs::set_permissions(t.join("locked"), std::fs::Permissions::from_mode(0o755)).unwrap();

    assert!(
        disagreements.is_empty(),
        "{} disagreement(s) with std::fs::canonicalize:\n{}",
        disagreements.len(),
        disagreements.join("\n")
    );
}
