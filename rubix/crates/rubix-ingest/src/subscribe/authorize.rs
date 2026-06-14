//! Resolve the permitted Zenoh key-space for a principal — once, at subscribe.
//!
//! The permitted key-space is a **single capability decision taken at subscribe
//! time**, never re-evaluated per message (`rubix/docs/SCOPE.md`, "Ingestion and
//! pre-processing"; `rubix/STACK-DEISGN.md`, contract #2: the Zenoh key-space is
//! an app-enforced capability). High-rate streams stay un-taxed because the gate
//! is consulted exactly once here; from then on the subscriber only matches the
//! key expression the engine resolved. The decision has two parts, both fail
//! closed: the principal must hold the WS-04 `zenoh-subscribe` grant, and the
//! requested key-space must be a sub-space of the principal's edge-identity root
//! (so one principal cannot subscribe into another edge's partition).

use std::str::FromStr;

use surrealdb::Surreal;
use surrealdb::engine::local::Db;
use zenoh::key_expr::KeyExpr;

use rubix_core::Principal;
use rubix_gate::{Capability, check_capability};

use crate::error::{IngestError, Result};
use crate::persist::keyspace_root;

/// The capability a principal must hold to open an ingest subscription.
const SUBSCRIBE_CAPABILITY: Capability = Capability::ZenohSubscribe;

/// A key-space the gate has authorized a principal to subscribe to.
///
/// Constructing one is the *only* way [`listen`](crate::subscribe::listen) gets a
/// scope, and it is constructable only through [`authorize_keyspace`] — so a
/// subscriber cannot be opened on an unauthorized key-space. The owned key
/// expression is the resolved scope the Zenoh session declares its subscriber on.
#[derive(Debug, Clone)]
pub struct AuthorizedKeySpace {
    scope: KeyExpr<'static>,
}

impl AuthorizedKeySpace {
    /// The resolved key expression the subscriber declares on.
    #[must_use]
    pub fn scope(&self) -> &KeyExpr<'static> {
        &self.scope
    }
}

/// Authorize `principal` to subscribe to `requested` — the one capability
/// decision taken at subscribe.
///
/// Checks the WS-04 `zenoh-subscribe` grant through the gate (fail closed, no
/// fallback) and confirms `requested` is a valid key expression included in the
/// principal's edge-identity key-space root. Returns the [`AuthorizedKeySpace`]
/// the subscriber is opened on, or refuses before any Zenoh session is touched.
///
/// # Errors
/// Returns [`IngestError::KeySpace`] if `requested` (or the principal's root) is
/// not a valid key expression, or [`IngestError::Denied`] if the grant is
/// missing or `requested` escapes the principal's partition.
pub async fn authorize_keyspace(
    db: &Surreal<Db>,
    principal: &Principal,
    requested: &str,
) -> Result<AuthorizedKeySpace> {
    let granted = check_capability(db, principal, SUBSCRIBE_CAPABILITY)
        .await
        .map_err(|e| IngestError::Denied(e.to_string()))?;
    if !granted {
        return Err(IngestError::Denied(format!(
            "{} lacks the {} grant",
            principal.subject,
            SUBSCRIBE_CAPABILITY.as_str()
        )));
    }

    let requested_scope = parse_keyexpr(requested)?;
    let root = parse_keyexpr(&keyspace_root(principal))?;
    if !root.includes(&requested_scope) {
        return Err(IngestError::Denied(format!(
            "key-space {requested} is outside the partition {root}"
        )));
    }

    Ok(AuthorizedKeySpace {
        scope: requested_scope,
    })
}

/// Parse a raw string into an owned key expression, mapping a malformed scope to
/// a domain error rather than a panic.
fn parse_keyexpr(raw: &str) -> Result<KeyExpr<'static>> {
    KeyExpr::from_str(raw)
        .map(KeyExpr::into_owned)
        .map_err(|e| IngestError::KeySpace(format!("{raw}: {e}")))
}
