use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;
use tree_core::config::ServerConfig;
use tree_server::create_router;
use tree_server::state::AppState;
use tree_storage::init_store;

#[derive(Parser, Debug)]
#[command(name = "tree-server", about = "Tree Git Hosting Server")]
struct Args {
    #[arg(long, default_value = "0.0.0.0", env = "TREE_HOST")]
    host: String,

    #[arg(long, default_value_t = 8080, env = "TREE_PORT")]
    port: u16,

    #[arg(long, env = "DATABASE_URL")]
    database_url: Option<String>,

    #[arg(long, default_value = "./data/git", env = "TREE_DATA_DIR")]
    data_dir: PathBuf,

    #[arg(long, env = "TREE_BASE_URL")]
    base_url: Option<String>,

    #[arg(long, default_value = "user", env = "TREE_DEFAULT_OWNER")]
    default_owner: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    let _ = tracing::subscriber::set_global_default(subscriber);

    let args = Args::parse();

    let base_url = args
        .base_url
        .unwrap_or_else(|| format!("http://localhost:{}", args.port));

    let database_url = args
        .database_url
        .unwrap_or_else(|| "postgres://localhost/tree_db".to_string());

    let config = ServerConfig {
        host: args.host.clone(),
        port: args.port,
        data_dir: args.data_dir.clone(),
        database_url: database_url.clone(),
        base_url,
        default_owner: args.default_owner,
    };

    // Ensure data directory exists
    tokio::fs::create_dir_all(&config.data_dir).await?;

    info!("Connecting to storage: {}", database_url);
    let store = match init_store(&database_url).await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(
                "Failed to connect to PostgreSQL ({}), falling back to in-memory store for local testing: {}",
                database_url,
                e
            );
            Arc::new(tree_storage::MemoryStore::new())
        }
    };

    let state = AppState::new(store, config);
    let app = create_router(state);

    let addr: SocketAddr = format!("{}:{}", args.host, args.port).parse()?;
    info!("🌳 Tree Git Server listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
