use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::error::{Error, Result};

/// Top-level canary-gate configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub logstream: LogstreamConfig,

    #[serde(default)]
    pub evaluation: EvaluationConfig,

    #[serde(default)]
    pub logging: LoggingConfig,

    #[serde(default)]
    pub tests: Vec<TestConfig>,

    #[serde(default)]
    pub packs: Vec<String>,

    #[serde(default)]
    pub overrides: HashMap<String, OverrideConfig>,

    #[serde(default)]
    pub recommendation: RecommendationConfig,

    #[serde(default)]
    pub metrics: Option<MetricsSourceConfig>,
}

/// Log stream boundary configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogstreamConfig {
    #[serde(default = "default_start_mode")]
    pub start: StartMode,

    #[serde(default = "default_lookback")]
    pub lookback: String,
}

impl Default for LogstreamConfig {
    fn default() -> Self {
        Self {
            start: default_start_mode(),
            lookback: default_lookback(),
        }
    }
}

/// Where in the log stream to begin reading.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StartMode {
    Beginning,
    Now,
    SinceTimestamp,
}

fn default_start_mode() -> StartMode {
    StartMode::Beginning
}

fn default_lookback() -> String {
    "60s".to_string()
}

/// Evaluation cycle parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvaluationConfig {
    #[serde(default = "default_interval")]
    pub interval: String,

    #[serde(default = "default_lookback")]
    pub lookback: String,

    #[serde(default = "default_min_cycles")]
    pub min_cycles: u32,

    #[serde(default = "default_max_duration")]
    pub max_duration: String,
}

impl Default for EvaluationConfig {
    fn default() -> Self {
        Self {
            interval: default_interval(),
            lookback: default_lookback(),
            min_cycles: default_min_cycles(),
            max_duration: default_max_duration(),
        }
    }
}

fn default_interval() -> String {
    "30s".to_string()
}

fn default_min_cycles() -> u32 {
    5
}

fn default_max_duration() -> String {
    "15m".to_string()
}

/// Logging configuration including event classification rules.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_format")]
    pub format: LogFormat,

    #[serde(default)]
    pub events: Vec<EventConfig>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            format: default_log_format(),
            events: Vec::new(),
        }
    }
}

/// Log format for parsing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum LogFormat {
    Auto,
    Plaintext,
    Json,
}

fn default_log_format() -> LogFormat {
    LogFormat::Auto
}

/// Event classification rule from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventConfig {
    #[serde(rename = "type")]
    pub event_type: String,

    pub level: EventLevel,

    #[serde(rename = "match")]
    pub match_rule: MatchRule,
}

/// Severity level for classified events.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum EventLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// Match combinators for event classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchRule {
    #[serde(default)]
    pub any: Vec<MatchCondition>,

    #[serde(default)]
    pub all: Vec<MatchCondition>,

    #[serde(default)]
    pub none: Vec<MatchCondition>,
}

/// A single match condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchCondition {
    pub contains: Option<String>,
    pub regex: Option<String>,
}

/// A behavior-driven test definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestConfig {
    pub name: String,

    #[serde(default = "default_fail_severity")]
    pub severity: FailSeverity,

    pub then: Vec<TestAssertion>,
}

/// How a test failure affects the recommendation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FailSeverity {
    Hard,
    Soft,
}

fn default_fail_severity() -> FailSeverity {
    FailSeverity::Hard
}

