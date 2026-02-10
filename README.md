# canary-gate

Production canary health gate for Kubernetes blue/green deployments.

Evaluates canary deployments by analyzing **logs** (YAML-driven event classification) and **Prometheus metrics** (PromQL queries), producing auditable promote/hold/rollback recommendations with full decision traces.

## Quickstart

```bash
# Build
cargo build --release

# Validate a configuration
canary-gate validate --config config/example.yaml

# Evaluate a log file
canary-gate evaluate --config config/example.yaml --log path/to/canary.log

# Watch with TUI dashboard
canary-gate watch --config config/example.yaml --tui

# Query decision history
canary-gate history --db canary-gate.db
```

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Recommend Promote |
| 1 | Recommend Hold |
| 2 | Recommend Rollback |
| >2 | Tool error |

## Architecture

```
Log File ──> Ingestion ──> Classification ──> Behavior Tests ──> Recommendation ──> Verdict
                              (YAML rules)     (YAML tests)      (state machine)    (exit code)
```

- **YAML is the control plane** — all classification rules and tests are YAML-configured
- **Streaming evaluation** — logs processed line-by-line, never fully buffered
- **Deterministic** — same inputs always produce the same outputs
- **Human-in-the-loop** — engine produces recommendations, humans take final action

## How canary-gate Compares

canary-gate is a log-first behavioral test engine for canary deployments — the only open-source tool that evaluates deployment health by analyzing application logs alongside Prometheus metrics.

### Feature Comparison

| Capability | canary-gate | Flagger | Argo Rollouts | Kayenta |
|---|---|---|---|---|
| Log-based behavioral tests | Yes (core) | No | No | No |
| Prometheus metrics | Yes | Yes | Yes | Yes |
| Statistical analysis (Mann-Whitney) | Yes | No | No | Yes (custom) |
| Standalone binary | Yes | No (K8s only) | No (K8s only) | No (Spinnaker) |
| Webhook provider for Argo/Flagger | Yes | N/A | N/A | N/A |
| K8s CRD operator | Yes (optional) | Yes (core) | Yes (core) | No |
| Hard/soft severity state machine | Yes | No | No | No |
| SQLite audit trail | Yes | No | No | No |
| YAML test packs | Yes | No | No | No |
| TUI dashboard | Yes | No | No | No |

### Key Differentiators

**Log-first analysis** — While existing tools operate exclusively on numeric metrics, canary-gate can classify and evaluate application log events using YAML-configured rules. Logs are often the earliest signal that a deployment is unhealthy.

**Standalone & composable** — Works as a standalone CLI/binary or as a webhook provider that plugs into existing Argo Rollouts / Flagger workflows. No Kubernetes dependency required for core functionality.

**Behavioral test framework** — Tests are expressed as behavioral assertions (`event_present`, `event_absent`, rate thresholds) with hard/soft severity levels, providing a structured way to define what "healthy" means for your application.

### Composition with Existing Tools

```
┌─────────────────────────────────────────────────────┐
│              Argo Rollouts / Flagger                │
│                                                      │
│   AnalysisTemplate:                                  │
│     webhook: canary-gate/api/v1/webhooks/argo       │
└──────────────────────┬───────────────────────────────┘
                       │ POST
                       ▼
┌─────────────────────────────────────────────────────┐
│                   canary-gate                        │
│                                                      │
│   Logs ──> Classification ──> Behavioral Tests ──┐  │
│                                                   ├──> Verdict
│   Prometheus ──> Metrics Queries ────────────────┘  │
│                  Statistical Analysis                │
└─────────────────────────────────────────────────────┘
```

## Configuration

See [config/example.yaml](config/example.yaml) for a full configuration reference.

### Built-in Test Packs

| Pack | Description |
|------|-------------|
| `runtime-basic` | Process lifecycle (start, ready, panic, OOM) |
| `http-server` | HTTP health (server start, 5xx rate) |
| `grpc-server` | gRPC health (server start, availability) |
| `rollout-k8s` | Kubernetes rollout signals (pod ready, crash loop, image pull) |

## Development

```bash
# Run tests
make test

# Lint
make lint

# Full check (fmt + clippy + test)
make check
```

## License

MIT
