# Tree

*A quiet, self-hosted Git hosting platform.*

> **Status Notice**: Tree is an experimental, self-hosted Git hosting platform under active development. Phase 0 and Phase 1 are currently implemented, tested, and verified. Tree is not yet intended as a full replacement for existing large-scale forge platforms.

---

## Public Engineering Record

- **Living GitHub Gist**: [Tree: Living Engineering Record (`tree-engineering-gist.md`)](https://gist.github.com/bniladridas/121f0d2f10f6900faf3ffab455be757f)
- **Architecture Document**: [docs/architecture.md](docs/architecture.md)
- **API Reference**: [docs/api.md](docs/api.md)
- **Local Engineering Record**: [docs/ENGINEERING_GIST.md](docs/ENGINEERING_GIST.md)

---

## Current Status (Phase 0 & Phase 1)

Tree focuses on the core Git repository engine: native transport, metadata management, and unbloated interfaces.

- **Phase 0 (Repository & Structure)**: Modular Rust virtual workspace, clean domain interfaces, database migrations, and Docker configurations.
- **Phase 1 (Git Foundation & Metadata)**: Native Git Smart HTTP transport (`clone`, `fetch`, `push`), bare Git filesystem management, PostgreSQL metadata persistence, RBAC permission evaluator, paginated commit/branch/tag/tree inspection, and a developer CLI.

---

## Technology Stack

- **Core & Backend Daemon**: Rust 1.80+ (Axum, Tokio, sqlx, clap)
- **Metadata Storage**: PostgreSQL 16+ (relational schemas, ACID guarantees)
- **Git Engine**: Bare Git storage + Git Smart HTTP protocol (`pkt-line`, stateless RPC subprocess streaming)
- **Web Interface**: TypeScript + quiet minimalist HTML/CSS
- **Containerization**: Multi-stage Docker & Docker Compose
- **Target OS**: Linux-first (macOS compatible)

---

## System Architecture

```text
                  ┌───────────────────────────────────────────────────────────┐
                  │                      Clients                              │
                  │  (Standard Git CLI, Tree CLI, Minimal Web Interface)      │
                  └─────────────────────────────┬─────────────────────────────┘
                                                │ HTTPS / REST
                                                ▼
┌─────────────────────────────────────────────────────────────────────────────────────────┐
│                                 Tree Server (Axum)                                      │
├───────────────────────────────┬─────────────────────────────┬───────────────────────────┤
│       REST Management API     │     Smart HTTP Transport    │       Web UI Server       │
│  (/repositories, /users, etc) │  (info/refs, upload-pack,   │  (Quiet HTML/TS Frontend) │
│                               │       receive-pack)         │                           │
├───────────────────────────────┴─────────────────────────────┴───────────────────────────┤
│                                   tree-core                                             │
│       • Permission Engine (RBAC)  • Error Model  • Domain Entities  • Store Trait       │
├─────────────────────────────────────────────┬───────────────────────────────────────────┤
│                 tree-git                    │                tree-storage               │
│  • Bare Git Engine                          │  • PostgreSQL Connection Pool (sqlx)      │
│  • Stateless RPC Streaming Subprocesses     │  • User, Org, Repo, Member Schemas        │
│  • Fast Refs, Commits, Trees & Blobs        │  • MemoryStore (Isolated Unit Tests)      │
└──────────────────────┬──────────────────────┴─────────────────────┬─────────────────────┘
                       │                                            │
                       ▼                                            ▼
           ┌───────────────────────┐                    ┌───────────────────────┐
           │   Filesystem Storage  │                    │   PostgreSQL 16+      │
           │  (Bare Git Repos)     │                    │  (Metadata & Roles)   │
           └───────────────────────┘                    └───────────────────────┘
```

---

## Repository Structure

```text
tree/
├── apps/
│   ├── tree-server/       # Axum HTTP service (REST API, Git Smart HTTP, Web UI)
│   └── tree-cli/          # Developer CLI tool (`tree`)
├── crates/
│   ├── tree-core/         # Domain models, RBAC evaluator, Store traits, errors
│   ├── tree-git/          # Bare storage management, Smart HTTP framing, Git inspectors
│   └── tree-storage/      # PostgreSQL store (sqlx), migrations, test MemoryStore
├── web/                   # Minimal TypeScript web frontend
├── migrations/            # SQL schema migrations (0001_init.sql)
├── tests/                 # End-to-end integration and concurrency test suites
├── docs/                  # Architecture specifications and living engineering gist
├── docker/                # Multi-stage Dockerfile and docker-compose setup
├── Cargo.toml             # Virtual workspace configuration
├── README.md
├── LICENSE                # Apache 2.0
└── CONTRIBUTING.md
```

---

## Test Results

**15/15 automated tests passing (100% pass rate)**:

```text
running 15 tests
test permissions::tests::test_public_repo_anonymous_read ... ok
test permissions::tests::test_owner_full_access ... ok
test permissions::tests::test_private_repo_anonymous_denied ... ok
test permissions::tests::test_member_role_permissions ... ok
test engine::tests::test_sanitize_name ... ok
test smart_http::tests::test_pkt_line_formatting ... ok
test smart_http::tests::test_advertise_refs_empty_repo ... ok
test engine::tests::test_init_and_delete_repo ... ok
test test_concurrency::test_concurrent_repository_creation ... ok
test test_concurrency::test_concurrent_reads_and_writes ... ok
test test_permissions::test_permissions_matrix ... ok
test test_postgres_storage::test_postgres_store_integration ... ok
test test_repo_lifecycle::test_invalid_repository_names ... ok
test test_repo_lifecycle::test_repository_lifecycle ... ok
test test_smart_http_git::test_git_end_to_end_smart_http ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; finished in 3.27s
```

---

## Development & Usage Instructions

### 1. Prerequisites

- Rust 1.80+ (`rustup toolchain install stable`)
- PostgreSQL 15+ (or Docker)
- Git 2.30+
- Node.js 20+

### 2. Run Database Migrations & Start Server

```bash
# Start PostgreSQL database
createdb tree_db

# Build workspace binaries
cargo build --release

# Run server with PostgreSQL
DATABASE_URL="postgres://localhost/tree_db" \
TREE_DATA_DIR="./data/git" \
cargo run -p tree-server
```

### 3. End-to-End Workflow Verification

```bash
# 1. Create repository via CLI
cargo run -p tree-cli -- create my-project --owner user

# 2. Clone repository
git clone http://localhost:8080/user/my-project.git
cd my-project

# 3. Commit and push
echo "hello" > README.md
git add README.md
git commit -m "initial commit"
git push http://user:password@localhost:8080/user/my-project.git main

# 4. Verify fresh clone
cd ..
git clone http://localhost:8080/user/my-project.git verify-project
cat verify-project/README.md
# Output: hello
```

---

## Roadmap

- [x] **Phase 0: Workspace & Repository Architecture** (Completed)
- [x] **Phase 1: Git Foundation & PostgreSQL Metadata** (Completed & Verified)
- [ ] **Phase 2: Trust & Boundary Enforcement** (Authentication, authorization enforcement at Git transport layer, repository isolation, request limits, failure recovery)
- [ ] **Phase 3: Extended Transports & Observability** (SSH daemon via russh, Prometheus metrics, audit logging)
- [ ] *Deliberately Postponed*: Pull requests, issue trackers, CI/CD runners (preserving focus on core repository engine).

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
