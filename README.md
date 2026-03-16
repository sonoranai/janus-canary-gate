# janus-canary-gate

<a href="#install"><img src="https://cdn.simpleicons.org/rust/DEA584" width="28" height="28" alt="Rust"></a>&nbsp;&nbsp;<a href="#architecture"><img src="https://cdn.simpleicons.org/sqlite" width="28" height="28" alt="SQLite"></a>&nbsp;&nbsp;<a href="#kubernetes-operator"><img src="https://cdn.simpleicons.org/kubernetes" width="28" height="28" alt="Kubernetes"></a>&nbsp;&nbsp;<a href="#prometheus-metrics"><img src="https://cdn.simpleicons.org/prometheus" width="28" height="28" alt="Prometheus"></a>&nbsp;&nbsp;<a href="#webhook-integration"><img src="https://cdn.simpleicons.org/argo/EF7B4D" width="28" height="28" alt="Argo"></a>&nbsp;&nbsp;<a href="https://www.anthropic.com/"><img src="https://cdn.simpleicons.org/anthropic" width="28" height="28" alt="Anthropic"></a>

Canary deployment health checks without a metrics pipeline. A single binary that reads your application's log output, decides if the deploy is healthy, and exits with a code — promote, hold, or rollback. Works anywhere you can run a process: bare metal, VMs, CI pipelines, systemd units, shell scripts. Kubernetes supported, not required.

When you do have Prometheus, Canary Gate queries it too. And it exposes webhook endpoints compatible with Argo Rollouts and Flagger, so you can plug log-based analysis into an existing progressive delivery setup without replacing anything.

---

<br>

## Table of Contents

