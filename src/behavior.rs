use crate::config::{MetricsQuery, RateOperator, StatisticalComparison, TestAssertion, TestConfig};
use crate::events::CanonicalEvent;
use crate::metrics::MetricResult;
use crate::stats::mann_whitney::mann_whitney_u;
use crate::stats::scoring::{
    aggregate_score, classify, AggregateScore, MetricAnalysis, MetricClassification,
};

/// Result of a single test evaluation.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TestResult {
    Pass,
    Fail,
    Unknown,
}

/// Result of evaluating a full test (name + individual assertion results).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TestEvaluation {
    pub test_name: String,
    pub result: TestResult,
    pub assertion_results: Vec<AssertionResult>,
}

/// Result of a single assertion within a test.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AssertionResult {
    pub description: String,
    pub result: TestResult,
}

/// Evaluate all tests against the event stream.
pub fn evaluate_tests(tests: &[TestConfig], events: &[CanonicalEvent]) -> Vec<TestEvaluation> {
    tests.iter().map(|t| evaluate_test(t, events)).collect()
}

/// Evaluate a single test against the event stream.
fn evaluate_test(test: &TestConfig, events: &[CanonicalEvent]) -> TestEvaluation {
    let assertion_results: Vec<AssertionResult> = test
        .then
        .iter()
        .map(|a| evaluate_assertion(a, events))
        .collect();

    // A test passes only if all assertions pass.
    // A test fails if any assertion fails.
    // A test is unknown if any assertion is unknown and none fail.
    let result = if assertion_results
        .iter()
        .any(|r| r.result == TestResult::Fail)
    {
        TestResult::Fail
    } else if assertion_results
        .iter()
        .any(|r| r.result == TestResult::Unknown)
    {
        TestResult::Unknown
    } else {
        TestResult::Pass
    };

    TestEvaluation {
        test_name: test.name.clone(),
        result,
        assertion_results,
    }
}

/// Evaluate a single assertion against the event stream.
fn evaluate_assertion(assertion: &TestAssertion, events: &[CanonicalEvent]) -> AssertionResult {
    if let Some(ref present) = assertion.event_present {
        let found = events.iter().any(|e| e.event_type == present.event_type);
        return AssertionResult {
            description: format!("event_present: {}", present.event_type),
            result: if found {
                TestResult::Pass
            } else {
                TestResult::Fail
            },
        };
    }

    if let Some(ref absent) = assertion.event_absent {
        let found = events.iter().any(|e| e.event_type == absent.event_type);
        return AssertionResult {
            description: format!("event_absent: {}", absent.event_type),
            result: if found {
                TestResult::Fail
            } else {
                TestResult::Pass
            },
        };
    }

    if let Some(ref rate) = assertion.rate {
        let count = events
            .iter()
            .filter(|e| e.event_type == rate.event_type)
            .count() as f64;

        if let (Some(threshold), Some(op)) = (&rate.threshold, &rate.operator) {
            let passes = match op {
                crate::config::RateOperator::LessThan => count < *threshold,
                crate::config::RateOperator::GreaterThan => count > *threshold,
                crate::config::RateOperator::LessThanOrEqual => count <= *threshold,
                crate::config::RateOperator::GreaterThanOrEqual => count >= *threshold,
            };
            return AssertionResult {
                description: format!("rate: {} {:?} {}", rate.event_type, op, threshold),
                result: if passes {
                    TestResult::Pass
                } else {
                    TestResult::Fail
                },
            };
        }

        return AssertionResult {
            description: format!("rate: {} (no threshold)", rate.event_type),
            result: TestResult::Unknown,
        };
    }

    AssertionResult {
        description: "empty assertion".to_string(),
        result: TestResult::Unknown,
    }
}

/// Evaluate metrics queries against metric results from Prometheus.
///
/// Each `MetricsQuery` becomes a `TestEvaluation` named `"metrics:{query.name}"`.
/// Missing results produce `TestResult::Unknown`.
/// Multiple results per query use worst-case (highest) value for IncreaseBad-style
/// comparisons and lowest for DecreaseBad, defaulting to worst-case for the operator.
pub fn evaluate_metrics_queries(
    queries: &[MetricsQuery],
    results: &[MetricResult],
) -> Vec<TestEvaluation> {
    queries
        .iter()
        .map(|q| evaluate_metrics_query(q, results))
        .collect()
}

