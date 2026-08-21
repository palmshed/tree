use crate::auth::extract_authenticated_user;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Json, Response};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tracing::{error, info, warn};
use tree_core::error::TreeError;
use tree_core::models::{
    CreateRepositoryRequest, CreateUserRequest, Role, SetPermissionRequest,
};
use tree_core::permissions::{Action, PermissionEngine};
use tree_git::{GitEngine, GitInspector};

#[derive(Debug, Deserialize)]
pub struct ListReposQuery {
    pub owner: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CommitListQuery {
    #[serde(rename = "ref")]
    pub revision: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub struct TreeQuery {
    #[serde(rename = "ref")]
    pub revision: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct BlobQuery {
    #[serde(rename = "ref")]
    pub revision: Option<String>,
    pub path: String,
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
}

pub async fn health_handler() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        version: "0.1.0",
    })
}

pub async fn create_user_handler(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Response {
    match state.store.create_user(payload).await {
        Ok(user) => (StatusCode::CREATED, Json(user)).into_response(),
        Err(TreeError::UserAlreadyExists { username }) => (
            StatusCode::CONFLICT,
            format!("User '{}' already exists", username),
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn get_user_handler(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> Response {
    match state.store.get_user_by_username(&username).await {
        Ok(Some(user)) => Json(user).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "User not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn list_repositories_handler(
    State(state): State<AppState>,
    Query(query): Query<ListReposQuery>,
) -> Response {
    let result = if let Some(owner) = query.owner {
        state.store.list_repos_by_owner(&owner).await
    } else {
        state.store.list_all_repos().await
    };

    match result {
        Ok(repos) => Json(repos).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Error: {}", e)).into_response(),
    }
}

pub async fn create_repository_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<CreateRepositoryRequest>,
) -> Response {
    let auth_user = extract_authenticated_user(&headers, &state.store).await;
    let owner_name = payload
        .owner
        .clone()
        .or_else(|| auth_user.as_ref().map(|u| u.username.clone()))
        .unwrap_or_else(|| state.config.default_owner.clone());

    // Validate names
    if let Err(e) = GitEngine::sanitize_name(&owner_name) {
        return (StatusCode::BAD_REQUEST, format!("Invalid owner: {}", e)).into_response();
    }
    if let Err(e) = GitEngine::sanitize_name(&payload.name) {
        return (StatusCode::BAD_REQUEST, format!("Invalid repository name: {}", e)).into_response();
    }

    let default_branch = payload
        .default_branch
        .clone()
        .unwrap_or_else(|| "main".to_string());

    // Compute disk path
    let disk_path = match GitEngine::compute_disk_path(&state.config.data_dir, &owner_name, &payload.name) {
        Ok(p) => p,
        Err(e) => return (StatusCode::BAD_REQUEST, format!("Path error: {}", e)).into_response(),
    };

    let disk_path_str = disk_path.to_string_lossy().to_string();

    // 1. Create DB metadata
    let repo = match state
        .store
        .create_repo(&owner_name, payload.clone(), &disk_path_str)
        .await
    {
        Ok(r) => r,
        Err(TreeError::RepositoryAlreadyExists { owner, name }) => {
            return (
                StatusCode::CONFLICT,
                format!("Repository '{}/{}' already exists", owner, name),
            )
                .into_response();
        }
        Err(e) => {
            error!("Failed to create repo metadata: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage error: {}", e),
            )
                .into_response();
        }
    };

    // 2. Initialize bare Git repository on disk
    if let Err(e) = GitEngine::init_bare_repository(
        &state.config.data_dir,
        &owner_name,
        &payload.name,
        &default_branch,
    )
    .await
    {
        error!("Failed to initialize bare git repository on disk: {}", e);
        // Rollback DB record
        let _ = state.store.delete_repo(&owner_name, &payload.name).await;
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Git init error: {}", e),
        )
            .into_response();
    }

    info!("Successfully created repository '{}/{}'", owner_name, repo.name);
    (StatusCode::CREATED, Json(repo)).into_response()
}

pub async fn get_repository_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    match GitEngine::get_repository_summary(&disk_path, &repo, &state.config.base_url).await {
        Ok(summary) => Json(summary).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Summary error: {}", e)).into_response(),
    }
}

pub async fn delete_repository_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Delete) {
        // If unauthenticated or forbidden
        if user.is_none() {
            // For simple CLI testing if no user is configured, let's allow if user matches owner_name
            // Or return forbidden
            return (StatusCode::UNAUTHORIZED, "Authentication required to delete repository").into_response();
        }
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);

    // Delete DB record
    if let Err(e) = state.store.delete_repo(&owner, clean_name).await {
        return (StatusCode::INTERNAL_SERVER_ERROR, format!("Delete error: {}", e)).into_response();
    }

    // Delete disk storage
    if let Err(e) = GitEngine::delete_repository(&disk_path).await {
        warn!("Failed to delete repository from disk {:?}: {}", disk_path, e);
    }

    (StatusCode::OK, Json(serde_json::json!({ "deleted": true }))).into_response()
}

