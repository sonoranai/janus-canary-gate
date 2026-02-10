use crossterm::event::KeyCode;

use super::state::{AppState, HumanAction};

/// Result of processing a keyboard input.
pub enum InputResult {
    Continue,
    Quit,
    Action(HumanAction),
}

/// Handle a key press and update state accordingly.
pub fn handle_key(key: KeyCode, state: &mut AppState) -> InputResult {
    match key {
        KeyCode::Char('q') | KeyCode::Esc => InputResult::Quit,
        KeyCode::Char('p') => {
            state.selected_action = Some(HumanAction::Promote);
            InputResult::Action(HumanAction::Promote)
        }
        KeyCode::Char('r') => {
            state.selected_action = Some(HumanAction::Rollback);
            InputResult::Action(HumanAction::Rollback)
        }
        KeyCode::Char('h') => {
            state.selected_action = Some(HumanAction::Hold);
            InputResult::Action(HumanAction::Hold)
        }
        _ => InputResult::Continue,
    }
}
