# canary-gate — Agent Directions

This file is authoritative for all agentic development work on canary-gate.

## Core Principles (Non-Negotiable)

1. Determinism first.
   - Given the same inputs, outputs must be identical.
   - No non-deterministic ordering, time dependence, or randomness.

2. Test-driven development is mandatory.
   - Every feature must be expressed as a failing test first.
   - Tests must use static fixtures (log files, YAML configs).
   - No test may depend on wall-clock time, network calls, or live Kubernetes.

3. YAML is the control plane.
   - Behavior, classification, heuristics, and defaults are defined in YAML.
   - Rust code interprets YAML; it does not encode policy.

4. No speculative intelligence.
   - No ML.
   - No probabilistic logic.
   - No heuristic that cannot be explained in plain English.

5. Human-in-the-loop safety.
   - Engine produces recommendations only.
   - Humans or external controllers take final action.

6. Scope discipline.
   - If a requirement is not explicitly in the PRD, do not implement it.
   - Prefer omission over guessing.

## Architecture Guardrails

- Streaming evaluation; never load entire logs into memory.
- Event classification precedes test evaluation.
- Tests operate on canonical events, not raw log lines.
- All verdict logic is pure and side-effect free.
- CLI, API, and dashboard share the same evaluation engine.

## Forbidden Actions

- Adding features “for convenience”
- Introducing baseline comparison (v1 is canary-only)
- Encoding service-specific logic in Rust
- Using regex without YAML configuration
- Logging secrets or raw logs verbatim

## Required Outputs per Unit of Work

- Tests (unit or integration)
- Implementation
- README or doc updates if behavior changes

If a requirement is ambiguous, stop and surface it explicitly.

