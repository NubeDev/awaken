use rubix_query::QueryEngine;
use rubix_server::bus::ZenohBus;
use rubix_server::store::Store;
use rubix_server::supervisor::{load_manifests, Supervisor};
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

    let mut supervisor: Option<Supervisor> = None;
    let bus = if env_or("RUBIX_ZENOH", "1") == "0" {
        tracing::info!("zenoh disabled (RUBIX_ZENOH=0); HTTP-only mode");
        None
    } else {
        let bus = ZenohBus::open(store.clone()).await?;
        bus.serve().await?;
        tracing::info!("zenoh data plane up: cur pub/sub, write + his queryables");

        // Spawn manifest-described drivers on the same mesh. The supervisor
        // watches their liveliness tokens, so it must share the bus session.
        let drivers_path = env_or("RUBIX_DRIVERS", "drivers.json");
        let manifests = load_manifests(std::path::Path::new(&drivers_path))?;
        if manifests.is_empty() {
            tracing::info!(path = %drivers_path, "no driver manifests; supervisor idle");
        } else {
            let names: Vec<_> = manifests.iter().map(|m| m.identity.name.clone()).collect();
            supervisor = Some(Supervisor::launch(bus.session_clone(), manifests)?);
            tracing::info!(drivers = ?names, "driver supervisor launched");
        }
        Some(bus)
    };

    let query = if env_or("RUBIX_QUERY", "1") == "0" {
        tracing::info!("query engine disabled (RUBIX_QUERY=0)");
        None
    } else {
        let engine = QueryEngine::open(std::path::Path::new(&db_path)).await?;
        tracing::info!("datafusion query surface up: POST /api/v1/query");
        Some(engine)
    };

    let state = AppState {
        store,
        bus,
        query,
        ai_min_priority,
    };

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!(%addr, db = %db_path, "rubix server listening");
    axum::serve(listener, app(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    // Stop supervised drivers cleanly so liveliness tokens clear before exit.
    if let Some(supervisor) = supervisor {
        tracing::info!("stopping driver supervisor");
        supervisor.shutdown().await;
    }
    Ok(())
}

/// Resolve on Ctrl-C (SIGINT). Drives axum's graceful shutdown so the
/// supervisor can stop drivers on the way out.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}
