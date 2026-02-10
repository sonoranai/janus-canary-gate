# ADR-001: Single Crate with lib.rs + main.rs

## Status

Accepted

## Context

Integration tests in `tests/` need access to internal types (config structs, event types, recommendation engine) to construct test scenarios without spawning the binary. A common Rust pattern is to separate the library and binary targets within a single crate.

## Decision

Use a single crate with both `lib.rs` and `main.rs`. The library (`lib.rs`) exports all modules publicly. The binary (`main.rs`) imports from `canary_gate::*` and handles only CLI dispatch and tracing initialization.

## Consequences

- Integration tests can import library types directly via `use canary_gate::*`
- No need for a workspace or multi-crate setup at this scale
- Binary-specific logic (process exit, tracing init) stays isolated in `main.rs`
- All domain logic is testable without subprocess spawning
