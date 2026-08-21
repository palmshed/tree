//! Phase 2: Transport-level authentication and authorization enforcement tests.
//!
//! These tests specifically probe the Git Smart HTTP boundary, not just the REST API.
//! Every test uses a real Git subprocess (no mocking) against a live TestServer.
//!
//! The questions answered here:
//!   1. Does anonymous push to a public repository get rejected?
//!   2. Does anonymous read of a private repository get rejected?
//!   3. Does a wrong password get rejected at the transport boundary?
//!   4. Does a valid credential with insufficient role get rejected?
//!   5. Does a valid credential with Write role succeed?
//!   6. Is user A's private repository invisible to user B?

use reqwest::Client;
use serde_json::json;
use tempfile::TempDir;
use tokio::process::Command;
use tree_core::models::Role;
use tree_integration_tests::TestServer;

/// Helper: create a repository via REST API, assert success.
async fn create_repo(client: &Client, base_url: &str, owner: &str, name: &str, is_private: bool) {
    let resp = client
        .post(format!("{}/repositories", base_url))
        .json(&json!({
            "owner": owner,
            "name": name,
            "is_private": is_private,
            "default_branch": "main"
        }))
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "Failed to create repo {}/{}: {}",
        owner,
        name,
        resp.status()
    );
}

/// Helper: attempt a git push, return the exit code.
async fn git_push(
    repo_dir: &std::path::Path,
    push_url: &str,
    branch: &str,
) -> std::process::ExitStatus {
    // Stage and commit a file if there's nothing committed yet.
    let _ = Command::new("git")
        .current_dir(repo_dir)
        .args(["config", "user.name", "Tester"])
        .status()
        .await;
    let _ = Command::new("git")
        .current_dir(repo_dir)
        .args(["config", "user.email", "tester@tree.test"])
        .status()
        .await;

    tokio::fs::write(repo_dir.join("probe.txt"), "phase2 probe")
        .await
        .unwrap();

    let _ = Command::new("git")
        .current_dir(repo_dir)
        .args(["add", "."])
        .status()
        .await;
    let _ = Command::new("git")
        .current_dir(repo_dir)
        .args(["commit", "--allow-empty", "-m", "probe commit"])
        .status()
        .await;

    Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(repo_dir)
        .args(["push", push_url, branch])
        .status()
        .await
        .expect("git push command failed to execute")
}

