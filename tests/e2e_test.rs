use canary_gate::behavior::{evaluate_metrics_queries, evaluate_tests};
use canary_gate::classification::classify_stream;
use canary_gate::config::load_config;
use canary_gate::ingestion::LogReader;
use canary_gate::metrics::MetricResult;
use canary_gate::recommendation::{CycleTracker, Recommendation};
use canary_gate::verdict::Verdict;
use std::collections::HashMap;
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

#[test]
fn verdict_from_tracker_zero_cycles() {
    let tracker = CycleTracker::new();
    let verdict = Verdict::from_tracker(&tracker);

    assert_eq!(verdict.recommendation, Recommendation::Hold);
    assert_eq!(verdict.total_cycles, 0);
    assert_eq!(verdict.consecutive_passes, 0);
    assert!(verdict.test_results.is_empty());
    assert!(verdict
        .reasoning
        .iter()
        .any(|r| r.contains("No evaluation cycles")));
}

#[test]
fn verdict_format_table_empty_results() {
    let verdict = Verdict {
        recommendation: Recommendation::Hold,
        total_cycles: 0,
        consecutive_passes: 0,
        test_results: vec![],
        reasoning: vec![],
        statistical_score: None,
    };
    let table = verdict.format_table();

    assert!(table.contains("RECOMMEND_HOLD"));
    assert!(table.contains("Cycles: 0"));
    // Should not contain test results or reasoning sections when empty
    assert!(!table.contains("Test Results"));
    assert!(!table.contains("Reasoning"));
}

#[test]
fn recommendation_display_format() {
    assert_eq!(Recommendation::Promote.to_string(), "RECOMMEND_PROMOTE");
    assert_eq!(Recommendation::Hold.to_string(), "RECOMMEND_HOLD");
    assert_eq!(Recommendation::Rollback.to_string(), "RECOMMEND_ROLLBACK");
}

/// Run the full evaluation pipeline for a golden scenario that includes metrics.
/// Uses canned metric results instead of querying Prometheus.
fn evaluate_scenario_with_metrics(scenario: &str, metric_results: Vec<MetricResult>) -> Verdict {
    let config = load_config(&golden(scenario, "config.yaml")).unwrap();
    let reader = LogReader::new(config.logging.format.clone());
    let lines = reader.read_file(&golden(scenario, "canary.log")).unwrap();
    let events = classify_stream(&lines, &config.logging.events);
    let mut evaluations = evaluate_tests(&config.tests, &events);

    // Evaluate metrics queries against canned results
    let mut all_test_configs = config.tests.clone();
    if let Some(ref metrics_cfg) = config.metrics {
        let metrics_evals = evaluate_metrics_queries(&metrics_cfg.queries, &metric_results);
        let metrics_test_configs: Vec<_> = metrics_cfg
            .queries
            .iter()
            .map(|q| q.to_test_config())
            .collect();
        evaluations.extend(metrics_evals);
        all_test_configs.extend(metrics_test_configs);
    }

    let mut tracker = CycleTracker::new();
    tracker.record_cycle(&all_test_configs, &evaluations, &config.recommendation);
    Verdict::from_tracker(&tracker)
}

#[test]
fn e2e_metrics_promote_scenario() {
    // Simulate Prometheus returning healthy metrics (below thresholds)
    let metric_results = vec![
        MetricResult {
            name: "error_rate".to_string(),
            value: 0.01,
            labels: HashMap::new(),
        },
        MetricResult {
            name: "latency_p99".to_string(),
            value: 0.5,
            labels: HashMap::new(),
        },
    ];
    let verdict = evaluate_scenario_with_metrics("scenario_metrics_promote", metric_results);
    let expected = load_expected("scenario_metrics_promote");

    assert_eq!(verdict.recommendation, Recommendation::Promote);
    assert_eq!(
        verdict.total_cycles,
        expected["total_cycles"].as_u64().unwrap() as u32
    );
    // Should have log tests + metrics tests
    assert!(verdict.test_results.len() >= 4);
}

#[test]
fn e2e_metrics_rollback_scenario() {
    // Simulate Prometheus returning error_rate above threshold (0.10 > 0.05)
    let metric_results = vec![MetricResult {
        name: "error_rate".to_string(),
        value: 0.10,
        labels: HashMap::new(),
    }];
    let verdict = evaluate_scenario_with_metrics("scenario_metrics_rollback", metric_results);
    let expected = load_expected("scenario_metrics_rollback");

    assert_eq!(verdict.recommendation, Recommendation::Rollback);
    assert_eq!(
        verdict.total_cycles,
        expected["total_cycles"].as_u64().unwrap() as u32
    );
}
