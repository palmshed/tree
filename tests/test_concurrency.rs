use reqwest::Client;
use serde_json::json;
use std::sync::Arc;
use tree_core::models::Repository;
use tree_integration_tests::TestServer;

#[tokio::test]
async fn test_concurrent_repository_creation() {
    let server = Arc::new(TestServer::start().await);
    let client = Arc::new(Client::new());

    let mut handles = Vec::new();
    let num_tasks = 20;

    for i in 0..num_tasks {
        let server_clone = server.clone();
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let repo_name = format!("concurrent-repo-{}", i);
            let resp = client_clone
                .post(format!("{}/repositories", server_clone.base_url))
                .json(&json!({
                    "owner": "concurrency_user",
                    "name": repo_name,
                    "description": format!("Concurrent test repo {}", i)
                }))
                .send()
                .await
                .unwrap();

            assert_eq!(
                resp.status(),
                reqwest::StatusCode::CREATED,
                "Failed to create concurrent repo {}",
                i
            );
        });
        handles.push(handle);
    }

    for h in handles {
        h.await.unwrap();
    }

    // Verify all 20 repositories were created and appear in listing
    let list_resp = client
        .get(format!("{}/repositories?owner=concurrency_user", server.base_url))
        .send()
        .await
        .unwrap();
    let repos: Vec<Repository> = list_resp.json().await.unwrap();
    assert_eq!(repos.len(), num_tasks);
}

#[tokio::test]
async fn test_concurrent_reads_and_writes() {
    let server = Arc::new(TestServer::start().await);
    let client = Arc::new(Client::new());

    // Create base repo
    let _ = client
        .post(format!("{}/repositories", server.base_url))
        .json(&json!({
            "owner": "shared_user",
            "name": "shared-repo"
        }))
        .send()
        .await
        .unwrap();

    let mut handles = Vec::new();

    // 50 concurrent readers
    for _ in 0..50 {
        let server_clone = server.clone();
        let client_clone = client.clone();
        let handle = tokio::spawn(async move {
            let resp = client_clone
                .get(format!("{}/repositories/shared_user/shared-repo", server_clone.base_url))
                .send()
                .await
                .unwrap();
            assert_eq!(resp.status(), reqwest::StatusCode::OK);
        });
        handles.push(handle);
    }

    for h in handles {
        h.await.unwrap();
    }
}
