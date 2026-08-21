pub mod api;
pub mod auth;
pub mod git_http;
pub mod state;

use api::*;
use axum::extract::DefaultBodyLimit;
use axum::response::{Html, IntoResponse, Response};
use axum::routing::{get, post};
use axum::Router;
use state::AppState;
use std::time::Duration;
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tower_http::timeout::TimeoutLayer;
use tower_http::trace::TraceLayer;

pub fn create_router(state: AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(health_handler))
        .route("/api/v1/health", get(health_handler))
        // User endpoints
        .route("/users", post(create_user_handler))
        .route("/users/:username", get(get_user_handler))
        // API endpoints specified in prompt
        .route(
            "/repositories",
            post(create_repository_handler).get(list_repositories_handler),
        )
        .route(
            "/repositories/:owner/:name",
            get(get_repository_handler).delete(delete_repository_handler),
        )
        .route(
            "/repositories/:owner/:name/branches",
            get(list_branches_handler),
        )
        .route("/repositories/:owner/:name/tags", get(list_tags_handler))
        .route(
            "/repositories/:owner/:name/commits",
            get(list_commits_handler),
        )
        .route("/repositories/:owner/:name/tree", get(get_tree_handler))
        .route("/repositories/:owner/:name/blob", get(get_blob_handler))
        .route(
            "/repositories/:owner/:name/permissions",
            post(set_permission_handler).get(list_permissions_handler),
        )
        // Alias under /api/v1 for convenience
        .route(
            "/api/v1/repositories",
            post(create_repository_handler).get(list_repositories_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name",
            get(get_repository_handler).delete(delete_repository_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/branches",
            get(list_branches_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/tags",
            get(list_tags_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/commits",
            get(list_commits_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/tree",
            get(get_tree_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/blob",
            get(get_blob_handler),
        )
        .route(
            "/api/v1/repositories/:owner/:name/permissions",
            post(set_permission_handler).get(list_permissions_handler),
        )
        // Git Smart HTTP transport endpoints
        .route("/:owner/:name/info/refs", get(git_http::info_refs_handler))
        .route(
            "/:owner/:name/git-upload-pack",
            post(git_http::upload_pack_handler),
        )
        .route(
            "/:owner/:name/git-receive-pack",
            post(git_http::receive_pack_handler),
        )
        // Static UI handler
        .route("/", get(serve_ui))
        .route("/ui", get(serve_ui))
        .route("/ui/*path", get(serve_ui))
        .layer(
            ServiceBuilder::new()
                // Reject request bodies larger than 100 MiB.
                // This prevents a single large git push from exhausting server memory.
                .layer(DefaultBodyLimit::max(100 * 1024 * 1024))
                .layer(TraceLayer::new_for_http())
                // Drop connections that have not completed within 120 seconds.
                // Covers stalled git clients that open a receive-pack stream but
                // never send EOF.
                //
                // Cors and Timeout must sit below Trace here: both synthesize
                // empty default responses (preflight / expiry), which requires
                // the wrapped response body type to implement `Default`, which is
                // true for axum's route bodies, not for trace/cors wrappers.
                .layer(TimeoutLayer::new(Duration::from_secs(120)))
                .layer(CorsLayer::permissive()),
        )
        .with_state(state)
}

async fn serve_ui() -> Response {
    let html = include_str!("../../../web/dist/index.html");
    Html(html).into_response()
}
