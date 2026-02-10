use canary_gate::db::Database;
use canary_gate::recommendation::Recommendation;
use canary_gate::verdict::Verdict;

fn make_verdict(recommendation: Recommendation, cycles: u32, passes: u32) -> Verdict {
    Verdict {
        recommendation,
        total_cycles: cycles,
        consecutive_passes: passes,
        test_results: vec![],
        reasoning: vec!["test reasoning".to_string()],
    }
}

#[test]
fn open_in_memory() {
    let db = Database::open_in_memory();
    assert!(db.is_ok());
}

#[test]
fn insert_and_retrieve_evaluation() {
    let db = Database::open_in_memory().unwrap();
    let verdict = make_verdict(Recommendation::Promote, 5, 3);

    let id = db
        .insert_evaluation("deploy-123", "abc123", &verdict)
        .unwrap();
    let record = db.get_evaluation(id).unwrap().unwrap();

    assert_eq!(record.deployment_id, "deploy-123");
    assert_eq!(record.config_hash, "abc123");
    assert_eq!(record.recommendation, "promote");
    assert_eq!(record.total_cycles, 5);
    assert_eq!(record.consecutive_passes, 3);
}

#[test]
fn get_nonexistent_evaluation() {
    let db = Database::open_in_memory().unwrap();
    let record = db.get_evaluation(9999).unwrap();
    assert!(record.is_none());
}

#[test]
fn insert_criteria_result() {
    let db = Database::open_in_memory().unwrap();
    let verdict = make_verdict(Recommendation::Hold, 1, 0);
    let eval_id = db.insert_evaluation("deploy-1", "hash1", &verdict).unwrap();

    let result = db.insert_criteria_result(eval_id, "service_starts", "pass", "");
    assert!(result.is_ok());
}

#[test]
fn insert_verdict_log() {
    let db = Database::open_in_memory().unwrap();
    let verdict = make_verdict(Recommendation::Promote, 5, 3);
    let eval_id = db.insert_evaluation("deploy-1", "hash1", &verdict).unwrap();

    let result = db.insert_verdict_log(eval_id, "human:user@example.com", "promote", "looks good");
    assert!(result.is_ok());
}

#[test]
fn query_history_all() {
    let db = Database::open_in_memory().unwrap();

    db.insert_evaluation(
        "deploy-1",
        "h1",
        &make_verdict(Recommendation::Promote, 5, 3),
    )
    .unwrap();
    db.insert_evaluation(
        "deploy-2",
        "h2",
        &make_verdict(Recommendation::Rollback, 2, 0),
    )
    .unwrap();
    db.insert_evaluation("deploy-1", "h3", &make_verdict(Recommendation::Hold, 3, 1))
        .unwrap();

    let records = db.query_history(None, None, None, 100).unwrap();
    assert_eq!(records.len(), 3);
}

#[test]
fn query_history_filter_by_deployment() {
    let db = Database::open_in_memory().unwrap();

    db.insert_evaluation(
        "deploy-1",
        "h1",
        &make_verdict(Recommendation::Promote, 5, 3),
    )
    .unwrap();
    db.insert_evaluation(
        "deploy-2",
        "h2",
        &make_verdict(Recommendation::Rollback, 2, 0),
    )
    .unwrap();

    let records = db.query_history(Some("deploy-1"), None, None, 100).unwrap();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].deployment_id, "deploy-1");
}

#[test]
fn query_history_filter_by_verdict() {
    let db = Database::open_in_memory().unwrap();

    db.insert_evaluation("d1", "h1", &make_verdict(Recommendation::Promote, 5, 3))
        .unwrap();
    db.insert_evaluation("d2", "h2", &make_verdict(Recommendation::Rollback, 2, 0))
        .unwrap();
    db.insert_evaluation("d3", "h3", &make_verdict(Recommendation::Promote, 6, 4))
        .unwrap();

    let records = db.query_history(None, Some("promote"), None, 100).unwrap();
    assert_eq!(records.len(), 2);
}

#[test]
fn query_history_respects_limit() {
    let db = Database::open_in_memory().unwrap();

    for i in 0..10 {
        db.insert_evaluation(
            &format!("deploy-{}", i),
            "hash",
            &make_verdict(Recommendation::Hold, 1, 0),
        )
        .unwrap();
    }

    let records = db.query_history(None, None, None, 3).unwrap();
    assert_eq!(records.len(), 3);
}

#[test]
fn get_current_evaluation() {
    let db = Database::open_in_memory().unwrap();

    db.insert_evaluation(
        "deploy-old",
        "h1",
        &make_verdict(Recommendation::Hold, 1, 0),
    )
    .unwrap();
    db.insert_evaluation(
        "deploy-new",
        "h2",
        &make_verdict(Recommendation::Promote, 5, 3),
    )
    .unwrap();

    let current = db.get_current_evaluation().unwrap().unwrap();
    assert_eq!(current.deployment_id, "deploy-new");
}

#[test]
fn get_current_evaluation_empty_db() {
    let db = Database::open_in_memory().unwrap();
    let current = db.get_current_evaluation().unwrap();
    assert!(current.is_none());
}

#[test]
fn migration_idempotent() {
    // Opening the same DB twice should not fail (migrations use IF NOT EXISTS)
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");

    let _db1 = Database::open(&path).unwrap();
    let _db2 = Database::open(&path).unwrap();
}

#[test]
fn reasoning_stored_as_json() {
    let db = Database::open_in_memory().unwrap();
    let verdict = Verdict {
        recommendation: Recommendation::Promote,
        total_cycles: 5,
        consecutive_passes: 3,
        test_results: vec![],
        reasoning: vec!["reason 1".to_string(), "reason 2".to_string()],
    };

    let id = db.insert_evaluation("deploy-1", "hash", &verdict).unwrap();
    let record = db.get_evaluation(id).unwrap().unwrap();

    let reasons: Vec<String> = serde_json::from_str(&record.reasoning).unwrap();
    assert_eq!(reasons.len(), 2);
    assert_eq!(reasons[0], "reason 1");
}
