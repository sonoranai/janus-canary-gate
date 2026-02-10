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
    };
    db.insert_evaluation("deploy-1", "hash1", &verdict).unwrap();

    Arc::new(api::AppState {
        db: Mutex::new(db),
        start_time: std::time::Instant::now(),
        version: "0.1.0-test".to_string(),
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
