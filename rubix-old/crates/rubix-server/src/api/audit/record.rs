//! The handler-facing change recorder (docs/design/audit-and-undo.md "Recording").
//! A mutation handler, holding the `before` it already read for authz and the
//! `after` it just wrote, appends one ledger row through [`Recorder`]. This is the
//! sole change write path the HTTP layer uses; it delegates to the WS-07 substrate
//! ([`Store::record_change`]), which validates the row and is the only writer of the
//! `changes` table.
//!
//! Atomicity note: the substrate also exposes a transaction-scoped record
//! (`record_in_sqlite_tx`) that commits the change inside the mutation's own
//! transaction. The domain store mutators do not yet expose a caller-owned
//! transaction, so this recorder records immediately after the successful mutation.
//! See the WS-08 session log and TODOs for the transaction-threading follow-up.

use rubix_core::{Actor, Change};
use serde_json::Value;
use uuid::Uuid;

use crate::auth::RequestPrincipal;
use crate::store::{new_change_id, new_group_id, Store};

/// The sentinel a redacted secret field is replaced with so a plaintext credential
/// never lands in the ledger (docs/design/audit-and-undo.md "Secret redaction is a
/// recording contract"). The audit log shows that a field changed, never its value.
pub(crate) const REDACTED: &str = "***";

/// Replace the named top-level fields of a snapshot with [`REDACTED`] before it is
/// recorded, so a secret-bearing kind (a datasource connection string, a token
/// secret, an agent API key) never captures a plaintext value. Owners of a
/// secret-bearing kind redact at record time and assert it in tests. Absent fields
/// are ignored; the snapshot is otherwise untouched.
//
// The secret-bearing kinds (datasource/token) adopt this when their handlers are
// wired into the recorder; see the WS-08 session log. Kept as the single redaction
// contract so those kinds share one implementation rather than re-rolling it.
#[allow(dead_code)]
pub(crate) fn redact_fields(mut snapshot: Value, secret_fields: &[&str]) -> Value {
    if let Value::Object(map) = &mut snapshot {
        for field in secret_fields {
            if let Some(slot) = map.get_mut(*field) {
                *slot = Value::String(REDACTED.to_string());
            }
        }
    }
    snapshot
}

/// Resolve the [`Actor`] for a recorded change from the request principal. An
/// authenticated caller records as `User { subject }`; an unauthenticated edge
/// request (auth disabled) records as `System` — there is no human identity to
/// attribute, and fabricating one would corrupt the audit trail.
pub(crate) fn actor_of(principal: &RequestPrincipal) -> Actor {
    match &principal.0 {
        Some(p) => Actor::User { subject: p.subject.clone() },
        None => Actor::System,
    }
}

/// Records ledger rows for a logical operation. One recorder spans one operation:
/// all its rows share a `group_id`, so a multi-row mutation (e.g. a cascade) undoes
/// as a single step. Build one per handler call.
pub(crate) struct Recorder {
    actor: Actor,
    org: String,
    site_id: Option<Uuid>,
    group_id: Uuid,
}

impl Recorder {
    /// A recorder for one operation in `org`, attributed to `actor`. `site_id`
    /// scopes the rows when the entity is site-scoped (`None` for org-level).
    pub(crate) fn new(actor: Actor, org: impl Into<String>, site_id: Option<Uuid>) -> Self {
        Self {
            actor,
            org: org.into(),
            site_id,
            group_id: new_group_id(),
        }
    }

    /// Record a create: `after` is the new row's snapshot. Clears the actor's redo
    /// stack — a fresh edit invalidates any previously-undone groups.
    pub(crate) fn create(
        &self,
        store: &Store,
        kind: &str,
        resource_id: Uuid,
        after: Value,
    ) -> Result<(), crate::store::StoreError> {
        let (id, at) = new_change_id();
        let change = Change::create(
            id,
            at,
            self.org.clone(),
            self.site_id,
            self.actor.clone(),
            kind,
            resource_id,
            after,
            self.group_id,
            None,
        );
        self.commit(store, &change)
    }

    /// Record an update: both `before` and `after` snapshots, captured around the
    /// mutation.
    pub(crate) fn update(
        &self,
        store: &Store,
        kind: &str,
        resource_id: Uuid,
        before: Value,
        after: Value,
    ) -> Result<(), crate::store::StoreError> {
        let (id, at) = new_change_id();
        let change = Change::update(
            id,
            at,
            self.org.clone(),
            self.site_id,
            self.actor.clone(),
            kind,
            resource_id,
            before,
            after,
            self.group_id,
            None,
        );
        self.commit(store, &change)
    }

    /// Record a delete: `before` is the removed row's snapshot.
    pub(crate) fn delete(
        &self,
        store: &Store,
        kind: &str,
        resource_id: Uuid,
        before: Value,
    ) -> Result<(), crate::store::StoreError> {
        let (id, at) = new_change_id();
        let change = Change::delete(
            id,
            at,
            self.org.clone(),
            self.site_id,
            self.actor.clone(),
            kind,
            resource_id,
            before,
            self.group_id,
            None,
        );
        self.commit(store, &change)
    }

    fn commit(&self, store: &Store, change: &Change) -> Result<(), crate::store::StoreError> {
        store.record_change(change)?;
        // Standard undo semantics: a new change invalidates the actor's redo stack.
        store.clear_redo_stack(&self.org, self.actor.cursor_subject())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::{Principal, Role, Scope};

    fn principal(subject: &str) -> RequestPrincipal {
        RequestPrincipal(Some(Principal {
            subject: subject.into(),
            scope: Scope::org("kfc"),
            role: Role::Operator,
            user_id: None,
            team_ids: Vec::new(),
        }))
    }

    #[test]
    fn redact_replaces_named_secret_fields_only() {
        let snapshot = serde_json::json!({
            "id": "ds-1",
            "name": "historian",
            "connection": "postgres://user:hunter2@db/h",
            "token": "pat_abc",
        });
        let redacted = redact_fields(snapshot, &["connection", "token"]);
        assert_eq!(redacted["connection"], REDACTED);
        assert_eq!(redacted["token"], REDACTED);
        // Non-secret fields are untouched; an absent field is ignored.
        assert_eq!(redacted["name"], "historian");
        let again = redact_fields(serde_json::json!({"name": "x"}), &["connection"]);
        assert_eq!(again, serde_json::json!({"name": "x"}));
    }

    #[test]
    fn actor_is_the_principal_subject_or_system_when_open() {
        assert_eq!(
            actor_of(&principal("sub-1")),
            Actor::User { subject: "sub-1".into() }
        );
        // Auth disabled (no principal) attributes to System, never a fabricated user.
        assert_eq!(actor_of(&RequestPrincipal(None)), Actor::System);
    }
}