/// Evaluate statistical comparisons of baseline vs canary metric distributions.
///
/// Each comparison runs a Mann-Whitney U test and classifies the result.
/// Returns both per-comparison test evaluations and an aggregate score.
pub fn evaluate_statistical_comparisons(
    comparisons: &[StatisticalComparison],
    value_pairs: &[(&[f64], &[f64])],
) -> (Vec<TestEvaluation>, AggregateScore) {
    let mut evaluations = Vec::new();
    let mut analyses = Vec::new();

    for (comp, (baseline, canary)) in comparisons.iter().zip(value_pairs.iter()) {
        let mw_result = mann_whitney_u(baseline, canary);

        let baseline_mean = if baseline.is_empty() {
            0.0
        } else {
            baseline.iter().sum::<f64>() / baseline.len() as f64
        };
        let canary_mean = if canary.is_empty() {
            0.0
        } else {
            canary.iter().sum::<f64>() / canary.len() as f64
        };

        let classification = classify(
            mw_result.p_value,
            &comp.direction,
            baseline_mean,
            canary_mean,
        );

        let test_result = match classification {
            MetricClassification::Fail => TestResult::Fail,
            _ => TestResult::Pass,
        };

        evaluations.push(TestEvaluation {
            test_name: format!("stats:{}", comp.name),
            result: test_result.clone(),
            assertion_results: vec![AssertionResult {
                description: format!(
                    "Mann-Whitney U: p={:.4}, baseline_mean={:.4}, canary_mean={:.4}",
                    mw_result.p_value, baseline_mean, canary_mean
                ),
                result: test_result,
            }],
        });

        analyses.push(MetricAnalysis {
            name: comp.name.clone(),
            baseline_mean,
            canary_mean,
            p_value: mw_result.p_value,
            direction: comp.direction.clone(),
            classification,
            weight: comp.weight,
        });
    }

    let score = aggregate_score(&analyses);
    (evaluations, score)
}

fn evaluate_metrics_query(query: &MetricsQuery, results: &[MetricResult]) -> TestEvaluation {
    let test_name = format!("metrics:{}", query.name);

    // Find all results matching this query (by query name)
    let matching: Vec<&MetricResult> = results.iter().filter(|r| r.name == query.name).collect();

    if matching.is_empty() {
        return TestEvaluation {
            test_name,
            result: TestResult::Unknown,
            assertion_results: vec![AssertionResult {
                description: format!("metrics query '{}': no results", query.name),
                result: TestResult::Unknown,
            }],
        };
    }

    let (threshold, operator) = match (&query.threshold, &query.operator) {
        (Some(t), Some(op)) => (*t, op),
        _ => {
            return TestEvaluation {
                test_name,
                result: TestResult::Unknown,
                assertion_results: vec![AssertionResult {
                    description: format!(
                        "metrics query '{}': no threshold/operator configured",
                        query.name
                    ),
                    result: TestResult::Unknown,
                }],
            };
        }
    };

    // Use the worst-case value across all matching results.
    // For LessThan / LessThanOrEqual: worst case is the maximum value.
    // For GreaterThan / GreaterThanOrEqual: worst case is the minimum value.
    let worst_value = match operator {
        RateOperator::LessThan | RateOperator::LessThanOrEqual => matching
            .iter()
            .map(|r| r.value)
            .fold(f64::NEG_INFINITY, f64::max),
        RateOperator::GreaterThan | RateOperator::GreaterThanOrEqual => matching
            .iter()
            .map(|r| r.value)
            .fold(f64::INFINITY, f64::min),
    };

    let passes = match operator {
        RateOperator::LessThan => worst_value < threshold,
        RateOperator::GreaterThan => worst_value > threshold,
        RateOperator::LessThanOrEqual => worst_value <= threshold,
        RateOperator::GreaterThanOrEqual => worst_value >= threshold,
    };

    let result = if passes {
        TestResult::Pass
    } else {
        TestResult::Fail
    };

    TestEvaluation {
        test_name,
        result: result.clone(),
        assertion_results: vec![AssertionResult {
            description: format!(
                "metrics:{} value={:.4} {:?} {:.4}",
                query.name, worst_value, operator, threshold
            ),
            result,
        }],
    }
}
