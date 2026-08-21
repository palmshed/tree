# Tree API Documentation

Tree provides a minimalist, well-structured RESTful JSON API alongside the native Git Smart HTTP transport protocol.

---

## Base URL

By default: `http://localhost:8080` (or `https://tree.local`)

---

## 1. Repositories API

### Create Repository
`POST /repositories`

#### Request Body
```json
{
  "owner": "alice",
  "name": "my-project",
  "description": "My quiet project",
  "is_private": false,
  "default_branch": "main"
}
```

#### Responses
- `201 Created`: Returns repository object.
- `400 Bad Request`: Invalid owner or repository name.
- `409 Conflict`: Repository already exists for this owner.

---

### Get Repository Summary
`GET /repositories/:owner/:name`

#### Responses
- `200 OK`: Returns repository metadata, branch count, commit count, tag count, clone URLs, and README content.
```json
{
  "repository": {
    "id": "a1b2c3d4-e5f6-4a5b-8c9d-0e1f2a3b4c5d",
    "owner_type": "user",
    "owner_id": "...",
    "owner_name": "alice",
    "name": "my-project",
    "description": "My quiet project",
    "is_private": false,
    "default_branch": "main",
    "disk_path": "/var/lib/tree/git/alice/my-project.git",
    "created_at": "2026-08-21T18:00:00Z",
    "updated_at": "2026-08-21T18:00:00Z"
  },
  "default_branch": "main",
  "branches_count": 1,
  "tags_count": 1,
  "commits_count": 5,
  "is_empty": false,
  "clone_url_http": "http://localhost:8080/alice/my-project.git",
  "clone_url_ssh": "git@tree.local:alice/my-project.git",
  "readme_content": "# My Project\n\nWelcome to Tree."
}
```
- `404 Not Found`: Repository does not exist.
- `403 Forbidden`: Unauthorized access to private repository.

---

### Delete Repository
`DELETE /repositories/:owner/:name`

#### Headers
- `Authorization: Basic <base64(user:pass)>`

#### Responses
- `200 OK`: `{"deleted": true}`
- `401 Unauthorized`: Missing credentials.
- `403 Forbidden`: User is not repository owner or admin.
- `404 Not Found`: Repository does not exist.

---

### List Branches
`GET /repositories/:owner/:name/branches`

#### Responses
- `200 OK`: Array of branch items.
```json
[
  {
    "name": "main",
    "commit_id": "9f83a21...4e8",
    "is_default": true,
    "commit_message": "initial commit",
    "commit_author": "Alice",
    "commit_date": "2026-08-21T18:00:00Z"
  }
]
```

---

### List Tags
`GET /repositories/:owner/:name/tags`

#### Responses
- `200 OK`: Array of tag items.
```json
[
  {
    "name": "v0.1.0",
    "commit_id": "9f83a21...4e8",
    "message": "Release v0.1.0",
    "tagger": "Alice",
    "date": "2026-08-21T18:00:00Z"
  }
]
```

---

### List Commits
`GET /repositories/:owner/:name/commits?ref=main&limit=50&offset=0`

#### Query Parameters
- `ref` (optional): Branch or commit SHA (default: default branch).
- `limit` (optional): Maximum commits to return (default: 50).
- `offset` (optional): Number of commits to skip.

#### Responses
- `200 OK`: Array of commit items.
```json
[
  {
    "id": "9f83a2108759...4e8",
    "short_id": "9f83a21",
    "author_name": "Alice",
    "author_email": "alice@example.com",
    "committer_name": "Alice",
    "committer_email": "alice@example.com",
    "message": "feat: initial commit",
    "summary": "feat: initial commit",
    "timestamp": "2026-08-21T18:00:00Z",
    "parents": []
  }
]
```

---

### List Directory Tree
`GET /repositories/:owner/:name/tree?ref=main&path=src`

#### Responses
- `200 OK`: Array of directory and file items.

---

### Get File Content
`GET /repositories/:owner/:name/blob?ref=main&path=README.md`

#### Responses
- `200 OK`: File metadata and content.

---

### Manage Permissions
`POST /repositories/:owner/:name/permissions`

#### Request Body
```json
{
  "username": "bob",
  "permission": "write"
}
```
Valid permissions: `"read"`, `"write"`, `"admin"`, `"owner"`.

---

## 2. Git Smart HTTP Transport

### Reference Advertisement
`GET /:owner/:name.git/info/refs?service=git-upload-pack` (clone/fetch)
`GET /:owner/:name.git/info/refs?service=git-receive-pack` (push)

### Service Execution
`POST /:owner/:name.git/git-upload-pack`
`POST /:owner/:name.git/git-receive-pack`
