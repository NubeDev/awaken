//! The change-ledger model (docs/design/audit-and-undo.md "The substrate"): one
//! append-only `Change` row per logical mutation. The same row powers two reads —
//! the audit log queries the rows, undo/redo replays them. `before`/`after` are
//! **full JSON snapshots**, never diffs, so undo is "write the snapshot back" and
//! every row is self-contained.
//!
//! These are pure types: shape, the `Op`/`Actor` unions, and the constructors that
//! keep `before`/`after` consistent with the op. Persistence and replay live in the
//! server (`store::changes`); no IO here.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use utoipa::ToSchema;
use uuid::Uuid;

/// The kind of mutation a [`Change`] records. The snapshot fields are constrained
/// by the op: a `Create` has no `before`, a `Delete` has no `after`, an `Update`
/// has both. The generic reverser keys its inverse/forward off this.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    Create,
    Update,
    Delete,
}

impl Op {
    pub fn as_str(self) -> &'static str {
        match self {
            Op::Create => "create",
            Op::Update => "update",
            Op::Delete => "delete",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "create" => Op::Create,
            "update" => Op::Update,
            "delete" => Op::Delete,
            _ => return None,
        })
    }
}

/// Who made the change. `Agent` is the AI-agent runtime writing to the **same**
/// ledger (`run_id` + `model`), so agent edits are auditable and undoable by the
/// user; `System` is the scheduler / provisioning path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Actor {
    /// A human principal, identified by their RBAC subject (OIDC `sub` / PAT id).
    User { subject: String },
    /// The agent runtime, identified by its run and model.
    Agent { run_id: Uuid, model: String },
    /// The scheduler or provisioning path — no human principal.
    System,
}

impl Actor {
    /// The subject string for the per-actor undo cursor. Agent and System edits
    /// share their own stable cursor keys so an undo by one does not pop another's.
    /// (Agent runs are short-lived; the design's undo target is *your* changes, and
    /// agent/system changes are undone by the user via the audit surface, not a
    /// per-run cursor — so they collapse onto one stable key here.)
    pub fn cursor_subject(&self) -> &str {
        match self {
            Actor::User { subject } => subject,
            Actor::Agent { .. } => "@agent",
            Actor::System => "@system",
        }
    }
}

/// One immutable change-ledger row (docs/design/audit-and-undo.md "The change
/// row"). Append-only, monotonic by `(at, id)`, always org-scoped. `group_id`
/// joins the rows of one logical operation (e.g. a cascade delete) so they undo as
/// a single step.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, ToSchema)]
pub struct Change {
    pub id: Uuid,
    pub at: DateTime<Utc>,
    /// Tenant key — every audit read filters by it; cross-tenant reads are
    /// impossible by construction.
    pub org: String,
    /// Site scope when the mutated entity is site-scoped; `None` for org-level.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub site_id: Option<Uuid>,
    pub actor: Actor,
    /// Resource kind token — `"dashboard"`, `"point"`, `"rule"`, … Matches a
    /// registered reverser.
    pub kind: String,
    pub resource_id: Uuid,
    pub op: Op,
    /// Full snapshot before the mutation (`None` for `Create`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub before: Option<Value>,
    /// Full snapshot after the mutation (`None` for `Delete`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub after: Option<Value>,
    /// Groups multi-row mutations into one undo step.
    pub group_id: Uuid,
    /// Request id / agent run id for tracing.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation: Option<String>,
}

/// Why a change is malformed. A change whose snapshots disagree with its op would
/// make undo/redo ambiguous, so the recorder rejects it before persistence.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ChangeError {
    #[error("a {op} change must carry {field}")]
    MissingSnapshot { op: &'static str, field: &'static str },
    #[error("a {op} change must not carry {field}")]
    UnexpectedSnapshot { op: &'static str, field: &'static str },
    #[error("change kind must be non-empty")]
    EmptyKind,
}

impl Change {
    /// A `Create`: `after` is the new row, `before` is absent. Caller supplies a
    /// fresh `group_id` (or shares one to group with sibling rows).
    #[allow(clippy::too_many_arguments)]
    pub fn create(
        id: Uuid,
        at: DateTime<Utc>,
        org: impl Into<String>,
        site_id: Option<Uuid>,
        actor: Actor,
        kind: impl Into<String>,
        resource_id: Uuid,
        after: Value,
        group_id: Uuid,
        correlation: Option<String>,
    ) -> Self {
        Self {
            id,
            at,
            org: org.into(),
            site_id,
            actor,
            kind: kind.into(),
            resource_id,
            op: Op::Create,
            before: None,
            after: Some(after),
            group_id,
            correlation,
        }
    }

    /// An `Update`: both snapshots present.
    #[allow(clippy::too_many_arguments)]
    pub fn update(
        id: Uuid,
        at: DateTime<Utc>,
        org: impl Into<String>,
        site_id: Option<Uuid>,
        actor: Actor,
        kind: impl Into<String>,
        resource_id: Uuid,
        before: Value,
        after: Value,
        group_id: Uuid,
        correlation: Option<String>,
    ) -> Self {
        Self {
            id,
            at,
            org: org.into(),
            site_id,
            actor,
            kind: kind.into(),
            resource_id,
            op: Op::Update,
            before: Some(before),
            after: Some(after),
            group_id,
            correlation,
        }
    }

