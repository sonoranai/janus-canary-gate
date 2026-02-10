use canary_gate::config::*;
use std::path::Path;

fn fixture(name: &str) -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/configs")
        .join(name)
}

#[test]
fn load_valid_minimal_config() {
    let config = load_config(&fixture("valid_minimal.yaml")).unwrap();
    assert_eq!(config.tests.len(), 1);
    assert_eq!(config.tests[0].name, "service_starts");
    assert_eq!(config.logging.format, LogFormat::Plaintext);
}

#[test]
fn load_valid_full_config() {
    let config = load_config(&fixture("valid_full.yaml")).unwrap();
    assert_eq!(config.tests.len(), 3);
    assert_eq!(config.logging.events.len(), 5);
    assert_eq!(config.recommendation.promote.require_min_cycles, 5);
    assert_eq!(config.recommendation.promote.require_consecutive_passes, 2);
    assert_eq!(
        config.recommendation.rollback.soft_fail_consecutive_cycles,
        3
    );
    assert_eq!(config.recommendation.bias, VerdictBias::HoldOnAmbiguity);
}

#[test]
fn load_valid_with_packs() {
    let config = load_config(&fixture("valid_with_packs.yaml")).unwrap();
    assert_eq!(config.packs.len(), 2);
    assert!(config.packs.contains(&"runtime-basic".to_string()));
    assert!(config.packs.contains(&"grpc-server".to_string()));
}

#[test]
fn invalid_missing_tests_fails() {
    let result = load_config(&fixture("invalid_missing_tests.yaml"));
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("at least one test, pack, metrics query, or comparison"),
        "Expected 'at least one test, pack, metrics query, or comparison', got: {}",
        err
    );
}

#[test]
fn invalid_bad_enum_fails() {
    let result = load_config(&fixture("invalid_bad_enum.yaml"));
    assert!(result.is_err());
}

#[test]
fn invalid_schema_fails() {
    // This file has unknown fields but serde_yaml with deny_unknown_fields
    // would reject it. Since we use default deserialization, it may succeed
    // but fail validation due to missing tests/packs.
    let result = load_config(&fixture("invalid_schema.yaml"));
    assert!(result.is_err());
}

#[test]
fn defaults_applied_when_omitted() {
    let yaml = r#"
tests:
  - name: test1
    then:
      - event_present:
          type: foo
"#;
    let config = parse_config(yaml).unwrap();

    assert_eq!(config.logstream.start, StartMode::Beginning);
    assert_eq!(config.logstream.lookback, "60s");
    assert_eq!(config.evaluation.interval, "30s");
    assert_eq!(config.evaluation.min_cycles, 5);
    assert_eq!(config.evaluation.max_duration, "15m");
    assert_eq!(config.logging.format, LogFormat::Auto);
    assert_eq!(config.recommendation.promote.require_min_cycles, 5);
    assert_eq!(config.recommendation.promote.require_consecutive_passes, 2);
    assert_eq!(config.recommendation.bias, VerdictBias::HoldOnAmbiguity);
}

#[test]
fn test_severity_defaults_to_hard() {
    let yaml = r#"
tests:
  - name: test1
    then:
      - event_present:
          type: foo
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.tests[0].severity, FailSeverity::Hard);
}

#[test]
fn event_config_with_all_combinators() {
    let yaml = r#"
logging:
  events:
    - type: combined_event
      level: info
      match:
        all:
          - contains: "ready"
          - contains: "accepting"
        none:
          - contains: "error"

tests:
  - name: test1
    then:
      - event_present:
          type: combined_event
"#;
    let config = parse_config(yaml).unwrap();
    assert_eq!(config.logging.events.len(), 1);
    assert_eq!(config.logging.events[0].match_rule.all.len(), 2);
    assert_eq!(config.logging.events[0].match_rule.none.len(), 1);
}

#[test]
fn empty_test_then_fails_validation() {
    let yaml = r#"
tests:
  - name: empty_test
    then: []
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
}

#[test]
fn empty_assertion_fails_validation() {
    let yaml = r#"
tests:
  - name: bad_assertion
    then:
      - {}
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
}

#[test]
fn invalid_regex_fails_validation() {
    let yaml = r#"
logging:
  events:
    - type: bad_regex
      level: error
      match:
        any:
          - regex: "[invalid(regex"

tests:
  - name: test1
    then:
      - event_present:
          type: bad_regex
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("invalid regex"),
        "Expected 'invalid regex', got: {}",
        err
    );
}

#[test]
fn zero_min_cycles_fails_validation() {
    let yaml = r#"
recommendation:
  promote:
    require_min_cycles: 0
    require_consecutive_passes: 2

tests:
  - name: test1
    then:
      - event_present:
          type: foo
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("require_min_cycles"),
        "Expected 'require_min_cycles', got: {}",
        err
    );
}

#[test]
fn zero_consecutive_passes_fails_validation() {
    let yaml = r#"
recommendation:
  promote:
    require_min_cycles: 5
    require_consecutive_passes: 0

tests:
  - name: test1
    then:
      - event_present:
          type: foo
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("require_consecutive_passes"),
        "Expected 'require_consecutive_passes', got: {}",
        err
    );
}

#[test]
fn empty_match_rule_fails_validation() {
    let yaml = r#"
logging:
  events:
    - type: empty_match
      level: info
      match:
        any: []
        all: []
        none: []

tests:
  - name: test1
    then:
      - event_present:
          type: empty_match
"#;
    let result = parse_config(yaml);
    assert!(result.is_err());
    let err = result.unwrap_err().to_string();
    assert!(
        err.contains("match rule"),
        "Expected 'match rule', got: {}",
        err
    );
}
