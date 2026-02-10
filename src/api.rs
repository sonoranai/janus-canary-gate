use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

use crate::db::Database;

/// Shared application state for the API server.
///
/// Database is behind a std::sync::Mutex because rusqlite::Connection
/// is not Send+Sync. We use spawn_blocking-style access through the mutex.
pub struct AppState {
    pub db: Mutex<Database>,
    pub start_time: std::time::Instant,
    pub version: String,
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
    pub offset: Option<usize>,
    pub deployment_id: Option<String>,
    pub verdict: Option<String>,
    pub since: Option<String>,
}

/// Build the API router.
pub fn router(state: SharedState) -> Router {
    Router::new()
        .route("/api/v1/health", get(health))
        .route("/api/v1/evaluations/current", get(current_evaluation))
        .route("/api/v1/evaluations/{id}", get(get_evaluation))
        .route("/api/v1/evaluations", get(list_evaluations))
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
    let db = state.db.lock().unwrap();
    match db.get_current_evaluation() {
        Ok(Some(eval)) => Json(serde_json::to_value(eval).unwrap()).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "not_found".to_string(),
                    message: "no evaluations found".to_string(),
                },
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "internal_error".to_string(),
                    message: e.to_string(),
                },
            }),
        )
            .into_response(),
    }
}

async fn get_evaluation(State(state): State<SharedState>, Path(id): Path<i64>) -> Response {
    let db = state.db.lock().unwrap();
    match db.get_evaluation(id) {
        Ok(Some(eval)) => Json(serde_json::to_value(eval).unwrap()).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "not_found".to_string(),
                    message: format!("evaluation {} not found", id),
                },
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "internal_error".to_string(),
                    message: e.to_string(),
                },
            }),
        )
            .into_response(),
    }
}

async fn list_evaluations(
    State(state): State<SharedState>,
    Query(params): Query<PaginationParams>,
) -> Response {
    let db = state.db.lock().unwrap();
    let limit = params.limit.unwrap_or(20);

    match db.query_history(
        params.deployment_id.as_deref(),
        params.verdict.as_deref(),
        params.since.as_deref(),
        limit,
    ) {
        Ok(evals) => Json(serde_json::to_value(evals).unwrap()).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: ErrorDetail {
                    code: "internal_error".to_string(),
                    message: e.to_string(),
                },
            }),
        )
            .into_response(),
    }
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
