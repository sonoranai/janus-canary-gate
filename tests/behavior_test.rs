use canary_gate::behavior::*;
use canary_gate::config::*;
use canary_gate::events::CanonicalEvent;
use canary_gate::metrics::MetricResult;
use std::collections::HashMap;

fn make_event(event_type: &str, level: EventLevel) -> CanonicalEvent {
    CanonicalEvent {
        timestamp: "2024-01-15T10:30:00Z".to_string(),
        level,
        event_type: event_type.to_string(),
        fingerprint: canary_gate::events::fingerprint(event_type, &EventLevel::Info),
        raw_line: None,
    }
}

fn make_test(name: &str, assertions: Vec<TestAssertion>) -> TestConfig {
    TestConfig {
        name: name.to_string(),
        severity: FailSeverity::Hard,
        then: assertions,
    }
}

#[test]
fn event_present_with_event_in_stream() {
    let events = vec![make_event("grpc_server_started", EventLevel::Info)];
    let test = make_test(
        "service_starts",
        vec![TestAssertion {
            event_present: Some(EventPresentAssertion {
                event_type: "grpc_server_started".to_string(),
                within: None,
            }),
            event_absent: None,
            rate: None,
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result.len(), 1);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn event_present_with_event_missing() {
    let events = vec![make_event("http_server_started", EventLevel::Info)];
    let test = make_test(
        "service_starts",
        vec![TestAssertion {
            event_present: Some(EventPresentAssertion {
                event_type: "grpc_server_started".to_string(),
                within: None,
            }),
            event_absent: None,
            rate: None,
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Fail);
}

#[test]
fn event_absent_with_event_missing() {
    let events = vec![make_event("grpc_server_started", EventLevel::Info)];
    let test = make_test(
        "no_panics",
        vec![TestAssertion {
            event_present: None,
            event_absent: Some(EventAbsentAssertion {
                event_type: "panic".to_string(),
            }),
            rate: None,
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn event_absent_with_event_present() {
    let events = vec![
        make_event("grpc_server_started", EventLevel::Info),
        make_event("panic", EventLevel::Fatal),
    ];
    let test = make_test(
        "no_panics",
        vec![TestAssertion {
            event_present: None,
            event_absent: Some(EventAbsentAssertion {
                event_type: "panic".to_string(),
            }),
            rate: None,
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Fail);
}

#[test]
fn rate_below_threshold_passes() {
    let events = vec![
        make_event("http_5xx", EventLevel::Error),
        make_event("http_5xx", EventLevel::Error),
    ];
    let test = make_test(
        "low_error_rate",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "http_5xx".to_string(),
                threshold: Some(5.0),
                operator: Some(RateOperator::LessThan),
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn rate_above_threshold_fails() {
    let events: Vec<CanonicalEvent> = (0..10)
        .map(|_| make_event("http_5xx", EventLevel::Error))
        .collect();

    let test = make_test(
        "low_error_rate",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "http_5xx".to_string(),
                threshold: Some(5.0),
                operator: Some(RateOperator::LessThan),
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Fail);
}

#[test]
fn rate_without_threshold_is_unknown() {
    let events = vec![make_event("http_5xx", EventLevel::Error)];
    let test = make_test(
        "rate_check",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "http_5xx".to_string(),
                threshold: None,
                operator: None,
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Unknown);
}

#[test]
fn multiple_assertions_all_must_pass() {
    let events = vec![make_event("grpc_server_started", EventLevel::Info)];
    let test = make_test(
        "healthy_startup",
        vec![
            TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "grpc_server_started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            },
            TestAssertion {
                event_present: None,
                event_absent: Some(EventAbsentAssertion {
                    event_type: "panic".to_string(),
                }),
                rate: None,
            },
        ],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn one_failing_assertion_fails_test() {
    let events = vec![make_event("panic", EventLevel::Fatal)];
    let test = make_test(
        "healthy_startup",
        vec![
            TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "grpc_server_started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            },
            TestAssertion {
                event_present: None,
                event_absent: Some(EventAbsentAssertion {
                    event_type: "panic".to_string(),
                }),
                rate: None,
            },
        ],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Fail);
}

#[test]
fn empty_event_stream() {
    let events: Vec<CanonicalEvent> = vec![];
    let test = make_test(
        "service_starts",
        vec![TestAssertion {
            event_present: Some(EventPresentAssertion {
                event_type: "grpc_server_started".to_string(),
                within: None,
            }),
            event_absent: None,
            rate: None,
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Fail);
}

#[test]
fn rate_greater_than_operator() {
    let events = vec![
        make_event("request", EventLevel::Info),
        make_event("request", EventLevel::Info),
        make_event("request", EventLevel::Info),
    ];
    let test = make_test(
        "enough_requests",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "request".to_string(),
                threshold: Some(2.0),
                operator: Some(RateOperator::GreaterThan),
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn rate_less_than_or_equal_at_boundary() {
    let events = vec![
        make_event("http_5xx", EventLevel::Error),
        make_event("http_5xx", EventLevel::Error),
    ];
    let test = make_test(
        "error_rate_lte",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "http_5xx".to_string(),
                threshold: Some(2.0),
                operator: Some(RateOperator::LessThanOrEqual),
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

#[test]
fn rate_greater_than_or_equal_at_boundary() {
    let events = vec![
        make_event("request", EventLevel::Info),
        make_event("request", EventLevel::Info),
        make_event("request", EventLevel::Info),
    ];
    let test = make_test(
        "request_rate_gte",
        vec![TestAssertion {
            event_present: None,
            event_absent: None,
            rate: Some(RateAssertion {
                event_type: "request".to_string(),
                threshold: Some(3.0),
                operator: Some(RateOperator::GreaterThanOrEqual),
            }),
        }],
    );

    let result = evaluate_tests(&[test], &events);
    assert_eq!(result[0].result, TestResult::Pass);
}

// --- Metrics evaluation tests ---

fn make_metric(name: &str, value: f64) -> MetricResult {
    MetricResult {
        name: name.to_string(),
        value,
        labels: HashMap::new(),
    }
}

fn make_metrics_query(name: &str, threshold: f64, operator: RateOperator) -> MetricsQuery {
    MetricsQuery {
        name: name.to_string(),
        query: format!("rate({}[5m])", name),
        threshold: Some(threshold),
        operator: Some(operator),
        severity: FailSeverity::Hard,
    }
}

#[test]
fn metrics_below_threshold_passes() {
    let queries = vec![make_metrics_query(
        "error_rate",
        0.05,
        RateOperator::LessThan,
    )];
    let results = vec![make_metric("error_rate", 0.01)];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals.len(), 1);
    assert_eq!(evals[0].test_name, "metrics:error_rate");
    assert_eq!(evals[0].result, TestResult::Pass);
}

#[test]
fn metrics_above_threshold_fails() {
    let queries = vec![make_metrics_query(
        "error_rate",
        0.05,
        RateOperator::LessThan,
    )];
    let results = vec![make_metric("error_rate", 0.10)];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Fail);
}

#[test]
fn metrics_missing_results_unknown() {
    let queries = vec![make_metrics_query(
        "error_rate",
        0.05,
        RateOperator::LessThan,
    )];
    let results: Vec<MetricResult> = vec![];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Unknown);
}

#[test]
fn metrics_no_threshold_unknown() {
    let queries = vec![MetricsQuery {
        name: "error_rate".to_string(),
        query: "rate(errors[5m])".to_string(),
        threshold: None,
        operator: None,
        severity: FailSeverity::Hard,
    }];
    let results = vec![make_metric("error_rate", 0.01)];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Unknown);
}

#[test]
fn metrics_multiple_results_worst_case_less_than() {
    // For LessThan, worst case is the max value
    let queries = vec![make_metrics_query(
        "error_rate",
        0.05,
        RateOperator::LessThan,
    )];
    let results = vec![
        make_metric("error_rate", 0.01),
        make_metric("error_rate", 0.03),
        make_metric("error_rate", 0.10), // This breaches threshold
    ];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Fail);
}

#[test]
fn metrics_multiple_results_worst_case_greater_than() {
    // For GreaterThan, worst case is the min value
    let queries = vec![make_metrics_query(
        "throughput",
        100.0,
        RateOperator::GreaterThan,
    )];
    let results = vec![
        make_metric("throughput", 150.0),
        make_metric("throughput", 80.0), // This fails threshold
    ];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Fail);
}

#[test]
fn metrics_greater_than_or_equal_at_boundary() {
    let queries = vec![make_metrics_query(
        "throughput",
        100.0,
        RateOperator::GreaterThanOrEqual,
    )];
    let results = vec![make_metric("throughput", 100.0)];

    let evals = evaluate_metrics_queries(&queries, &results);
    assert_eq!(evals[0].result, TestResult::Pass);
}
