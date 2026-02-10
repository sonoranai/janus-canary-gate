# ADR-003: Streaming Evaluation

## Status

Accepted

## Context

Canary logs can be very large in production. Loading entire log files into memory is neither necessary nor safe for long-running evaluations.

## Decision

Log ingestion processes lines one at a time using streaming I/O. The ingestion module reads via `BufRead`, never buffering the entire file. Event classification operates on individual lines.

## Consequences

- Memory usage stays bounded regardless of log file size
- Line-by-line processing preserves order stability
- Some operations (multi-line log entries) require future consideration
- Streaming makes the evaluation pipeline naturally composable
