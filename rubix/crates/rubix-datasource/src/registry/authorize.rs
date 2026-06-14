//! Gate registering a datasource on the WS-04 `datasource-register` capability.
//!
//! Registering a datasource is a cross-plane action SurrealDB's permission engine
//! does not see, so it is an **app-enforced capability**, not a row permission
//! (`rubix/STACK-DEISGN.md`, contract #2; the second authz layer of
//! `rubix/docs/SCOPE.md`). A principal may register a connector only if it holds
//! the [`Capability::DatasourceRegister`] grant. This verb makes that check fail
//! closed: a missing grant denies, and an error in the grant lookup is surfaced,
//! never read as allow.

use rubix_core::Principal;
use rubix_gate::{Capability, check_capability};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{DatasourceError, Result};

/// The capability registering a datasource requires.
const REGISTER_CAPABILITY: Capability = Capability::DatasourceRegister;

/// Decide whether `principal` may register a datasource, fail closed.
///
/// `grant_reader` is a connection that can read the `grant` table — the store root
/// handle's connection — because grants carry no scoped-session `select`
/// permission (they are app-enforced, WS-04).
///
/// # Errors
/// Returns [`DatasourceError::Denied`] if the principal lacks the grant, or
/// [`DatasourceError::Capability`] if the grant lookup itself fails.
pub async fn authorize_register(grant_reader: &Surreal<Db>, principal: &Principal) -> Result<()> {
    let granted = check_capability(grant_reader, principal, REGISTER_CAPABILITY)
        .await
        .map_err(|e| DatasourceError::Capability(e.to_string()))?;
    if granted {
        Ok(())
    } else {
        Err(DatasourceError::Denied)
    }
}
