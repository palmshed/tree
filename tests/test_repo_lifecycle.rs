use reqwest::Client;
use serde_json::json;
use tree_core::models::{Repository, RepositorySummary};
use tree_integration_tests::TestServer;

#[tokio::test]
async fn test_repository_lifecycle() {
    let server = TestServer::start().await;
    let client = Client::new();

    // 1. Create Repository
    let create_req = json!({
        "owner": "alice",
        "name": "project-one",
        "description": "Lifecycle test repository",
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
    let repo: Repository = resp.json().await.unwrap();
    assert_eq!(repo.owner_name, "alice");
    assert_eq!(repo.name, "project-one");
    assert_eq!(repo.default_branch, "main");
    assert!(!repo.is_private);

    // 2. Reject duplicate creation
    let dup_resp = client
        .post(format!("{}/repositories", server.base_url))
        .json(&create_req)
        .send()
        .await
        .unwrap();
    assert_eq!(dup_resp.status(), reqwest::StatusCode::CONFLICT);

    // 3. Get repository metadata & summary
    let get_resp = client
        .get(format!("{}/repositories/alice/project-one", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(get_resp.status(), reqwest::StatusCode::OK);
    let summary: RepositorySummary = get_resp.json().await.unwrap();
    assert_eq!(summary.repository.name, "project-one");
    assert!(summary.is_empty);
    assert_eq!(summary.branches_count, 0);

    // 4. List repositories
    let list_resp = client
        .get(format!("{}/repositories?owner=alice", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(list_resp.status(), reqwest::StatusCode::OK);
    let list: Vec<Repository> = list_resp.json().await.unwrap();
    assert_eq!(list.len(), 1);

    // 5. Delete repository
    let del_resp = client
        .delete(format!("{}/repositories/alice/project-one", server.base_url))
        .basic_auth("alice", Some("password"))
        .send()
        .await
        .unwrap();
    assert_eq!(del_resp.status(), reqwest::StatusCode::OK);

    // 6. Verify repository no longer exists
    let verify_resp = client
        .get(format!("{}/repositories/alice/project-one", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(verify_resp.status(), reqwest::StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn test_invalid_repository_names() {
    let server = TestServer::start().await;
    let client = Client::new();

    let invalid_names = vec![
        "../traversal",
        "foo/bar",
        ".hidden",
        "-dashstart",
        "has spaces",
        "invalid@char",
    ];

    for name in invalid_names {
        let resp = client
            .post(format!("{}/repositories", server.base_url))
            .json(&json!({
                "owner": "alice",
                "name": name,
            }))
            .send()
            .await
            .unwrap();

        assert_eq!(
            resp.status(),
            reqwest::StatusCode::BAD_REQUEST,
            "Expected bad request for invalid name: {}",
            name
        );
    }
}
