use crate::behavior::TestEvaluation;
use crate::recommendation::Recommendation;

/// The full TUI application state.
#[derive(Debug, Clone)]
pub struct AppState {
    pub deployment_id: String,
    pub recommendation: Recommendation,
    pub total_cycles: u32,
    pub consecutive_passes: u32,
    pub test_results: Vec<TestEvaluation>,
    pub reasoning: Vec<String>,
    pub selected_action: Option<HumanAction>,
}

/// A human override action taken through the TUI.
#[derive(Debug, Clone, PartialEq)]
pub enum HumanAction {
    Promote,
    Rollback,
    Hold,
}

impl std::fmt::Display for HumanAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HumanAction::Promote => write!(f, "promote"),
            HumanAction::Rollback => write!(f, "rollback"),
            HumanAction::Hold => write!(f, "hold"),
        }
    }
}

impl AppState {
    pub fn new(deployment_id: &str) -> Self {
        Self {
            deployment_id: deployment_id.to_string(),
            recommendation: Recommendation::Hold,
            total_cycles: 0,
            consecutive_passes: 0,
            test_results: Vec::new(),
            reasoning: Vec::new(),
            selected_action: None,
        }
    }
}
