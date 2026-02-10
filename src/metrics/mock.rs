use crate::error::Result;
use crate::metrics::{MetricResult, MetricsSource};

/// A mock metrics source for testing that returns canned results.
pub struct MockMetricsSource {
    results: Vec<MetricResult>,
    healthy: bool,
}

impl MockMetricsSource {
    pub fn new(results: Vec<MetricResult>) -> Self {
        Self {
            results,
            healthy: true,
        }
    }

    pub fn unhealthy() -> Self {
        Self {
            results: vec![],
            healthy: false,
        }
    }
}

impl MetricsSource for MockMetricsSource {
    async fn query(&self, _query: &str) -> Result<Vec<MetricResult>> {
        if !self.healthy {
            return Err(crate::error::Error::Metrics(
                "mock source is unhealthy".to_string(),
            ));
        }
        Ok(self.results.clone())
    }

    async fn health_check(&self) -> Result<bool> {
        Ok(self.healthy)
    }
}
