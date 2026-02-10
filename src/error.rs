use std::path::PathBuf;

/// Library-level errors using thiserror for typed, matchable error variants.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("configuration error: {0}")]
    Config(String),

    #[error("failed to read configuration file {path}: {source}")]
    ConfigRead {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("failed to parse YAML configuration: {0}")]
    ConfigParse(#[from] serde_yaml::Error),

    #[error("ingestion error: {0}")]
    Ingestion(String),

    #[error("classification error: {0}")]
    Classification(String),

    #[error("behavior test error: {0}")]
    Behavior(String),

    #[error("recommendation error: {0}")]
    Recommendation(String),

    #[error("metrics error: {0}")]
    Metrics(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("HTTP request error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
