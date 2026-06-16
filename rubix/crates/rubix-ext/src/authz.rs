//! The fail-closed capability check every extension plane shares.
//!
//! Before an extension takes any cross-plane action — a control command
//! ([`crate::control`]) or opening the event bus ([`crate::bus`]) — it must hold
//! the WS-04 capability that action requires (`rubix/docs/sessions/WS-13.md`,
//! contract #2). This check runs *first* and **fails closed**: a missing grant
//! returns [`ExtError::Denied`] before any effect, so an out-of-grant action
//! produces no record, no audit row, and no event. The grant check itself goes
//! through the gate's [`check_capability`](rubix_gate::check_capability), so the
//! extension is authorized off the *same* `Principal` and grant table a user is —
//! one mechanism, no extension-only authz path. It lives at the crate root, not
//! inside any one plane, because every plane that gates a principal shares it.

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
pub(crate) async fn authorize(
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
