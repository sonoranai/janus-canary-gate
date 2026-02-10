# ADR-006: Prometheus as Metrics Source

## Status

Accepted

## Context

Log-based evaluation alone may miss performance degradation or error rate spikes that are better captured by metrics. Prometheus is the most common metrics system in Kubernetes environments.

## Decision

Support Prometheus as a metrics source via the HTTP API v1. PromQL queries are defined in the YAML configuration. A `MetricsSource` trait allows plugging in alternative backends.

## Consequences

- Metrics evaluation complements log-based analysis
- PromQL provides powerful aggregation and filtering
- The trait-based design allows mock implementations for testing
- Prometheus dependency is optional (metrics section in config)
- reqwest adds an HTTP client dependency
