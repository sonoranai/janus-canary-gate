use canary_gate::config::{FailSeverity, MetricDirection, StatisticalComparison};
use canary_gate::stats::mann_whitney::{mann_whitney_u, normal_cdf};
use canary_gate::stats::scoring::{
    aggregate_score, classify, MetricAnalysis, MetricClassification,
};

// --- Mann-Whitney U tests ---

#[test]
fn identical_distributions_not_significant() {
    let baseline: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let canary: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let result = mann_whitney_u(&baseline, &canary);
    assert!(
        result.p_value > 0.05,
        "identical distributions should not be significant, got p={}",
        result.p_value
    );
    assert!(!result.significant);
}

#[test]
fn clearly_different_distributions_significant() {
    // Baseline: low values, canary: high values
    let baseline: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let canary: Vec<f64> = (100..130).map(|i| i as f64).collect();
    let result = mann_whitney_u(&baseline, &canary);
    assert!(
        result.p_value < 0.05,
        "clearly different distributions should be significant, got p={}",
        result.p_value
    );
    assert!(result.significant);
}

#[test]
fn empty_baseline_returns_default() {
    let baseline: Vec<f64> = vec![];
    let canary: Vec<f64> = vec![1.0, 2.0, 3.0];
    let result = mann_whitney_u(&baseline, &canary);
    assert_eq!(result.u_statistic, 0.0);
    assert_eq!(result.z_score, 0.0);
    assert_eq!(result.p_value, 1.0);
    assert!(!result.significant);
}

#[test]
fn empty_canary_returns_default() {
    let baseline: Vec<f64> = vec![1.0, 2.0, 3.0];
    let canary: Vec<f64> = vec![];
    let result = mann_whitney_u(&baseline, &canary);
    assert_eq!(result.u_statistic, 0.0);
    assert_eq!(result.z_score, 0.0);
    assert_eq!(result.p_value, 1.0);
    assert!(!result.significant);
}

#[test]
fn single_element_each() {
    let baseline = vec![1.0];
    let canary = vec![2.0];
    let result = mann_whitney_u(&baseline, &canary);
    // With only 1 element each, can't be significant
    assert!(!result.significant);
    assert!(result.p_value >= 0.05);
}

#[test]
fn known_textbook_values() {
    // Classic example: two small samples
    // Baseline: [1, 2, 3, 4, 5]
    // Canary: [6, 7, 8, 9, 10]
    // All baseline < all canary → U = 0 for baseline (min U)
    let baseline = vec![1.0, 2.0, 3.0, 4.0, 5.0];
    let canary = vec![6.0, 7.0, 8.0, 9.0, 10.0];
    let result = mann_whitney_u(&baseline, &canary);

    // U_baseline = R_baseline - n1*(n1+1)/2 = (1+2+3+4+5) - 5*6/2 = 15-15 = 0
    // U_canary = n1*n2 - U_baseline = 25 - 0 = 25
    // min(U) = 0
    assert_eq!(result.u_statistic, 0.0);

    // With n=5 each and perfect separation, should be significant even for small n
    // (using normal approximation)
    assert!(result.p_value < 0.05);
}

#[test]
fn tied_ranks_handled() {
    // Data with many ties
    let baseline = vec![1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0];
    let canary = vec![1.0, 1.0, 1.0, 2.0, 2.0, 3.0, 3.0, 3.0, 4.0, 4.0];
    let result = mann_whitney_u(&baseline, &canary);

    // Identical distributions with ties → not significant
    assert!(!result.significant);
    assert!(result.p_value > 0.05);
}

#[test]
fn normal_cdf_accuracy() {
    // Test against known z-table values
    assert!((normal_cdf(0.0) - 0.5).abs() < 1e-4);
    assert!((normal_cdf(1.0) - 0.8413).abs() < 1e-3);
    assert!((normal_cdf(-1.0) - 0.1587).abs() < 1e-3);
    assert!((normal_cdf(1.96) - 0.975).abs() < 1e-3);
    assert!((normal_cdf(2.576) - 0.995).abs() < 1e-3);
    assert!((normal_cdf(3.0) - 0.9987).abs() < 1e-3);
    // Symmetry
    let x = 1.5;
    assert!((normal_cdf(x) + normal_cdf(-x) - 1.0).abs() < 1e-5);
}

// --- Classification tests ---

#[test]
fn direction_increase_bad_higher_canary_fails() {
    // IncreaseBad: canary is higher → bad direction, low p-value → Fail
    let classification = classify(0.001, &MetricDirection::IncreaseBad, 10.0, 20.0);
    assert_eq!(classification, MetricClassification::Fail);
}

#[test]
fn direction_decrease_bad_lower_canary_fails() {
    // DecreaseBad: canary is lower → bad direction, low p-value → Fail
    let classification = classify(0.001, &MetricDirection::DecreaseBad, 20.0, 10.0);
    assert_eq!(classification, MetricClassification::Fail);
}

#[test]
fn direction_increase_bad_lower_canary_passes() {
    // IncreaseBad: canary is lower → good direction → Pass even with low p-value
    let classification = classify(0.001, &MetricDirection::IncreaseBad, 20.0, 10.0);
    assert_eq!(classification, MetricClassification::Pass);
}

