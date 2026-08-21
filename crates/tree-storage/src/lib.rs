pub mod memory;
pub mod postgres;

pub use memory::MemoryStore;
pub use postgres::PgStore;

use std::sync::Arc;
use tree_core::error::Result;
use tree_core::store::Store;

pub async fn init_store(database_url: &str) -> Result<Arc<dyn Store>> {
    if database_url.starts_with("postgres://") || database_url.starts_with("postgresql://") {
        let pg = PgStore::new(database_url).await?;
        Ok(Arc::new(pg))
    } else {
        // Fallback / mock
        Ok(Arc::new(MemoryStore::new()))
    }
}
