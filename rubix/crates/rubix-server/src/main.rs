//! Rubix server binary: select the profile, open the store, build state, serve.
//!
//! Selects the deployment profile from `RUBIX_PROFILE` among the compiled-in
//! profiles (WS-14), fails the boot closed if the name is unknown/uncompiled or a
//! required backend is absent, then serves the full WS-16 transport (HTTP routes,
//! the WebSocket live-query bridge, and the OpenAPI document). The selected
//! [`Profile`](rubix_server::Profile) is threaded into `AppState` so the gate
//! resolves a request's tenant namespace per profile.

use rubix_core::{
    Error, Result, ResultExt, RuntimeConfig, bootstrap_meta_collection, migrate_history_to_readings,
};
use rubix_gate::{define_audit_schema, define_gate_schema};
use rubix_trace::define_trace_schema;
use rubix_server::{
    AppState, define_datasource_schema, define_tenant_schema, profile as server_profile, rehydrate,
    router, seed_dev, spawn_hook_dispatcher,
};
use rubix_store::StoreHandle;

const DEFAULT_NAMESPACE: &str = "rubix";
const DEFAULT_DATABASE: &str = "main";
const DEFAULT_DATA_DIR: &str = "rubix-data";
const DEFAULT_BIND: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> Result<()> {
    // Select the deployment profile before touching the store: an unknown or
    // uncompiled `RUBIX_PROFILE`, or a profile whose required backend is absent
    // from the build, must fail the boot closed with a clear error and bind no
    // socket (WS-14, "fails closed at boot").
    let profile = server_profile::from_env().map_err(|e| Error::Config(e.to_string()))?;
    profile
        .verify_backends()
        .map_err(|e| Error::Config(e.to_string()))?;
    println!("rubix profile: {:?}", profile.kind);

    let config = load_config(profile.kind);
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

    // The trace tables back the per-evaluation span tree a fired rule persists.
    // Define them at boot now that the hook dispatcher fires rules at runtime (the
    // first runtime caller of the full evaluate path) — without this a hook firing
    // would fail to record its span. Idempotent.
    define_trace_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining trace schema: {e}")))?;

    // The datasource control plane persists registered connectors in its own
    // config table (server config, not tenant data), so define it at boot too.
    define_datasource_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining datasource schema: {e}")))?;

    // The tenant registry (onboarded-namespace bookkeeping) is server config too,
    // mounted on every build so the route table is identical edge-to-cloud.
    define_tenant_schema(store.raw())
        .await
        .map_err(|e| Error::Config(format!("defining tenant schema: {e}")))?;

    // Seed the bootstrap meta-collection (the collection-defining-collection) for
    // the default namespace so collection records are discoverable and, under
    // strict mode, validated against it. Idempotent — a no-op on a non-fresh store.
    bootstrap_meta_collection(store.raw(), &config.namespace)
        .await
        .map_err(|e| Error::Config(format!("seeding meta-collection: {e}")))?;

    // One-shot maintenance: move any legacy `kind:"history"` records into the
    // `reading` data plane, then exit without binding a socket. Idempotent (keyed
    // by the deterministic `(series, at)` id), so a re-run or a crash mid-migration
    // is safe (`rubix/docs/design/READINGS-TIMESERIES.md`, "Migration").
    if std::env::args().any(|arg| arg == "--migrate-history") {
        let report = migrate_history_to_readings(store.raw())
            .await
            .map_err(|e| Error::Config(e.to_string()))?;
        println!(
            "history migration: {} migrated, {} deleted, {} skipped",
            report.migrated, report.deleted, report.skipped
        );
        return Ok(());
    }

    if std::env::args().any(|arg| arg == "--seed-dev") {
        seed_dev(store.raw())
            .await
            .map_err(|e| Error::Config(e.to_string()))?;
    }

    let mut state = AppState::with_profile(
        store,
        config.namespace.clone(),
        config.database.clone(),
        profile,
    );

    // Root the blob store under the configured data directory so file uploads
    // persist across restarts on a file-backed deployment (the constructor default
    // is an ephemeral temp dir, for tests). Edge uses the local-filesystem store;
    // an object-store backend (cloud) is the deferred follow-up.
    let blob_root = std::path::PathBuf::from(
        std::env::var("RUBIX_DATA_DIR").unwrap_or_else(|_| DEFAULT_DATA_DIR.to_owned()),
    )
    .join("blobs");
    state.blobs = std::sync::Arc::new(rubix_blob::LocalFsBlobStore::open(blob_root));

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

    // Start the after-write hook dispatcher: a background subscriber on the
    // live-query data plane that fires a rule when a watched record is written
    // (BACKEND-COLLECTIONS.md, build-order step 5). It fires after the commit, so a
    // hook is a side effect, never a veto; before-hooks are out of scope.
    spawn_hook_dispatcher(state.clone());

    let bind = std::env::var("RUBIX_BIND").unwrap_or_else(|_| DEFAULT_BIND.to_owned());
    let listener = tokio::net::TcpListener::bind(&bind)
        .await
        .map_err(|e| rubix_core::Error::Config(format!("binding {bind}: {e}")))?;

    axum::serve(listener, router(state))
        .await
        .map_err(|e| rubix_core::Error::Config(format!("serving HTTP: {e}")))?;
    Ok(())
}

/// Build the runtime config from the environment, file-backed, carrying the
/// selected deployment profile so the store/runtime layer sees the same choice.
fn load_config(profile: rubix_core::Profile) -> RuntimeConfig {
    let namespace =
        std::env::var("RUBIX_NAMESPACE").unwrap_or_else(|_| DEFAULT_NAMESPACE.to_owned());
    let database = std::env::var("RUBIX_DATABASE").unwrap_or_else(|_| DEFAULT_DATABASE.to_owned());
    let data_dir = std::env::var("RUBIX_DATA_DIR").unwrap_or_else(|_| DEFAULT_DATA_DIR.to_owned());
    let mut config = RuntimeConfig::file_backed(data_dir, namespace, database);
    config.profile = profile;
    config
}
