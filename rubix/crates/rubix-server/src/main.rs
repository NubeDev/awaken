use rubix_server::store::Store;
use rubix_server::{app, AppState};

fn env_or(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,tower_http=debug".into()),
        )
        .init();

    let db_path = env_or("RUBIX_DB", "rubix.db");
    let addr = env_or("RUBIX_ADDR", "0.0.0.0:8080");
    let ai_min_priority: u8 = env_or("RUBIX_AI_MIN_PRIORITY", "13")
        .parse()
        .map_err(|e| anyhow::anyhow!("RUBIX_AI_MIN_PRIORITY must be 1..=16: {e}"))?;
    if !(1..=16).contains(&ai_min_priority) {
        anyhow::bail!("RUBIX_AI_MIN_PRIORITY must be 1..=16, got {ai_min_priority}");
    }

    let store = Store::open(std::path::Path::new(&db_path))?;
    let state = AppState {
        store,
        ai_min_priority,
    };

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, db = %db_path, "rubix server listening");
    axum::serve(listener, app(state)).await?;
    Ok(())
}
