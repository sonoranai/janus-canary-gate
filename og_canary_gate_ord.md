# canary-gate

**Progressive Delivery Decision Engine**

Version 1.0 | February 2026 | SonoranAI

---

## 1. Problem Statement

Canary deployments are standard practice, but the promotion decision is often manual or relies on ad-hoc scripts that check a single metric. Teams either promote too aggressively (missing regressions) or too conservatively (blocking velocity). There is no lightweight, standalone tool that consumes metrics from standard observability infrastructure, evaluates them against declarative criteria, and produces auditable promotion decisions with full reasoning traces.

canary-gate is a CLI tool and lightweight HTTP service that consumes metrics from Prometheus (or any OpenMetrics-compatible source), evaluates them against declarative YAML criteria, and emits structured verdicts (promote, rollback, hold) with full decision traces.

---

## 2. Technology Stack

| Component | Technology | Rationale |
|---|---|---|
| Language | Rust (2021 edition) | Performance for tight polling loops; strong type system for correctness in decision logic; single static binary distribution |
| HTTP framework | axum 0.8+ | Tokio-native, production-proven, minimal API surface |
| HTTP client | reqwest 0.12+ | De facto Rust HTTP client; async, well-maintained |
| CLI framework | clap 4.x | Standard Rust CLI parser; derive macros for clean argument definitions |
| Configuration | serde + serde_yaml | YAML config for human-editable deployment criteria |
| Database | SQLite via rusqlite 0.32+ | Zero-config persistence for decision history; Postgres via sqlx 0.8+ as optional feature flag |
| Logging | tracing + tracing-subscriber | Structured JSON logging; span-based context propagation |
| Metrics export | prometheus-client 0.23+ | Expose internal metrics in OpenMetrics format |
| Frontend (optional) | Svelte 5 + SvelteKit 2 | Lightweight dashboard for decision timeline visualization |
| Charting | D3.js 7.x via Svelte wrapper | Metric trend visualization in dashboard |
| Containerization | Docker multi-stage build | Minimal final image; docker-compose.yaml for demo with Prometheus |

---

## 3. End-User Features

### 3.1 CLI Interface

- **canary-gate evaluate --config \<path\>**: Run a single evaluation cycle against the configured metrics source. Prints a structured verdict (promote, rollback, hold) with reasoning to stdout.
- **canary-gate watch --config \<path\>**: Continuous evaluation mode. Polls metrics at a configurable interval, evaluates criteria, and emits verdicts. Exits with appropriate code on terminal decision.
- **canary-gate history**: Query past decisions from the local SQLite database. Supports `--since`, `--verdict`, and `--deployment-id` filters. Output as JSON or human-readable table (`--format table`).
- **canary-gate validate --config \<path\>**: Validate a configuration file without running an evaluation. Checks PromQL syntax, threshold logic, and required fields.
- **canary-gate explain --decision-id \<id\>**: Retrieve a specific past decision and display the full reasoning trace: which metrics were queried, what values were returned, how each criterion evaluated, and the final aggregation logic.

### 3.2 Configuration

All deployment criteria are defined in a single YAML file. The configuration specifies:

- Metrics source (Prometheus endpoint URL)
- Canary and baseline label selectors
- Evaluation criteria: metric name, PromQL query template, threshold, comparator, weight
- Aggregation strategy: weighted score, all-pass, or majority
- Timing parameters: evaluation interval, minimum evaluation count before promotion, maximum evaluation duration before forced rollback

### 3.3 Web Dashboard (Optional)

A lightweight Svelte application served by the canary-gate binary (behind a `--serve` flag) that displays:

- Active evaluations with real-time metric charts
- Decision history with drill-down into reasoning traces
- Configuration status showing which criteria are passing or failing

The dashboard consumes the same API endpoints described in Section 4 and requires no additional infrastructure.

---

## 4. API Specification

