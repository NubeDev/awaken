//! Rubix server binary: open the store, build state, serve the transport.
//!
//! Boots on the committed `RuntimeConfig` edge default and serves the full WS-16
//! transport (HTTP routes, the WebSocket live-query bridge, and the OpenAPI
//! document). Edge/cloud **profile selection** into `AppState` is WS-14 and is
//! deferred (see `rubix/docs/sessions/TODOs.md`); the binary uses the default
//! edge profile until that lands.

use rubix_core::{Error, Result, ResultExt, RuntimeConfig};
use rubix_gate::{define_audit_schema, define_gate_schema};
use rubix_server::{AppState, define_datasource_schema, rehydrate, router, seed_dev};
use rubix_store::StoreHandle;

const DEFAULT_NAMESPACE: &str = "rubix";
const DEFAULT_DATABASE: &str = "main";
const DEFAULT_DATA_DIR: &str = "rubix-data";
const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<()> {
    let config = load_config();
    let store = StoreHandle::open(&config)
        .await
        .context("opening store on startup")?;

    // The gate's identity/grant/audit tables are not part of the store's base
    // schema, so define them at boot (idempotent) — without this no principal
    // can authenticate and every mutation's audit append would fail.
    define_gate_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining gate schema: {e}")))?;
    define_audit_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining audit schema: {e}")))?;

    // The datasource control plane persists registered connectors in its own
    // config table (server config, not tenant data), so define it at boot too.
    define_datasource_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining datasource schema: {e}")))?;

    if std::env::args().any(|arg| arg == "--seed-dev") {
        seed_dev(store.raw())
            .await
            .map_err(|e| Error::Config(e.to_string()))?;
    }

    let state = AppState::new(store, config.namespace.clone(), config.database.clone());

    // Rebuild any datasource connectors registered in a prior run into the shared
    // registry before serving, so the registry reflects persisted state. A
    // connector whose backend is unreachable is logged and skipped, not fatal.
    {
        let mut registry = state.datasources.write().await;
        let restored = rehydrate(&mut registry, state.store.raw())
            .await
            .map_err(|e| Error::Config(format!("rehydrating datasources: {e}")))?;
        if restored > 0 {
            println!("rehydrated {restored} datasource connector(s)");
        }
    }

    let bind = std::env::var("RUBIX_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| rubix_core::Error::Config(format!("binding {bind}: {e}")))?;

    axum::serve(listener, router(state))
        .await
        .map_err(|e| rubix_core::Error::Config(format!("serving HTTP: {e}")))?;
    Ok(())
}

/// Build the runtime config from the environment, defaulting to a file-backed
/// edge node.
fn load_config() -> RuntimeConfig {
    let namespace = std::env::var("RUBIX_NAMESPACE").unwrap_or_else(|_| DEFAULT_NAMESPACE.to_owned());
    let database = std::env::var("RUBIX_DATABASE").unwrap_or_else(|_| DEFAULT_DATABASE.to_owned());
    let data_dir = std::env::var("RUBIX_DATA_DIR").unwrap_or_else(|_| DEFAULT_DATA_DIR.to_owned());
    RuntimeConfig::file_backed(data_dir, namespace, database)
}
