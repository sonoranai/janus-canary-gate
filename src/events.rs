use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::EventLevel;

/// A canonical event extracted from a log line.
///
/// Raw logs are never used in decision logic — only canonical events.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanonicalEvent {
    /// ISO-8601 timestamp (from log line or ingestion time).
    pub timestamp: String,

    /// Severity level of the event.
    pub level: EventLevel,

    /// The classified event type (e.g., "grpc_server_started", "panic").
    pub event_type: String,

    /// Deterministic fingerprint for deduplication and tracking.
    pub fingerprint: String,

    /// The raw log line that produced this event (for audit trail, not decision logic).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_line: Option<String>,
}

/// Generate a deterministic fingerprint for an event.
///
/// The fingerprint is derived from the event type and level only,
/// ensuring the same classification always produces the same fingerprint.
pub fn fingerprint(event_type: &str, level: &EventLevel) -> String {
    let level_str = match level {
        EventLevel::Debug => "debug",
        EventLevel::Info => "info",
        EventLevel::Warn => "warn",
        EventLevel::Error => "error",
        EventLevel::Fatal => "fatal",
    };
    let input = format!("{}:{}", event_type, level_str);
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let result = hasher.finalize();
    format!("{:x}", result)[..16].to_string()
}
