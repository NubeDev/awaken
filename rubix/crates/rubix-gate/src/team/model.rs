//! The domain shapes for teams and memberships.
//!
//! A [`Team`] is a named group of principals within a namespace; a
//! [`Membership`] is the row that links one principal to one team. Both are
//! gate-owned identity primitives (`rubix/docs/SCOPE.md`, principle 5): a team
//! is the unit an admin grants access to, and membership is what makes a grant
//! to a team flow to its members. Teams carry no secret and never widen the data
//! scope by themselves — they are a grouping the authz layers key off.

/// A team: a named group of principals scoped to a namespace.
///
/// `slug` is the team's stable, API-local key (unique within the namespace); the
/// store key is `{namespace}:{slug}`. `display_name` is the human label shown in
/// the UI. A team has no role of its own — authority still comes from the grants
/// attached to it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Team {
    /// The team's stable key within its namespace (e.g. `engineers`).
    pub slug: String,
    /// The namespace (tenant) the team belongs to.
    pub namespace: String,
    /// A human-readable label for the team.
    pub display_name: String,
}

impl Team {
    /// Build a team in `namespace` with `slug` and `display_name`.
    #[must_use]
    pub fn new(
        slug: impl Into<String>,
        namespace: impl Into<String>,
        display_name: impl Into<String>,
    ) -> Self {
        Self {
            slug: slug.into(),
            namespace: namespace.into(),
            display_name: display_name.into(),
        }
    }
}

/// A membership: one principal's place in one team, within a namespace.
///
/// The link is identified by `(namespace, team_slug, subject)`, so a principal
/// joins a team at most once. `subject` is the principal's **full** storage
/// subject (the `{namespace}_{local}` key), matching how grants and principals
/// are keyed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Membership {
    /// The namespace the membership is confined to.
    pub namespace: String,
    /// The slug of the team the principal belongs to.
    pub team_slug: String,
    /// The full subject of the member principal.
    pub subject: String,
}

impl Membership {
    /// Build a membership of `subject` in `team_slug` within `namespace`.
    #[must_use]
    pub fn new(
        namespace: impl Into<String>,
        team_slug: impl Into<String>,
        subject: impl Into<String>,
    ) -> Self {
        Self {
            namespace: namespace.into(),
            team_slug: team_slug.into(),
            subject: subject.into(),
        }
    }
}