    /// A `Delete`: `before` is the removed row, `after` is absent.
    #[allow(clippy::too_many_arguments)]
    pub fn delete(
        id: Uuid,
        at: DateTime<Utc>,
        org: impl Into<String>,
        site_id: Option<Uuid>,
        actor: Actor,
        kind: impl Into<String>,
        resource_id: Uuid,
        before: Value,
        group_id: Uuid,
        correlation: Option<String>,
    ) -> Self {
        Self {
            id,
            at,
            org: org.into(),
            site_id,
            actor,
            kind: kind.into(),
            resource_id,
            op: Op::Delete,
            before: Some(before),
            after: None,
            group_id,
            correlation,
        }
    }

    /// Reject a row whose snapshots disagree with its op — the rule undo/redo
    /// depends on. The recorder calls this before persistence so a malformed row
    /// never lands. (Recording an update outside the mutation's transaction makes
    /// the pre-read return nothing, yielding a `before`-less update — exactly what
    /// the coverage guard catches; this is the type-level half of that guard.)
    pub fn validate(&self) -> Result<(), ChangeError> {
        if self.kind.trim().is_empty() {
            return Err(ChangeError::EmptyKind);
        }
        match self.op {
            Op::Create => {
                if self.after.is_none() {
                    return Err(ChangeError::MissingSnapshot { op: "create", field: "after" });
                }
                if self.before.is_some() {
                    return Err(ChangeError::UnexpectedSnapshot { op: "create", field: "before" });
                }
            }
            Op::Update => {
                if self.before.is_none() {
                    return Err(ChangeError::MissingSnapshot { op: "update", field: "before" });
                }
                if self.after.is_none() {
                    return Err(ChangeError::MissingSnapshot { op: "update", field: "after" });
                }
            }
            Op::Delete => {
                if self.before.is_none() {
                    return Err(ChangeError::MissingSnapshot { op: "delete", field: "before" });
                }
                if self.after.is_some() {
                    return Err(ChangeError::UnexpectedSnapshot { op: "delete", field: "after" });
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap() -> Value {
        serde_json::json!({"id": "x", "title": "T"})
    }

    #[test]
    fn op_token_round_trips_and_fails_closed() {
        for op in [Op::Create, Op::Update, Op::Delete] {
            assert_eq!(Op::parse(op.as_str()), Some(op));
        }
        assert!(Op::parse("merge").is_none());
    }

    #[test]
    fn actor_tagged_union_round_trips() {
        let a = Actor::Agent { run_id: Uuid::new_v4(), model: "claude".into() };
        let json = serde_json::to_value(&a).unwrap();
        let back: Actor = serde_json::from_value(json).unwrap();
        assert_eq!(back, a);
        assert_eq!(Actor::System.cursor_subject(), "@system");
        assert_eq!(
            Actor::User { subject: "sub-1".into() }.cursor_subject(),
            "sub-1"
        );
    }

    #[test]
    fn constructors_keep_snapshots_consistent_with_op() {
        let g = Uuid::new_v4();
        let c = Change::create(
            Uuid::new_v4(),
            Utc::now(),
            "kfc",
            None,
            Actor::System,
            "dashboard",
            Uuid::new_v4(),
            snap(),
            g,
            None,
        );
        assert!(c.validate().is_ok());
        assert!(c.before.is_none() && c.after.is_some());

        let u = Change::update(
            Uuid::new_v4(),
            Utc::now(),
            "kfc",
            None,
            Actor::System,
            "dashboard",
            Uuid::new_v4(),
            snap(),
            snap(),
            g,
            None,
        );
        assert!(u.validate().is_ok());

        let d = Change::delete(
            Uuid::new_v4(),
            Utc::now(),
            "kfc",
            None,
            Actor::System,
            "dashboard",
            Uuid::new_v4(),
            snap(),
            g,
            None,
        );
        assert!(d.validate().is_ok());
        assert!(d.after.is_none());
    }

    #[test]
    fn update_without_before_is_rejected() {
        // The classic "recorded outside the transaction" mistake: the pre-read
        // returned nothing, so `before` is null on an update.
        let mut u = Change::update(
            Uuid::new_v4(),
            Utc::now(),
            "kfc",
            None,
            Actor::System,
            "dashboard",
            Uuid::new_v4(),
            snap(),
            snap(),
            Uuid::new_v4(),
            None,
        );
        u.before = None;
        assert_eq!(
            u.validate(),
            Err(ChangeError::MissingSnapshot { op: "update", field: "before" })
        );
    }

    #[test]
    fn empty_kind_is_rejected() {
        let mut c = Change::create(
            Uuid::new_v4(),
            Utc::now(),
            "kfc",
            None,
            Actor::System,
            "  ",
            Uuid::new_v4(),
            snap(),
            Uuid::new_v4(),
            None,
        );
        assert_eq!(c.validate(), Err(ChangeError::EmptyKind));
        c.kind = "dashboard".into();
        assert!(c.validate().is_ok());
    }
}
