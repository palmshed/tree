use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub data_dir: PathBuf,
    pub database_url: String,
    pub base_url: String,
    pub default_owner: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let data_dir = std::env::var("TREE_DATA_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("./data/git"));

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://localhost/tree_db".to_string());

        let host = std::env::var("TREE_HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
        let port = std::env::var("TREE_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let base_url = std::env::var("TREE_BASE_URL")
            .unwrap_or_else(|_| format!("http://localhost:{}", port));

        let default_owner = std::env::var("TREE_DEFAULT_OWNER")
            .unwrap_or_else(|_| "user".to_string());

        Self {
            host,
            port,
            data_dir,
            database_url,
            base_url,
            default_owner,
        }
    }
}
