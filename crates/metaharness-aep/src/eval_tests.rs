#[cfg(test)]
mod tests {
use aep_cli::eval::{RunManifest, Refusal, RawRunManifest, Record, RawRecord, Session, DIGEST_WIDTH, Outcome, micro_usd_stated, cost_of, MarketplacePlugin, manifest_text};
use super::*;
/// A manifest that passes every rule, as a starting point for one-line mutations.
    const HONEST: &str = "\
format: eval.run-manifest/1
arm: plugin
harness: claude
workflow: adp/default
case: case:create-a-story
plugin_digest: 7258e0b6ac95f748bf5304b12b9c8c29d479ae4b812ee5b98640a8ab7f090332
model: claude-sonnet-5
harness_version: claude 2.1.239
transcript_digest: 6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b
observed_at: 2026-08-23
";
/// Reads a manifest, returning the refusals rather than a message.
    fn read(text: &str) -> Result<RunManifest, Vec<Refusal>> {
        let raw: RawRunManifest = serde_yaml::from_str(text).expect("the fixture is YAML");
        RunManifest::try_from(raw)
    }
/// The codes a refusal set carries, which is what a test matches on.
    fn codes(refusals: &[Refusal]) -> Vec<&'static str> {
        refusals.iter().map(Refusal::code).collect()
    }
#[test]
    fn an_omitted_model_is_refused_and_an_explicit_null_is_not() {
        // The rule `plugin_digest` already had, extended to `model` by a live run. Codex's wire
        // names no model at session start, so *the harness did not say* is a fact a manifest must
        // be able to state — and *nobody wrote the key* must still be refused, because a runner
        // that dropped it would produce the same document.
        let omitted = HONEST
            .lines()
            .filter(|line| !line.starts_with("model:"))
            .collect::<Vec<_>>()
            .join("\n");
        let refusals = read(&omitted).expect_err("the key must be written");
        assert_eq!(codes(&refusals), ["EVAL-MANIFEST-003"]);

        let unstated = read(&HONEST.replace("model: claude-sonnet-5", "model: null"))
            .expect("a wire that states no model is stating something");
        assert_eq!(unstated.model, None);
    }
/// A record with the verdicts given, in the shape `protocol observe trace check --format json` writes.
    fn record_of(verdicts: &[(&str, &str)]) -> String {
        let rows: Vec<String> = verdicts
            .iter()
            .map(|(id, verdict)| {
                format!(r#"{{"id":"{id}","kind":"tool.called","verdict":{verdict}}}"#)
            })
            .collect();
        format!(
            r#"{{"format":"trace-report/1","spec_id":"eval/development-story",
                 "spec_digest":"fd6bcd8ab28806f92f487276ffa60d21f51ebc0576b2027d9d279e3685e38466",
                 "transcript_digest":"6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b",
                 "expectations":[{}]}}"#,
            rows.join(",")
        )
    }
/// Reads a record, returning the refusals rather than a message.
    fn read_record(text: &str) -> Result<Record, Vec<Refusal>> {
        let raw: RawRecord = serde_json::from_str(text).expect("the fixture is JSON");
        Record::try_from(raw)
    }
/// One complete record as a mutable JSON object.
    fn record_value_json() -> serde_json::Value {
        serde_json::from_str(&record_of(&[("held", "\"ok\"")])).expect("record fixture is JSON")
    }
#[test]
    fn each_required_record_key_is_refused_when_omitted() {
        for field in [
            "spec_id",
            "spec_digest",
            "transcript_digest",
            "expectations",
        ] {
            let mut document = record_value_json();
            document
                .as_object_mut()
                .expect("a record object")
                .remove(field);
            let refusals = read_record(&document.to_string()).expect_err("the key is required");
            assert_eq!(codes(&refusals), ["EVAL-RECORD-004"], "{field}");
            assert!(refusals[0].to_string().contains(field), "{field}");
        }
    }
