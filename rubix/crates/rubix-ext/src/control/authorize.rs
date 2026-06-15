//! The fail-closed capability check every mutating control method shares.
//!
//! Before a control method routes its effect as a [`Command`](rubix_gate::Command)
//! through the gate, the extension must hold the WS-04 capability the method
//! requires (`rubix/docs/sessions/WS-13.md`, contract #2). This check runs
//! *first* and **fails closed**: a missing grant returns [`ExtError::Denied`]
//! before any command is built, so an out-of-grant control action produces no
//! record and no audit row. The grant check itself goes through the gate's
//! [`check_capability`](rubix_gate::check_capability), so the extension is
//! authorized off the *same* `Principal` and grant table a user is — one
//! mechanism, no extension-only authz path.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use rubix_core::Principal;
use rubix_gate::{Capability, check_capability};

use crate::error::{ExtError, Result};

/// Confirm `extension` holds `capability`, or deny before any effect.
///
/// Consults the WS-04 grant check on the root handle. Returns `Ok(())` only when
/// the grant exists; a missing grant or a grant-store failure both surface as
/// [`ExtError::Denied`] — never a silent allow.
///
/// # Errors
/// Returns [`ExtError::Denied`] if the extension lacks the grant or the grant
/// lookup fails.
pub async fn authorize(
    db: &Surreal<Db>,
    extension: &Principal,
    capability: Capability,
) -> Result<()> {
    let granted = check_capability(db, extension, capability)
        .await
        .map_err(|e| ExtError::Denied(e.to_string()))?;
    if !granted {
        return Err(ExtError::Denied(format!(
            "{} lacks the {} grant",
            extension.subject,
            capability.as_str()
        )));
    }
    Ok(())
}
