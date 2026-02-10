-- canary-gate schema v1

CREATE TABLE IF NOT EXISTS evaluations (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    deployment_id TEXT NOT NULL,
    config_hash TEXT NOT NULL,
    recommendation TEXT NOT NULL CHECK (recommendation IN ('promote', 'hold', 'rollback')),
    total_cycles INTEGER NOT NULL,
    consecutive_passes INTEGER NOT NULL,
    reasoning TEXT NOT NULL DEFAULT '[]',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_evaluations_deployment ON evaluations(deployment_id);
CREATE INDEX IF NOT EXISTS idx_evaluations_recommendation ON evaluations(recommendation);
CREATE INDEX IF NOT EXISTS idx_evaluations_created_at ON evaluations(created_at);

CREATE TABLE IF NOT EXISTS criteria_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluation_id INTEGER NOT NULL REFERENCES evaluations(id),
    test_name TEXT NOT NULL,
    result TEXT NOT NULL CHECK (result IN ('pass', 'fail', 'unknown')),
    details TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_criteria_evaluation ON criteria_results(evaluation_id);

CREATE TABLE IF NOT EXISTS verdicts_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    evaluation_id INTEGER NOT NULL REFERENCES evaluations(id),
    actor TEXT NOT NULL,
    action TEXT NOT NULL,
    reason TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX IF NOT EXISTS idx_verdicts_evaluation ON verdicts_log(evaluation_id);
