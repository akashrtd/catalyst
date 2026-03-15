# CLI Development Process (March 2026)

## Overview

Catalyst is a CLI-first tool. This document outlines the development process for building Catalyst.

## Prerequisites

- **Rust** 1.70+ (2021 edition or later)
- **cargo** 1.70+ (2021 edition or later)

## Project Setup

### 1. Initialize Git Repository

```bash
mkdir catalyst
cd catalyst
git init
```

### 2. Create Cargo Workspace

```bash
cargo new --lib catalyst-core
cargo new --lib catalyst-llm
cargo new --lib catalyst-tools
cargo new --lib catalyst-tui
cargo new --lib catalyst-cli
```

### 3. Create workspace Cargo.toml

```toml
[workspace]
members = [
    { name = "catalyst-core", path = "catalyst-core" },
    { name = "catalyst-llm", path = "catalyst-llm" },
    { name = "catalyst-tools", path = "catalyst-tools" },
    { name = "catalyst-tui", path = "catalyst-tui" },
    { name = "catalyst-cli", path = "catalyst-cli" },
]
resolver = "2")
package = "1"

[workspace.dependencies]
tokio = { version = "1" }
serde = { version = "1", features = ["derive"] }
anyhow = "1"
thiserror = "1"
ratatui = "0.26"
crossterm = "0.28"
```

### 4. Initialize Crates

```bash
cd catalyst-core && cargo init --lib
cd catalyst-llm && cargo init --lib
cd catalyst-tools && cargo init --lib
cd catalyst-tui && cargo init --lib
cd catalyst-cli && cargo init --lib
```

## Development Workflow

### 1. Start with Core Library

```bash
cd catalyst-core
cargo test
```

### 2. Build All Crates

```bash
cargo build --workspace
```

### 3. Run the CLI

```bash
cargo run --package catalyst-cli
```

## Hot Reload Development

Use `cargo-watch` for automatic recompilation:

```bash
cargo watch -p catalyst-core
```

## Testing

```bash
cargo test --workspace
cargo test --package catalyst-core
cargo test --package catalyst-llm
```

## Release Process

1. Update version in all Cargo.toml files
2. Update CHANGELOG.md in each crate
3. Create git tag
4. Build and publish to crates.io

