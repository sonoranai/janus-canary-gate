use canary_gate::behavior::*;
use canary_gate::config::*;
use canary_gate::recommendation::*;

fn pass_eval(name: &str) -> TestEvaluation {
    TestEvaluation {
        test_name: name.to_string(),
        result: TestResult::Pass,
        assertion_results: vec![],
    }
}

fn fail_eval(name: &str) -> TestEvaluation {
    TestEvaluation {
        test_name: name.to_string(),
        result: TestResult::Fail,
        assertion_results: vec![],
    }
}

fn unknown_eval(name: &str) -> TestEvaluation {
    TestEvaluation {
        test_name: name.to_string(),
        result: TestResult::Unknown,
        assertion_results: vec![],
    }
}

fn hard_test(name: &str) -> TestConfig {
    TestConfig {
        name: name.to_string(),
        severity: FailSeverity::Hard,
        then: vec![],
    }
}

fn soft_test(name: &str) -> TestConfig {
    TestConfig {
        name: name.to_string(),
        severity: FailSeverity::Soft,
        then: vec![],
    }
}

fn default_config() -> RecommendationConfig {
    RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 5,
            require_consecutive_passes: 2,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 3,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    }
}

#[test]
fn initial_state_is_hold() {
    let tracker = CycleTracker::new();
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
    assert_eq!(tracker.total_cycles, 0);
}

#[test]
fn hard_fail_triggers_immediate_rollback() {
    let mut tracker = CycleTracker::new();
    let config = default_config();
    let tests = vec![hard_test("service_starts")];
    let evals = vec![fail_eval("service_starts")];

    tracker.record_cycle(&tests, &evals, &config);

    assert_eq!(tracker.current_recommendation, Recommendation::Rollback);
    assert!(tracker.hard_failure_seen);
}

#[test]
fn hard_failure_is_permanent() {
    let mut tracker = CycleTracker::new();
    let config = default_config();
    let tests = vec![hard_test("service_starts")];

    // Cycle 1: hard fail
    tracker.record_cycle(&tests, &[fail_eval("service_starts")], &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Rollback);

    // Cycle 2: even if it passes now, still rollback
    tracker.record_cycle(&tests, &[pass_eval("service_starts")], &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Rollback);
}

#[test]
fn all_pass_but_not_enough_cycles_holds() {
    let mut tracker = CycleTracker::new();
    let config = default_config();
    let tests = vec![hard_test("service_starts")];
    let evals = vec![pass_eval("service_starts")];

    // Only 1 cycle, need 5
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
}

#[test]
fn promote_after_min_cycles_and_consecutive_passes() {
    let mut tracker = CycleTracker::new();
    let config = RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 3,
            require_consecutive_passes: 2,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 3,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    };
    let tests = vec![hard_test("service_starts")];
    let evals = vec![pass_eval("service_starts")];

    // Cycles 1-2: hold (not enough cycles)
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);

    // Cycle 3: promote (min_cycles=3, consecutive_passes=2 since we had 2 before this one + this = 3)
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Promote);
}

#[test]
fn soft_fail_streaks_trigger_rollback() {
    let mut tracker = CycleTracker::new();
    let config = RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 5,
            require_consecutive_passes: 2,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 3,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    };
    let tests = vec![soft_test("error_rate")];
    let evals = vec![fail_eval("error_rate")];

    // Cycles 1-2: still hold
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);

    // Cycle 3: rollback (3 consecutive soft failures)
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Rollback);
}

#[test]
fn soft_fail_streak_resets_on_pass() {
    let mut tracker = CycleTracker::new();
    let config = RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 10,
            require_consecutive_passes: 2,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 3,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    };
    let tests = vec![soft_test("error_rate")];

    // 2 failures
    tracker.record_cycle(&tests, &[fail_eval("error_rate")], &config);
    tracker.record_cycle(&tests, &[fail_eval("error_rate")], &config);

    // 1 pass resets streak
    tracker.record_cycle(&tests, &[pass_eval("error_rate")], &config);

    // 2 more failures — still only 2 consecutive
    tracker.record_cycle(&tests, &[fail_eval("error_rate")], &config);
    tracker.record_cycle(&tests, &[fail_eval("error_rate")], &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);

    // 3rd consecutive failure triggers rollback
    tracker.record_cycle(&tests, &[fail_eval("error_rate")], &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Rollback);
}

#[test]
fn unknown_results_prevent_promotion() {
    let mut tracker = CycleTracker::new();
    let config = RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 1,
            require_consecutive_passes: 1,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 3,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    };
    let tests = vec![hard_test("check")];
    let evals = vec![unknown_eval("check")];

    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
}

#[test]
fn consecutive_passes_reset_on_failure() {
    let mut tracker = CycleTracker::new();
    let config = RecommendationConfig {
        promote: PromoteConfig {
            require_min_cycles: 5,
            require_consecutive_passes: 3,
        },
        rollback: RollbackConfig {
            soft_fail_consecutive_cycles: 10,
        },
        bias: VerdictBias::HoldOnAmbiguity,
    };
    let tests = vec![soft_test("check")];

    // 2 passes
    tracker.record_cycle(&tests, &[pass_eval("check")], &config);
    tracker.record_cycle(&tests, &[pass_eval("check")], &config);
    assert_eq!(tracker.consecutive_passes, 2);

    // 1 failure resets
    tracker.record_cycle(&tests, &[fail_eval("check")], &config);
    assert_eq!(tracker.consecutive_passes, 0);
}

#[test]
fn cycle_history_tracked() {
    let mut tracker = CycleTracker::new();
    let config = default_config();
    let tests = vec![hard_test("check")];

    tracker.record_cycle(&tests, &[pass_eval("check")], &config);
    tracker.record_cycle(&tests, &[pass_eval("check")], &config);

    assert_eq!(tracker.cycle_history.len(), 2);
    assert_eq!(tracker.cycle_history[0].cycle_number, 1);
    assert_eq!(tracker.cycle_history[1].cycle_number, 2);
}

#[test]
fn mixed_hard_and_soft_tests() {
    let mut tracker = CycleTracker::new();
    let config = default_config();
    let tests = vec![hard_test("critical"), soft_test("minor")];
    let evals = vec![pass_eval("critical"), fail_eval("minor")];

    // Soft fail, but hard test passes — hold
    tracker.record_cycle(&tests, &evals, &config);
    assert_eq!(tracker.current_recommendation, Recommendation::Hold);
}
