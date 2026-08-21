//! Authentication utilities for the HTTP boundary.
//!
//! Password hashing lives in `tree_core::auth` (Argon2id, per-password
//! random salt).  This module re-exports the canonical functions and
//! provides Basic Auth extraction for the Git Smart HTTP transport.
//! There is no `mock_hash` bypass; test environments must create users
//! with real credentials via the normal store API.

pub use tree_core::auth::{hash_password, verify_password};

use axum::http::HeaderMap;
use base64::Engine;
use std::sync::Arc;
use tree_core::models::User;
use tree_core::store::Store;

/// Extract a verified `User` from HTTP Basic Auth credentials.
///
/// Returns `None` if:
/// - No `Authorization` header is present.
/// - The header cannot be decoded.
/// - The username does not exist in the store.
/// - The password does not match the stored Argon2id hash.
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
    let mut parts = credentials.splitn(2, ':');
    let username = parts.next()?;
    let password = parts.next().unwrap_or("");

    let user = store.get_user_by_username(username).await.ok()??;

    if verify_password(password, &user.password_hash) {
        Some(user)
    } else {
        None
    }
}
