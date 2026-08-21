# Tree

*A quiet, self-hosted Git hosting platform.*

> **Status Notice**: Tree is an experimental, self-hosted Git hosting platform under active development. Phase 0, Phase 1 and Phase 2 are implemented, tested and verified with a production-oriented CI foundation. Tree is not yet intended as a full replacement for existing large-scale forge platforms.

---

## Public Engineering Record

- **Living GitHub Gist**: [Tree: Living Engineering Record (`tree-engineering-gist.md`)](https://gist.github.com/bniladridas/121f0d2f10f6900faf3ffab455be757f)
- **Architecture Document**: [docs/architecture.md](docs/architecture.md)
- **API Reference**: [docs/api.md](docs/api.md)
- **Local Engineering Record**: [docs/ENGINEERING_GIST.md](docs/ENGINEERING_GIST.md)

---

## Current Status (Phase 0, Phase 1 and Phase 2)

Tree focuses on the core Git repository engine: native transport, metadata management, and unbloated interfaces.

- **Phase 0 (Repository and Structure)**: Modular Rust virtual workspace, clean domain interfaces, database migrations, and Docker configurations.
- **Phase 1 (Git Foundation and Metadata)**: Native Git Smart HTTP transport (`clone`, `fetch`, `push`), bare Git filesystem management, PostgreSQL metadata persistence, RBAC permission evaluator, paginated commit/branch/tag/tree inspection, and a developer CLI.
- **Phase 2 (Trust and Boundary plus External Infrastructure)**: Argon2id password hashing with per-password random salt, request body limit and timeout at the HTTP boundary, structured audit logging for pushes and permission denials, six transport auth boundary tests, and a minimal GitHub Actions foundation (`ci.yml`, `security.yml`, `release.yml`) with BuildKit builds to `ghcr.io`.

---

## Technology Stack

- **Core and Backend Daemon**: Rust stable (Axum, Tokio, sqlx, clap) - `rust:1-bookworm` builder, `1.98` locally
- **Metadata Storage**: PostgreSQL 16+ (relational schemas, ACID guarantees)
- **Git Engine**: Bare Git storage plus Git Smart HTTP protocol (`pkt-line`, stateless RPC subprocess streaming)
- **Web Interface**: TypeScript plus quiet minimalist HTML/CSS
- **Containerization**: Multi-stage Docker with BuildKit and Docker Compose
- **CI and Supply Chain**: GitHub Actions, `cargo audit` and CodeQL, Dependabot, GHCR with provenance and SBOM
- **Target OS**: Linux first (macOS compatible)

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
│   ├── tree-core/         # Domain models, RBAC evaluator, Store traits, errors, auth
│   ├── tree-git/          # Bare storage management, Smart HTTP framing, Git inspectors
│   └── tree-storage/      # PostgreSQL store (sqlx), migrations, test MemoryStore
├── web/                   # Minimal TypeScript web frontend
├── migrations/            # SQL schema migrations (0001_init.sql)
├── tests/                 # End-to-end integration and concurrency test suites
├── docs/                  # Architecture specifications and living engineering gist
├── docker/                # Multi-stage Dockerfile and docker-compose setup
├── .github/workflows/     # CI, security and release workflows (BuildKit to GHCR)
├── Cargo.toml             # Virtual workspace configuration
├── README.md
├── LICENSE                # Apache 2.0
└── CONTRIBUTING.md
```

---

## Test Results

**21/21 automated tests passing (100 percent pass rate)**:
`cargo test --workspace --locked` with `DATABASE_URL` pointing at PostgreSQL 16 (CI uses `postgres:16` service, local uses Homebrew `postgres@16`):

```text
running 4 tests  (tree-core)           - permission engine, RBAC
running 4 tests  (tree-git)            - sanitize_name, pkt_line, advertise_refs, init/delete
running 6 tests  (test_auth_enforcement) - anonymous push rejected, anonymous private read 401, wrong password 401, read-only member 403, authenticated write succeeds, cross-user isolation 403
running 2 tests  (test_concurrency)    - concurrent repository creation, concurrent reads and writes
running 1 test   (test_permissions)    - permissions matrix
running 1 test   (test_postgres_storage) - PostgreSQL store integration
running 2 tests  (test_repo_lifecycle) - invalid names, repository lifecycle
running 1 test   (test_smart_http_git) - Git Smart HTTP end-to-end

test result: ok. 21 passed; 0 failed; 0 ignored
```

Original 15/15 suite remains intact, six additive auth boundary tests were added in Phase 2.

---

## Development and Usage Instructions

### 1. Prerequisites

- Rust stable (`rustup toolchain install stable`, tested with `1.98`)
- PostgreSQL 16+ (or Docker `postgres:16`)
- Git 2.30+
- Node.js 20+

### 2. Run Database Migrations and Start Server

```bash
# Start PostgreSQL database (Homebrew)
brew services start postgresql@16
createdb tree_db

# Or via Docker
docker compose -f docker/docker-compose.yml up -d postgres

# Build workspace binaries
cargo build --release --locked

# Run server with PostgreSQL
DATABASE_URL="postgres://tree:treepassword@localhost:5432/tree_db" \
TREE_DATA_DIR="./data/git" \
cargo run -p tree-server

# Without DATABASE_URL, tree-server falls back to MemoryStore for local testing
cargo run -p tree-server
```

### 3. Verify Exactly as CI Does

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL="postgres://tree:treepassword@localhost:5432/tree_db" cargo test --workspace --locked
cargo build --release --locked
```

### 4. End-to-End Workflow Verification

```bash
# 1. Create repository via CLI
cargo run -p tree -- create my-project --owner user

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

### 5. Container Build (BuildKit)

```bash
docker buildx build -f docker/Dockerfile.server -t ghcr.io/palmshed/tree:local --load .
docker run -d -p 8080:8080 --name tree ghcr.io/palmshed/tree:local
curl -fsS http://localhost:8080/health
docker rm -f tree
```

---

## CI and Release

- **CI** (`.github/workflows/ci.yml`): on PR and push to `main`, `cargo fmt`, `clippy -D warnings`, real `postgres:16` service, `cargo test --workspace`, `cargo build --release`.
- **Security** (`.github/workflows/security.yml`): on PR and weekly schedule, `cargo audit`, CodeQL `rust` (`security-and-quality`), Dependabot for `cargo` and `github-actions`. Secret scanning and push protection are repository settings.
- **Release** (`.github/workflows/release.yml`): on `push: tags v*.*.*` or published Release. Builds from the tagged commit, runs tests, records `GITHUB_SHA`, produces `SHA256SUMS.txt`, builds with BuildKit via `buildx`, starts the container and curls `/health` before any push, then pushes to `ghcr.io/palmshed/tree` (`:x.y.z`, `:x.y`, `:x`, `:stable`, `:latest` only for stable semver) and attaches `tree-server`, `tree`, checksums and commit SHA to the GitHub Release.

Dependabot is configured in `.github/dependabot.yml` (weekly `cargo` and `github-actions`).

---

## Roadmap

- [x] **Phase 0: Workspace and Repository Architecture** (Completed)
- [x] **Phase 1: Git Foundation and PostgreSQL Metadata** (Completed and Verified)
- [x] **Phase 2: Trust and Boundary plus External Infrastructure** (Completed, 21/21 locally, BuildKit pipeline to GHCR, awaiting clean-runner verification)
- [ ] **Phase 3: Extended Transports and Observability** (SSH daemon via `russh`, Prometheus metrics, audit log retention, webhook dispatcher)
- [ ] *Deliberately Postponed*: Pull requests, issue trackers, CI/CD runners (preserving focus on core repository engine).

---

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE) for details.
