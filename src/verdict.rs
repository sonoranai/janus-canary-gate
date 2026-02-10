use crate::behavior::TestEvaluation;
use crate::cli::exit_codes;
use crate::recommendation::{CycleTracker, Recommendation};

/// A complete verdict with all supporting evidence.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Verdict {
    pub recommendation: Recommendation,
    pub total_cycles: u32,
    pub consecutive_passes: u32,
    pub test_results: Vec<TestEvaluation>,
    pub reasoning: Vec<String>,
}

impl Verdict {
    /// Build a verdict from the current state of the cycle tracker.
    pub fn from_tracker(tracker: &CycleTracker) -> Self {
        let latest_results = tracker
            .cycle_history
            .last()
            .map(|c| c.test_results.clone())
            .unwrap_or_default();

        let mut reasoning = Vec::new();

        match tracker.current_recommendation {
            Recommendation::Promote => {
                reasoning.push(format!(
                    "All tests passing for {} consecutive cycles (required: min cycles met)",
                    tracker.consecutive_passes
                ));
            }
            Recommendation::Hold => {
                if tracker.total_cycles == 0 {
                    reasoning.push("No evaluation cycles completed yet".to_string());
                } else {
                    reasoning.push(format!(
                        "Completed {} cycles, {} consecutive passes — not yet meeting promote criteria",
                        tracker.total_cycles, tracker.consecutive_passes
                    ));
                }
            }
            Recommendation::Rollback => {
                if tracker.hard_failure_seen {
                    reasoning.push("Hard failure detected — immediate rollback".to_string());
                } else {
                    for (test, streak) in &tracker.soft_fail_streaks {
                        reasoning.push(format!(
                            "Soft-fail test '{}' failed for {} consecutive cycles",
                            test, streak
                        ));
                    }
                }
            }
        }

        Self {
            recommendation: tracker.current_recommendation.clone(),
            total_cycles: tracker.total_cycles,
            consecutive_passes: tracker.consecutive_passes,
            test_results: latest_results,
            reasoning,
        }
    }

    /// Map recommendation to exit code.
    pub fn exit_code(&self) -> i32 {
        match self.recommendation {
            Recommendation::Promote => exit_codes::PROMOTE,
            Recommendation::Hold => exit_codes::HOLD,
            Recommendation::Rollback => exit_codes::ROLLBACK,
        }
    }

    /// Format as a human-readable table.
    pub fn format_table(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!("Recommendation: {}\n", self.recommendation));
        output.push_str(&format!(
            "Cycles: {} total, {} consecutive passes\n",
            self.total_cycles, self.consecutive_passes
        ));
        output.push('\n');

        if !self.test_results.is_empty() {
            output.push_str("Test Results:\n");
            output.push_str(&format!("  {:<30} {:<10}\n", "Test", "Result"));
            output.push_str(&format!("  {:-<30} {:-<10}\n", "", ""));
            for eval in &self.test_results {
                output.push_str(&format!(
                    "  {:<30} {:<10}\n",
                    eval.test_name,
                    format!("{:?}", eval.result)
                ));
            }
            output.push('\n');
        }

        if !self.reasoning.is_empty() {
            output.push_str("Reasoning:\n");
            for reason in &self.reasoning {
                output.push_str(&format!("  - {}\n", reason));
            }
        }

        output
    }
}
