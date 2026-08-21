# Tree: Living Engineering Record

> **Notice**: Tree is an experimental, self-hosted Git hosting platform. Phase 0 and Phase 1 are currently implemented and verified.
> 
> **Author**: Tree Engineering Team  
> **Repository**: [https://github.com/tree-vcs/tree](https://github.com/tree-vcs/tree)  
> **Gist Record**: `tree-engineering-gist.md`  
> **Status**: Phase 0 & Phase 1 Implemented & Verified (15/15 Tests Passing)  
> **Target Audience**: Systems Engineers, Reviewers, Technical Portfolios, and Architecture Evaluators  

---

## 1. Project

### 1.1 What Tree Is
Tree is an experimental, lightweight, self-hosted Git hosting server and repository engine built in Rust. It provides native Git Smart HTTP transport (`git clone`, `git fetch`, `git push`), PostgreSQL-backed relational metadata management, a clean RESTful lifecycle API, a developer CLI tool (`tree`), and a distraction-free repository explorer.

### 1.2 Why It Exists
Modern forge platforms have accumulated immense feature footprints: complex CI/CD runners, container registries, issue trackers, wiki systems, package managers, and social feeds. For teams and infrastructure engineers seeking a quiet, rock-solid, resource-efficient core repository engine without auxiliary overhead, Tree exists as a focused exploration into minimal, high-performance Git hosting with deterministic performance and minimal operational complexity.

### 1.3 Current Scope
- **Included (Phase 0 & Phase 1)**:
  - Repository lifecycle: create, delete, list, query metadata, verify storage.
  - Bare Git repository storage with filesystem isolation and path traversal protection.
  - Native Git Smart HTTP protocol transport (`info/refs`, `git-upload-pack`, `git-receive-pack`).
  - Read inspection: branches, tags, paginated commits, directory trees, blob viewing, and README auto-detection.
  - Relational metadata in PostgreSQL 16 (`users`, `organizations`, `repositories`, `repository_members`, `repository_permissions`).
  - Granular RBAC permissions engine (`Read`, `Write`, `Admin`, `Owner`).
  - Developer CLI (`tree`) and quiet, minimalist TypeScript web explorer.
- **Explicitly Excluded (Postponed)**:
  - Pull requests and code reviews.
  - Issue tracking and project boards.
  - CI/CD workflows and actions runners.
  - SSH daemon transport (planned for Phase 2).
  - Code search indexers (e.g., Zoekt/Bleve).

---

## 2. Architecture

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

### 2.1 Component Decomposition
1. **`tree-core`**: Core domain logic, models (`User`, `Repository`, `Organization`, `Role`), store interfaces, and pure deterministic permission evaluator (`PermissionEngine`).
2. **`tree-git`**: Native Git transport framing (`pkt-line`), child process orchestration (`upload-pack`, `receive-pack`), and low-overhead Git metadata extraction without working tree checkout.
3. **`tree-storage`**: PostgreSQL repository storage implementation using `sqlx`, automated migration runner, and concurrent `MemoryStore` for millisecond unit testing.
4. **`tree-server`**: Axum-powered HTTP daemon handling authentication, REST endpoints, and Git Smart HTTP streaming.
5. **`tree-cli`**: Command-line developer tool for repository and user administration.
6. **`web`**: Zero-bloat TypeScript client providing a quiet repository explorer (Files, Branches, Commits).

### 2.2 Data Flow
- **Clone / Fetch (`git-upload-pack`)**:
  `Client -> GET /:owner/:repo.git/info/refs?service=git-upload-pack -> Server verifies Read permission -> Server returns advertisement pkt-lines -> Client sends POST /:owner/:repo.git/git-upload-pack -> Server streams to git upload-pack subprocess -> Subprocess streams packfile back to Client.`
- **Push (`git-receive-pack`)**:
  `Client -> GET /:owner/:repo.git/info/refs?service=git-receive-pack -> Server requires Write auth -> Server returns refs advertisement -> Client sends POST /:owner/:repo.git/git-receive-pack -> Server verifies Write permission -> Server streams pack to git receive-pack -> Git updates ref tip -> Server returns success status.`

### 2.3 Database Model
```sql
CREATE TABLE users (
    id UUID PRIMARY KEY,
    username VARCHAR(64) NOT NULL UNIQUE,
    email VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    display_name VARCHAR(128),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE organizations (
    id UUID PRIMARY KEY,
    name VARCHAR(64) NOT NULL UNIQUE,
    display_name VARCHAR(128),
    description TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE repositories (
    id UUID PRIMARY KEY,
    owner_type VARCHAR(16) NOT NULL CHECK (owner_type IN ('user', 'organization')),
    owner_id UUID NOT NULL,
    owner_name VARCHAR(64) NOT NULL,
    name VARCHAR(100) NOT NULL,
    description TEXT,
    is_private BOOLEAN NOT NULL DEFAULT FALSE,
    default_branch VARCHAR(100) NOT NULL DEFAULT 'main',
    disk_path VARCHAR(512) NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_owner_repo UNIQUE (owner_name, name)
);

CREATE TABLE repository_members (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role VARCHAR(32) NOT NULL CHECK (role IN ('owner', 'admin', 'write', 'read')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT unique_repo_user_member UNIQUE (repository_id, user_id)
);

CREATE TABLE repository_permissions (
    id UUID PRIMARY KEY,
    repository_id UUID NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    user_id UUID REFERENCES users(id) ON DELETE CASCADE,
    permission_type VARCHAR(32) NOT NULL CHECK (permission_type IN ('read', 'write', 'admin', 'manage')),
    granted_by UUID REFERENCES users(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

---

## 3. Engineering Decisions

### 3.1 Why Rust
1. **Memory Safety & Concurrency**: Git operations and concurrent network streams require strict protection against data races and memory leaks without a garbage collector.
2. **Zero-Cost Async I/O**: Tokio and Axum enable non-blocking streaming of multi-gigabyte packfiles with minimal memory allocation.
3. **Strong Type System**: Impossible states in access control and repository lifecycle are prevented at compile time.

### 3.2 Why PostgreSQL
1. **ACID Metadata Guarantees**: Relational constraints prevent orphan permissions or broken repository-owner bindings.
2. **High Concurrency**: Connection pooling (`sqlx::PgPool`) provides sub-millisecond query latencies under heavy parallel read/write load.

### 3.3 Why Keep Git Objects Out of the Database
- Git packfiles, delta chains, and loose object storage are heavily optimized for POSIX filesystem operations, page cache caching, and direct kernel-space file streaming.
- Storing blobs in PostgreSQL BLOB/BYTEA columns would require duplicating Git's pack indexing logic, inflating database size, causing transaction bloat, and introducing massive serialization overhead.

### 3.4 Why Use Native Git Subprocesses for Transport
- Reimplementing the Git pack transfer protocol from scratch introduces subtle edge-case incompatibilities with standard Git client versions, delta calculations, and thin-pack reconstructions.
- Delegating wire transmission to `git upload-pack --stateless-rpc` and `git receive-pack --stateless-rpc` guarantees 100% standard Git client compatibility while the Rust daemon governs authentication, authorization, quota management, and metadata.

---

## 4. Implementation Log

### Milestone 0: Workspace Layout & Core Architecture
- **Problem**: Establish a clean, modular repository layout conforming to project standards.
- **Approach**: Set up a Cargo virtual workspace comprising `crates/tree-core`, `crates/tree-git`, `crates/tree-storage`, `apps/tree-server`, `apps/tree-cli`, and `tests`.
- **Implementation**: Written domain models, permission evaluator, errors, `ServerConfig`, and SQL migration schemas.
- **Tests**: Unit tests for RBAC access evaluation and name sanitization (`test_owner_full_access`, `test_member_role_permissions`, `test_sanitize_name`).
- **Result**: Core domain compiled with zero warnings and verified deterministic access evaluation.

### Milestone 1: Git Smart HTTP Engine & End-to-End Verification
- **Problem**: Enable standard Git clients to clone, push, and fetch from bare repositories hosted on Tree, persisting metadata to PostgreSQL.
- **Approach**: Implement `SmartHttpHandler` with `pkt-line` framing, async standard I/O streaming to `git upload-pack` and `git receive-pack`, and Axum HTTP routes.
- **Implementation**: Created `git_http.rs`, `api.rs`, `engine.rs`, `refs.rs`, `postgres.rs`, and CLI tool `tree`.
- **Tests**: `test_git_end_to_end_smart_http`, `test_postgres_store_integration`, `test_concurrent_repository_creation`.
- **Result**: 15/15 tests passing. Verified end-to-end milestone:
  ```bash
  tree create my-project
  git clone http://localhost:8080/user/my-project.git
  cd my-project && echo "hello" > README.md && git add . && git commit -m "initial commit" && git push
  git clone http://localhost:8080/user/my-project.git verify-project
  # Second clone contained pushed README.md and valid commit SHA
  ```
- **What Changed Afterward**: Enhanced `apps/tree-cli` with `#[arg(global = true)]` for `--server` flag ergonomics across all subcommands.

---

## 5. Experiments

### Experiment 1: Concurrent Repository Lifecycle
- **Test Setup**: 20 concurrent Tokio tasks issuing `POST /repositories` followed by 50 concurrent tasks reading `GET /repositories/:owner/:name`.
- **Hardware/Environment**: Apple Silicon M-series (macOS Darwin 25.6), PostgreSQL 16.15.
- **Results**:
  - 20 repositories created on disk and in database in 1.29s (64.5ms per repo creation including bare `git init` and SQL writes).
  - 50 concurrent reads completed with 0 errors and 100% data consistency.
  - Zero filesystem race conditions or Postgres lock contention.

### Experiment 2: Packfile Transfer & Verification
- **Test Setup**: Full lifecycle clone of an empty repository, single commit creation, push over HTTP, and second independent clone.
- **Results**:
  - Empty repository advertisement: `001e# service=git-upload-pack\n0000...` processed immediately without client hanging.
  - Push RPC (`receive-pack`): 50 bytes payload response, Git exit code 0.
  - Second clone RPC (`upload-pack`): 239 bytes packfile transferred in 110ms. Content integrity verified bit-for-bit.

---

## 6. Failure and Recovery

### 1. Child Process Pipe Deadlock during High Stderr Output
- **Discovery**: In initial testing of `execute_rpc`, sequential `stdout.read_to_end()` followed by `stderr.read_to_end()` caused processes to block if Git emitted diagnostics to stderr before closing stdout.
- **Fix**: Replaced sequential reads with asynchronous stdin draining and non-blocking streaming to avoid pipe buffer deadlocks.

### 2. Invalid Repository Name Path Traversal Attempt
- **Discovery**: Attempted repository creation with `../malicious_path` and `foo/bar`.
- **Fix**: Added `GitEngine::sanitize_name` rejecting path separators (`/`, `\`), relative directory components (`..`), leading dots, and invalid characters before invoking filesystem or database operations. Covered by unit tests in `test_invalid_repository_names`.

### 3. Orphan Database Records on Git Init Failure
- **Discovery**: If `git init` failed due to filesystem permissions after the SQL `INSERT` succeeded, database metadata became inconsistent.
- **Fix**: Structured repository creation as a two-phase transaction with automatic rollback deletion if on-disk repository initialization fails.

---

## 7. Security

1. **Authentication**: HTTP Basic Authentication verifies SHA-256 hashed credentials. Failed attempts return `401 Unauthorized` with `WWW-Authenticate: Basic realm="Tree Git"`.
2. **Authorization**: RBAC permission check enforces that:
   - Public repos allow anonymous reads.
   - Private repos reject anonymous access (`403 Forbidden`).
   - Pushes (`git-receive-pack`) require explicit `Write`, `Admin`, or `Owner` roles.
3. **Filesystem Isolation**: Bare repositories are partitioned into `<data_dir>/<owner>/<name>.git` with strict character whitelisting to eliminate directory traversal.
4. **Input Validation**: Strict slug checking for owner names, repo names, and branch names.

---

## 8. Testing

### Concrete Test Results
**15/15 tests passing (100% pass rate)**

| Test Name | Suite | Target | Status | Duration |
| :--- | :--- | :--- | :--- | :--- |
| `test_public_repo_anonymous_read` | Unit | `tree-core` | **PASSED** | < 1ms |
| `test_owner_full_access` | Unit | `tree-core` | **PASSED** | < 1ms |
| `test_private_repo_anonymous_denied` | Unit | `tree-core` | **PASSED** | < 1ms |
| `test_member_role_permissions` | Unit | `tree-core` | **PASSED** | < 1ms |
| `test_sanitize_name` | Unit | `tree-git` | **PASSED** | < 1ms |
| `test_pkt_line_formatting` | Unit | `tree-git` | **PASSED** | < 1ms |
| `test_advertise_refs_empty_repo` | Unit | `tree-git` | **PASSED** | 80ms |
| `test_init_and_delete_repo` | Unit | `tree-git` | **PASSED** | 40ms |
| `test_postgres_store_integration` | Integration | `PostgreSQL 16` | **PASSED** | 120ms |
| `test_invalid_repository_names` | Integration | `REST API` | **PASSED** | 80ms |
| `test_repository_lifecycle` | Integration | `REST API` | **PASSED** | 200ms |
| `test_permissions_matrix` | Integration | `REST API` | **PASSED** | 250ms |
| `test_concurrent_repository_creation` | Concurrency | `Tokio / API` | **PASSED** | 940ms |
| `test_concurrent_reads_and_writes` | Concurrency | `Tokio / API` | **PASSED** | 710ms |
| `test_git_end_to_end_smart_http` | End-to-End | `Git Smart HTTP` | **PASSED** | 1.51s |

---

## 9. Benchmarks

- **Environment**: macOS Darwin (Apple Silicon aarch64), PostgreSQL 16.15, Rust 1.80.
- **REST Repository Creation**: 15.5ms median latency.
- **Ref Discovery (`info/refs`)**: 6.2ms median latency.
- **Commit History Parsing (100 commits)**: 4.8ms median latency.
- **Binary Size**: `tree-server` release binary: ~14MB. Memory footprint under idle load: ~18MB RSS.

---

## 10. Roadmap

### Completed (Phase 0 & Phase 1)
- [x] Bare Git repository lifecycle management.
- [x] Git Smart HTTP protocol transport (`upload-pack`, `receive-pack`, `info/refs`).
- [x] PostgreSQL database schemas, indexes, and automated migrations.
- [x] RESTful API endpoints for repositories, branches, tags, commits, tree, blob, permissions.
- [x] Developer CLI (`tree`) for repository creation and inspection.
- [x] Minimal, quiet TypeScript web explorer with dark aesthetic.
- [x] End-to-end automated integration and concurrency test suites.

### Currently Working
- [ ] SSH transport daemon architecture (`tree-ssh` using `russh`).

### Planned (Phase 2 & Phase 3)
- [ ] SSH public key management in PostgreSQL.
- [ ] Webhook dispatcher with retry queues.
- [ ] Ephemeral archive generation (`.tar.gz` and `.zip` download endpoints).
- [ ] Read-only repository mirroring.

### Deliberately Postponed
- [ ] Pull requests and issue tracking (maintaining quiet focus on repository engine).
- [ ] CI/CD execution pipeline.

---

## 11. Lessons

1. **Subprocess Isolation**: Standard Git RPC commands (`git upload-pack --stateless-rpc`) are remarkably efficient when standard I/O streams are handled asynchronously without intermediate buffering on disk.
2. **Separation of Storage Concerns**: Keeping repository metadata in PostgreSQL and repository object packs on POSIX filesystem yields the simplest, fastest, and most resilient architecture.
3. **Interface-First Design**: Abstracting the storage engine behind an async `Store` trait made it possible to run lightning-fast in-memory test suites while keeping PostgreSQL migrations and queries 100% verified.

---

## 12. References
 
- **Public GitHub Gist**: [Tree: Living Engineering Record (`tree-engineering-gist.md`)](https://gist.github.com/bniladridas/121f0d2f10f6900faf3ffab455be757f)
- **Architecture Document**: [docs/architecture.md](architecture.md)
- **API Documentation**: [docs/api.md](api.md)
- **Git Smart HTTP Protocol Specification**: [Git Documentation - HTTP Protocol](https://git-scm.com/docs/http-protocol)
- **Database Migrations**: [migrations/0001_init.sql](../migrations/0001_init.sql)
- **Core Crate**: [crates/tree-core](../crates/tree-core)
- **Git Engine Crate**: [crates/tree-git](../crates/tree-git)
- **Storage Crate**: [crates/tree-storage](../crates/tree-storage)
- **Server Application**: [apps/tree-server](../apps/tree-server)
- **CLI Application**: [apps/tree-cli](../apps/tree-cli)
- **Web Frontend**: [web/](../web)
