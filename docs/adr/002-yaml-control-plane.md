# ADR-002: YAML as the Control Plane

## Status

Accepted

## Context

canary-gate needs a way to define event classification rules, behavior tests, and recommendation parameters. Encoding these in Rust code would make the tool inflexible and require recompilation for each new service.

## Decision

All classification rules, test definitions, and recommendation parameters are defined in YAML configuration files. Rust code interprets YAML; it does not encode policy. Built-in test packs are also YAML files shipped with the binary.

## Consequences

- Users can customize behavior without writing code
- Classification rules are auditable and version-controllable
- Built-in packs provide sensible defaults for common patterns
- YAML parsing adds a dependency (serde_yaml) but enables flexibility
- No service-specific logic in Rust code