| Method | Path | Description | Response |
|---|---|---|---|
| GET | /api/v1/health | Liveness check | 200 OK with version and uptime |
| GET | /api/v1/evaluations/current | Current active evaluation state | Verdict, metric values, criteria results |
| POST | /api/v1/evaluations | Trigger an ad-hoc evaluation | Verdict with full reasoning trace |
| GET | /api/v1/evaluations/:id | Retrieve a specific evaluation | Full evaluation record with traces |
| GET | /api/v1/evaluations | List past evaluations with filters | Paginated list; filters: since, verdict, deployment_id |
| GET | /api/v1/config | Current active configuration | Parsed and validated config object |
| GET | /metrics | OpenMetrics endpoint | canary_gate_evaluations_total, canary_gate_last_verdict, etc. |

All endpoints return JSON. List endpoints support `limit` and `offset` pagination parameters. Error responses use a consistent envelope: `{ "error": { "code": "...", "message": "..." } }`.

---

## 5. Data Sources

- **Primary**: Prometheus-compatible metrics endpoint. Queries via PromQL over the Prometheus HTTP API v1 (`/api/v1/query` and `/api/v1/query_range`). Supports any Prometheus-compatible backend: Prometheus, Thanos, VictoriaMetrics, Grafana Mimir.
- **Secondary (future)**: Datadog API, CloudWatch Metrics. These are deferred to keep the initial scope tight; the internal `MetricsSource` trait is designed to be pluggable.

---

## 6. Database Schema (SQLite / Postgres)

Three core tables. The schema is intentionally flat to support simple queries and fast inserts during evaluation loops.

### evaluations

| Column | Type | Description |
|---|---|---|
| id | TEXT (ULID) | Primary key; sortable unique identifier |
| deployment_id | TEXT | User-provided deployment identifier from config |
| started_at | TIMESTAMP | Evaluation start time (UTC) |
| completed_at | TIMESTAMP | Evaluation completion time (UTC) |
| verdict | TEXT | One of: promote, rollback, hold |
| config_hash | TEXT | SHA-256 of the config file used |
| config_snapshot | TEXT (JSON) | Full config at time of evaluation |

### criteria_results

| Column | Type | Description |
|---|---|---|
| id | TEXT (ULID) | Primary key |
| evaluation_id | TEXT | FK to evaluations.id |
| criterion_name | TEXT | Name from config (e.g., error_rate_5xx) |
| query | TEXT | Actual PromQL query executed |
| canary_value | REAL | Metric value for canary |
| baseline_value | REAL | Metric value for baseline |
| threshold | REAL | Configured threshold |
| passed | BOOLEAN | Whether this criterion passed |
| raw_response | TEXT (JSON) | Raw Prometheus API response |

### verdicts_log

| Column | Type | Description |
|---|---|---|
| id | TEXT (ULID) | Primary key |
| evaluation_id | TEXT | FK to evaluations.id |
| timestamp | TIMESTAMP | When verdict was computed |
| verdict | TEXT | promote, rollback, or hold |
| reason | TEXT | Human-readable summary of decision |
| score | REAL | Weighted aggregate score (if applicable) |

### Indexes

```sql
CREATE INDEX idx_evaluations_deployment ON evaluations(deployment_id);
CREATE INDEX idx_evaluations_verdict ON evaluations(verdict);
CREATE INDEX idx_evaluations_started ON evaluations(started_at);
CREATE INDEX idx_criteria_evaluation ON criteria_results(evaluation_id);
CREATE INDEX idx_verdicts_evaluation ON verdicts_log(evaluation_id);
```

### Migrations

Schema versioning via sequential SQL files in `migrations/`. Migration runner embedded in the binary; runs automatically on startup.

---

## 7. Repository Structure

```
canary-gate/
  README.md
  LICENSE (MIT)
  Cargo.toml
  Cargo.lock
  Dockerfile
  docker-compose.yaml
  config/
    example.yaml
  migrations/
    001_initial.sql
  src/
    main.rs
    cli.rs
    config.rs
    evaluator.rs
    metrics_source.rs
    prometheus.rs
    db.rs
    api.rs
    verdict.rs
  tests/
    evaluator_test.rs
    api_test.rs
    config_test.rs
  ui/               (optional Svelte dashboard)
    package.json
    src/
  docs/
    adr/
  .github/
    workflows/
      ci.yaml
```