#[test]
    fn required_record_keys_distinguish_null_empty_and_malformed() {
        for field in [
            "spec_id",
            "spec_digest",
            "transcript_digest",
            "expectations",
        ] {
            let mut document = record_value_json();
            document[field] = serde_json::Value::Null;
            let refusals = read_record(&document.to_string()).expect_err("null is not an identity");
            assert_eq!(codes(&refusals), ["EVAL-RECORD-005"], "{field}");
        }

        let mut empty = record_value_json();
        empty["spec_id"] = serde_json::json!("  ");
        empty["spec_digest"] = serde_json::json!("");
        empty["transcript_digest"] = serde_json::json!("");
        empty["expectations"] = serde_json::json!([]);
        let refusals = read_record(&empty.to_string()).expect_err("empty fields bind nothing");
        assert_eq!(
            codes(&refusals),
            [
                "EVAL-RECORD-006",
                "EVAL-RECORD-006",
                "EVAL-RECORD-006",
                "EVAL-RECORD-006",
            ]
        );

        let mut malformed = record_value_json();
        malformed["spec_id"] = serde_json::json!(42);
        malformed["spec_digest"] = serde_json::json!("sha256:short");
        malformed["transcript_digest"] = serde_json::json!("ABCDEF");
        malformed["expectations"] = serde_json::json!({"id": "not-an-array"});
        let refusals =
            read_record(&malformed.to_string()).expect_err("malformed fields cannot be joined");
        assert_eq!(
            codes(&refusals),
            [
                "EVAL-RECORD-007",
                "EVAL-RECORD-007",
                "EVAL-RECORD-007",
                "EVAL-RECORD-007",
            ]
        );
    }
#[test]
    fn the_three_verdicts_map_onto_the_three_columns() {
        let record = read_record(&record_of(&[
            ("held", "\"ok\""),
            ("violated", "\"gap\""),
            ("unobservable", "\"unknown\""),
        ]))
        .expect("the shape the checker writes");
        assert_eq!(
            record.rows,
            vec![
                ("held".to_owned(), Outcome::Held),
                ("violated".to_owned(), Outcome::Violated),
                ("unobservable".to_owned(), Outcome::Unobservable),
            ]
        );
    }
#[test]
    fn a_row_whose_verdict_is_null_is_unobservable_and_never_held() {
        // The polarity of the whole verb, and the mutation that breaks it is one character in
        // `outcome_of`: `None => Ok(Outcome::Held)`. Both spellings of *nothing was recorded* are
        // asserted, because a checker that dropped the key and one that wrote `null` produce
        // documents that must be read the same way.
        for record in [
            record_of(&[("silent", "null")]),
            r#"{"format":"trace-report/1","spec_id":"s",
                 "spec_digest":"fd6bcd8ab28806f92f487276ffa60d21f51ebc0576b2027d9d279e3685e38466",
                 "transcript_digest":"6522e1ebe318da1e0a604e595ecc9afed1d1041c6e418a1382e4f1600a17640b",
                 "expectations":[{"id":"silent"}]}"#.to_owned(),
        ] {
            let read = read_record(&record).expect("a silent row is read, not refused");
            assert_eq!(
                read.rows,
                vec![("silent".to_owned(), Outcome::Unobservable)],
                "a row the checker recorded no verdict for is unobservable, never held"
            );
        }
    }
#[test]
    fn a_verdict_word_this_build_cannot_read_is_refused_rather_than_bucketed() {
        let refusals = read_record(&record_of(&[("new", "\"probably\"")]))
            .expect_err("an unreadable word is not a third answer");
        assert_eq!(codes(&refusals), ["EVAL-RECORD-003"]);
    }
#[test]
    fn a_record_of_another_shape_is_refused_by_the_format_it_states() {
        let refusals =
            read_record(&record_of(&[("held", "\"ok\"")]).replace("trace-report/1", "trace-ir/1"))
                .expect_err("the matrix reads a check report");
        assert_eq!(codes(&refusals), ["EVAL-RECORD-001"]);
        assert!(
            refusals[0].to_string().contains("trace-ir/1"),
            "named: {}",
            refusals[0]
        );
    }
