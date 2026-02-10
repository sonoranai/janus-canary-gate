use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::behavior::evaluate_tests;
use crate::classification::classify_stream;
use crate::config::Config;
use crate::db::Database;
use crate::ingestion::RawLogLine;
use crate::recommendation::{CycleTracker, Recommendation};
use crate::verdict::Verdict;

/// Shared application state for the API server.
///
/// Database is behind a std::sync::Mutex because rusqlite::Connection
/// is not Send+Sync. We use spawn_blocking-style access through the mutex.
pub struct AppState {
    pub db: Mutex<Database>,
    pub start_time: std::time::Instant,
    pub version: String,
    pub config: Option<Config>,
    pub last_verdict: Mutex<Option<Verdict>>,
}

pub type SharedState = Arc<AppState>;

/// Standard error envelope per API spec.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorDetail,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetail {
    pub code: String,
    pub message: String,
}

/// Health check response.
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub uptime_seconds: u64,
}

/// Pagination parameters.
#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<usize>,
    pub deployment_id: Option<String>,
    pub verdict: Option<String>,
    pub since: Option<String>,
}

/// Request body for the evaluate endpoint.
#[derive(Debug, Deserialize)]
pub struct EvaluateRequest {
    #[serde(default)]
    pub log_lines: Vec<String>,
}

/// Request body for Argo Rollouts webhook.
#[derive(Debug, Deserialize)]
pub struct ArgoWebhookRequest {
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

/// Response body for Argo Rollouts webhook.
#[derive(Debug, Serialize)]
pub struct ArgoWebhookResponse {
    pub recommendation: String,
    pub score: u32,
    pub passed: bool,
    pub metadata: HashMap<String, String>,
}

/// Request body for Flagger webhook.
#[derive(Debug, Deserialize)]
pub struct FlaggerWebhookRequest {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub namespace: String,
    #[serde(default)]
    pub phase: String,
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

/// Response body for Flagger webhook.
#[derive(Debug, Serialize)]
pub struct FlaggerWebhookResponse {
    pub passed: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

/// Build the API router.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/evaluations/current", get(current_evaluation))
        .route("/api/v1/evaluations/{id}", get(get_evaluation))
        .route("/api/v1/evaluations", get(list_evaluations))
        .route("/api/v1/evaluate", post(evaluate))
        .route("/api/v1/webhooks/argo", post(argo_webhook))
        .route("/api/v1/webhooks/flagger", post(flagger_webhook))
        .route("/metrics", get(metrics))
        .with_state(state)
}

async fn health(State(state): State<SharedState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        version: state.version.clone(),
        uptime_seconds: state.start_time.elapsed().as_secs(),
    })
}

async fn current_evaluation(State(state): State<SharedState>) -> Response {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return internal_error("database lock poisoned"),
    };
    match db.get_current_evaluation() {
        Ok(Some(eval)) => json_response(eval),
        Ok(None) => not_found("no evaluations found"),
        Err(e) => internal_error(&e.to_string()),
    }
}

async fn get_evaluation(State(state): State<SharedState>, Path(id): Path<i64>) -> Response {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return internal_error("database lock poisoned"),
    };
    match db.get_evaluation(id) {
        Ok(Some(eval)) => json_response(eval),
        Ok(None) => not_found(&format!("evaluation {} not found", id)),
        Err(e) => internal_error(&e.to_string()),
    }
}

async fn list_evaluations(
    State(state): State<SharedState>,
    Query(params): Query<PaginationParams>,
) -> Response {
    let db = match state.db.lock() {
        Ok(db) => db,
        Err(_) => return internal_error("database lock poisoned"),
    };
    let limit = params.limit.unwrap_or(20);

    match db.query_history(
        params.deployment_id.as_deref(),
        params.verdict.as_deref(),
        params.since.as_deref(),
        limit,
    ) {
        Ok(evals) => json_response(evals),
        Err(e) => internal_error(&e.to_string()),
    }
}

