use serde::{Deserialize, Serialize};

use crate::config::MetricDirection;

/// Classification of a metric comparison result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MetricClassification {
    Pass,
    Marginal,
    Fail,
}

/// Analysis of a single metric comparison (baseline vs canary).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricAnalysis {
    pub name: String,
    pub baseline_mean: f64,
    pub canary_mean: f64,
    pub p_value: f64,
    pub direction: MetricDirection,
    pub classification: MetricClassification,
    pub weight: f64,
}

/// Aggregate score across all metric comparisons.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AggregateScore {
    pub score: f64,
    pub pass: usize,
    pub marginal: usize,
    pub fail: usize,
    pub per_metric: Vec<MetricAnalysis>,
}

/// Classify a metric comparison based on p-value and direction.
///
/// - Pass: p_value >= 0.05 OR change in "good" direction
/// - Marginal: 0.01 <= p_value < 0.05
/// - Fail: p_value < 0.01 AND change in "bad" direction
pub fn classify(
    p_value: f64,
    direction: &MetricDirection,
    baseline_mean: f64,
    canary_mean: f64,
) -> MetricClassification {
    let good_direction = match direction {
        MetricDirection::IncreaseBad => canary_mean <= baseline_mean,
        MetricDirection::DecreaseBad => canary_mean >= baseline_mean,
        MetricDirection::Either => false,
    };

    if p_value >= 0.05 || good_direction {
        MetricClassification::Pass
    } else if p_value >= 0.01 {
        MetricClassification::Marginal
    } else {
        MetricClassification::Fail
    }
}

/// Compute an aggregate score (0–100) from per-metric analyses.
///
/// Score formula: (sum of pass_weight×100 + marginal_weight×50) / total_weight.
/// Empty input returns score 100 (vacuous pass).
pub fn aggregate_score(results: &[MetricAnalysis]) -> AggregateScore {
    if results.is_empty() {
        return AggregateScore {
            score: 100.0,
            pass: 0,
            marginal: 0,
            fail: 0,
            per_metric: Vec::new(),
        };
    }

    let mut pass_weight = 0.0;
    let mut marginal_weight = 0.0;
    let mut total_weight = 0.0;
    let mut pass_count = 0;
    let mut marginal_count = 0;
    let mut fail_count = 0;

    for m in results {
        total_weight += m.weight;
        match m.classification {
            MetricClassification::Pass => {
                pass_weight += m.weight;
                pass_count += 1;
            }
            MetricClassification::Marginal => {
                marginal_weight += m.weight;
                marginal_count += 1;
            }
            MetricClassification::Fail => {
                fail_count += 1;
            }
        }
    }

    let score = if total_weight > 0.0 {
        (pass_weight * 100.0 + marginal_weight * 50.0) / total_weight
    } else {
        100.0
    };

    AggregateScore {
        score,
        pass: pass_count,
        marginal: marginal_count,
        fail: fail_count,
        per_metric: results.to_vec(),
    }
}