#[test]
    fn a_cost_a_harness_computed_in_floating_point_is_read_and_not_refused() {
        // **The live defect.** A Claude run stated `0.7977854999999999` — the shortest text that
        // round-trips the `f64` sum of its per-turn costs — and the strict reader refused it for
        // having seventeen significant figures. Eighty cents then entered the ledger as the
        // assumed twenty-five and the manifest as no cost at all.
        assert_eq!(micro_usd_stated("0.7977854999999999"), Ok(797_785));
        assert_eq!(
            cost_of(&serde_json::json!({ "total_cost_usd": 0.797_785_499_999_999_9 })),
            Ok(Some(797_785))
        );
        // Half-up on everything past the sixth place, so both sides of the boundary are pinned
        // rather than whichever one the first fixture happened to have.
        assert_eq!(micro_usd_stated("0.1234564999"), Ok(123_456));
        assert_eq!(micro_usd_stated("0.1234565"), Ok(123_457));
        // And the carry is ordinary integer addition, so it crosses into the dollar.
        assert_eq!(micro_usd_stated("0.9999995"), Ok(1_000_000));
        // Six places or fewer still read exactly, which is every committed fixture.
        assert_eq!(micro_usd_stated("0.5216"), Ok(521_600));
        assert_eq!(micro_usd_stated("0"), Ok(0));
    }
/// A plan over a case, for the argv tests.
    // --- the cap on the argv, and the child's PATH ---------------------------------------------

    #[test]
    fn what_is_left_of_the_cap_travels_to_a_claude_run_and_to_no_other() {
        let mut plan = plan_of(Arm::Raw, Harness::Claude);
        let argv = spawn_argv(
            &plan,
            "metaharness",
            Path::new("/work/subject"),
            "do the thing",
            None,
            None,
            Some("4.200000"),
        );
        let at = argv
            .iter()
            .position(|word| word == "--max-budget-usd")
            .expect("the cap is on the argv");
        assert_eq!(argv[at + 1], "4.200000");
        assert!(
            at < argv.iter().position(|word| word == "-p").expect("a prompt"),
            "the cap is a run option, placed before the prompt: {argv:?}"
        );

        plan.harness = Harness::Codex;
        let codex = spawn_argv(
            &plan,
            "metaharness",
            Path::new("/work/subject"),
            "do the thing",
            None,
            None,
            Some("4.200000"),
        );
        assert!(
            !codex.iter().any(|word| word == "--max-budget-usd"),
            "codex takes no cap and metaharness would refuse it: {codex:?}"
        );
    }
#[test]
    fn micro_dollars_render_as_the_plain_decimal_a_vendor_flag_takes() {
        assert_eq!(usd_plain(0), "0.000000");
        assert_eq!(usd_plain(5_000_000), "5.000000");
        assert_eq!(usd_plain(4_200_000), "4.200000");
        assert_eq!(usd_plain(10_962_418), "10.962418");
    }
#[test]
    fn the_childs_path_is_the_one_metaharness_constructs() {
        assert_eq!(
            child_path_for(Some("/home/ada")),
            "/home/ada/.local/bin:/usr/local/bin:/usr/bin:/bin"
        );
        assert_eq!(child_path_for(None), "/usr/local/bin:/usr/bin:/bin");
        assert_eq!(child_path_for(Some("")), "/usr/local/bin:/usr/bin:/bin");
    }
#[test]
    fn the_version_is_the_first_token_that_starts_with_a_digit() {
        assert_eq!(version_token("protocol 0.44.0").as_deref(), Some("0.44.0"));
        assert_eq!(version_token("aep 0.45.0\n").as_deref(), Some("0.45.0"));
        assert_eq!(version_token("").as_deref(), None);
        assert_eq!(version_token("error: no such flag").as_deref(), None);
    }
fn plan_of(arm: Arm, harness: Harness) -> Plan {
        Plan {
            case: Case {
                id: "development-honest".to_owned(),
                workflow: "adp/default".to_owned(),
                task: "Add a `--json` flag.".to_owned(),
                expectations: PathBuf::from(
                    "conformance/eval/development-honest/expectations.trace.yaml",
                ),
                needs_ess: false,
            },
            arm,
            harness,
            plugins: Vec::new(),
            model_requested: None,
        }
    }
