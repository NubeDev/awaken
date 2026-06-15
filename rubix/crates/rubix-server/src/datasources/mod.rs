//! Datasource control-plane persistence and connector construction.
//!
//! The registry the query surface reads (`rubix/docs/SCOPE.md`, "Datasources") is
//! in-memory, so a registered connector would vanish on restart. This module is
//! the durable half: a server-owned `datasource` table records each registered
//! connector's declaration (id, label, kind, connection string, tables), the boot
//! path [`rehydrate`]s them back into the shared [`Registry`], and the control
//! plane [`save`]s/[`forget`]s a row alongside each register/remove.
//!
//! The `datasource` table is server configuration, not tenant data: it carries no
//! scoped-session permission and is read/written on the root store handle, like
//! the gate's `grant` table. Connection strings are a deployment secret — stored
//! here to rehydrate the pool, never returned on the wire (the DTO omits them).
//!
//! Building a live connector from a stored declaration is [`build_connector`]. It
//! is keyed on `kind`; only `"postgres"` is supported, and only when the crate is
//! built with the `postgres` feature — absent it, a register of that kind fails
//! closed (`UnsupportedKind`) rather than silently degrading.

mod persist;

use rubix_core::Principal;
use rubix_datasource::Registry;
#[cfg(feature = "postgres")]
use rubix_datasource::{register, register_materialized};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

pub use persist::{StoredDatasource, define_datasource_schema, forget, load_all, save};

/// A control-plane failure registering, persisting, or rehydrating a datasource.
#[derive(Debug, thiserror::Error)]
pub enum ControlError {
    /// The connector `kind` is unknown or its feature is not compiled in.
    #[error("unsupported datasource kind `{0}` (feature not built or unknown)")]
    UnsupportedKind(String),
    /// The datasource crate refused the registration (denied, duplicate, connect).
    #[error(transparent)]
    Datasource(#[from] rubix_datasource::DatasourceError),
    /// The `datasource` table read/write failed.
    #[error("datasource store error: {0}")]
    Store(String),
}

/// Build a live connector from a stored declaration and register it into
/// `registry`, capability-checked against `principal` on the root `grant_reader`.
///
/// This is the single construction path both the HTTP register route and boot
/// [`rehydrate`] go through, so the capability gate (`datasource-register`) and
/// the connect attempt run identically for a fresh register and a replay. The
/// providers are materialised here (the connect reaches the backend), so a
/// register that returns `Ok` has a live, queryable datasource.
///
/// # Errors
/// - [`ControlError::UnsupportedKind`] if `kind` is not a built connector.
/// - [`ControlError::Datasource`] from the capability check, a duplicate id, or a
///   failed connect.
pub async fn build_and_register(
    registry: &mut Registry,
    grant_reader: &Surreal<Db>,
    principal: &Principal,
    decl: &StoredDatasource,
) -> Result<(), ControlError> {
    register_decl(registry, Some((grant_reader, principal)), decl).await
}

/// The connector-build + register core, shared by the capability-checked route
/// path and the trusted boot replay.
///
/// `authz` carries the `(grant_reader, principal)` to capability-check against when
/// present (the HTTP register path); `None` is the trusted replay path
/// ([`rehydrate`]), where the row was only persisted after an authorized register
/// and re-checking would be redundant. The connector is built once and its
/// providers materialised either way.
#[cfg_attr(not(feature = "postgres"), allow(unused_variables))]
async fn register_decl(
    registry: &mut Registry,
    authz: Option<(&Surreal<Db>, &Principal)>,
    decl: &StoredDatasource,
) -> Result<(), ControlError> {
    match decl.kind.as_str() {
        #[cfg(feature = "postgres")]
        "postgres" => {
            let connector = rubix_datasource::PostgresConnector::connect(
                decl.id.clone(),
                decl.label.clone(),
                &decl.connection_string,
                decl.tables.clone(),
            )
            .await?;
            match authz {
                Some((grant_reader, principal)) => {
                    register(registry, grant_reader, principal, connector).await?;
                }
                None => register_materialized(registry, connector).await?,
            }
            Ok(())
        }
        other => Err(ControlError::UnsupportedKind(other.to_owned())),
    }
}

/// Reload every persisted datasource declaration into `registry` at boot.
///
/// Runs after the registry is created with its native default and before the
/// server serves, so a restart restores the same set of connectors a client
/// registered. A declaration that fails to reconnect (the backend is down, the
/// credentials rotated) is logged and skipped rather than aborting boot — the
/// surviving datasources stay queryable, and a re-register repairs the broken one.
///
/// This is a trusted replay: each row was only persisted after an authorized
/// register passed its `datasource-register` check, so rehydrate skips the
/// capability gate (the principal that registered may no longer exist) — it never
/// runs from the wire.
///
/// # Errors
/// Returns [`ControlError::Store`] if the `datasource` table cannot be read; a
/// per-connector connect failure is logged and skipped, not surfaced.
pub async fn rehydrate(registry: &mut Registry, db: &Surreal<Db>) -> Result<usize, ControlError> {
    let stored = load_all(db).await.map_err(ControlError::Store)?;
    let mut restored = 0;
    for decl in stored {
        match register_decl(registry, None, &decl).await {
            Ok(()) => restored += 1,
            Err(e) => eprintln!(
                "datasource rehydrate: skipping `{}` ({}): {e}",
                decl.id, decl.kind
            ),
        }
    }
    Ok(restored)
}
