# canary-gate — Agentic Work Plan (v1)

This file defines the ordered, test-driven units of work for agentic development.

Each unit must be completed fully before moving to the next.

---

## UOW-01: Project Skeleton + CLI

Goal:
- Compilable Rust project
- CLI commands wired

Scope:
- Project scaffolding
- clap-based CLI
- Stub commands: watch, evaluate, validate, explain

Tests:
- CLI argument parsing
- Exit code mapping

---

## UOW-02: YAML Schema + Validation

Goal:
- Deterministic config parsing

Scope:
- YAML structs
- Enum validation
- Default injection

Tests:
- Valid config loads
- Invalid config fails
- Defaults applied correctly

---

## UOW-03: Log Ingestion (File Mode)

Goal:
- Deterministic streaming log reader

Scope:
- Plaintext logs
- JSON logs
- No full buffering

Tests:
- Static fixture logs
- Order stability

---

## UOW-04: Event Classification Engine

Goal:
- YAML-driven event extraction

Scope:
- Match rules
- Fingerprint normalization

Tests:
- Same input → same fingerprint
- First-match-wins semantics

---

## UOW-05: Behavior Test Engine

Goal:
- Evaluate BDD tests over event streams

Scope:
- event_present
- event_absent
- rate-based tests

Tests:
- Per-operator unit tests

---

## UOW-06: Recommendation State Machine

Goal:
- Promote / Hold / Rollback logic

Scope:
- Cycle tracking
- Consecutive failure logic

Tests:
- All state transitions

---

## UOW-07: End-to-End Canary Fixtures

Goal:
- Prove start-to-finish correctness

Scope:
- Sample HTTP/gRPC logs
- Promote case
- Rollback case
- Hold case

Tests:
- End-to-end evaluation

---

## UOW-08: Dashboard (Read-Only)

Goal:
- Observability and human decision support

Scope:
- Current state endpoint
- Evidence visualization

Tests:
- API snapshot tests

---

## Rules

- Every UOW must be test-first
- No cross-UOW feature creep
- Deterministic fixtures only