#[test]
    fn a_pinned_plugin_reaches_the_argv_with_the_bytes_the_operator_wrote() {
        // *Verbatim* is a claim about bytes, so it is asserted on bytes. The parse splits on the
        // last two `@` and [`fmt::Display`] rejoins on them, which round-trips exactly — including
        // a repository spelling that is not `owner/repo` and a commit pin, neither of which this
        // runner interprets.
        //
        // The pin is a plugin an operator could install today: `aep-plan` at `agentplugins@a2077d2`,
        // the commit that renamed it. A coordinate naming the old `aep-planning` would still
        // round-trip — this runner resolves nothing — but it would model an install nobody can
        // make, and the only such coordinate in the suite is the released `@0.4.0` pin in
        // `tests/eval_run.rs`, which is kept old on purpose because that release really is named
        // that.
        let mut plan = plan_of(Arm::Plugin, Harness::Claude);
        for given in [
            "bdfinst/agentic-dev-team@dev-team@1.4.0",
            "beyond10x/agentplugins@aep-plan@a2077d25a7d56fd34a4d8a0f37b0a152c39ad7ab",
        ] {
            plan.plugins
                .push(MarketplacePlugin::parse(given).expect("a pinned spelling"));
        }
        let argv = spawn_argv(
            &plan,
            "metaharness",
            Path::new("/work/subject"),
            "do the thing",
            Some(Path::new("/plugins/aep-plan")),
            None,
        None,
    );
        let forwarded: Vec<&String> = argv
            .iter()
            .zip(argv.iter().skip(1))
            .filter(|(flag, _)| *flag == "--plugin")
            .map(|(_, value)| value)
            .collect();
        assert_eq!(
            forwarded,
            vec![
                "bdfinst/agentic-dev-team@dev-team@1.4.0",
                "beyond10x/agentplugins@aep-plan@a2077d25a7d56fd34a4d8a0f37b0a152c39ad7ab"
            ],
            "the operator's bytes, in the operator's order: {argv:?}"
        );
        assert!(
            argv.windows(2)
                .any(|pair| pair == ["--plugin-dir", "/plugins/aep-plan"]),
            "beside the directory rather than instead of it: {argv:?}"
        );
    }
