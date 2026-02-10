use canary_gate::recommendation::Recommendation;
use canary_gate::tui::input::{handle_key, InputResult};
use canary_gate::tui::state::{AppState, HumanAction};
use crossterm::event::KeyCode;

fn new_state() -> AppState {
    AppState::new("deploy-test")
}

#[test]
fn initial_state_defaults() {
    let state = new_state();
    assert_eq!(state.deployment_id, "deploy-test");
    assert_eq!(state.recommendation, Recommendation::Hold);
    assert_eq!(state.total_cycles, 0);
    assert_eq!(state.consecutive_passes, 0);
    assert!(state.test_results.is_empty());
    assert!(state.reasoning.is_empty());
    assert!(state.selected_action.is_none());
}

#[test]
fn key_p_sets_promote_action() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Char('p'), &mut state);
    assert!(matches!(result, InputResult::Action(HumanAction::Promote)));
    assert_eq!(state.selected_action, Some(HumanAction::Promote));
}

#[test]
fn key_r_sets_rollback_action() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Char('r'), &mut state);
    assert!(matches!(result, InputResult::Action(HumanAction::Rollback)));
    assert_eq!(state.selected_action, Some(HumanAction::Rollback));
}

#[test]
fn key_h_sets_hold_action() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Char('h'), &mut state);
    assert!(matches!(result, InputResult::Action(HumanAction::Hold)));
    assert_eq!(state.selected_action, Some(HumanAction::Hold));
}

#[test]
fn key_q_quits() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Char('q'), &mut state);
    assert!(matches!(result, InputResult::Quit));
}

#[test]
fn key_esc_quits() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Esc, &mut state);
    assert!(matches!(result, InputResult::Quit));
}

#[test]
fn unknown_key_continues() {
    let mut state = new_state();
    let result = handle_key(KeyCode::Char('x'), &mut state);
    assert!(matches!(result, InputResult::Continue));
    assert!(state.selected_action.is_none());
}

#[test]
fn human_action_display() {
    assert_eq!(HumanAction::Promote.to_string(), "promote");
    assert_eq!(HumanAction::Rollback.to_string(), "rollback");
    assert_eq!(HumanAction::Hold.to_string(), "hold");
}

#[test]
fn action_overrides_previous() {
    let mut state = new_state();
    handle_key(KeyCode::Char('p'), &mut state);
    assert_eq!(state.selected_action, Some(HumanAction::Promote));

    handle_key(KeyCode::Char('r'), &mut state);
    assert_eq!(state.selected_action, Some(HumanAction::Rollback));
}
