# Catalyst - Research Overview

## Project Summary

Catalyst is a minimal, research-driven AI coding CLI with full TUI support. Unlike traditional AI coding agents that optimize for speed and the shortest path, Catalyst optimizes for **correctness** through rigorous research, formal verification, and simulation.

## Core Philosophy

| Principle | Description |
|-----------|-------------|
| **Research-driven** | Investigate best practices, don't guess |
| **TLA+ verification** | Formally prove correctness, prevent bugs before they exist |
| **Explain choices** | Show WHY, not just WHAT |
| **Correct user errors** | Push back when user is wrong |
| **Inert = Secure + Stable + Flawless** | Code that doesn't break, doesn't have vulnerabilities, doesn't fail |
| **No shortcuts** | Complex projects need rigorous paths, not shortest ones |

## Key Differentiators from Other AI Coding Agents

| Aspect | Traditional Agents | Catalyst |
|--------|-------------------|----------|
| Goal | Speed, shortest path | Correctness, rigorous path |
| Decision making | Auto-generated | Debated, explained |
| Verification | Testing after code | TLA+ before, simulation after |
| State | Mutable, can be changed | Inert (immutable), superseded only |
| User interaction | Compliant, executes requests | Pushes back on errors |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                      CATALYST CORE                          │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  │
│  │   Research  │  │   TLA+      │  │    Simulation       │  │
│  │   Engine    │→ │  Verifier   │→ │    Engine           │  │
│  │             │  │             │  │  (Hybrid Sandbox)   │  │
│  └─────────────┘  └─────────────┘  └─────────────────────┘  │
│         ↓                ↓                   ↓              │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Decision Engine (Debate & Explain)        │  │
│  └───────────────────────────────────────────────────────┘  │
│                            ↓                                │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Code Generator (No Shortcuts)             │  │
│  └───────────────────────────────────────────────────────┘  │
│                            ↓                                │
│  ┌───────────────────────────────────────────────────────┐  │
│  │              Inert State (Immutable Output)            │  │
│  └───────────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────────┘
                              ↓
                    ┌─────────────────┐
                    │   TUI Layer     │
                    │   (ratatui)     │
                    └─────────────────┘
```

## Research Documents

### Core Technology
1. [Language Comparison](./01-language-comparison.md) - Rust vs alternatives
2. [Async Runtime](./08-async-runtime.md) - tokio vs async-std vs smol
3. [TUI Frameworks](./02-tui-frameworks.md) - Terminal UI options for Rust
4. [Terminal Handling](./13-terminal-handling.md) - crossterm, raw mode, events

### LLM & Integration
5. [LLM Integration](./03-llm-integration.md) - Anthropic API integration
6. [Tool System](./04-tool-system.md) - read/write/edit/bash implementation
7. [Event Architecture](./05-event-architecture.md) - Async event loop design

### Data & Configuration
8. [Serialization](./10-serialization.md) - serde, JSON, TOML strategies
9. [Configuration Management](./14-configuration-management.md) - Config loading, validation
10. [Error Handling](./09-error-handling.md) - anyhow, thiserror patterns

### Quality & Observability
11. [Testing Framework](./12-testing-framework.md) - Unit, integration, mocking
12. [Logging & Observability](./11-logging-observability.md) - tracing, metrics
13. [Security](./15-security.md) - API keys, tool restrictions, input validation
14. [Performance](./16-performance.md) - Optimization techniques, benchmarks

### Future Features
15. [TLA+ Integration](./06-tla-integration.md) - Formal verification approach
16. [Simulation Engine](./07-simulation-engine.md) - Hybrid sandbox system

### Market Research
17. [AI Coding Agent Landscape](./17-ai-coding-agent-landscape.md) - Competitive analysis, success patterns

## MVP Scope

- Basic TUI (ratatui + crossterm)
- LLM client (Anthropic API only)
- Tool system (read/write/edit/bash)
- Event loop architecture

## Future Phases

- Phase 2: Research engine + basic simulation
- Phase 3: TLA+ integration
- Phase 4: Multi-provider LLM support
- Phase 5: Extension system

## Technology Stack Summary (March 2026)

| Category | Choice | Version | Rationale |
|----------|--------|---------|-----------|
| **Language** | Rust | 1.85 | Memory safety, performance, strong type system |
| **Async Runtime** | tokio | 1.42 | Industry standard, rich ecosystem, work-stealing scheduler |
| **TUI Framework** | ratatui | 0.29 | Mature, widget-based, active community |
| **Terminal Backend** | crossterm | 0.28 | Cross-platform, Windows support, event streaming |
| **HTTP Client** | reqwest | 0.12 | Async, streaming support, connection pooling |
| **Serialization** | serde + serde_json + toml | 1.0 / 1.0 / 0.8 | Standard, zero-copy parsing |
| **Error Handling** | anyhow (app) + thiserror (lib) | 1.0 / 2.0 | User-friendly + type-safe |
| **Logging** | tracing | 0.1 | Structured, async-aware, spans |
| **Testing** | built-in + mockall + wiremock | 0.13 / 0.6 | Unit + mocks + HTTP mocking |
| **Configuration** | TOML | 0.8 | Human-readable, serde support |
| **CLI Parsing** | clap | 4.5 | Derive macros, popular |

### MVP Dependencies (Cargo.toml)

```toml
[dependencies]
# Async
tokio = { version = "1.42", features = ["full"] }

# TUI
ratatui = "0.29"
crossterm = { version = "0.28", features = ["event-stream"] }

# HTTP & Serialization
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "2.0"

# Logging
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }

# CLI
clap = { version = "4.5", features = ["derive"] }

# Utilities
dirs = "6.0"
uuid = { version = "1.11", features = ["v4"] }
```
