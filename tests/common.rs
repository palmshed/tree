use std::net::SocketAddr;
use std::sync::Arc;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tree_core::config::ServerConfig;
use tree_storage::MemoryStore;

pub struct TestServer {
    pub addr: SocketAddr,
    pub base_url: String,
    pub temp_dir: TempDir,
    _handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    pub async fn start() -> Self {
        let temp_dir = TempDir::new().unwrap();
        let data_dir = temp_dir.path().join("git");
        tokio::fs::create_dir_all(&data_dir).await.unwrap();

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let base_url = format!("http://{}", addr);

        let config = ServerConfig {
            host: addr.ip().to_string(),
            port: addr.port(),
            data_dir,
            database_url: "memory://".to_string(),
            base_url: base_url.clone(),
            default_owner: "user".to_string(),
        };

        let store = Arc::new(MemoryStore::new());
        let state = tree_server::state::AppState::new(store, config);
        let app = tree_server::create_router(state);

        let handle = tokio::spawn(async move {
            let _ = axum::serve(listener, app).await;
        });

        // Small wait for server readiness
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Self {
            addr,
            base_url,
            temp_dir,
            _handle: handle,
        }
    }
}
