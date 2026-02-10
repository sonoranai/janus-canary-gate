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
