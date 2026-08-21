use reqwest::Client;
use serde_json::json;
use tree_integration_tests::TestServer;

#[tokio::test]
async fn test_permissions_matrix() {
    let server = TestServer::start().await;
    let client = Client::new();

    // 1. Create users alice (owner) and bob (member)
    let _ = client
        .post(format!("{}/users", server.base_url))
        .json(&json!({
            "username": "alice",
            "email": "alice@example.com",
            "password": "alicepassword"
        }))
        .send()
        .await
        .unwrap();

    let _ = client
        .post(format!("{}/users", server.base_url))
        .json(&json!({
            "username": "bob",
            "email": "bob@example.com",
            "password": "bobpassword"
        }))
        .send()
        .await
        .unwrap();

    // 2. Alice creates a private repository
    let create_resp = client
        .post(format!("{}/repositories", server.base_url))
        .basic_auth("alice", Some("alicepassword"))
        .json(&json!({
            "owner": "alice",
            "name": "secret-project",
            "is_private": true
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), reqwest::StatusCode::CREATED);

    // 3. Anonymous user tries to access private repo -> 403 Forbidden
    let anon_resp = client
        .get(format!("{}/repositories/alice/secret-project", server.base_url))
        .send()
        .await
        .unwrap();
    assert_eq!(anon_resp.status(), reqwest::StatusCode::FORBIDDEN);

    // 4. Bob tries to access private repo before permission -> 403 Forbidden
    let bob_resp = client
        .get(format!("{}/repositories/alice/secret-project", server.base_url))
        .basic_auth("bob", Some("bobpassword"))
        .send()
        .await
        .unwrap();
    assert_eq!(bob_resp.status(), reqwest::StatusCode::FORBIDDEN);

    // 5. Alice grants Bob 'read' permission
    let perm_resp = client
        .post(format!("{}/repositories/alice/secret-project/permissions", server.base_url))
        .basic_auth("alice", Some("alicepassword"))
        .json(&json!({
            "username": "bob",
            "permission": "read"
        }))
        .send()
        .await
        .unwrap();
    assert_eq!(perm_resp.status(), reqwest::StatusCode::OK);

    // 6. Now Bob can view repository metadata
    let bob_view_resp = client
        .get(format!("{}/repositories/alice/secret-project", server.base_url))
        .basic_auth("bob", Some("bobpassword"))
        .send()
        .await
        .unwrap();
    assert_eq!(bob_view_resp.status(), reqwest::StatusCode::OK);

    // 7. List permissions
    let list_perm_resp = client
        .get(format!("{}/repositories/alice/secret-project/permissions", server.base_url))
        .basic_auth("alice", Some("alicepassword"))
        .send()
        .await
        .unwrap();
    assert_eq!(list_perm_resp.status(), reqwest::StatusCode::OK);
    let members: Vec<serde_json::Value> = list_perm_resp.json().await.unwrap();
    assert_eq!(members.len(), 2); // alice (owner) and bob (read)
}
