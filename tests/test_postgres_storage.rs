use tree_core::models::{CreateOrgRequest, CreateRepositoryRequest, CreateUserRequest, Role};
use tree_core::store::Store;
use tree_storage::PgStore;

#[tokio::test]
async fn test_postgres_store_integration() {
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://localhost/tree_db".to_string());

    let pg_store = match PgStore::new(&db_url).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Skipping postgres integration test (no PG server): {}", e);
            return;
        }
    };

    let user_name = format!("pg_user_{}", uuid::Uuid::new_v4().simple());
    let repo_name = format!("pg_repo_{}", uuid::Uuid::new_v4().simple());

    // 1. Create user
    let user = pg_store
        .create_user(CreateUserRequest {
            username: user_name.clone(),
            email: format!("{}@example.com", user_name),
            password: Some("secure_pass".into()),
            display_name: Some("PG Tester".into()),
        })
        .await
        .expect("Failed to create PG user");

    assert_eq!(user.username, user_name);

    // 2. Create organization
    let org_name = format!("pg_org_{}", uuid::Uuid::new_v4().simple());
    let org = pg_store
        .create_org(CreateOrgRequest {
            name: org_name.clone(),
            display_name: Some("PG Organization".into()),
            description: Some("Org test".into()),
        })
        .await
        .expect("Failed to create PG org");

    assert_eq!(org.name, org_name);

    // 3. Create repository
    let disk_path = format!("/tmp/tree_pg_{}", repo_name);
    let repo = pg_store
        .create_repo(
            &user_name,
            CreateRepositoryRequest {
                owner: Some(user_name.clone()),
                name: repo_name.clone(),
                description: Some("PG repo test".into()),
                is_private: Some(false),
                default_branch: Some("main".into()),
            },
            &disk_path,
        )
        .await
        .expect("Failed to create PG repo");

    assert_eq!(repo.name, repo_name);
    assert_eq!(repo.owner_name, user_name);

    // 4. Query repository
    let fetched = pg_store
        .get_repo(&user_name, &repo_name)
        .await
        .expect("Failed to query PG repo")
        .expect("Repo not found in PG");
    assert_eq!(fetched.id, repo.id);

    // 5. Add repository member
    let member = pg_store
        .add_or_update_member(repo.id, user.id, Role::Owner)
        .await
        .expect("Failed to add PG member");
    assert_eq!(member.user_id, user.id);

    // 6. Delete repository
    pg_store
        .delete_repo(&user_name, &repo_name)
        .await
        .expect("Failed to delete PG repo");
}
