# Contributing to Tree

Thank you for your interest in contributing to Tree! Tree is a quiet, lightweight, self-hosted Git hosting platform built with Rust, PostgreSQL, and TypeScript.

## Core Philosophy

- **Quiet and Minimal**: Focus on the core repository engine and clean developer ergonomics.
- **Evidence-Based Engineering**: Every significant claim or optimization must be backed by a test or measurement.
- **Modular and Robust**: Strong component isolation, explicit error handling, and concurrency safety.

## Development Setup

### Prerequisites

- **Rust**: stable (`rustup toolchain install stable`, tested with `1.98`)
- **PostgreSQL**: 16+ (or Docker `postgres:16`)
- **Node.js**: 20+ and `npm`
- **Git**: 2.30+

### Setting Up the Database

```bash
# Create local development database
createdb tree_db

# Run migrations
export DATABASE_URL="postgres://localhost/tree_db"
cargo run -p tree-server -- migrate
```

### Running Tests

```bash
# Format and lint must pass
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings

# Tests require a real PostgreSQL 16 (CI uses postgres:16 service)
DATABASE_URL="postgres://tree:treepassword@localhost:5432/tree_db" cargo test --workspace --locked
```

### Building the Project

```bash
cargo build --workspace --locked
cargo build --release --locked
```

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. Ensure `cargo fmt`, `cargo clippy -D warnings` and `cargo test --workspace` (21/21 with PostgreSQL 16) all pass.
3. Maintain documentation and update the engineering log for major changes.
4. Open a clear, concise Pull Request.