#[test]
fn marginal_classification() {
    // p-value between 0.01 and 0.05, bad direction → Marginal
    let classification = classify(0.03, &MetricDirection::IncreaseBad, 10.0, 20.0);
    assert_eq!(classification, MetricClassification::Marginal);
}

#[test]
fn high_p_value_passes() {
    // p_value >= 0.05 → Pass regardless of direction
    let classification = classify(0.10, &MetricDirection::IncreaseBad, 10.0, 20.0);
    assert_eq!(classification, MetricClassification::Pass);
}

// --- Aggregate scoring tests ---

fn make_analysis(name: &str, classification: MetricClassification, weight: f64) -> MetricAnalysis {
    MetricAnalysis {
        name: name.to_string(),
        baseline_mean: 0.0,
        canary_mean: 0.0,
        p_value: match classification {
            MetricClassification::Pass => 0.10,
            MetricClassification::Marginal => 0.03,
            MetricClassification::Fail => 0.001,
        },
        direction: MetricDirection::IncreaseBad,
        classification,
        weight,
    }
}

#[test]
fn weighted_scoring_all_pass() {
    let analyses = vec![
        make_analysis("m1", MetricClassification::Pass, 1.0),
        make_analysis("m2", MetricClassification::Pass, 1.0),
        make_analysis("m3", MetricClassification::Pass, 1.0),
    ];
    let score = aggregate_score(&analyses);
    assert!((score.score - 100.0).abs() < f64::EPSILON);
    assert_eq!(score.pass, 3);
    assert_eq!(score.marginal, 0);
    assert_eq!(score.fail, 0);
}

#[test]
fn weighted_scoring_all_fail() {
    let analyses = vec![
        make_analysis("m1", MetricClassification::Fail, 1.0),
        make_analysis("m2", MetricClassification::Fail, 1.0),
    ];
    let score = aggregate_score(&analyses);
    assert!((score.score - 0.0).abs() < f64::EPSILON);
    assert_eq!(score.pass, 0);
    assert_eq!(score.fail, 2);
}

#[test]
fn weighted_scoring_mixed() {
    // 1 pass (weight 1.0), 1 marginal (weight 1.0), 1 fail (weight 1.0)
    // Score = (1.0*100 + 1.0*50 + 0) / 3.0 = 50.0
    let analyses = vec![
        make_analysis("m1", MetricClassification::Pass, 1.0),
        make_analysis("m2", MetricClassification::Marginal, 1.0),
        make_analysis("m3", MetricClassification::Fail, 1.0),
    ];
    let score = aggregate_score(&analyses);
    assert!((score.score - 50.0).abs() < f64::EPSILON);
    assert_eq!(score.pass, 1);
    assert_eq!(score.marginal, 1);
    assert_eq!(score.fail, 1);
}

#[test]
fn aggregate_score_empty_input() {
    let score = aggregate_score(&[]);
    assert!((score.score - 100.0).abs() < f64::EPSILON);
    assert_eq!(score.pass, 0);
    assert_eq!(score.marginal, 0);
    assert_eq!(score.fail, 0);
}

#[test]
fn weighted_scoring_respects_weights() {
    // High-weight pass + low-weight fail
    // Score = (3.0*100 + 0) / 4.0 = 75.0
    let analyses = vec![
        make_analysis("important", MetricClassification::Pass, 3.0),
        make_analysis("minor", MetricClassification::Fail, 1.0),
    ];
    let score = aggregate_score(&analyses);
    assert!((score.score - 75.0).abs() < f64::EPSILON);
}

// --- Statistical comparison integration test ---

#[test]
fn evaluate_statistical_comparisons_basic() {
    use canary_gate::behavior::evaluate_statistical_comparisons;

    let comparisons = vec![StatisticalComparison {
        name: "latency".to_string(),
        baseline_query: "latency_baseline".to_string(),
        canary_query: "latency_canary".to_string(),
        direction: MetricDirection::IncreaseBad,
        allowed_deviation: None,
        severity: FailSeverity::Hard,
        weight: 1.0,
    }];

    // Identical distributions → should pass
    let baseline: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let canary: Vec<f64> = (0..30).map(|i| i as f64).collect();

    let (evals, score) = evaluate_statistical_comparisons(&comparisons, &[(&baseline, &canary)]);

    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].test_name, "stats:latency");
    assert_eq!(evals[0].result, canary_gate::behavior::TestResult::Pass);
    assert!((score.score - 100.0).abs() < f64::EPSILON);
}

#[test]
fn evaluate_statistical_comparisons_detects_regression() {
    use canary_gate::behavior::evaluate_statistical_comparisons;

    let comparisons = vec![StatisticalComparison {
        name: "error_rate".to_string(),
        baseline_query: "errors_baseline".to_string(),
        canary_query: "errors_canary".to_string(),
        direction: MetricDirection::IncreaseBad,
        allowed_deviation: None,
        severity: FailSeverity::Hard,
        weight: 1.0,
    }];

    // Canary has significantly higher values → should fail (IncreaseBad)
    let baseline: Vec<f64> = (0..30).map(|i| i as f64).collect();
    let canary: Vec<f64> = (100..130).map(|i| i as f64).collect();

    let (evals, score) = evaluate_statistical_comparisons(&comparisons, &[(&baseline, &canary)]);

    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].result, canary_gate::behavior::TestResult::Fail);
    assert!(score.score < 50.0);
}
