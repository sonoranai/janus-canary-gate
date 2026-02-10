use canary_gate::metrics::prometheus::parse_prometheus_response;
use std::path::Path;

fn fixture(name: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/prometheus")
        .join(name);
    std::fs::read_to_string(path).unwrap()
}

#[test]
fn parse_healthy_response() {
    let body: serde_json::Value = serde_json::from_str(&fixture("healthy_response.json")).unwrap();
    let results = parse_prometheus_response(&body).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "http_requests_total");
    assert_eq!(results[0].value, 1542.0);
    assert_eq!(results[1].value, 328.0);
}

#[test]
fn parse_error_rate_spike() {
    let body: serde_json::Value = serde_json::from_str(&fixture("error_rate_spike.json")).unwrap();
    let results = parse_prometheus_response(&body).unwrap();
    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "http_errors_total");
    assert_eq!(results[0].value, 47.0);
}

#[test]
fn parse_latency_degradation() {
    let body: serde_json::Value =
        serde_json::from_str(&fixture("latency_degradation.json")).unwrap();
    let results = parse_prometheus_response(&body).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].value, 4.25);
}

#[test]
fn parse_error_response() {
    let body: serde_json::Value = serde_json::from_str(
        r#"{"status": "error", "error": "bad query", "errorType": "bad_data"}"#,
    )
    .unwrap();
    let result = parse_prometheus_response(&body);
    assert!(result.is_err());
    assert!(result.unwrap_err().to_string().contains("bad query"));
}

#[test]
fn parse_empty_result() {
    let body: serde_json::Value = serde_json::from_str(
        r#"{"status": "success", "data": {"resultType": "vector", "result": []}}"#,
    )
    .unwrap();
    let results = parse_prometheus_response(&body).unwrap();
    assert!(results.is_empty());
}

#[test]
fn metric_labels_extracted() {
    let body: serde_json::Value = serde_json::from_str(&fixture("healthy_response.json")).unwrap();
    let results = parse_prometheus_response(&body).unwrap();
    assert_eq!(results[0].labels.get("method").unwrap(), "GET");
    assert_eq!(results[0].labels.get("status").unwrap(), "200");
}
