# ADR-005: SQLite for Decision History

## Status

Accepted

## Context

canary-gate needs to persist evaluation results and human override actions for audit trails. The storage solution should be zero-config and not require external infrastructure.

## Decision

Use SQLite via rusqlite (with bundled feature) for all persistence. The database stores evaluations, criteria results, and verdict logs. Schema migrations are embedded in the binary.

## Consequences

- Zero external dependencies for storage (no PostgreSQL, Redis, etc.)
- Database file is portable and inspectable with standard SQLite tools
- Bundled SQLite means no system library dependency
- Single-writer limitation is acceptable for canary-gate's workload
- Audit trail is complete and queryable via SQL
