use super::super::tests::{b10x_map, config, driven_task, step_context};
use super::*;

fn explicit() -> SpendOptions {
    SpendOptions {
        uncapped_budget: true,
        spend_authorization: Some("operator:approval-14".to_owned()),
    }
}

fn launch(policy: &serde_json::Value) -> Launch {
    serde_json::from_value(
        serde_json::json!({"task":null,"map":null,"project":null,"root":null,
        "pause_on_approval":false,"plugin_dirs":[],"spend_policy":policy}),
    )
    .unwrap()
}

#[test]
fn uncapped_admission_requires_explicit_reference_and_live_opt_in() {
    let map = b10x_map();
    let selected = policy(&map, None, None, None, &explicit(), true)
        .unwrap()
        .unwrap();
    assert!(
        matches!(selected, SpendPolicy::Uncapped { authorization_ref } if authorization_ref == "operator:approval-14")
    );
    assert!(policy(&map, None, None, None, &explicit(), false).is_err());
    assert!(policy(&map, None, Some("5"), None, &explicit(), true).is_err());
    assert!(policy(&map, None, None, Some("1"), &explicit(), true).is_err());
    for options in [
        SpendOptions::default(),
        SpendOptions {
            uncapped_budget: true,
            spend_authorization: None,
        },
        SpendOptions {
            uncapped_budget: false,
            spend_authorization: Some("ref".to_owned()),
        },
        SpendOptions {
            uncapped_budget: true,
            spend_authorization: Some("  \t".to_owned()),
        },
    ] {
        assert!(policy(&map, None, None, None, &options, true).is_err());
    }
}

#[test]
fn resume_preserves_mode_and_reference_and_reads_legacy_finite_terms() {
    let map = b10x_map();
    let current =
        launch(&serde_json::json!({"mode":"uncapped","authorization_ref":"operator:approval-14"}));
    assert!(
        policy(
            &map,
            Some(&current),
            None,
            None,
            &SpendOptions::default(),
            true
        )
        .is_ok()
    );
    assert!(
        policy(
            &map,
            Some(&current),
            Some("5"),
            None,
            &SpendOptions::default(),
            true
        )
        .is_err()
    );
    let different = SpendOptions {
        spend_authorization: Some("another".to_owned()),
        ..explicit()
    };
    assert!(policy(&map, Some(&current), None, None, &different, true).is_err());
    let mut legacy = current.clone();
    legacy.extra.clear();
    legacy.extra.insert(
        "spend".to_owned(),
        serde_json::json!({"cap_micro_usd":5_000_000,"assumed_micro_usd_per_run":1_000_000}),
    );
    assert!(policy(&map, Some(&legacy), None, None, &explicit(), true).is_err());
    assert!(matches!(
        policy(
            &map,
            Some(&legacy),
            Some("3"),
            None,
            &SpendOptions::default(),
            true
        )
        .unwrap(),
        Some(SpendPolicy::Finite(SpendTerms {
            cap_micro_usd: 3_000_000,
            ..
        }))
    ));
    assert!(
        policy(
            &map,
            Some(&legacy),
            Some("6"),
            None,
            &SpendOptions::default(),
            true
        )
        .is_err()
    );
    let malformed = launch(&serde_json::json!({"mode":"uncapped","authorization_ref":""}));
    assert!(
        policy(
            &map,
            Some(&malformed),
            None,
            None,
            &SpendOptions::default(),
            true
        )
        .is_err()
    );
}

#[test]
fn invocation_precedes_spawn_and_unknown_cost_survives_crash_resume() {
    let directory = tempfile::tempdir().unwrap();
    let mut budget = UncappedBudget::open(directory.path(), "ref".to_owned(), false).unwrap();
    let tools = config(&[]);
    let state = "implement".parse().unwrap();
    let task = driven_task();
    let mut context = step_context(&tools, &state, &task);
    context.index = 3;
    context.attempt = 2;
    budget.reserve(&context).unwrap();
    // A failed spawn or crash leaves exactly this admission on disk, before any observation.
    let resumed = UncappedBudget::open(directory.path(), "ref".to_owned(), true).unwrap();
    assert_eq!(resumed.ledger.invocations.len(), 1);
    assert_eq!(
        resumed.ledger.invocations[0],
        Invocation {
            ordinal: 1,
            task: "T-1".to_owned(),
            state: "implement".to_owned(),
            step: 3,
            attempt: 2,
            observed_cost_usd: None
        }
    );
    assert!(UncappedBudget::open(directory.path(), "another-ref".to_owned(), true).is_err());
    assert!(UncappedBudget::open(directory.path(), "ref".to_owned(), false).is_err());
    let transcript = directory.path().join("events.jsonl");
    fs::write(
        &transcript,
        "{\"event\":\"session.ended\",\"total_cost_usd\":null}\n",
    )
    .unwrap();
    budget.observe(&transcript).unwrap();
    assert_eq!(budget.ledger.invocations[0].observed_cost_usd, None);
    budget.reserve(&context).unwrap();
    fs::write(
        &transcript,
        "{\"event\":\"session.ended\",\"total_cost_usd\":0.125}\n",
    )
    .unwrap();
    budget.observe(&transcript).unwrap();
    let resumed = UncappedBudget::open(directory.path(), "ref".to_owned(), true).unwrap();
    assert_eq!(resumed.ledger.invocations[1].ordinal, 2);
    assert_eq!(resumed.ledger.invocations[1].observed_cost_usd, Some(0.125));
    assert_eq!(resumed.ledger.invocations[0].observed_cost_usd, None);
}

#[test]
fn failed_persistence_grants_no_authority_and_corrupt_order_cannot_resume() {
    let directory = tempfile::tempdir().unwrap();
    let mut budget = UncappedBudget::open(directory.path(), "ref".to_owned(), false).unwrap();
    fs::create_dir(budget.path.with_extension("json.next")).unwrap();
    let tools = config(&[]);
    let state = "implement".parse().unwrap();
    let task = driven_task();
    let context = step_context(&tools, &state, &task);
    assert!(budget.reserve(&context).is_err());
    let disk = UncappedBudget::open(directory.path(), "ref".to_owned(), true).unwrap();
    assert!(
        disk.ledger.invocations.is_empty(),
        "no paid effect was authorized by a failed reservation"
    );
    let mut corrupt = budget.ledger;
    corrupt.invocations[0].ordinal = u64::MAX;
    fs::write(&disk.path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(UncappedBudget::open(directory.path(), "ref".to_owned(), true).is_err());
}
