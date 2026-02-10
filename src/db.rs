use std::path::Path;

use rusqlite::{params, Connection};

use crate::error::Result;
use crate::recommendation::Recommendation;
use crate::verdict::Verdict;

/// Database manager for SQLite persistence.
pub struct Database {
    conn: Connection,
}

/// A stored evaluation record.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EvaluationRecord {
    pub id: i64,
    pub deployment_id: String,
    pub config_hash: String,
    pub recommendation: String,
    pub total_cycles: u32,
    pub consecutive_passes: u32,
    pub reasoning: String,
    pub created_at: String,
}

impl Database {
    /// Open or create the database at the given path.
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Open an in-memory database (for testing).
    pub fn open_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.run_migrations()?;
        Ok(db)
    }

    /// Run database migrations.
    fn run_migrations(&self) -> Result<()> {
        let migration = include_str!("../migrations/001_initial.sql");
        self.conn.execute_batch(migration)?;
        Ok(())
    }

    /// Store an evaluation result.
    pub fn insert_evaluation(
        &self,
        deployment_id: &str,
        config_hash: &str,
        verdict: &Verdict,
    ) -> Result<i64> {
        let recommendation = match verdict.recommendation {
            Recommendation::Promote => "promote",
            Recommendation::Hold => "hold",
            Recommendation::Rollback => "rollback",
        };

        let reasoning =
            serde_json::to_string(&verdict.reasoning).unwrap_or_else(|_| "[]".to_string());

        self.conn.execute(
            "INSERT INTO evaluations (deployment_id, config_hash, recommendation, total_cycles, consecutive_passes, reasoning)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                deployment_id,
                config_hash,
                recommendation,
                verdict.total_cycles,
                verdict.consecutive_passes,
                reasoning,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Store individual test results for an evaluation.
    pub fn insert_criteria_result(
        &self,
        evaluation_id: i64,
        test_name: &str,
        result: &str,
        details: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO criteria_results (evaluation_id, test_name, result, details)
             VALUES (?1, ?2, ?3, ?4)",
            params![evaluation_id, test_name, result, details],
        )?;
        Ok(())
    }

    /// Log a verdict action (human or automated).
    pub fn insert_verdict_log(
        &self,
        evaluation_id: i64,
        actor: &str,
        action: &str,
        reason: &str,
    ) -> Result<()> {
        self.conn.execute(
            "INSERT INTO verdicts_log (evaluation_id, actor, action, reason)
             VALUES (?1, ?2, ?3, ?4)",
            params![evaluation_id, actor, action, reason],
        )?;
        Ok(())
    }

    /// Get an evaluation by ID.
    pub fn get_evaluation(&self, id: i64) -> Result<Option<EvaluationRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, deployment_id, config_hash, recommendation, total_cycles, consecutive_passes, reasoning, created_at
             FROM evaluations WHERE id = ?1",
        )?;

        let mut rows = stmt.query_map(params![id], |row| {
            Ok(EvaluationRecord {
                id: row.get(0)?,
                deployment_id: row.get(1)?,
                config_hash: row.get(2)?,
                recommendation: row.get(3)?,
                total_cycles: row.get(4)?,
                consecutive_passes: row.get(5)?,
                reasoning: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }

    /// Query evaluation history with optional filters.
    pub fn query_history(
        &self,
        deployment_id: Option<&str>,
        verdict: Option<&str>,
        since: Option<&str>,
        limit: usize,
    ) -> Result<Vec<EvaluationRecord>> {
        let mut sql = String::from(
            "SELECT id, deployment_id, config_hash, recommendation, total_cycles, consecutive_passes, reasoning, created_at
             FROM evaluations WHERE 1=1",
        );
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(did) = deployment_id {
            sql.push_str(" AND deployment_id = ?");
            param_values.push(Box::new(did.to_string()));
        }
        if let Some(v) = verdict {
            sql.push_str(" AND recommendation = ?");
            param_values.push(Box::new(v.to_string()));
        }
        if let Some(s) = since {
            sql.push_str(" AND created_at >= ?");
            param_values.push(Box::new(s.to_string()));
        }

        sql.push_str(" ORDER BY created_at DESC LIMIT ?");
        param_values.push(Box::new(limit as i64));

        let mut stmt = self.conn.prepare(&sql)?;
        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|p| p.as_ref()).collect();
        let rows = stmt.query_map(params.as_slice(), |row| {
            Ok(EvaluationRecord {
                id: row.get(0)?,
                deployment_id: row.get(1)?,
                config_hash: row.get(2)?,
                recommendation: row.get(3)?,
                total_cycles: row.get(4)?,
                consecutive_passes: row.get(5)?,
                reasoning: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Get the current (latest) evaluation.
    pub fn get_current_evaluation(&self) -> Result<Option<EvaluationRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, deployment_id, config_hash, recommendation, total_cycles, consecutive_passes, reasoning, created_at
             FROM evaluations ORDER BY created_at DESC LIMIT 1",
        )?;

        let mut rows = stmt.query_map([], |row| {
            Ok(EvaluationRecord {
                id: row.get(0)?,
                deployment_id: row.get(1)?,
                config_hash: row.get(2)?,
                recommendation: row.get(3)?,
                total_cycles: row.get(4)?,
                consecutive_passes: row.get(5)?,
                reasoning: row.get(6)?,
                created_at: row.get(7)?,
            })
        })?;

        match rows.next() {
            Some(Ok(record)) => Ok(Some(record)),
            Some(Err(e)) => Err(e.into()),
            None => Ok(None),
        }
    }
}
