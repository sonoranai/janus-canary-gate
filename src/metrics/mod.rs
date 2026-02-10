pub mod prometheus;

use crate::error::Result;

/// A metric result from a query.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricResult {
    pub name: String,
    pub value: f64,
    pub labels: std::collections::HashMap<String, String>,
}

/// Trait for querying metrics from an external source.
///
/// This trait allows plugging in different metrics backends
/// (Prometheus, mock, etc.) while keeping the evaluation engine generic.
#[allow(async_fn_in_trait)]
pub trait MetricsSource {
    /// Execute a query and return results.
    async fn query(&self, query: &str) -> Result<Vec<MetricResult>>;

    /// Check if the metrics source is reachable.
    async fn health_check(&self) -> Result<bool>;
}
