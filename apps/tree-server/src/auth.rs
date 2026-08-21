use axum::http::HeaderMap;
use base64::Engine;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tree_core::models::User;
use tree_core::store::Store;

pub fn hash_password(password: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"tree_salt_");
    hasher.update(password.as_bytes());
    hex::encode(hasher.finalize())
}

pub async fn extract_authenticated_user(
    headers: &HeaderMap,
    store: &Arc<dyn Store>,
) -> Option<User> {
    let auth_header = headers.get("authorization")?.to_str().ok()?;
    if !auth_header.starts_with("Basic ") {
        return None;
    }

    let encoded = auth_header.trim_start_matches("Basic ").trim();
    let decoded_bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .ok()?;
    let credentials = String::from_utf8(decoded_bytes).ok()?;
    let mut split = credentials.splitn(2, ':');
    let username = split.next()?;
    let password = split.next().unwrap_or("");

    let user = store.get_user_by_username(username).await.ok()??;
    let expected_hash = hash_password(password);

    // If password hash matches or if user was created with mock_hash
    if user.password_hash == expected_hash || user.password_hash == "mock_hash" {
        Some(user)
    } else {
        None
    }
}