/// Run the full evaluation pipeline and store the verdict.
async fn evaluate(
    State(state): State<SharedState>,
    body: Option<Json<EvaluateRequest>>,
) -> Response {
    let config = match &state.config {
        Some(c) => c.clone(),
        None => return bad_request("no configuration loaded; set config in AppState"),
    };

    let log_lines = body.map(|b| b.0.log_lines).unwrap_or_default();

    // Convert raw strings to RawLogLine for the classification pipeline
    let raw_lines: Vec<RawLogLine> = log_lines
        .iter()
        .enumerate()
        .map(|(i, line)| RawLogLine {
            content: line.clone(),
            line_number: i + 1,
            timestamp: None,
            is_json: false,
        })
        .collect();

    let events = classify_stream(&raw_lines, &config.logging.events);
    let evaluations = evaluate_tests(&config.tests, &events);

    let mut tracker = CycleTracker::new();
    tracker.record_cycle(&config.tests, &evaluations, &config.recommendation);

    let verdict = Verdict::from_tracker(&tracker);

    // Store in last_verdict
    if let Ok(mut lv) = state.last_verdict.lock() {
        *lv = Some(verdict.clone());
    }

    json_response(verdict)
}

/// Argo Rollouts webhook endpoint.
async fn argo_webhook(
    State(state): State<SharedState>,
    _body: Json<ArgoWebhookRequest>,
) -> Response {
    let verdict = get_last_verdict(&state);

    let (recommendation, score, passed) = match &verdict {
        Some(v) => match v.recommendation {
            Recommendation::Promote => ("promote", 100, true),
            Recommendation::Hold => ("hold", 50, false),
            Recommendation::Rollback => ("rollback", 0, false),
        },
        None => ("hold", 50, false),
    };

    let mut metadata = HashMap::new();
    if let Some(v) = &verdict {
        for (i, reason) in v.reasoning.iter().enumerate() {
            metadata.insert(format!("reason_{}", i), reason.clone());
        }
        metadata.insert("total_cycles".to_string(), v.total_cycles.to_string());
        metadata.insert(
            "consecutive_passes".to_string(),
            v.consecutive_passes.to_string(),
        );
    } else {
        metadata.insert(
            "reason_0".to_string(),
            "no evaluation completed yet".to_string(),
        );
    }

    json_response(ArgoWebhookResponse {
        recommendation: recommendation.to_string(),
        score,
        passed,
        metadata,
    })
}

/// Flagger webhook endpoint.
async fn flagger_webhook(
    State(state): State<SharedState>,
    _body: Json<FlaggerWebhookRequest>,
) -> Response {
    let verdict = get_last_verdict(&state);

    match &verdict {
        Some(v) => match v.recommendation {
            Recommendation::Promote => json_response(FlaggerWebhookResponse {
                passed: true,
                message: None,
            }),
            Recommendation::Hold => json_response(FlaggerWebhookResponse {
                passed: false,
                message: None,
            }),
            Recommendation::Rollback => {
                let reason = v.reasoning.first().cloned().unwrap_or_default();
                (
                    StatusCode::BAD_REQUEST,
                    Json(FlaggerWebhookResponse {
                        passed: false,
                        message: Some(reason),
                    }),
                )
                    .into_response()
            }
        },
        None => json_response(FlaggerWebhookResponse {
            passed: false,
            message: None,
        }),
    }
}

fn get_last_verdict(state: &SharedState) -> Option<Verdict> {
    state
        .last_verdict
        .lock()
        .ok()
        .and_then(|guard| guard.clone())
}

fn json_response(value: impl Serialize) -> Response {
    match serde_json::to_value(value) {
        Ok(json) => Json(json).into_response(),
        Err(e) => internal_error(&format!("serialization error: {}", e)),
    }
}

fn not_found(message: &str) -> Response {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "not_found".to_string(),
                message: message.to_string(),
            },
        }),
    )
        .into_response()
}

fn bad_request(message: &str) -> Response {
    (
        StatusCode::BAD_REQUEST,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "bad_request".to_string(),
                message: message.to_string(),
            },
        }),
    )
        .into_response()
}

fn internal_error(message: &str) -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: ErrorDetail {
                code: "internal_error".to_string(),
                message: message.to_string(),
            },
        }),
    )
        .into_response()
}

async fn metrics() -> impl IntoResponse {
    (
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4",
        )],
        "# HELP canary_gate_evaluations_total Total number of evaluations\n\
         # TYPE canary_gate_evaluations_total counter\n\
         canary_gate_evaluations_total 0\n",
    )
}
