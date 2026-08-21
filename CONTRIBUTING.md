# Contributing to Tree

Thank you for your interest in contributing to Tree! Tree is a quiet, lightweight, self-hosted Git hosting platform built with Rust, PostgreSQL, and TypeScript.

## Core Philosophy

- **Quiet and Minimal**: Focus on the core repository engine and clean developer ergonomics.
- **Evidence-Based Engineering**: Every significant claim or optimization must be backed by a test or measurement.
- **Modular and Robust**: Strong component isolation, explicit error handling, and concurrency safety.

## Development Setup

### Prerequisites

- **Rust**: 1.80+ (`rustup toolchain install stable`)
- **PostgreSQL**: 15+ (or Docker)
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
cargo test --all-targets --all-features
```

### Building the Project

```bash
cargo build --workspace
```

## Pull Request Process

1. Fork the repository and create your branch from `main`.
2. Ensure all tests pass (`cargo test`).
3. Maintain documentation and update the engineering log for major changes.
4. Open a clear, concise Pull Request.
