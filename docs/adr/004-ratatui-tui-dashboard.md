# ADR-004: ratatui for TUI Dashboard

## Status

Accepted

## Context

The dashboard needs to work over SSH connections, in containers, and anywhere a terminal is available. Web-based dashboards would require additional infrastructure and a JavaScript toolchain.

## Decision

Use ratatui (with crossterm backend) for the interactive dashboard. The TUI renders directly in the terminal, requiring no web server, browser, or JavaScript toolchain.

## Consequences

- Works anywhere a terminal exists (SSH, containers, CI)
- No JavaScript/npm toolchain in the project (pure Rust)
- Rich interactive UI with color-coded status, tables, and keyboard input
- Limited to terminal capabilities (no images, limited layout flexibility)
- crossterm provides cross-platform terminal handling