- [Why](#why)
- [Install](#install)
- [Quick Start](#quick-start)
- [Exit Codes](#exit-codes)
- [Configuration](#configuration)
- [CLI Commands](#cli-commands)
- [Prometheus Metrics](#prometheus-metrics)
- [Webhook Integration](#webhook-integration)
- [Kubernetes Operator](#kubernetes-operator)
- [Where Canary Gate Fits](#where-canary-gate-fits)
- [Architecture](#architecture)
- [Testing](#testing)

<br>

---

<br>

## Why

Most canary analysis tools — Flagger, Argo Rollouts, Kayenta — need a running Prometheus to tell you whether a deploy is healthy. But when an engineer watches a deploy, they don't start with dashboards. They tail the logs. They look for `listening on port 8080` or `FATAL: connection refused` or a stack trace. Logs are the first signal that something is wrong, and they show up before error rate metrics have time to aggregate.

Canary Gate encodes that workflow. You write YAML that describes what "healthy" looks like — which log events should appear, which ones shouldn't, what rates are acceptable — and the tool checks your logs against those rules. No metrics infrastructure required.

This also means self-hosted services outside of Kubernetes are first-class canary candidates, not afterthoughts. If the process produces logs, Canary Gate can evaluate it.

<br>

---

<br>

## Install

**Requirements:** Rust 1.70+

```bash
git clone https://github.com/sonoranai/janus-canary-gate.git
cd janus-canary-gate
cargo build --release
cp target/release/canary-gate /usr/local/bin/
```

<br>

With the optional Kubernetes operator:

```bash
cargo build --release --features operator
cp target/release/canary-gate-operator /usr/local/bin/
```

<br>

---

<br>

## Quick Start

**1. Define your health checks** — create a `canary.yaml`:

```yaml
logstream:
  start: beginning
  lookback: 60s

logging:
  events:
    - type: http_server_started
      level: info
      match:
        any:
          - contains: "HTTP server listening"

    - type: panic
      level: fatal
      match:
        any:
          - contains: "panic:"
          - contains: "fatal error"

    - type: http_5xx
      level: error
      match:
        any:
          - contains: "HTTP 5"
          - regex: "status=[5]\\d{2}"

tests:
  - name: service_starts
    severity: hard
    then:
      - event_present:
          type: http_server_started
          within: 30s

  - name: no_panics
    severity: hard
    then:
      - event_absent:
          type: panic

  - name: low_error_rate
    severity: soft
    then:
      - rate:
          type: http_5xx
          threshold: 5.0
          operator: less_than
```

<br>

**2. Validate your config:**

```bash
canary-gate validate --config canary.yaml
```

<br>

**3. Evaluate:**

```bash
canary-gate evaluate --config canary.yaml --log app.log
echo $?
#  0  promote
#  1  hold
#  2  rollback
```

<br>

**4. Gate a deploy:**

```bash
canary-gate evaluate -c canary.yaml -l app.log \
  && kubectl set image deploy/myapp app=myapp:v2
```

<br>

---

<br>

## Exit Codes

Exit codes are the original API. They work with everything.

| Code | Meaning | Action |
|:----:|:--------|:-------|
| `0` | Promote | Deploy is healthy — proceed |
| `1` | Hold | Insufficient signal — wait and re-evaluate |
| `2` | Rollback | Deploy is unhealthy — revert |
| `>2` | Tool error | Bad config, missing file, internal failure |

```bash
# Roll back on failure
canary-gate evaluate -c canary.yaml -l app.log \
  || kubectl rollout undo deploy/myapp
```

```bash
# Polling loop — same pattern as a health check
while true; do
    canary-gate evaluate -c canary.yaml -l app.log
    rc=$?
    [ $rc -eq 0 ] && echo "promote" && break
    [ $rc -eq 2 ] && echo "rollback" && break
    sleep 30  # hold — check again
done
```

<br>

---

<br>

## Configuration

Health definitions live in YAML, version-controlled next to your application code. See [config/example.yaml](config/example.yaml) for a full reference.

<br>

### Log Events

Define patterns to classify log lines into typed events:

```yaml
logging:
  format: auto  # auto-detects JSON, logfmt, or plain text

  events:
    - type: grpc_server_started
      level: info
      match:
        any:
          - contains: "gRPC server listening"
          - contains: "Started gRPC server"

    - type: panic
      level: fatal
      match:
        any:
          - contains: "panic:"
          - contains: "fatal error"
```

<br>

### Behavioral Tests

Tests assert conditions over classified events. Each test has a severity:

- **hard** — immediate rollback on failure
- **soft** — rollback after N consecutive failures

```yaml
tests:
  - name: service_starts
    severity: hard
    then:
      - event_present:
          type: grpc_server_started
          within: 30s

  - name: low_error_rate
    severity: soft
    then:
      - rate:
          type: http_5xx
          threshold: 5.0
          operator: less_than
```

<br>

### Recommendation Engine

The state machine that turns test results into verdicts:

```yaml
recommendation:
  promote:
    require_min_cycles: 5
    require_consecutive_passes: 2
  rollback:
    soft_fail_consecutive_cycles: 3
  bias: hold_on_ambiguity
```

| State | Behavior |
|:------|:---------|
| Hard fail | Immediate rollback |
| Soft fail | Rollback after N consecutive cycles |
| All pass | Promote after M cycles + K consecutive passes |
| Ambiguous | Hold |

<br>

### Built-in Test Packs

```
runtime-basic    Process lifecycle (start, ready, panic, OOM)
http-server      HTTP health (server start, 5xx rate)
grpc-server      gRPC health (server start, availability)
rollout-k8s      K8s rollout signals (pod ready, crash loop, image pull)
```

<br>

---

<br>

## CLI Commands

### `evaluate` — Run the analysis pipeline

```bash
canary-gate evaluate --config canary.yaml --log app.log
canary-gate evaluate --config canary.yaml --log app.log --format json
```

JSON output pipes into whatever you need downstream:

```bash
# Extract just the recommendation
canary-gate evaluate -c canary.yaml -l app.log -f json \
  | jq -r '.recommendation'

# Feed the full verdict into a notification
canary-gate evaluate -c canary.yaml -l app.log -f json \
  | jq '{text: .recommendation, details: .reasoning}' \
  | curl -X POST -d @- https://hooks.slack.com/services/...

# Log verdicts for audit
canary-gate evaluate -c canary.yaml -l app.log -f json \
  >> /var/log/canary-verdicts.jsonl
```

<br>

### `validate` — Check config syntax

```bash
canary-gate validate --config canary.yaml
```

<br>

### `watch` — Live TUI dashboard

```bash
canary-gate watch --config canary.yaml --tui
```

<br>

### `history` — Query past verdicts

```bash
canary-gate history --db canary-gate.db
```

Every verdict is stored in SQLite with full reasoning — classification results, test outcomes, metric values, and the final recommendation.

<br>

### CI Integration

Canary Gate slots into CI the same way tests do — a step that either passes or doesn't:

```yaml
# GitHub Actions
- name: Canary health gate
  run: canary-gate evaluate -c canary.yaml -l canary.log

# GitLab CI
canary_check:
  script: canary-gate evaluate -c canary.yaml -l canary.log
  allow_failure: false
```

<br>

---

<br>

## Prometheus Metrics

When you have a metrics pipeline, canary-gate queries Prometheus alongside log analysis. Metric results feed into the same verdict engine as behavioral tests.

```yaml
metrics:
  type: prometheus
  endpoint: http://prometheus:9090
  queries:
    - name: error_rate
      query: "rate(http_requests_total{status=~\"5..\"}[5m])"
      threshold: 0.01
      operator: less_than
      severity: soft
```

Statistical comparison uses the Mann-Whitney U test — baseline vs. canary metric distributions, no assumptions about normality.

> **TLS:** For non-localhost Prometheus endpoints, use `https://` to prevent metric queries from being transmitted in plaintext. In-cluster Prometheus often runs without TLS behind a network policy — verify your environment before relying on plain HTTP.

<br>

---

<br>

## Webhook Integration

canary-gate exposes webhook endpoints compatible with both Argo Rollouts and Flagger. Keep your existing progressive delivery controller and add log-based analysis alongside metric checks.

```
  Argo Rollouts / Flagger
  AnalysisTemplate:
    url: canary-gate:8080/api/v1/webhooks
            |
            | POST (on each analysis interval)
            v
       canary-gate
  logs ──────> classify ──> tests ──┐
                                    ├──> verdict
  prometheus ──> queries ──────────┘
            |
            | { "recommendation": "promote" }
            v
  Argo/Flagger acts on result
```

<br>

**Argo Rollouts** — returns `{ recommendation, score, passed }`. Configure with `successCondition: "result.recommendation == 'promote'"`.

**Flagger** — returns HTTP 200 for promote/hold, HTTP 400 for rollback. Follows the Flagger webhook contract directly.

> **Security:** The API server binds to `127.0.0.1:8080` by default and does not include authentication middleware. In a Kubernetes pod, localhost is reachable from any container in the same network namespace. For production deployments, place the webhook endpoints behind a network policy or service mesh mTLS, or front with an authenticating reverse proxy.

<br>

---

<br>

## Kubernetes Operator

Behind the `operator` cargo feature flag, canary-gate includes a CRD-based controller that watches `CanaryGate` resources and runs evaluations autonomously.

```bash
cargo build --release --features operator
```

The operator fetches pod logs via the K8s API, queries Prometheus, runs the full evaluation pipeline, and writes results back to the CRD status subresource. Useful when you want canary-gate to run as a long-lived controller rather than a CLI invocation.

<br>

---

<br>

## Where canary-gate Fits

The existing canary tooling ecosystem operates on numeric metrics. canary-gate adds log-based behavioral testing and plugs into those tools as a webhook provider.

<br>

| Capability | canary-gate | Flagger | Argo Rollouts | Kayenta |
|:-----------|:-----------:|:-------:|:-------------:|:-------:|
| Log-based behavioral tests | x | | | |
| Prometheus metrics | x | x | x | x |
| Statistical analysis | x | | | x |
| Hard/soft severity model | x | | | |
| Standalone binary (no K8s) | x | | | |
| K8s CRD operator | x | x | x | |
| Webhook provider | x | | | |
| Traffic shifting | | x | x | |
| SQLite audit trail | x | | | |
| TUI dashboard | x | | | |

canary-gate does not shift traffic. Flagger and Argo Rollouts do not analyze logs. These tools are complementary.

<br>

---

<br>

## Architecture

```
src/
├── ingestion.rs        Log file streaming, line-by-line processing
├── classification.rs   Pattern matching: log lines → typed events
├── behavior.rs         Behavioral test engine (event_present, event_absent, rate)
├── verdict.rs          Verdict assembly from test + metric results
├── recommendation.rs   State machine: hard/soft severity → promote/hold/rollback
├── stats/              Statistical analysis (Mann-Whitney U)
├── metrics/            Prometheus client + metric query engine
├── db.rs               SQLite audit trail — every verdict with full reasoning
├── api.rs              Webhook endpoints (Argo Rollouts, Flagger)
├── cli.rs              CLI interface (clap)
├── config.rs           YAML config parsing and validation
├── tui/                Terminal dashboard (ratatui)
└── operator/           K8s CRD controller (behind feature flag)
```

<br>

### Pipeline

```
Log File ──> Ingestion ──> Classification ──> Behavioral Tests ──┐
                            (YAML rules)       (YAML tests)      │
                                                                  ├──> Verdict
Prometheus ──> Metric Queries ──┐                                 │   (exit code)
                (PromQL)        ├──> Statistics ──────────────────┘
                                │    (Mann-Whitney U)
                                │
                            Baselines
                            vs Canary
```

<br>

### Design

- **YAML is the control plane.** Classification rules, tests, thresholds — all declared in config. No code changes to add a health check.
- **Streaming.** Logs processed line-by-line, never fully buffered.
- **Deterministic.** Same inputs, same outputs. Always.
- **Advisory.** The engine recommends. Humans (or automation) act.

<br>

---

<br>

## Testing

```bash
cargo test                    # unit + integration tests
cargo clippy                  # lint
cargo fmt -- --check          # format check
```

<br>

---

<br>

## License

MIT
