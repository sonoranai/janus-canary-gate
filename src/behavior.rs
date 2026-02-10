use crate::config::{TestAssertion, TestConfig};
use crate::events::CanonicalEvent;

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
