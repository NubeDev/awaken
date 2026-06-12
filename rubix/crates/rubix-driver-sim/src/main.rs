//! Reference rubix driver: a simulator that honors the full driver contract.
//!
//! The supervisor spawns it with `RUBIX_DRIVER_NAME`/`_CAPS`/`_CONFIG` in the
//! environment. It opens a peer zenoh session, declares its liveliness token so
//! the supervisor confirms bus attachment, and publishes simulated `cur`
//! samples on a granted point keyexpr until SIGINT — at which point the
//! liveliness token clears and the supervisor reaps it.
//!
//! Not a test stub: a real, capability-scoped driver useful for demos and as
//! the live spawn/restart target the supervisor tests drive.

mod config;
mod liveliness;
mod scoped;
mod simulate;
mod write;

use config::SimConfig;
use scoped::ScopedSession;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let cfg = SimConfig::from_env()?;
    tracing::info!(driver = %cfg.name, point = %cfg.point, "sim driver starting");

    let session = zenoh::open(zenoh::Config::default())
        .await
        .map_err(|e| anyhow::anyhow!("zenoh open: {e}"))?;

    // Hold the token for the process lifetime; it clears on drop at exit.
    let _token = liveliness::declare(&session, &cfg.name).await?;
    tracing::info!(driver = %cfg.name, "liveliness token declared; attached to bus");

    // Confine the publish path to the granted capabilities: an out-of-scope
    // keyexpr is refused locally, before it reaches the bus.
    let scoped = ScopedSession::new(cfg.name.clone(), cfg.caps.clone(), session);
    simulate::run(&scoped, &cfg, shutdown_signal()).await;
    Ok(())
}

/// Resolve on Ctrl-C (SIGINT). Dropping the session/token after this clears
/// the driver's liveliness on the mesh so the supervisor reaps it.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("sim received shutdown signal");
}
