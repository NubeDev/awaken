//! A user's administrative tier, persisted on the `users` row. Orthogonal to the
//! token [`Role`](super::Role): `admin_level` is a property of the *identity*
//! (who you are), while a token's role is a property of the *credential*. At
//! verify time the level is folded into the resolved principal's role —
//! `super_admin` → global [`Role::Admin`], `org_admin` → org-scoped
//! [`Role::Admin`] — so the gate keeps a single role ladder to reason about.
//!
//! See `docs/design/authz-rbac.md` (admin tiers).

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The three admin tiers. `None` is an ordinary member (access by scope-role +
/// grants only); `OrgAdmin` manages one org; `SuperAdmin` manages everything.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AdminLevel {
    /// Ordinary member: no management capability beyond scope-role + grants.
    #[default]
    None,
    /// Org-admin: manages users/teams/grants within their home org.
    OrgAdmin,
    /// Super-admin: manages every org, user, team, and grant globally.
    SuperAdmin,
}

impl AdminLevel {
    /// Parse the canonical token; unknown levels fail closed to [`AdminLevel::None`].
    pub fn parse(s: &str) -> Self {
        match s {
            "org_admin" => AdminLevel::OrgAdmin,
            "super_admin" => AdminLevel::SuperAdmin,
            // `none` and anything unrecognized are a plain member: fail closed
            // (no accidental elevation from a malformed column).
            _ => AdminLevel::None,
        }
    }

    /// The canonical lowercase token (the stored form).
    pub fn as_str(self) -> &'static str {
        match self {
            AdminLevel::None => "none",
            AdminLevel::OrgAdmin => "org_admin",
            AdminLevel::SuperAdmin => "super_admin",
        }
    }

    /// True when this level confers any admin capability (org or super).
    pub fn is_admin(self) -> bool {
        !matches!(self, AdminLevel::None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_fails_closed() {
        for lvl in [AdminLevel::None, AdminLevel::OrgAdmin, AdminLevel::SuperAdmin] {
            assert_eq!(AdminLevel::parse(lvl.as_str()), lvl);
        }
        assert_eq!(AdminLevel::parse("root"), AdminLevel::None);
        assert_eq!(AdminLevel::parse(""), AdminLevel::None);
        assert!(!AdminLevel::None.is_admin());
        assert!(AdminLevel::OrgAdmin.is_admin());
        assert!(AdminLevel::SuperAdmin.is_admin());
    }
}