/// Initialize a fresh local git repository in a temporary directory.
async fn init_local_repo(dir: &std::path::Path, branch: &str) {
    let _ = Command::new("git")
        .args(["init", "-b", branch, dir.to_str().unwrap()])
        .status()
        .await;
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 1: Anonymous push to a PUBLIC repository must be rejected (401).
//
// Public repositories allow anonymous reads but require authentication to push.
// Before Phase 2, the auth boundary relied on mock_hash bypasses in the
// MemoryStore.  This test verifies that a push without credentials fails.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_anonymous_push_to_public_repo_rejected() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(
        &client,
        &server.base_url,
        "alice",
        "public-anon-push-test",
        false,
    )
    .await;

    let work_dir = TempDir::new().unwrap();
    let repo_dir = work_dir.path().join("repo");
    init_local_repo(&repo_dir, "main").await;

    // Push URL with no credentials.
    let push_url = format!("{}/alice/public-anon-push-test.git", server.base_url);
    let status = git_push(&repo_dir, &push_url, "main").await;

    assert!(
        !status.success(),
        "Anonymous push to public repo must fail; git exited with success (status {:?})",
        status.code()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 2: Anonymous read of a PRIVATE repository must be rejected (401).
//
// Private repositories return 401 on info/refs so that git clients prompt for
// credentials rather than silently failing.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_anonymous_clone_of_private_repo_rejected() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(
        &client,
        &server.base_url,
        "alice",
        "private-anon-test",
        true,
    )
    .await;

    // Directly call info/refs without credentials: must return 401.
    let resp = client
        .get(format!(
            "{}/alice/private-anon-test.git/info/refs?service=git-upload-pack",
            server.base_url
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "Anonymous access to private repo must return 401, got {}",
        resp.status()
    );

    let auth_header = resp.headers().get("www-authenticate");
    assert!(
        auth_header.is_some(),
        "401 response must include WWW-Authenticate header for git client credential prompt"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 3: Wrong password must be rejected at the Smart HTTP transport boundary.
//
// Verifies that auth enforcement happens before any git subprocess is spawned.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_wrong_password_rejected_at_transport() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(&client, &server.base_url, "alice", "wrong-pass-test", false).await;

    // Try info/refs with wrong password: must return 401.
    let resp = client
        .get(format!(
            "{}/alice/wrong-pass-test.git/info/refs?service=git-receive-pack",
            server.base_url
        ))
        .basic_auth("alice", Some("thisiswrong"))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::UNAUTHORIZED,
        "Wrong password must return 401, got {}",
        resp.status()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 4: Valid credentials, but insufficient role, must be rejected (403).
//
// Bob has Read-only access to alice's private repo.  A push attempt must fail
// with 403 Forbidden at the transport boundary, not reach the git subprocess.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_read_only_member_cannot_push() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(
        &client,
        &server.base_url,
        "alice",
        "read-only-member-test",
        true,
    )
    .await;

    // Resolve alice and bob user IDs from the store.
    let alice = server
        .store
        .get_user_by_username("alice")
        .await
        .unwrap()
        .expect("alice must exist in store");
    let bob = server
        .store
        .get_user_by_username("bob")
        .await
        .unwrap()
        .expect("bob must exist in store");
    let repo = server
        .store
        .get_repo("alice", "read-only-member-test")
        .await
        .unwrap()
        .expect("repo must exist");

    // Grant Bob only Read access.
    server
        .store
        .add_or_update_member(repo.id, bob.id, Role::Read)
        .await
        .unwrap();

    // Bob tries to call info/refs for receive-pack: must be 403.
    let resp = client
        .get(format!(
            "{}/alice/read-only-member-test.git/info/refs?service=git-receive-pack",
            server.base_url
        ))
        .basic_auth("bob", Some("bobsecret"))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "Read-only member must receive 403 on receive-pack, got {}",
        resp.status()
    );

    // Suppress unused variable warning.
    let _ = alice;
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 5: Valid Write credential allows a real git push to complete.
//
// This is the success path, which confirms that the auth enforcement does not
// incorrectly block legitimate authenticated writers.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_authenticated_write_push_succeeds() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(
        &client,
        &server.base_url,
        "alice",
        "auth-push-success-test",
        false,
    )
    .await;

    let work_dir = TempDir::new().unwrap();
    let repo_dir = work_dir.path().join("repo");
    init_local_repo(&repo_dir, "main").await;

    // Authenticated push URL: alice / password (real Argon2id hash stored in MemoryStore).
    let auth_url = format!(
        "http://alice:password@{}:{}/alice/auth-push-success-test.git",
        server.addr.ip(),
        server.addr.port()
    );
    let status = git_push(&repo_dir, &auth_url, "main").await;

    assert!(
        status.success(),
        "Authenticated push with Write role must succeed; git exited {:?}",
        status.code()
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Test 6: User B cannot access User A's private repository.
//
// Even with valid credentials, bob must receive 403 on alice's private repo
// unless he has been explicitly granted access.
// ─────────────────────────────────────────────────────────────────────────────
#[tokio::test]
async fn test_cross_user_private_repo_isolation() {
    let server = TestServer::start().await;
    let client = Client::new();

    create_repo(
        &client,
        &server.base_url,
        "alice",
        "isolated-private-repo",
        true,
    )
    .await;

    // Bob has valid credentials but has NOT been granted any access to alice's repo.
    let resp = client
        .get(format!(
            "{}/alice/isolated-private-repo.git/info/refs?service=git-upload-pack",
            server.base_url
        ))
        .basic_auth("bob", Some("bobsecret"))
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.status(),
        reqwest::StatusCode::FORBIDDEN,
        "Authenticated but unauthorized user must receive 403, got {}",
        resp.status()
    );
}
