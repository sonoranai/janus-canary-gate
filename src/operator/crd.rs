use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{
    EvaluationConfig, FailSeverity, LoggingConfig, MetricsSourceConfig, RecommendationConfig,
    TestConfig,
};

/// Reference to the target workload being evaluated.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TargetRef {
    /// Kubernetes API version (e.g., "apps/v1").
    #[serde(rename = "apiVersion")]
    pub api_version: String,
    /// Resource kind (e.g., "Deployment").
    pub kind: String,
    /// Resource name.
    pub name: String,
}

/// The spec for a CanaryGate custom resource.
#[derive(CustomResource, Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[kube(
    group = "canary-gate.io",
    version = "v1alpha1",
    kind = "CanaryGate",
    namespaced,
    status = "CanaryGateStatus",
    printcolumn = r#"{"name":"Phase","type":"string","jsonPath":".status.phase"}"#,
    printcolumn = r#"{"name":"Recommendation","type":"string","jsonPath":".status.recommendation"}"#,
    printcolumn = r#"{"name":"Cycles","type":"integer","jsonPath":".status.cycles"}"#
)]
pub struct CanaryGateSpec {
    /// Reference to the target workload.
    pub target_ref: TargetRef,

    /// Label selector for canary pods (used to fetch logs).
    #[serde(default)]
    pub pod_selector: Option<String>,

    /// Evaluation cycle parameters.
    #[serde(default)]
    pub evaluation: Option<EvaluationConfig>,

    /// Logging and event classification rules.
    #[serde(default)]
    pub logging: Option<LoggingConfig>,

    /// Behavioral tests.
    #[serde(default)]
    pub tests: Vec<TestConfig>,

    /// Built-in test pack references.
    #[serde(default)]
    pub packs: Vec<String>,

    /// Recommendation engine configuration.
    #[serde(default)]
    pub recommendation: Option<RecommendationConfig>,

    /// External metrics source configuration.
    #[serde(default)]
    pub metrics: Option<MetricsSourceConfig>,
}

/// The phase of the CanaryGate evaluation lifecycle.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub enum CanaryGatePhase {
    Pending,
    Evaluating,
    Promote,
    Hold,
    Rollback,
}

impl std::fmt::Display for CanaryGatePhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CanaryGatePhase::Pending => write!(f, "Pending"),
            CanaryGatePhase::Evaluating => write!(f, "Evaluating"),
            CanaryGatePhase::Promote => write!(f, "Promote"),
            CanaryGatePhase::Hold => write!(f, "Hold"),
            CanaryGatePhase::Rollback => write!(f, "Rollback"),
        }
    }
}

/// Status subresource for a CanaryGate custom resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanaryGateStatus {
    /// Current phase of the evaluation lifecycle.
    pub phase: CanaryGatePhase,

    /// Current recommendation.
    #[serde(default)]
    pub recommendation: Option<String>,

    /// Total evaluation cycles completed.
    #[serde(default)]
    pub cycles: u32,

    /// Number of consecutive passing cycles.
    #[serde(default)]
    pub consecutive_passes: u32,

    /// Human-readable conditions describing the current state.
    #[serde(default)]
    pub conditions: Vec<CanaryGateCondition>,

    /// Reasoning from the last evaluation.
    #[serde(default)]
    pub reasoning: Vec<String>,
}

/// A condition on the CanaryGate resource.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CanaryGateCondition {
    #[serde(rename = "type")]
    pub condition_type: String,
    pub status: String,
    pub message: String,
    #[serde(default)]
    pub last_transition_time: Option<String>,
}

impl From<&CanaryGateSpec> for crate::config::Config {
    fn from(spec: &CanaryGateSpec) -> Self {
        crate::config::Config {
            logstream: Default::default(),
            evaluation: spec.evaluation.clone().unwrap_or_default(),
            logging: spec.logging.clone().unwrap_or_default(),
            tests: spec.tests.clone(),
            packs: spec.packs.clone(),
            overrides: Default::default(),
            recommendation: spec.recommendation.clone().unwrap_or_default(),
            metrics: spec.metrics.clone(),
            analysis: None,
        }
    }
}

/// Convert a CRD severity string into a FailSeverity enum.
/// Defaults to Hard if not recognized.
pub fn parse_severity(s: &str) -> FailSeverity {
    match s.to_lowercase().as_str() {
        "soft" => FailSeverity::Soft,
        _ => FailSeverity::Hard,
    }
}
