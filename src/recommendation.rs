use crate::behavior::{TestEvaluation, TestResult};
use crate::config::{FailSeverity, RecommendationConfig, TestConfig};

/// The recommendation produced by the engine.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Recommendation {
    Promote,
    Hold,
    Rollback,
}

impl std::fmt::Display for Recommendation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Recommendation::Promote => write!(f, "RECOMMEND_PROMOTE"),
            Recommendation::Hold => write!(f, "RECOMMEND_HOLD"),
            Recommendation::Rollback => write!(f, "RECOMMEND_ROLLBACK"),
        }
    }
}

/// Tracks state across evaluation cycles.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CycleTracker {
    /// Total cycles completed.
    pub total_cycles: u32,

    /// Consecutive passing cycles (all tests pass).
    pub consecutive_passes: u32,

    /// Consecutive failing cycles per soft-fail test name.
    pub soft_fail_streaks: std::collections::HashMap<String, u32>,

    /// Whether a hard failure has been seen.
    pub hard_failure_seen: bool,

    /// The current recommendation.
    pub current_recommendation: Recommendation,

    /// History of per-cycle evaluations.
    pub cycle_history: Vec<CycleResult>,
}

/// Result of a single evaluation cycle.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CycleResult {
    pub cycle_number: u32,
    pub test_results: Vec<TestEvaluation>,
    pub recommendation: Recommendation,
}

impl CycleTracker {
    pub fn new() -> Self {
        Self {
            total_cycles: 0,
            consecutive_passes: 0,
            soft_fail_streaks: std::collections::HashMap::new(),
            hard_failure_seen: false,
            current_recommendation: Recommendation::Hold,
            cycle_history: Vec::new(),
        }
    }

    /// Record a new cycle's test results and update the recommendation.
    pub fn record_cycle(
        &mut self,
        test_configs: &[TestConfig],
        evaluations: &[TestEvaluation],
        config: &RecommendationConfig,
    ) {
        self.total_cycles += 1;

        // If already in rollback from a hard fail, stay there
        if self.hard_failure_seen {
            let result = CycleResult {
                cycle_number: self.total_cycles,
                test_results: evaluations.to_vec(),
                recommendation: Recommendation::Rollback,
            };
            self.cycle_history.push(result);
            self.current_recommendation = Recommendation::Rollback;
            return;
        }

        let mut all_pass = true;
        let mut any_hard_fail = false;

        for eval in evaluations {
            // Find the corresponding test config for severity
            let severity = test_configs
                .iter()
                .find(|t| t.name == eval.test_name)
                .map(|t| &t.severity)
                .unwrap_or(&FailSeverity::Hard);

            match eval.result {
                TestResult::Pass => {
                    // Reset soft-fail streak for this test
                    self.soft_fail_streaks.remove(&eval.test_name);
                }
                TestResult::Fail => {
                    all_pass = false;
                    match severity {
                        FailSeverity::Hard => {
                            any_hard_fail = true;
                        }
                        FailSeverity::Soft => {
                            let streak = self
                                .soft_fail_streaks
                                .entry(eval.test_name.clone())
                                .or_insert(0);
                            *streak += 1;
                        }
                    }
                }
                TestResult::Unknown => {
                    all_pass = false;
                }
            }
        }

        // Determine recommendation
        let recommendation = if any_hard_fail {
            self.hard_failure_seen = true;
            Recommendation::Rollback
        } else if self
            .soft_fail_streaks
            .values()
            .any(|&streak| streak >= config.rollback.soft_fail_consecutive_cycles)
        {
            Recommendation::Rollback
        } else if all_pass {
            self.consecutive_passes += 1;
            if self.total_cycles >= config.promote.require_min_cycles
                && self.consecutive_passes >= config.promote.require_consecutive_passes
            {
                Recommendation::Promote
            } else {
                Recommendation::Hold
            }
        } else {
            self.consecutive_passes = 0;
            Recommendation::Hold
        };

        let result = CycleResult {
            cycle_number: self.total_cycles,
            test_results: evaluations.to_vec(),
            recommendation: recommendation.clone(),
        };
        self.cycle_history.push(result);
        self.current_recommendation = recommendation;
    }
}

impl Default for CycleTracker {
    fn default() -> Self {
        Self::new()
    }
}
