use canary_gate::classification::{classify_line, classify_stream, CompiledRules};
use canary_gate::config::*;
use canary_gate::events::fingerprint;
use canary_gate::ingestion::RawLogLine;

/// Helper to classify with pre-compiled rules.
fn classify(
    line: &RawLogLine,
    rules: &[EventConfig],
) -> Option<canary_gate::events::CanonicalEvent> {
    let compiled = CompiledRules::new(rules);
    classify_line(line, rules, &compiled)
}

fn make_line(content: &str) -> RawLogLine {
    RawLogLine {
        content: content.to_string(),
        line_number: 1,
        timestamp: Some("2024-01-15T10:30:00Z".to_string()),
        is_json: false,
        source: None,
    }
}

fn grpc_rule() -> EventConfig {
    EventConfig {
        event_type: "grpc_server_started".to_string(),
        level: EventLevel::Info,
        match_rule: MatchRule {
            any: vec![
                MatchCondition {
                    contains: Some("gRPC server listening".to_string()),
                    regex: None,
                },
                MatchCondition {
                    contains: Some("Started gRPC server".to_string()),
                    regex: None,
                },
            ],
            all: vec![],
            none: vec![],
        },
    }
}

fn panic_rule() -> EventConfig {
    EventConfig {
        event_type: "panic".to_string(),
        level: EventLevel::Fatal,
        match_rule: MatchRule {
            any: vec![
                MatchCondition {
                    contains: Some("panic:".to_string()),
                    regex: None,
                },
                MatchCondition {
                    contains: Some("fatal error".to_string()),
                    regex: None,
                },
            ],
            all: vec![],
            none: vec![],
        },
    }
}

#[test]
fn classify_matching_line() {
    let rules = vec![grpc_rule()];
    let line = make_line("2024-01-15 INFO gRPC server listening on 0.0.0.0:50051");
    let event = classify(&line, &rules);
    assert!(event.is_some());
    let event = event.unwrap();
    assert_eq!(event.event_type, "grpc_server_started");
    assert_eq!(event.level, EventLevel::Info);
}

#[test]
fn classify_nonmatching_line() {
    let rules = vec![grpc_rule()];
    let line = make_line("2024-01-15 INFO Loading configuration");
    let event = classify(&line, &rules);
    assert!(event.is_none());
}

#[test]
fn first_match_wins() {
    // Both rules could match a line with "panic:" — first one wins
    let broad_rule = EventConfig {
        event_type: "broad_error".to_string(),
        level: EventLevel::Error,
        match_rule: MatchRule {
            any: vec![MatchCondition {
                contains: Some("panic:".to_string()),
                regex: None,
            }],
            all: vec![],
            none: vec![],
        },
    };
    let rules = vec![broad_rule, panic_rule()];
    let line = make_line("panic: something went wrong");
    let event = classify(&line, &rules).unwrap();
    assert_eq!(event.event_type, "broad_error");
}

#[test]
fn fingerprint_determinism() {
    let fp1 = fingerprint("grpc_server_started", &EventLevel::Info);
    let fp2 = fingerprint("grpc_server_started", &EventLevel::Info);
    assert_eq!(fp1, fp2);
}

#[test]
fn fingerprint_differs_by_type() {
    let fp1 = fingerprint("grpc_server_started", &EventLevel::Info);
    let fp2 = fingerprint("http_server_started", &EventLevel::Info);
    assert_ne!(fp1, fp2);
}

#[test]
fn fingerprint_differs_by_level() {
    let fp1 = fingerprint("event", &EventLevel::Info);
    let fp2 = fingerprint("event", &EventLevel::Error);
    assert_ne!(fp1, fp2);
}

#[test]
fn classify_stream_filters_unmatched() {
    let rules = vec![grpc_rule(), panic_rule()];
    let lines = vec![
        make_line("INFO Loading config"),
        make_line("INFO gRPC server listening on 0.0.0.0:50051"),
        make_line("DEBUG Some debug log"),
        make_line("FATAL panic: runtime error"),
    ];
    let events = classify_stream(&lines, &rules);
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].event_type, "grpc_server_started");
    assert_eq!(events[1].event_type, "panic");
}

#[test]
fn any_combinator_matches_any_condition() {
    let rules = vec![grpc_rule()];
    // Should match second condition
    let line = make_line("Started gRPC server on port 50051");
    let event = classify(&line, &rules);
    assert!(event.is_some());
}

#[test]
fn all_combinator_requires_all_conditions() {
    let rule = EventConfig {
        event_type: "ready".to_string(),
        level: EventLevel::Info,
        match_rule: MatchRule {
            any: vec![],
            all: vec![
                MatchCondition {
                    contains: Some("ready".to_string()),
                    regex: None,
                },
                MatchCondition {
                    contains: Some("accepting".to_string()),
                    regex: None,
                },
            ],
            none: vec![],
        },
    };

    // Both conditions present
    let line1 = make_line("Service ready, accepting connections");
    assert!(classify(&line1, &[rule.clone()]).is_some());

    // Only one condition present — "accepting" is not in this line
    let line2 = make_line("Service ready, but still initializing");
    assert!(classify(&line2, &[rule.clone()]).is_none());
}

#[test]
fn none_combinator_excludes_matching() {
    let rule = EventConfig {
        event_type: "startup".to_string(),
        level: EventLevel::Info,
        match_rule: MatchRule {
            any: vec![MatchCondition {
                contains: Some("starting".to_string()),
                regex: None,
            }],
            all: vec![],
            none: vec![MatchCondition {
                contains: Some("error".to_string()),
                regex: None,
            }],
        },
    };

    // Matches 'any' but not excluded by 'none'
    let line1 = make_line("starting service");
    assert!(classify(&line1, &[rule.clone()]).is_some());

    // Matches 'any' but also matches 'none' exclusion
    let line2 = make_line("starting service with error");
    assert!(classify(&line2, &[rule]).is_none());
}

#[test]
fn regex_match_condition() {
    let rule = EventConfig {
        event_type: "http_error".to_string(),
        level: EventLevel::Error,
        match_rule: MatchRule {
            any: vec![MatchCondition {
                contains: None,
                regex: Some(r"status=5\d{2}".to_string()),
            }],
            all: vec![],
            none: vec![],
        },
    };

    let line1 = make_line("request completed status=500 method=GET");
    assert!(classify(&line1, &[rule.clone()]).is_some());

    let line2 = make_line("request completed status=200 method=GET");
    assert!(classify(&line2, &[rule]).is_none());
}
