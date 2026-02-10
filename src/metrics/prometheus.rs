use std::collections::HashMap;

use crate::error::{Error, Result};
use crate::metrics::{MetricResult, MetricsSource};

/// Prometheus metrics source that queries the Prometheus HTTP API v1.
pub struct PrometheusSource {
    client: reqwest::Client,
    endpoint: String,
}

impl PrometheusSource {
    pub fn new(endpoint: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            endpoint: endpoint.trim_end_matches('/').to_string(),
        }
    }
}

impl PrometheusSource {
    /// Execute a range query and return a flat list of f64 values across all series.
    pub async fn query_range(
        &self,
        query: &str,
        start: &str,
        end: &str,
        step: &str,
    ) -> Result<Vec<f64>> {
        let url = format!("{}/api/v1/query_range", self.endpoint);
        let response = self
            .client
            .get(&url)
            .query(&[
                ("query", query),
                ("start", start),
                ("end", end),
                ("step", step),
            ])
            .send()
            .await?;
        let body: serde_json::Value = response.json().await?;
        parse_range_values(&body)
    }
}

impl MetricsSource for PrometheusSource {
    async fn query(&self, query: &str) -> Result<Vec<MetricResult>> {
        let url = format!("{}/api/v1/query", self.endpoint);
        let response = self
            .client
            .get(&url)
            .query(&[("query", query)])
            .send()
            .await?;

        let body: serde_json::Value = response.json().await?;

        parse_prometheus_response(&body)
    }

    async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/-/healthy", self.endpoint);
        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(_) => Ok(false),
        }
    }
}

/// Parse Prometheus API v1 query response into MetricResults.
pub fn parse_prometheus_response(body: &serde_json::Value) -> Result<Vec<MetricResult>> {
    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("error");

    if status != "success" {
        let error_msg = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(Error::Metrics(format!(
            "Prometheus query failed: {}",
            error_msg
        )));
    }

    let data = body
        .get("data")
        .ok_or_else(|| Error::Metrics("missing 'data' field in Prometheus response".to_string()))?;

    let result_type = data
        .get("resultType")
        .and_then(|v| v.as_str())
        .unwrap_or("vector");

    let results = data
        .get("result")
        .and_then(|v| v.as_array())
        .ok_or_else(|| {
            Error::Metrics("missing 'result' array in Prometheus response".to_string())
        })?;

    let mut metrics = Vec::new();

    for result in results {
        let metric = result.get("metric").and_then(|v| v.as_object());
        let labels: HashMap<String, String> = metric
            .map(|m| {
                m.iter()
                    .map(|(k, v)| (k.clone(), v.as_str().unwrap_or("").to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let name = labels
            .get("__name__")
            .cloned()
            .unwrap_or_else(|| "unnamed".to_string());

        let value = match result_type {
            "vector" => result
                .get("value")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.get(1))
                .and_then(|v| v.as_str())
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            _ => 0.0,
        };

        metrics.push(MetricResult {
            name,
            value,
            labels,
        });
    }

    Ok(metrics)
}

/// Parse a Prometheus range query response into a flat list of f64 values.
pub fn parse_range_values(body: &serde_json::Value) -> Result<Vec<f64>> {
    let status = body
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("error");

    if status != "success" {
        let error_msg = body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown error");
        return Err(Error::Metrics(format!(
            "Prometheus range query failed: {}",
            error_msg
        )));
    }

    let data = body
        .get("data")
        .ok_or_else(|| Error::Metrics("missing 'data' field in range response".to_string()))?;

    let results = data
        .get("result")
        .and_then(|v| v.as_array())
        .ok_or_else(|| Error::Metrics("missing 'result' array in range response".to_string()))?;

    let mut values = Vec::new();
    for result in results {
        if let Some(vals) = result.get("values").and_then(|v| v.as_array()) {
            for pair in vals {
                if let Some(arr) = pair.as_array() {
                    if let Some(v) = arr
                        .get(1)
                        .and_then(|v| v.as_str())
                        .and_then(|s| s.parse::<f64>().ok())
                    {
                        values.push(v);
                    }
                }
            }
        }
    }

    Ok(values)
}
