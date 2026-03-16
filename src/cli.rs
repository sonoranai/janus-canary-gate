use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Production canary health gate for Kubernetes deployments.
///
/// Evaluates canary deployments by analyzing logs and metrics,
/// producing auditable promote/hold/rollback recommendations.
#[derive(Parser, Debug)]
#[command(name = "canary-gate", version, about)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Watch a canary deployment continuously, evaluating on each cycle.
    Watch {
        /// Path to the canary-gate configuration file.
        #[arg(short, long)]
        config: PathBuf,

        /// Path to the log file to evaluate.
        #[arg(short, long)]
        log: Option<PathBuf>,

        /// Path to a directory of log files to evaluate.
        #[arg(long)]
        log_dir: Option<PathBuf>,

        /// Glob pattern to filter files in --log-dir (e.g. "webapp-*.log").
        #[arg(long)]
        r#match: Option<String>,

        /// Launch the interactive TUI dashboard.
        #[arg(long)]
        tui: bool,

        /// Start the HTTP API server for programmatic access.
        #[arg(long)]
        api: bool,

        /// Address to bind the API server to.
        #[arg(long, default_value = "127.0.0.1:8080")]
        api_addr: String,
    },

    /// Run a single evaluation pass against a log file.
    Evaluate {
        /// Path to the canary-gate configuration file.
        #[arg(short, long)]
        config: PathBuf,

        /// Path to a single log file to evaluate.
        #[arg(short, long, group = "log_source")]
        log: Option<PathBuf>,

        /// Path to a directory of log files to evaluate.
        #[arg(long, group = "log_source")]
        log_dir: Option<PathBuf>,

        /// Glob pattern to filter files in --log-dir (e.g. "webapp-*.log").
        #[arg(long)]
        r#match: Option<String>,

        /// Output format: json or table.
        #[arg(short, long, default_value = "table")]
        format: OutputFormat,
    },

    /// Validate a configuration file without running an evaluation.
    Validate {
        /// Path to the canary-gate configuration file.
        #[arg(short, long)]
        config: PathBuf,
    },

    /// Explain the reasoning behind a past decision.
    Explain {
        /// The decision ID to explain.
        #[arg(long)]
        decision_id: String,

        /// Path to the SQLite database file.
        #[arg(long, default_value = "canary-gate.db")]
        db: PathBuf,
    },

    /// Query decision history from the SQLite audit trail.
    History {
        /// Filter verdicts by deployment ID.
        #[arg(long)]
        deployment_id: Option<String>,

        /// Filter verdicts by result (promote, hold, rollback).
        #[arg(long)]
        verdict: Option<String>,

        /// Only show verdicts since this ISO-8601 timestamp.
        #[arg(long)]
        since: Option<String>,

        /// Maximum number of results to return.
        #[arg(long, default_value = "20")]
        limit: usize,

        /// Path to the SQLite database file.
        #[arg(long, default_value = "canary-gate.db")]
        db: PathBuf,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
}

/// Exit codes per the CLI contract in the PRD.
pub mod exit_codes {
    pub const PROMOTE: i32 = 0;
    pub const HOLD: i32 = 1;
    pub const ROLLBACK: i32 = 2;
    pub const ERROR: i32 = 3;
}
