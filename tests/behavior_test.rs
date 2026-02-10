use canary_gate::behavior::*;
use canary_gate::config::*;
use canary_gate::events::CanonicalEvent;

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
