# Tree: Architecture Specification

## 1. System Overview

> **Public Engineering Record**: [Tree: Living Engineering Record (GitHub Gist)](https://gist.github.com/bniladridas/121f0d2f10f6900faf3ffab455be757f)

Tree is a lightweight, high-reliability self-hosted Git hosting server built in Rust. It serves as an unbloated alternative to monolithic software forge platforms, focusing specifically on Git repository hosting, transport, metadata management, and minimal inspection interfaces.

```
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

## 2. Component Isolation & Responsibilities

The codebase is organized into modular crates and applications:

### `crates/tree-core`
- **Domain Models**: Strongly typed definitions for `User`, `Organization`, `Repository`, `RepositoryMember`, `Role`, `PermissionType`, `CommitInfo`, `BranchInfo`, `TagInfo`, `FileEntry`.
- **Permission Engine**: Pure deterministic access control evaluator (`PermissionEngine::check_permission`) enforcing Read, Write, Admin, and Owner access across public and private repositories.
- **Store Trait**: Async storage abstraction enabling zero-dependency in-memory mocks for testing alongside PostgreSQL in production.
- **Error Model**: Structured domain errors mapped to appropriate HTTP status codes and Git pack error streams.

### `crates/tree-git`
- **Bare Storage Manager**: Responsible for on-disk creation (`git init --bare`), deletion, path isolation, and directory traversal defense.
- **Smart HTTP Protocol Transport**: Implements Git Smart HTTP protocol version 1 & 2 framing:
  - Packet-line (`pkt-line`) encoder and flush delimiters (`0000`).
  - Reference advertisement (`info/refs?service=git-upload-pack` and `git-receive-pack`).
  - Stateless RPC streaming subprocesses (`git upload-pack --stateless-rpc` and `git receive-pack --stateless-rpc`) with non-blocking asynchronous standard I/O streaming.
- **Git Inspector**: Querying commit history, branches, tags, trees, blobs, and README files directly from bare repository object databases without working tree checkout.

### `crates/tree-storage`
- **PostgreSQL Store (`PgStore`)**: High-performance PostgreSQL store powered by `sqlx`, connection pooling, transactions, and migration runner (`0001_init.sql`).
- **Memory Store (`MemoryStore`)**: Lock-protected in-memory store for instantaneous test execution and environment independence.

### `apps/tree-server`
- **Axum Web Server**: Exposes REST endpoints, Git Smart HTTP endpoints, HTTP Basic Authentication middleware, and embeds the lightweight Web UI.

### `apps/tree-cli`
- **Developer CLI (`tree`)**: Fast CLI for repository lifecycle operations (`tree create`, `tree delete`, `tree list`, `tree user create`).

---

## 3. Git Storage Model: Keeping Git Objects Out of the Database

A fundamental design decision in Tree is keeping Git objects strictly on the filesystem rather than inside PostgreSQL:

1. **Native Git Performance**: Git's packfile storage, delta compression, and loose object format are heavily optimized for POSIX filesystem operations, mmap, and filesystem caching.
2. **Stateless RPC Compatibility**: Standard Git transport tools (`git-upload-pack`, `git-receive-pack`) operate natively on bare filesystem repositories. Storing Git objects in PostgreSQL BLOBs/BYTEA would require custom reimplementation of pack negotiation, delta calculation, and object indexing, adding vast complexity and severe latency.
3. **Database Footprint**: Metadata stays small, fast, and cacheable in RAM, while repository blobs stream with zero database locking.

---

## 4. Git Smart HTTP Transport Mechanics

Tree implements the official Git Smart HTTP protocol specification:

### 1. Reference Discovery (`GET /:owner/:name.git/info/refs?service=git-upload-pack`)
- Client sends an HTTP GET request with the requested service.
- Server validates repository existence and read permission.
- Server sends:
  - Header: `Content-Type: application/x-git-upload-pack-advertisement`
  - Body: `001e# service=git-upload-pack\n0000` followed by the stdout of `git upload-pack --stateless-rpc --advertise-refs <repo_path>`.

### 2. Packfile Negotiation & Transfer (`POST /:owner/:name.git/git-upload-pack` or `git-receive-pack`)
- Client sends packfile negotiation stream.
- Server verifies write permissions (for `receive-pack`) or read permissions (for `upload-pack`).
- Server spawns asynchronous `git <service> --stateless-rpc <repo_path>`, pipes HTTP request body directly into process standard input, and streams standard output back with `Content-Type: application/x-git-<service>-result`.

---

## 5. Security & Isolation Model

- **Filesystem Isolation**: Owner and repository names are sanitized (`GitEngine::sanitize_name`). Names cannot contain `..`, `/`, `\`, null bytes, or illegal shell characters, preventing path traversal outside the repository base directory.
- **Authentication**: HTTP Basic Authentication verifies password hashes using SHA-256 + salt (or argon2).
- **Authorization**: Granular RBAC permissions with four roles (`Read`, `Write`, `Admin`, `Owner`). Anonymous read access is allowed only for repositories explicitly flagged `is_private = false`.