/// A test assertion (what to check in the event stream).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestAssertion {
    #[serde(default)]
    pub event_present: Option<EventPresentAssertion>,

    #[serde(default)]
    pub event_absent: Option<EventAbsentAssertion>,

    #[serde(default)]
    pub rate: Option<RateAssertion>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventPresentAssertion {
    #[serde(rename = "type")]
    pub event_type: String,

    #[serde(default)]
    pub within: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventAbsentAssertion {
    #[serde(rename = "type")]
    pub event_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateAssertion {
    #[serde(rename = "type")]
    pub event_type: String,

    #[serde(default)]
    pub threshold: Option<f64>,

    #[serde(default)]
    pub operator: Option<RateOperator>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RateOperator {
    LessThan,
    GreaterThan,
    LessThanOrEqual,
    GreaterThanOrEqual,
}

/// Override settings for pack tests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverrideConfig {
    #[serde(default)]
    pub threshold: Option<f64>,

    #[serde(default)]
    pub within: Option<String>,

    #[serde(default)]
    pub severity: Option<FailSeverity>,
}

/// Recommendation engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendationConfig {
    #[serde(default)]
    pub promote: PromoteConfig,

    #[serde(default)]
    pub rollback: RollbackConfig,

    #[serde(default = "default_verdict_bias")]
    pub bias: VerdictBias,
}

impl Default for RecommendationConfig {
    fn default() -> Self {
        Self {
            promote: PromoteConfig::default(),
            rollback: RollbackConfig::default(),
            bias: default_verdict_bias(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromoteConfig {
    #[serde(default = "default_min_cycles")]
    pub require_min_cycles: u32,

    #[serde(default = "default_consecutive_passes")]
    pub require_consecutive_passes: u32,
}

impl Default for PromoteConfig {
    fn default() -> Self {
        Self {
            require_min_cycles: default_min_cycles(),
            require_consecutive_passes: default_consecutive_passes(),
        }
    }
}

fn default_consecutive_passes() -> u32 {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RollbackConfig {
    #[serde(default = "default_soft_fail_consecutive")]
    pub soft_fail_consecutive_cycles: u32,
}

impl Default for RollbackConfig {
    fn default() -> Self {
        Self {
            soft_fail_consecutive_cycles: default_soft_fail_consecutive(),
        }
    }
}

fn default_soft_fail_consecutive() -> u32 {
    3
}

/// Verdict bias for ambiguous situations.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum VerdictBias {
    HoldOnAmbiguity,
    PromoteOnAmbiguity,
}

fn default_verdict_bias() -> VerdictBias {
    VerdictBias::HoldOnAmbiguity
}

/// External metrics source configuration (e.g., Prometheus).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSourceConfig {
    #[serde(rename = "type")]
    pub source_type: MetricsSourceType,

    pub endpoint: String,

    #[serde(default)]
    pub queries: Vec<MetricsQuery>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MetricsSourceType {
    Prometheus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsQuery {
    pub name: String,
    pub query: String,

    #[serde(default)]
    pub threshold: Option<f64>,

    #[serde(default)]
    pub operator: Option<RateOperator>,

    #[serde(default = "default_fail_severity")]
    pub severity: FailSeverity,
}

/// Load and validate a configuration file.
pub fn load_config(path: &Path) -> Result<Config> {
    let contents = std::fs::read_to_string(path).map_err(|e| Error::ConfigRead {
        path: path.to_path_buf(),
        source: e,
    })?;
    let config: Config = serde_yaml::from_str(&contents)?;
    validate_config(&config)?;
    Ok(config)
}

/// Parse config from a YAML string (useful for tests).
pub fn parse_config(yaml: &str) -> Result<Config> {
    let config: Config = serde_yaml::from_str(yaml)?;
    validate_config(&config)?;
    Ok(config)
}

/// Validate the configuration for logical consistency.
fn validate_config(config: &Config) -> Result<()> {
    // Must have tests or packs defined
    if config.tests.is_empty() && config.packs.is_empty() {
        return Err(Error::Config(
            "configuration must define at least one test or pack".to_string(),
        ));
    }

    // Validate event configs have match rules
    for event in &config.logging.events {
        if event.match_rule.any.is_empty()
            && event.match_rule.all.is_empty()
            && event.match_rule.none.is_empty()
        {
            return Err(Error::Config(format!(
                "event '{}' must have at least one match rule (any, all, or none)",
                event.event_type
            )));
        }
    }

    // Validate test assertions reference something
    for test in &config.tests {
        if test.then.is_empty() {
            return Err(Error::Config(format!(
                "test '{}' must have at least one assertion in 'then'",
                test.name
            )));
        }
        for assertion in &test.then {
            if assertion.event_present.is_none()
                && assertion.event_absent.is_none()
                && assertion.rate.is_none()
            {
                return Err(Error::Config(format!(
                    "test '{}' has an empty assertion (must specify event_present, event_absent, or rate)",
                    test.name
                )));
            }
        }
    }

    // Validate recommendation thresholds
    if config.recommendation.promote.require_min_cycles == 0 {
        return Err(Error::Config(
            "require_min_cycles must be at least 1".to_string(),
        ));
    }
    if config.recommendation.promote.require_consecutive_passes == 0 {
        return Err(Error::Config(
            "require_consecutive_passes must be at least 1".to_string(),
        ));
    }

    Ok(())
}
