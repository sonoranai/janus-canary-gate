use axum::body::Body;
use axum::http::{Request, StatusCode};
use canary_gate::api;
use canary_gate::db::Database;
use canary_gate::recommendation::Recommendation;
use canary_gate::verdict::Verdict;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

fn make_state() -> api::SharedState {
    let db = Database::open_in_memory().unwrap();
    Arc::new(api::AppState {
        db: Mutex::new(db),
        start_time: std::time::Instant::now(),
        version: "0.1.0-test".to_string(),
        config: None,
        last_verdict: Mutex::new(None),
    })
}

fn make_state_with_data() -> api::SharedState {
    let db = Database::open_in_memory().unwrap();
    let verdict = Verdict {
        recommendation: Recommendation::Promote,
        total_cycles: 5,
        consecutive_passes: 3,
        test_results: vec![],
        reasoning: vec!["all tests passed".to_string()],
        statistical_score: None,
    };
    db.insert_evaluation("deploy-1", "hash1", &verdict).unwrap();

    Arc::new(api::AppState {
        db: Mutex::new(db),
        start_time: std::time::Instant::now(),
        version: "0.1.0-test".to_string(),
        config: None,
        last_verdict: Mutex::new(Some(verdict)),
    })
}

fn make_verdict(recommendation: Recommendation) -> Verdict {
    let reasoning = match recommendation {
        Recommendation::Promote => vec!["all tests passing for 3 consecutive cycles".to_string()],
        Recommendation::Hold => {
            vec![
                "completed 2 cycles, 1 consecutive passes — not yet meeting promote criteria"
                    .to_string(),
            ]
        }
        Recommendation::Rollback => vec!["hard failure detected — immediate rollback".to_string()],
    };
    Verdict {
        recommendation,
        total_cycles: 5,
        consecutive_passes: 3,
        test_results: vec![],
        reasoning,
        statistical_score: None,
    }
}

fn make_state_with_verdict(recommendation: Recommendation) -> api::SharedState {
    let db = Database::open_in_memory().unwrap();
    let verdict = make_verdict(recommendation);
    Arc::new(api::AppState {
        db: Mutex::new(db),
        start_time: std::time::Instant::now(),
        version: "0.1.0-test".to_string(),
        config: None,
        last_verdict: Mutex::new(Some(verdict)),
    })
}

#[tokio::test]
async fn health_endpoint() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/health")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["status"], "ok");
    assert_eq!(json["version"], "0.1.0-test");
}

#[tokio::test]
async fn current_evaluation_not_found() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json["error"]["code"].as_str().is_some());
}

#[tokio::test]
async fn current_evaluation_found() {
    let app = api::router(make_state_with_data());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["deployment_id"], "deploy-1");
    assert_eq!(json["recommendation"], "promote");
}

#[tokio::test]
async fn get_evaluation_by_id() {
    let app = api::router(make_state_with_data());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations/1")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}

#[tokio::test]
async fn get_evaluation_not_found() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations/999")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn list_evaluations_empty() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert!(json.as_array().unwrap().is_empty());
}

#[tokio::test]
async fn list_evaluations_with_data() {
    let app = api::router(make_state_with_data());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn metrics_endpoint() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/metrics")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let text = String::from_utf8(body.to_vec()).unwrap();
    assert!(text.contains("canary_gate_evaluations_total"));
}

#[tokio::test]
async fn error_envelope_format() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/v1/evaluations/current")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();

    // Verify error envelope structure
    assert!(json["error"].is_object());
    assert!(json["error"]["code"].is_string());
    assert!(json["error"]["message"].is_string());
}

// --- Evaluate endpoint tests ---

#[tokio::test]
async fn evaluate_endpoint_no_config() {
    let app = api::router(make_state());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"log_lines":[]}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["error"]["code"], "bad_request");
}

#[tokio::test]
async fn evaluate_endpoint_with_config() {
    let db = Database::open_in_memory().unwrap();
    let config = canary_gate::config::parse_config(
        r#"
tests:
  - name: "startup_check"
    then:
      - event_present:
          type: "app_started"
logging:
  events:
    - type: "app_started"
      level: info
      match:
        any:
          - contains: "Application started"
"#,
    )
    .unwrap();

    let state = Arc::new(api::AppState {
        db: Mutex::new(db),
        start_time: std::time::Instant::now(),
        version: "0.1.0-test".to_string(),
        config: Some(config),
        last_verdict: Mutex::new(None),
    });

    let app = api::router(state.clone());
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/evaluate")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"log_lines":["Application started successfully"]}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    // With only 1 cycle and default min_cycles=5, recommendation should be hold
    assert_eq!(json["recommendation"], "hold");

    // Verify last_verdict was stored
    let lv = state.last_verdict.lock().unwrap();
    assert!(lv.is_some());
}

// --- Argo Rollouts webhook tests ---

#[tokio::test]
async fn argo_webhook_promote() {
    let state = make_state_with_verdict(Recommendation::Promote);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/argo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"metadata":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["recommendation"], "promote");
    assert_eq!(json["score"], 100);
    assert_eq!(json["passed"], true);
}

#[tokio::test]
async fn argo_webhook_hold() {
    let state = make_state_with_verdict(Recommendation::Hold);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/argo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"metadata":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["recommendation"], "hold");
    assert_eq!(json["score"], 50);
    assert_eq!(json["passed"], false);
}

#[tokio::test]
async fn argo_webhook_rollback() {
    let state = make_state_with_verdict(Recommendation::Rollback);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/argo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"metadata":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["recommendation"], "rollback");
    assert_eq!(json["score"], 0);
    assert_eq!(json["passed"], false);
}

#[tokio::test]
async fn argo_webhook_no_verdict() {
    let state = make_state();
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/argo")
                .header("content-type", "application/json")
                .body(Body::from(r#"{"metadata":{}}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["recommendation"], "hold");
    assert_eq!(json["score"], 50);
    assert_eq!(json["passed"], false);
    assert!(json["metadata"]["reason_0"]
        .as_str()
        .unwrap()
        .contains("no evaluation"));
}

// --- Flagger webhook tests ---

#[tokio::test]
async fn flagger_webhook_promote() {
    let state = make_state_with_verdict(Recommendation::Promote);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/flagger")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"canary","namespace":"default","phase":"Progressing","metadata":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["passed"], true);
    assert!(json.get("message").is_none());
}

#[tokio::test]
async fn flagger_webhook_hold() {
    let state = make_state_with_verdict(Recommendation::Hold);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/flagger")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"canary","namespace":"default","phase":"Progressing","metadata":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["passed"], false);
}

#[tokio::test]
async fn flagger_webhook_rollback() {
    let state = make_state_with_verdict(Recommendation::Rollback);
    let app = api::router(state);
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/api/v1/webhooks/flagger")
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"name":"canary","namespace":"default","phase":"Progressing","metadata":{}}"#,
                ))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(json["passed"], false);
    assert!(json["message"].as_str().unwrap().contains("rollback"));
}
