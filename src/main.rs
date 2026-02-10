use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use clap::Parser;
use tracing_subscriber::EnvFilter;

use canary_gate::behavior::evaluate_tests;
use canary_gate::classification::classify_stream;
use canary_gate::cli::{exit_codes, Cli, Command, OutputFormat};
use canary_gate::config::load_config;
use canary_gate::db::Database;
use canary_gate::ingestion::LogReader;
use canary_gate::recommendation::CycleTracker;
use canary_gate::verdict::Verdict;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let cli = Cli::parse();

    let exit_code = match cli.command {
        Command::Evaluate {
            config,
            log,
            format,
        } => cmd_evaluate(&config, &log, &format)?,

        Command::Validate { config } => cmd_validate(&config)?,

        Command::Watch {
            config,
            log,
            tui,
            api,
            api_addr,
        } => cmd_watch(&config, log.as_deref(), tui, api, &api_addr).await?,

        Command::Explain { decision_id, db } => cmd_explain(&decision_id, &db)?,

        Command::History {
            deployment_id,
            verdict,
            since,
            limit,
            db,
        } => cmd_history(
            deployment_id.as_deref(),
            verdict.as_deref(),
            since.as_deref(),
            limit,
            &db,
        )?,
    };

    std::process::exit(exit_code);
}

fn cmd_evaluate(config_path: &Path, log_path: &Path, format: &OutputFormat) -> Result<i32> {
    let config = load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    let reader = LogReader::new(config.logging.format.clone());
    let lines = reader
        .read_file(log_path)
        .with_context(|| format!("reading log file {}", log_path.display()))?;

    let events = classify_stream(&lines, &config.logging.events);
    let evaluations = evaluate_tests(&config.tests, &events);

    let mut tracker = CycleTracker::new();
    tracker.record_cycle(&config.tests, &evaluations, &config.recommendation);

    let verdict = Verdict::from_tracker(&tracker);

    match format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&verdict)?);
        }
        OutputFormat::Table => {
            print!("{}", verdict.format_table());
        }
    }

    Ok(verdict.exit_code())
}

fn cmd_validate(config_path: &Path) -> Result<i32> {
    match load_config(config_path) {
        Ok(_) => {
            println!("Configuration is valid: {}", config_path.display());
            Ok(exit_codes::PROMOTE)
        }
        Err(e) => {
            eprintln!("Configuration error: {}", e);
            Ok(exit_codes::ERROR)
        }
    }
}

async fn cmd_watch(
    config_path: &Path,
    _log_path: Option<&Path>,
    tui: bool,
    api: bool,
    api_addr: &str,
) -> Result<i32> {
    let _config = load_config(config_path)
        .with_context(|| format!("loading config from {}", config_path.display()))?;

    if api {
        let db = Database::open_in_memory()
            .map_err(|e| anyhow::anyhow!("failed to open database: {}", e))?;

        let state = Arc::new(canary_gate::api::AppState {
            db: Mutex::new(db),
            start_time: std::time::Instant::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        });

        let app = canary_gate::api::router(state);
        let listener = tokio::net::TcpListener::bind(api_addr).await?;
        tracing::info!("API server listening on {}", api_addr);
        axum::serve(listener, app).await?;
    }

    if tui {
        let tui_state = canary_gate::tui::state::AppState::new("unknown");
        let _action = canary_gate::tui::run(tui_state)?;
    }

    Ok(exit_codes::HOLD)
}

fn cmd_explain(decision_id: &str, db_path: &Path) -> Result<i32> {
    let db =
        Database::open(db_path).map_err(|e| anyhow::anyhow!("failed to open database: {}", e))?;

    let id: i64 = decision_id
        .parse()
        .with_context(|| format!("invalid decision ID: {}", decision_id))?;

    match db
        .get_evaluation(id)
        .map_err(|e| anyhow::anyhow!("database query failed: {}", e))?
    {
        Some(eval) => {
            println!("{}", serde_json::to_string_pretty(&eval)?);
            Ok(exit_codes::PROMOTE)
        }
        None => {
            eprintln!("Decision {} not found", decision_id);
            Ok(exit_codes::ERROR)
        }
    }
}

fn cmd_history(
    deployment_id: Option<&str>,
    verdict: Option<&str>,
    since: Option<&str>,
    limit: usize,
    db_path: &Path,
) -> Result<i32> {
    let db =
        Database::open(db_path).map_err(|e| anyhow::anyhow!("failed to open database: {}", e))?;

    let records = db
        .query_history(deployment_id, verdict, since, limit)
        .map_err(|e| anyhow::anyhow!("database query failed: {}", e))?;

    if records.is_empty() {
        println!("No evaluation history found.");
    } else {
        let header = format!(
            "{:<6} {:<20} {:<12} {:<8} Created",
            "ID", "Deployment", "Verdict", "Cycles"
        );
        println!("{header}");
        println!("{:-<70}", "");
        for record in &records {
            println!(
                "{:<6} {:<20} {:<12} {:<8} {}",
                record.id,
                record.deployment_id,
                record.recommendation,
                record.total_cycles,
                record.created_at,
            );
        }
    }

    Ok(exit_codes::PROMOTE)
}
