#![cfg(feature = "operator")]

use canary_gate::config::*;
use canary_gate::operator::crd::*;

#[test]
fn crd_to_config_minimal() {
    let spec = CanaryGateSpec {
        target_ref: TargetRef {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: "my-app".to_string(),
        },
        pod_selector: None,
        evaluation: None,
        logging: None,
        tests: vec![TestConfig {
            name: "service_starts".to_string(),
            severity: FailSeverity::Hard,
            then: vec![TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "server_started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            }],
        }],
        packs: vec![],
        recommendation: None,
        metrics: None,
    };

    let config: canary_gate::config::Config = (&spec).into();

    assert_eq!(config.tests.len(), 1);
    assert_eq!(config.tests[0].name, "service_starts");
    assert!(config.packs.is_empty());
    assert!(config.metrics.is_none());
}

#[test]
fn crd_to_config_with_evaluation() {
    let spec = CanaryGateSpec {
        target_ref: TargetRef {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: "my-app".to_string(),
        },
        pod_selector: Some("app=my-app,track=canary".to_string()),
        evaluation: Some(EvaluationConfig {
            interval: "15s".to_string(),
            lookback: "30s".to_string(),
            min_cycles: 3,
            max_duration: "10m".to_string(),
        }),
        logging: Some(LoggingConfig {
            format: LogFormat::Json,
            events: vec![EventConfig {
                event_type: "server_started".to_string(),
                level: EventLevel::Info,
                match_rule: MatchRule {
                    any: vec![MatchCondition {
                        contains: Some("listening".to_string()),
                        regex: None,
                    }],
                    all: vec![],
                    none: vec![],
                },
            }],
        }),
        tests: vec![TestConfig {
            name: "service_starts".to_string(),
            severity: FailSeverity::Hard,
            then: vec![TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "server_started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            }],
        }],
        packs: vec!["runtime-basic".to_string()],
        recommendation: Some(RecommendationConfig {
            promote: PromoteConfig {
                require_min_cycles: 3,
                require_consecutive_passes: 2,
            },
            rollback: RollbackConfig {
                soft_fail_consecutive_cycles: 5,
            },
            bias: VerdictBias::HoldOnAmbiguity,
        }),
        metrics: None,
    };

    let config: canary_gate::config::Config = (&spec).into();

    assert_eq!(config.evaluation.interval, "15s");
    assert_eq!(config.evaluation.min_cycles, 3);
    assert_eq!(config.logging.format, LogFormat::Json);
    assert_eq!(config.logging.events.len(), 1);
    assert_eq!(config.tests.len(), 1);
    assert_eq!(config.packs, vec!["runtime-basic"]);
    assert_eq!(config.recommendation.promote.require_min_cycles, 3);
    assert_eq!(config.recommendation.promote.require_consecutive_passes, 2);
    assert_eq!(
        config.recommendation.rollback.soft_fail_consecutive_cycles,
        5
    );
}

#[test]
fn crd_to_config_with_metrics() {
    let spec = CanaryGateSpec {
        target_ref: TargetRef {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: "my-app".to_string(),
        },
        pod_selector: None,
        evaluation: None,
        logging: None,
        tests: vec![TestConfig {
            name: "basic".to_string(),
            severity: FailSeverity::Hard,
            then: vec![TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            }],
        }],
        packs: vec![],
        recommendation: None,
        metrics: Some(MetricsSourceConfig {
            source_type: MetricsSourceType::Prometheus,
            endpoint: "http://prometheus:9090".to_string(),
            queries: vec![MetricsQuery {
                name: "error_rate".to_string(),
                query: "rate(http_errors_total[5m])".to_string(),
                threshold: Some(0.05),
                operator: Some(RateOperator::LessThan),
                severity: FailSeverity::Hard,
            }],
            comparisons: vec![],
        }),
    };

    let config: canary_gate::config::Config = (&spec).into();

    assert!(config.metrics.is_some());
    let metrics = config.metrics.unwrap();
    assert_eq!(metrics.endpoint, "http://prometheus:9090");
    assert_eq!(metrics.queries.len(), 1);
    assert_eq!(metrics.queries[0].name, "error_rate");
}

#[test]
fn crd_to_config_defaults() {
    let spec = CanaryGateSpec {
        target_ref: TargetRef {
            api_version: "apps/v1".to_string(),
            kind: "Deployment".to_string(),
            name: "my-app".to_string(),
        },
        pod_selector: None,
        evaluation: None,
        logging: None,
        tests: vec![TestConfig {
            name: "basic".to_string(),
            severity: FailSeverity::Hard,
            then: vec![TestAssertion {
                event_present: Some(EventPresentAssertion {
                    event_type: "started".to_string(),
                    within: None,
                }),
                event_absent: None,
                rate: None,
            }],
        }],
        packs: vec![],
        recommendation: None,
        metrics: None,
    };

    let config: canary_gate::config::Config = (&spec).into();

    // Defaults should be applied
    assert_eq!(config.evaluation.min_cycles, 5); // default
    assert_eq!(config.recommendation.promote.require_min_cycles, 5);
    assert_eq!(config.recommendation.promote.require_consecutive_passes, 2);
    assert_eq!(
        config.recommendation.rollback.soft_fail_consecutive_cycles,
        3
    );
    assert_eq!(config.recommendation.bias, VerdictBias::HoldOnAmbiguity);
}

#[test]
fn canary_gate_phase_display() {
    assert_eq!(CanaryGatePhase::Pending.to_string(), "Pending");
    assert_eq!(CanaryGatePhase::Evaluating.to_string(), "Evaluating");
    assert_eq!(CanaryGatePhase::Promote.to_string(), "Promote");
    assert_eq!(CanaryGatePhase::Hold.to_string(), "Hold");
    assert_eq!(CanaryGatePhase::Rollback.to_string(), "Rollback");
}

#[test]
fn parse_severity_values() {
    assert_eq!(parse_severity("soft"), FailSeverity::Soft);
    assert_eq!(parse_severity("Soft"), FailSeverity::Soft);
    assert_eq!(parse_severity("SOFT"), FailSeverity::Soft);
    assert_eq!(parse_severity("hard"), FailSeverity::Hard);
    assert_eq!(parse_severity("unknown"), FailSeverity::Hard);
}

#[test]
fn target_ref_serialization() {
    let target = TargetRef {
        api_version: "apps/v1".to_string(),
        kind: "Deployment".to_string(),
        name: "my-app".to_string(),
    };
    let json = serde_json::to_value(&target).unwrap();
    assert_eq!(json["apiVersion"], "apps/v1");
    assert_eq!(json["kind"], "Deployment");
    assert_eq!(json["name"], "my-app");
}

#[test]
fn canary_gate_status_serialization() {
    let status = CanaryGateStatus {
        phase: CanaryGatePhase::Evaluating,
        recommendation: Some("hold".to_string()),
        cycles: 3,
        consecutive_passes: 2,
        conditions: vec![CanaryGateCondition {
            condition_type: "Ready".to_string(),
            status: "True".to_string(),
            message: "Evaluation in progress".to_string(),
            last_transition_time: None,
        }],
        reasoning: vec!["Not enough cycles".to_string()],
    };

    let json = serde_json::to_value(&status).unwrap();
    assert_eq!(json["phase"], "Evaluating");
    assert_eq!(json["recommendation"], "hold");
    assert_eq!(json["cycles"], 3);
    assert_eq!(json["consecutive_passes"], 2);
    assert_eq!(json["conditions"].as_array().unwrap().len(), 1);
    assert_eq!(json["reasoning"].as_array().unwrap().len(), 1);
}
