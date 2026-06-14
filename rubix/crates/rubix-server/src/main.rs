//! Minimal rubix server binary: open the store, build state, serve HTTP.
//!
//! Edge/cloud profile selection and the full transport surface (JSON-RPC, WS
//! live-query bridge, OpenAPI) land in WS-16; this binary establishes the
//! `AppState` wiring and the `/health` route every later route hangs off.

use rubix_core::{Result, ResultExt, RuntimeConfig};
use rubix_server::{AppState, router};
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
    let state = AppState::new(store);

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
