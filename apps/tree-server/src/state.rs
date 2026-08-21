use std::path::PathBuf;
use std::sync::Arc;
use tree_core::config::ServerConfig;
use tree_core::store::Store;

#[derive(Clone)]
pub struct AppState {
    pub store: Arc<dyn Store>,
    pub config: ServerConfig,
}

impl AppState {
    pub fn new(store: Arc<dyn Store>, config: ServerConfig) -> Self {
        Self { store, config }
    }

    pub fn git_dir(&self) -> &PathBuf {
        &self.config.data_dir
    }
}
