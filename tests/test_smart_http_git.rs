use reqwest::Client;
use serde_json::json;
use tempfile::TempDir;
use tokio::process::Command;
use tree_core::models::{BranchInfo, CommitInfo, RepositorySummary, TagInfo};
use tree_integration_tests::TestServer;

#[tokio::test]
async fn test_git_end_to_end_smart_http() {
    let server = TestServer::start().await;
    let client = Client::new();

    // 1. Create repository via REST API
    let create_req = json!({
        "owner": "alice",
        "name": "my-project",
        "description": "Milestone end-to-end repository",
        "is_private": false,
        "default_branch": "main"
    });

    let resp = client
        .post(format!("{}/repositories", server.base_url))
        .json(&create_req)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), reqwest::StatusCode::CREATED);

    let clone_url = format!("{}/alice/my-project.git", server.base_url);

    // 2. Clone repository to temp client dir 1
    let client_dir1 = TempDir::new().unwrap();
    let repo_dir1 = client_dir1.path().join("my-project");

    let clone_status = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["clone", &clone_url, repo_dir1.to_str().unwrap()])
        .status()
        .await
        .expect("git clone failed to execute");
    assert!(clone_status.success(), "Initial git clone failed");

    // 3. Configure git committer and create files
    let _ = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["config", "user.name", "Alice Engineer"])
        .status()
        .await;
    let _ = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["config", "user.email", "alice@tree.local"])
        .status()
        .await;

    let _ = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["checkout", "-b", "main"])
        .status()
        .await;

    let readme_path = repo_dir1.join("README.md");
    std::fs::write(
        &readme_path,
        "# Tree Project\n\nQuiet, lightweight Git hosting.",
    )
    .unwrap();

    let add_status = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["add", "."])
        .status()
        .await
        .unwrap();
    assert!(add_status.success());

    let commit_status = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["commit", "-m", "initial commit: add README"])
        .status()
        .await
        .unwrap();
    assert!(commit_status.success());

    // Create a tag
    let tag_status = Command::new("git")
        .current_dir(&repo_dir1)
        .args(["tag", "-a", "v0.1.0", "-m", "Release v0.1.0"])
        .status()
        .await
        .unwrap();
    assert!(tag_status.success());

    // 4. Push commit and tag to Tree server
    let auth_clone_url = format!(
        "http://alice:password@{}:{}/alice/my-project.git",
        server.addr.ip(),
        server.addr.port()
    );
    let push_status = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .current_dir(&repo_dir1)
        .args(["push", &auth_clone_url, "main", "--tags"])
        .status()
        .await
        .unwrap();
    assert!(push_status.success(), "Git push failed");

    // 5. Verify repository summary, branches, tags, and commits via API
    let summary_resp = client
        .get(format!("{}/repositories/alice/my-project", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(summary_resp.status(), reqwest::StatusCode::OK);
    let summary: RepositorySummary = summary_resp.json().await.unwrap();
    assert!(!summary.is_empty);
    assert_eq!(summary.branches_count, 1);
    assert_eq!(summary.tags_count, 1);
    assert_eq!(summary.commits_count, 1);
    assert!(summary
        .readme_content
        .unwrap()
        .contains("Quiet, lightweight Git hosting"));

    // Verify branches endpoint
    let branches_resp = client
        .get(format!(
            "{}/repositories/alice/my-project/branches",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    let branches: Vec<BranchInfo> = branches_resp.json().await.unwrap();
    assert_eq!(branches.len(), 1);
    assert_eq!(branches[0].name, "main");
    assert_eq!(
        branches[0].commit_message.as_deref(),
        Some("initial commit: add README")
    );

    // Verify tags endpoint
    let tags_resp = client
        .get(format!(
            "{}/repositories/alice/my-project/tags",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    let tags: Vec<TagInfo> = tags_resp.json().await.unwrap();
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].name, "v0.1.0");

    // Verify commits endpoint
    let commits_resp = client
        .get(format!(
            "{}/repositories/alice/my-project/commits",
            server.base_url
        ))
        .send()
        .await
        .unwrap();
    let commits: Vec<CommitInfo> = commits_resp.json().await.unwrap();
    assert_eq!(commits.len(), 1);
    assert_eq!(commits[0].summary, "initial commit: add README");
    assert_eq!(commits[0].author_name, "Alice Engineer");

    // 6. Perform SECOND CLONE to independent directory
    let client_dir2 = TempDir::new().unwrap();
    let repo_dir2 = client_dir2.path().join("my-project-clone2");

    let clone2_status = Command::new("git")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["clone", &clone_url, repo_dir2.to_str().unwrap()])
        .status()
        .await
        .expect("Second git clone failed to execute");
    assert!(clone2_status.success(), "Second git clone must succeed");

    // 7. Verification: Second clone must contain the pushed README.md with exact content
    let clone2_readme = repo_dir2.join("README.md");
    assert!(
        clone2_readme.exists(),
        "Pushed README.md must exist in second clone"
    );
    let content = std::fs::read_to_string(&clone2_readme).unwrap();
    assert_eq!(content, "# Tree Project\n\nQuiet, lightweight Git hosting.");

    // Verify commit history in clone 2
    let log_output = Command::new("git")
        .current_dir(&repo_dir2)
        .args(["log", "-n", "1", "--format=%s"])
        .output()
        .await
        .unwrap();
    let log_msg = String::from_utf8_lossy(&log_output.stdout);
    assert_eq!(log_msg.trim(), "initial commit: add README");
}
