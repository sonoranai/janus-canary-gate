use canary_gate::behavior::evaluate_tests;
use canary_gate::classification::classify_stream;
use canary_gate::config::load_config;
use canary_gate::ingestion::LogReader;
use canary_gate::recommendation::{CycleTracker, Recommendation};
use canary_gate::verdict::Verdict;
use std::path::Path;

fn golden(scenario: &str, file: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/golden")
        .join(scenario)
        .join(file)
}

/// Run the full evaluation pipeline for a golden scenario.
fn evaluate_scenario(scenario: &str) -> Verdict {
    let config = load_config(&golden(scenario, "config.yaml")).unwrap();
    let reader = LogReader::new(config.logging.format.clone());
    let lines = reader.read_file(&golden(scenario, "canary.log")).unwrap();
    let events = classify_stream(&lines, &config.logging.events);
    let evaluations = evaluate_tests(&config.tests, &events);

    let mut tracker = CycleTracker::new();
    tracker.record_cycle(&config.tests, &evaluations, &config.recommendation);
    Verdict::from_tracker(&tracker)
}

/// Load expected verdict from golden fixture.
fn load_expected(scenario: &str) -> serde_json::Value {
    let path = golden(scenario, "expected_verdict.json");
    let contents = std::fs::read_to_string(path).unwrap();
    serde_json::from_str(&contents).unwrap()
}

#[test]
fn e2e_promote_scenario() {
    let verdict = evaluate_scenario("scenario_promote");
    let expected = load_expected("scenario_promote");

    assert_eq!(verdict.recommendation, Recommendation::Promote);
    assert_eq!(
        verdict.total_cycles,
        expected["total_cycles"].as_u64().unwrap() as u32
    );
    assert_eq!(
        verdict.consecutive_passes,
        expected["consecutive_passes"].as_u64().unwrap() as u32
    );
}

#[test]
fn e2e_rollback_scenario() {
    let verdict = evaluate_scenario("scenario_rollback");
    let expected = load_expected("scenario_rollback");

    assert_eq!(verdict.recommendation, Recommendation::Rollback);
    assert_eq!(
        verdict.total_cycles,
        expected["total_cycles"].as_u64().unwrap() as u32
    );
}

#[test]
fn e2e_hold_scenario() {
    let verdict = evaluate_scenario("scenario_hold");
    let expected = load_expected("scenario_hold");

    assert_eq!(verdict.recommendation, Recommendation::Hold);
    assert_eq!(
        verdict.total_cycles,
        expected["total_cycles"].as_u64().unwrap() as u32
    );
}

#[test]
fn e2e_promote_json_output() {
    let verdict = evaluate_scenario("scenario_promote");
    let json = serde_json::to_value(&verdict).unwrap();

    assert_eq!(json["recommendation"], "promote");
    assert!(json["test_results"].is_array());
    assert!(json["reasoning"].is_array());
}

#[test]
fn e2e_promote_table_output() {
    let verdict = evaluate_scenario("scenario_promote");
    let table = verdict.format_table();

    assert!(table.contains("RECOMMEND_PROMOTE"));
    assert!(table.contains("Test Results"));
}

#[test]
fn e2e_rollback_has_reasoning() {
    let verdict = evaluate_scenario("scenario_rollback");
    assert!(!verdict.reasoning.is_empty());
}

#[test]
fn e2e_exit_codes() {
    let promote = evaluate_scenario("scenario_promote");
    let rollback = evaluate_scenario("scenario_rollback");
    let hold = evaluate_scenario("scenario_hold");

    assert_eq!(promote.exit_code(), 0);
    assert_eq!(hold.exit_code(), 1);
    assert_eq!(rollback.exit_code(), 2);
}
