# canary-gate v1 — Product Requirements Document

## 1. Product Definition

canary-gate is a production canary health gate for Kubernetes blue/green deployments.

It evaluates a single canary deployment attempt by analyzing logs over time and produces a recommendation:

- RECOMMEND_PROMOTE
- RECOMMEND_HOLD
- RECOMMEND_ROLLBACK

A dashboard allows humans to make the final decision. In non-interactive mode, the recommendation maps to the process exit code.

---

## 2. Execution Modes

| Mode | Description |
|---|---|
| Sidecar | Reads pod logs via Kubernetes Logs API |
| CLI | Reads logs from files (CI, replay, debugging) |

The evaluation engine is identical in both modes.

---

## 3. Unit of Evaluation

Deployment Attempt

Defined by:
- workload selector (namespace + labels)
- canary log stream boundary
- evaluation timing parameters

v1 evaluates canary logs only (no baseline comparison).

---

## 4. Canary Log Stream Boundary

Configured in YAML with intelligent defaults.

Defaults:
- Start: beginning of log stream or file
- End: sliding window defined by lookback
- Evaluation continues until Promote, Rollback, or Max Duration

```yaml
logstream:
  start: beginning        # beginning | now | since_timestamp
  lookback: 60s
```

---

## 5. Evaluation Cycles

A cycle is one evaluation pass over the most recent lookback window.

```yaml
evaluation:
  interval: 30s
  lookback: 60s
  min_cycles: 5
  max_duration: 15m
```

---

## 6. Canonical Event Model

Every log line may emit zero or one canonical event.

- timestamp
- level
- event_type
- fingerprint

Raw logs are never used in decision logic.

---

## 7. Event Classification (YAML-Defined)

```yaml
logging:
  format: auto   # plaintext | json | auto

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

Rules:
- First match wins
- Unmatched lines are ignored
- Classification is deterministic

---

## 8. Behavior-Driven Tests

Tests operate on event streams.

```yaml
tests:
  - name: service_starts
    then:
      - event_present:
          type: grpc_server_started
          within: 30s

  - name: no_panics
    then:
      - event_absent:
          type: panic
```

Each test returns pass, fail, or unknown.

---

## 9. Test Packs

Built-in packs:
- runtime-basic
- http-server
- grpc-server
- rollout-k8s

```yaml
packs:
  - runtime-basic
  - grpc-server

overrides:
  http_5xx:
    threshold: 2
```

---

## 10. Recommendation Engine

Hard Fail:
- Any hard-fail test fails in any cycle → RECOMMEND_ROLLBACK

Soft Fail:
- Soft-fail test fails for N consecutive cycles → RECOMMEND_ROLLBACK

Promote:
- All required tests pass
- min_cycles satisfied
- M consecutive passing cycles

Hold:
- Default state
- Any ambiguity or insufficient data

```yaml
recommendation:
  promote:
    require_min_cycles: 5
    require_consecutive_passes: 2

  rollback:
    soft_fail_consecutive_cycles: 3

  bias: hold_on_ambiguity
```

---

## 11. CLI Contract

Commands:
- watch
- evaluate
- validate
- explain

Exit Codes:
- 0: Recommend Promote
- 1: Recommend Hold
- 2: Recommend Rollback
- >2: Tool error

Output:
- JSON
- Human-readable table (TTY)

---

## 12. Dashboard

- Displays recommendation state and evidence
- Allows human action
- Records final action with actor metadata
- Does not perform deployments

---

## 13. Golden Logs

Golden logs are checked into the service repo and versioned with the service.

Used to:
- Derive expected fingerprints
- Suppress known-noise events
- Validate startup sequences

Not used for similarity scoring.

---

## 14. Non-Goals (v1)

- Baseline comparison
- Metrics-only decisions
- Auto-remediation
- ML or probabilistic inference

