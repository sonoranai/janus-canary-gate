# canary-gate

Canary deployment health checks without a metrics pipeline. A single
binary that reads your application's log output, decides if the deploy
is healthy, and exits with a code -- promote, hold, or rollback. It
works anywhere you can run a process: bare metal, VMs, CI pipelines,
systemd units, shell scripts. Kubernetes supported, not required.

### Why

Most canary analysis tools -- Flagger, Argo Rollouts, Kayenta -- need a
running Prometheus to tell you whether a deploy is healthy. But when an
engineer watches a deploy, they don't start with dashboards. They tail
the logs. They look for "listening on port 8080" or "FATAL: connection
refused" or a stack trace. Logs are the first signal that something is
wrong, and they show up before error rate metrics have time to aggregate.

canary-gate encodes that workflow. You write YAML that describes what
"healthy" looks like -- which log events should appear, which ones
shouldn't, what rates are acceptable -- and the tool checks your logs
against those rules. No metrics infrastructure required.

This also means self-hosted services outside of Kubernetes are
first-class canary candidates, not afterthoughts. If the process
produces logs, canary-gate can evaluate it.

When you do have Prometheus, canary-gate can query it too. And it exposes
webhook endpoints compatible with Argo Rollouts and Flagger, so you can
plug log-based analysis into an existing progressive delivery setup
without replacing anything.

### How

canary-gate follows the Unix philosophy: do one thing, use exit codes,
produce structured output, compose with other tools. The best deploy
tooling isn't a platform -- it's a pipeline of small tools that each
do their job and get out of the way.

The interface is simple. Feed it a config and a log file. It exits
with a code.

```
canary-gate evaluate -c canary.yaml -l app.log
echo $?

  0   promote
  1   hold
  2   rollback
```

Exit codes are the original API. They work with everything:

```bash
# Gate a deploy on canary health -- promote only if healthy
canary-gate evaluate -c canary.yaml -l app.log \
  && kubectl set image deploy/myapp app=myapp:v2

# Roll back on failure
canary-gate evaluate -c canary.yaml -l app.log \
  || kubectl rollout undo deploy/myapp
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

# Log verdicts to a file for audit
canary-gate evaluate -c canary.yaml -l app.log -f json \
  >> /var/log/canary-verdicts.jsonl
```

It fits naturally into a polling loop -- the same pattern as a
health check:

```bash
# Wait for the canary to either pass or fail
while true; do
    canary-gate evaluate -c canary.yaml -l app.log
    rc=$?
    [ $rc -eq 0 ] && echo "promote" && break
    [ $rc -eq 2 ] && echo "rollback" && break
    sleep 30  # hold -- check again
done
```

It slots into CI the same way tests do -- a step that either
passes or doesn't:

```yaml
# GitHub Actions
- name: canary health gate
  run: canary-gate evaluate -c canary.yaml -l canary.log

# GitLab CI
canary_check:
  script: canary-gate evaluate -c canary.yaml -l canary.log
  allow_failure: false
```

The health definition is a YAML file, version-controlled next to
your application code. The tool reads logs, applies rules, prints
a result. No daemon, no cluster dependency, no SDK integration.
It's a filter -- log lines go in, a verdict comes out.

## Quickstart

```bash
cargo build --release

canary-gate validate --config config/example.yaml
canary-gate evaluate --config config/example.yaml --log path/to/canary.log
canary-gate watch    --config config/example.yaml --tui
canary-gate history  --db canary-gate.db
```

## Exit Codes

```
0   promote
1   hold
2   rollback
>2  tool error
```

## Architecture

```
                          canary-gate
  +------------------------------------------------------------------+
  |                                                                  |
  |  Log File                                                        |
  |    |                                                             |
  |    v                                                             |
  |  Ingestion --> Classification --> Behavior Tests --+             |
  |                 (YAML rules)      (YAML tests)     |             |
  |                                                    +--> Verdict  |
  |  Prometheus --> Metric Queries ---+                |   (exit code)|
  |                 (PromQL)          |                |             |
  |                                   +--> Statistics -+             |
  |                                        (Mann-Whitney U)          |
  |                                                                  |
  |  [ Recommendation State Machine ]                                |
  |    hard fail  --> immediate rollback                             |
  |    soft fail  --> rollback after N consecutive cycles             |
  |    all pass   --> promote after M cycles + K consecutive passes  |
  |    ambiguous  --> hold                                           |
  |                                                                  |
  |  [ SQLite ]  <-- every verdict stored with full reasoning        |
  |                                                                  |
  +------------------------------------------------------------------+
```