pub async fn list_branches_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    match GitInspector::list_branches(&disk_path, &repo.default_branch).await {
        Ok(branches) => Json(branches).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response(),
    }
}

pub async fn list_tags_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    match GitInspector::list_tags(&disk_path).await {
        Ok(tags) => Json(tags).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response(),
    }
}

pub async fn list_commits_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    Query(query): Query<CommitListQuery>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    let rev = query.revision.as_deref().unwrap_or(&repo.default_branch);
    let limit = query.limit.unwrap_or(50);
    let offset = query.offset.unwrap_or(0);

    match GitInspector::list_commits(&disk_path, Some(rev), limit, offset).await {
        Ok(commits) => Json(commits).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response(),
    }
}

pub async fn get_tree_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    Query(query): Query<TreeQuery>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    let rev = query.revision.as_deref().unwrap_or(&repo.default_branch);

    match GitInspector::get_tree(&disk_path, Some(rev), query.path.as_deref()).await {
        Ok(entries) => Json(entries).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response(),
    }
}

pub async fn get_blob_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    Query(query): Query<BlobQuery>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    let rev = query.revision.as_deref().unwrap_or(&repo.default_branch);

    match GitInspector::get_blob(&disk_path, Some(rev), &query.path).await {
        Ok(Some(blob)) => Json(blob).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, "File not found").into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response(),
    }
}

pub async fn set_permission_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
    Json(payload): Json<SetPermissionRequest>,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let auth_user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = auth_user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    // Only Admin or Owner can set permissions
    if let Err(e) = PermissionEngine::check_permission(&repo, auth_user.as_ref(), member.as_ref(), Action::Admin) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let target_user = match state.store.get_user_by_username(&payload.username).await {
        Ok(Some(u)) => u,
        Ok(None) => return (StatusCode::NOT_FOUND, format!("User '{}' not found", payload.username)).into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let role = match payload.permission.to_lowercase().as_str() {
        "read" => Role::Read,
        "write" => Role::Write,
        "admin" => Role::Admin,
        "owner" => Role::Owner,
        other => return (StatusCode::BAD_REQUEST, format!("Invalid role '{}'", other)).into_response(),
    };

    match state.store.add_or_update_member(repo.id, target_user.id, role).await {
        Ok(m) => (StatusCode::OK, Json(m)).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update permissions: {}", e)).into_response(),
    }
}

pub async fn list_permissions_handler(
    State(state): State<AppState>,
    Path((owner, name)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    let clean_name = name.trim_end_matches(".git");
    let repo = match state.store.get_repo(&owner, clean_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    match state.store.list_members(repo.id).await {
        Ok(members) => Json(members).into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list members: {}", e)).into_response(),
    }
}
