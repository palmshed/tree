use crate::auth::extract_authenticated_user;
use crate::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, EXPIRES, PRAGMA, WWW_AUTHENTICATE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{error, warn};
use tree_core::permissions::{Action, PermissionEngine};
use tree_git::{GitEngine, GitService, SmartHttpHandler};

#[derive(Debug, Deserialize)]
pub struct InfoRefsQuery {
    pub service: Option<String>,
}

fn normalize_repo_name(name_raw: &str) -> String {
    name_raw.trim_end_matches(".git").to_string()
}

fn auth_challenge() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        [
            (WWW_AUTHENTICATE, "Basic realm=\"Tree Git\""),
            (CONTENT_TYPE, "text/plain"),
        ],
        "Authentication required\n",
    )
        .into_response()
}

pub async fn info_refs_handler(
    State(state): State<AppState>,
    Path((owner, name_raw)): Path<(String, String)>,
    Query(query): Query<InfoRefsQuery>,
    headers: HeaderMap,
) -> Response {
    let repo_name = normalize_repo_name(&name_raw);
    let service_name = match query.service {
        Some(s) => s,
        None => {
            // Dumb HTTP is disabled
            return (
                StatusCode::FORBIDDEN,
                "Tree only supports Git Smart HTTP protocol",
            )
                .into_response();
        }
    };

    let service = match GitService::from_service_name(&service_name) {
        Some(s) => s,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                format!("Unsupported git service: {}", service_name),
            )
                .into_response();
        }
    };

    // Find repository in store
    let repo = match state.store.get_repo(&owner, &repo_name).await {
        Ok(Some(r)) => r,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, "Repository not found").into_response();
        }
        Err(e) => {
            error!("Storage error finding repo {}/{}: {}", owner, repo_name, e);
            return (StatusCode::INTERNAL_SERVER_ERROR, "Storage error").into_response();
        }
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    let required_action = match service {
        GitService::UploadPack => Action::Read,
        GitService::ReceivePack => Action::Write,
    };

    // If writing or if private repo, and no user provided, issue 401 challenge for git client to prompt
    if (required_action == Action::Write || repo.is_private) && user.is_none() {
        return auth_challenge();
    }

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), required_action) {
        warn!("Permission denied for info_refs on {}/{}: {}", owner, repo_name, e);
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    if !GitEngine::is_valid_bare_repo(&disk_path) {
        error!("Bare repository missing or corrupted on disk at {:?}", disk_path);
        return (StatusCode::INTERNAL_SERVER_ERROR, "Repository corrupted on disk").into_response();
    }

    match SmartHttpHandler::advertise_refs(&disk_path, service).await {
        Ok((content_type, body)) => {
            (
                StatusCode::OK,
                [
                    (CONTENT_TYPE, content_type.as_str()),
                    (CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
                    (PRAGMA, "no-cache"),
                    (EXPIRES, "0"),
                ],
                body,
            )
                .into_response()
        }
        Err(e) => {
            error!("Advertise refs failed: {}", e);
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Git error: {}", e)).into_response()
        }
    }
}

pub async fn upload_pack_handler(
    State(state): State<AppState>,
    Path((owner, name_raw)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let repo_name = normalize_repo_name(&name_raw);
    let repo = match state.store.get_repo(&owner, &repo_name).await {
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

    if repo.is_private && user.is_none() {
        return auth_challenge();
    }

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Read) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    match SmartHttpHandler::execute_rpc(&disk_path, GitService::UploadPack, body).await {
        Ok((content_type, out)) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, content_type.as_str()),
                (CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            out,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git RPC error: {}", e)).into_response(),
    }
}

pub async fn receive_pack_handler(
    State(state): State<AppState>,
    Path((owner, name_raw)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let repo_name = normalize_repo_name(&name_raw);
    let repo = match state.store.get_repo(&owner, &repo_name).await {
        Ok(Some(r)) => r,
        Ok(None) => return (StatusCode::NOT_FOUND, "Repository not found").into_response(),
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, format!("Storage error: {}", e)).into_response(),
    };

    let user = extract_authenticated_user(&headers, &state.store).await;
    if user.is_none() {
        return auth_challenge();
    }

    let member = if let Some(ref u) = user {
        state.store.get_member(repo.id, u.id).await.ok().flatten()
    } else {
        None
    };

    if let Err(e) = PermissionEngine::check_permission(&repo, user.as_ref(), member.as_ref(), Action::Write) {
        return (StatusCode::FORBIDDEN, format!("Permission denied: {}", e)).into_response();
    }

    let disk_path = PathBuf::from(&repo.disk_path);
    match SmartHttpHandler::execute_rpc(&disk_path, GitService::ReceivePack, body).await {
        Ok((content_type, out)) => (
            StatusCode::OK,
            [
                (CONTENT_TYPE, content_type.as_str()),
                (CACHE_CONTROL, "no-cache, no-store, must-revalidate"),
            ],
            out,
        )
            .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, format!("Git RPC error: {}", e)).into_response(),
    }
}