#[test]
    fn the_model_is_forwarded_verbatim_and_before_the_treatment() {
        // The dry run of `--model`: the argv, with nothing spawned. *Verbatim* is a claim about
        // bytes, so it is asserted on bytes — a runner that normalised `claude-sonnet-4-6` to a
        // dated id would be pinning something other than what the operator wrote down.
        let tree = PathBuf::from("/work/scratch");
        let raw = spawn_argv(
            &plan_of(Arm::Raw, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
            None,
            Some("claude-sonnet-4-6"),
        None,
    );
        assert_eq!(
            raw,
            vec![
                "metaharness",
                "run",
                "claude",
                "--hermetic",
                "--cwd",
                "/work/scratch",
                "--decisions",
                "observe",
                "-p",
                "do it",
                "--model",
                "claude-sonnet-4-6",
            ]
        );
        let plugin = spawn_argv(
            &plan_of(Arm::Plugin, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
            Some(Path::new("/plugins/aep-plan")),
            Some("claude-sonnet-4-6"),
        None,
    );
        assert_eq!(
            plugin[..raw.len()],
            raw[..],
            "the model is a condition both arms are held at, so it sits before the treatment and \
             not inside it"
        );
        assert!(
            !spawn_argv(
                &plan_of(Arm::Raw, Harness::Claude),
                "metaharness",
                &tree,
                "do it",
                None,
                None,
            None,
        )
            .iter()
            .any(|word| word == "--model"),
            "an invocation that pins no model keeps the argv it had before the flag existed"
        );
    }
#[test]
    fn an_unpinned_plugin_is_refused_in_metaharness_own_words_and_never_defaulted() {
        // Two segments name a plugin whose contents can change between two runs that both claim to
        // have used it. metaharness refuses it at parse; this refuses it before the spawn, with the
        // same sentence, so an operator does not read two different explanations of one mistake.
        let refusal = MarketplacePlugin::parse("beyond10x/agentplugins@aep-plan")
            .expect_err("two segments name no pin");
        assert!(
            refusal.contains("names no pin")
                && refusal.contains("<repo>@<name>@<version-or-commit>"),
            "{refusal}"
        );
        let blank = MarketplacePlugin::parse("beyond10x/agentplugins@aep-plan@")
            .expect_err("an empty pin names everything");
        assert!(blank.contains("empty pin"), "{blank}");
    }
#[test]
    fn every_arm_is_spawned_by_one_instrument_and_only_the_treatment_varies() {
        // Design constant 1, as an assertion on the argv. The two invocations differ in exactly
        // two words — `--plugin-dir` and the directory — and in nothing else: same hermetic mode,
        // same decision mode, same working tree. An instrument that varied with the arm would make
        // the comparison meaningless whatever the matrix said.
        let tree = PathBuf::from("/work/scratch");
        let raw = spawn_argv(
            &plan_of(Arm::Raw, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
            None,
            None,
        None,
    );
        let plugin = spawn_argv(
            &plan_of(Arm::Plugin, Harness::Claude),
            "metaharness",
            &tree,
            "do it",
            Some(Path::new("/plugins/aep-plan")),
            None,
        None,
    );

        assert_eq!(
            raw,
            vec![
                "metaharness",
                "run",
                "claude",
                "--hermetic",
                "--cwd",
                "/work/scratch",
                "--decisions",
                "observe",
                "-p",
                "do it",
            ]
        );
        assert_eq!(
            plugin[..raw.len()],
            raw[..],
            "arm b is arm a plus its treatment, and nothing else"
        );
        assert_eq!(
            &plugin[raw.len()..],
            &[
                "--plugin-dir".to_owned(),
                "/plugins/aep-plan".to_owned()
            ]
        );
        assert_eq!(
            spawn_argv(
                &plan_of(Arm::Plugin, Harness::Codex),
                "metaharness",
                &tree,
                "do it",
                Some(Path::new("/plugins/aep-plan")),
                None,
            None,
        )
            .last()
            .expect("a plugin directory"),
            "/plugins/aep-plan",
            "the caller-selected plugin is independent of the harness"
        );
    }
#[test]
    fn a_transcript_of_several_sessions_totals_them_rather_than_reporting_the_last_one() {
        // The driven shape. `protocol drive` starts a fresh session per workflow state, so a driven
        // run's transcript is a concatenation with one terminal record per state — and this reader
        // took the last of them until the first live driven run (2026-08-23) reported `$1.135363`
        // for a walk that had cost `$15.014604` across six sessions.
        //
        // Doubling a committed fixture is the whole assertion: whatever one session states, two
        // copies of it must state twice, on all three columns. A reader that takes the last record
        // answers the single figure and fails here.
        let one = std::fs::read(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/eval-run/claude-driven-attested.jsonl"),
        )
        .expect("the committed driven fixture");
        let mut two = one.clone();
        two.extend_from_slice(&one);

        let single =
            Session::read(&one, Arm::Driven, Harness::Claude, &[]).expect("a readable stream");
        let doubled = Session::read(&two, Arm::Driven, Harness::Claude, &[])
            .expect("two of them, concatenated");

        assert_eq!(
            doubled.cost_micro_usd,
            single.cost_micro_usd.map(|cost| cost * 2),
            "two sessions cost what both of them cost"
        );
        assert_eq!(
            doubled.tokens,
            single.tokens.map(|tokens| tokens * 2),
            "and used what both of them used"
        );
        assert_eq!(
            doubled.wall_time_ms,
            single.wall_time_ms.map(|wall| wall * 2),
            "and took as long as both of them took: the sessions run one after another"
        );
        assert_eq!(
            (
                doubled.harness_version.clone(),
                doubled.model.clone(),
                doubled.plugin_digest.clone()
            ),
            (
                single.harness_version.clone(),
                single.model.clone(),
                single.plugin_digest.clone()
            ),
            "and nothing else moved: the opening record is still the first one"
        );
    }
#[test]
    fn a_run_that_states_no_cost_writes_no_cost_key_rather_than_a_zero() {
        let session = Session {
            // The shape the first live pilot run recorded: codex states no model at session start.
            harness_version: "codex 0.144.0".to_owned(),
            model: None,
            plugin_digest: None,
            plugins: Vec::new(),
            cost_micro_usd: None,
            tokens: None,
            wall_time_ms: None,
        };
        let text = manifest_text(
            &plan_of(Arm::Raw, Harness::Codex),
            &session,
            &"c".repeat(DIGEST_WIDTH),
            "2026-08-23",
        );
        assert!(
            !text.contains("cost_micro_usd") && !text.contains("tokens"),
            "an unpriced run states nothing, and the matrix reports its cell over the runs that \
             did: {text}"
        );
        assert!(
            text.contains("plugin_digest: null") && text.contains("model: null"),
            "and both keys that must be written even when they say nothing are written: {text}"
        );
        assert!(
            read(&text).is_ok(),
            "and a manifest with two written nulls in it is one the matrix reads: {text}"
        );
    }
}
