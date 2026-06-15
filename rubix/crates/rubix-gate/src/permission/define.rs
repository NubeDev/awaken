//! Define the principal access method and the SurrealDB-native row-level
//! permissions that scope reads to a principal's namespace.
//!
//! Contract #2 (`rubix/STACK-DEISGN.md`): data-record permissions are
//! SurrealDB-native, enforced by the engine on the scoped session — not by an
//! app filter. This statement wires that enforcement:
//!
//! - the `principal` table holds the identity records a session signs in as;
//! - the `grant` table holds the app-enforced capability grants (the second
//!   authz layer, `rubix/docs/SCOPE.md`); it carries no scoped-session `select`
//!   permission because grants are read by the gate on the store handle, never
//!   on a principal's scoped session;
//! - the `session_token` table holds opaque server-side login tokens (a hashed
//!   token → credentials, with an expiry); like `grant` it is read only by the
//!   gate on the store handle, never on a scoped session, so it carries no
//!   row-level permission;
//! - `DEFINE ACCESS principal ON DATABASE TYPE RECORD` with a `SIGNIN` clause
//!   resolves a token to a principal record and binds it to `$auth`;
//! - the `record` table's `PERMISSIONS FOR select` clause keys on
//!   `$auth.namespace`, so the engine returns only the session principal's
//!   namespace data;
//! - the `tag` and `tagged` tables carry scoped-session `select` permissions so a
//!   principal can traverse the classification graph (`record→tagged→tag`) it owns:
//!   `tag` nodes are shared, non-sensitive names (`FOR select FULL`), while a
//!   `tagged` edge is readable only when the record it leaves (`in`) is in the
//!   session's namespace — so the graph walk narrows with the record scope and
//!   never reveals another tenant's edges. Without these, a scoped read of
//!   `->tagged->tag` returns empty (the tag filter and the wire tag projection both
//!   depend on this).
//!
//! Run once against the root store handle at bootstrap. Idempotent via
//! `IF NOT EXISTS` / `OVERWRITE`-free `DEFINE` re-application.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{GateError, Result};

/// The table principals live in, the record access method, and the row-level
/// read permission on the generic record table.
///
/// The `SIGNIN` query authenticates a principal by subject + secret, building the
/// `principal` record id with `type::record('principal', $subject)` so a subject
/// containing hyphens (a kebab-case extension name, a UUID) resolves to the right
/// record instead of being parsed as an arithmetic expression. The
/// `PERMISSIONS FOR select WHERE namespace = $auth.namespace` clause is the
/// load-bearing line: SurrealDB itself confines a scoped session's reads to its
/// own namespace.
const GATE_SCHEMA: &str = "\
DEFINE TABLE IF NOT EXISTS principal SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS grant SCHEMALESS;\n\
DEFINE TABLE IF NOT EXISTS session_token SCHEMALESS;\n\
DEFINE ACCESS IF NOT EXISTS principal ON DATABASE TYPE RECORD\n\
  SIGNIN (\n\
    SELECT * FROM principal\n\
    WHERE id = type::record('principal', $subject) AND secret = $secret\n\
  )\n\
  DURATION FOR SESSION 1h;\n\
DEFINE TABLE OVERWRITE record SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select WHERE namespace = $auth.namespace\n\
    FOR create, update, delete NONE;\n\
DEFINE TABLE OVERWRITE tag SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select FULL\n\
    FOR create, update, delete NONE;\n\
DEFINE TABLE OVERWRITE tagged SCHEMALESS\n\
  PERMISSIONS\n\
    FOR select WHERE in.namespace = $auth.namespace\n\
    FOR create, update, delete NONE;";

/// Apply the gate's access method and record permissions on the root handle.
///
/// Must run on a root/owner session (the `rubix-store` handle's connection),
/// because defining access methods and table permissions is an owner action.
///
/// # Errors
/// Returns [`GateError::DefineSchema`] if a statement fails to apply.
pub async fn define_gate_schema(db: &Surreal<Db>) -> Result<()> {
    db.query(GATE_SCHEMA)
        .await
        .map_err(GateError::DefineSchema)?
        .check()
        .map_err(GateError::DefineSchema)?;
    Ok(())
}