Design choices:

- **YAML is the control plane.** Classification rules, tests, thresholds --
  all declared in config. No code changes to add a health check.
- **Streaming.** Logs processed line-by-line, never fully buffered.
- **Deterministic.** Same inputs, same outputs. Always.
- **Advisory.** The engine recommends. Humans (or automation) act.

## Where canary-gate Fits

The existing canary tooling ecosystem -- Flagger, Argo Rollouts, Kayenta --
operates on numeric metrics. canary-gate adds log-based behavioral testing
and plugs into those tools as a webhook provider.

```
  What each tool does
  ===================

  Flagger          Progressive delivery controller for K8s.
                   Drives traffic shifting. Calls webhooks for analysis.

  Argo Rollouts    Progressive delivery controller for K8s.
                   Drives traffic shifting. Calls webhooks for analysis.

  Kayenta          Automated canary analysis from Netflix/Google.
                   Statistical comparison of baseline vs canary metrics.
                   Tightly coupled to Spinnaker.

  canary-gate      Log + metrics analysis engine.
                   Behavioral test framework with severity state machine.
                   Runs standalone or as a webhook backend for the above.
```

### Comparison

```
  +------------------------------+-------------+---------+-------+---------+
  |                              | canary-gate | Flagger | Argo  | Kayenta |
  +------------------------------+-------------+---------+-------+---------+
  | Log-based behavioral tests   |     x       |         |       |         |
  | Prometheus metrics           |     x       |    x    |   x   |    x    |
  | Statistical analysis         |     x       |         |       |    x    |
  | Hard/soft severity model     |     x       |         |       |         |
  | Standalone binary (no K8s)   |     x       |         |       |         |
  | K8s CRD operator             |     x       |    x    |   x   |         |
  | Webhook provider             |     x       |         |       |         |
  | Traffic shifting             |             |    x    |   x   |         |
  | SQLite audit trail           |     x       |         |       |         |
  | TUI dashboard                |     x       |         |       |         |
  +------------------------------+-------------+---------+-------+---------+
```

canary-gate does not shift traffic. Flagger and Argo Rollouts do not analyze
logs. These tools are complementary.

### Webhook Integration

canary-gate exposes webhook endpoints compatible with both Argo Rollouts
and Flagger. This lets you keep your existing progressive delivery
controller and add log-based analysis alongside metric checks.

```
  +-------------------------------------------+
  |        Argo Rollouts / Flagger            |
  |                                           |
  |  AnalysisTemplate / webhook config:       |
  |    url: canary-gate:8080/api/v1/webhooks  |
  +---------------------+---------------------+
                        |
                        | POST (on each analysis interval)
                        v
  +-------------------------------------------+
  |             canary-gate                   |
  |                                           |
  |  logs ---------> classify --> tests --+   |
  |                                      |   |
  |  prometheus ---> queries ----------+-+   |
  |                                    |     |
  |  prometheus ---> baselines --------+     |
  |                  vs canary         |     |
  |                  (Mann-Whitney)    v     |
  |                               verdict   |
  +-------------------------------------------+
                        |
                        | { "recommendation": "promote" }
                        v
          Argo/Flagger acts on result
```

**Argo Rollouts**: returns `{ recommendation, score, passed }`.
Configure with `successCondition: "result.recommendation == 'promote'"`.

**Flagger**: returns HTTP 200 for promote/hold, HTTP 400 for rollback.
Follows the Flagger webhook contract directly.

### K8s Operator (Optional)

Behind the `operator` cargo feature flag, canary-gate includes a CRD-based
controller that watches `CanaryGate` resources and runs evaluations
autonomously.

```bash
cargo build --release --features operator
```

The operator fetches pod logs via the K8s API, queries Prometheus, runs the
full evaluation pipeline, and writes results back to the CRD status
subresource. Useful when you want canary-gate to run as a long-lived
controller rather than a CLI invocation.

## Configuration

See [config/example.yaml](config/example.yaml) for a full reference.

### Built-in Test Packs

```
  runtime-basic    Process lifecycle (start, ready, panic, OOM)
  http-server      HTTP health (server start, 5xx rate)
  grpc-server      gRPC health (server start, availability)
  rollout-k8s      K8s rollout signals (pod ready, crash loop, image pull)
```

## Development

```bash
make test       # run tests
make lint       # clippy
make check      # fmt + clippy + test
```

## License

MIT
