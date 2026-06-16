//! The persisted shapes of teams and memberships at the SurrealDB boundary.
//!
//! Teams live in the `team` table, memberships in the `membership` table. Like
//! grants, these are app-enforced records (not the generic `record` table and
//! not governed by a record permission), so the gate writes and reads them on
//! the store handle and does the authority/scope checks itself. Both keys are
//! deterministic so create is idempotent and delete targets a single key
//! without a scan: a team is keyed `{namespace}:{slug}`, a membership
//! `{namespace}:{slug}:{subject}`.

use surrealdb::types::{RecordId, SurrealValue};

use super::model::{Membership, Team};

/// The table teams are stored in.
pub(crate) const TEAM_TABLE: &str = "team";
/// The table memberships are stored in.
pub(crate) const MEMBERSHIP_TABLE: &str = "membership";

/// SurrealDB-facing team: the reserved `id` thing plus the team fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct TeamRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) slug: String,
    pub(crate) display_name: String,
}

impl TeamRow {
    /// Project a domain [`Team`] into its persisted row.
    pub(crate) fn from_team(team: &Team) -> Self {
        Self {
            id: RecordId::new(TEAM_TABLE, team_key(&team.namespace, &team.slug)),
            namespace: team.namespace.clone(),
            slug: team.slug.clone(),
            display_name: team.display_name.clone(),
        }
    }

    /// Reconstruct the domain [`Team`].
    pub(crate) fn into_team(self) -> Team {
        Team {
            slug: self.slug,
            namespace: self.namespace,
            display_name: self.display_name,
        }
    }
}

/// SurrealDB-facing membership: the reserved `id` thing plus the link fields.
#[derive(Debug, Clone, PartialEq, SurrealValue)]
pub(crate) struct MembershipRow {
    pub(crate) id: RecordId,
    pub(crate) namespace: String,
    pub(crate) team_slug: String,
    pub(crate) subject: String,
}

impl MembershipRow {
    /// Project a domain [`Membership`] into its persisted row.
    pub(crate) fn from_membership(membership: &Membership) -> Self {
        Self {
            id: RecordId::new(
                MEMBERSHIP_TABLE,
                membership_key(
                    &membership.namespace,
                    &membership.team_slug,
                    &membership.subject,
                ),
            ),
            namespace: membership.namespace.clone(),
            team_slug: membership.team_slug.clone(),
            subject: membership.subject.clone(),
        }
    }
}

/// The deterministic record key for a team: `namespace:slug`.
pub(crate) fn team_key(namespace: &str, slug: &str) -> String {
    format!("{namespace}:{slug}")
}

/// The deterministic record key for a membership: `namespace:slug:subject`.
pub(crate) fn membership_key(namespace: &str, slug: &str, subject: &str) -> String {
    format!("{namespace}:{slug}:{subject}")
}
