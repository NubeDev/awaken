//! Per-profile defaults, centralized so the boot path reads them once and
//! threads them through `AppState`. STACK-DEISGN.md "Single binary, two
//! profiles" — the feature gate decides what is *compilable*, this config
//! decides what an enabled profile *defaults to*.

use super::ProfileKind;

/// Which relational store a profile expects.
///
/// `Postgres` is the cloud target; its backend attaches behind this seam. It is
/// a real variant here only so the profile seam is typed; selecting it before
/// its backend exists is rejected at boot rather than silently downgraded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoreKind {
    /// On-box SQLite (rusqlite/r2d2). The only store backend that exists today.
    Sqlite,
    /// Cloud Postgres. Backend not yet present; boot fails closed if selected.
    Postgres,
}

/// Which history tier a profile defaults to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryTier {
    /// SQLite hot tier with an optional local Parquet cold tier
    /// (`RUBIX_HIS_PARQUET`). Today's edge behavior.
    LocalParquet,
    /// Object-store cold tier (cloud). The cross-tier `his` provider exists; a
    /// remote `object_store` backend is constructor-only today.
    ObjectStore,
}

/// The resolved profile config: per-profile defaults, read once at boot and
/// threaded into `AppState`. Later workstreams attach their backends to these
/// fields behind the same seam.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Profile {
    /// Which profile this node runs as.
    pub kind: ProfileKind,
    /// Relational store backend the profile expects.
    pub store: StoreKind,
    /// History tier the profile defaults to.
    pub history: HistoryTier,
    /// Whether the driver supervisor launches on this profile. Edge stations
    /// supervise on-box drivers; the cloud supervisor does not.
    pub supervise_drivers: bool,
    /// Whether the profile requires authenticated requests. Auth middleware
    /// attaches behind this seam; the cloud profile sets this so that seam is
    /// wired, the edge profile leaves it off (today's behavior).
    pub auth_required: bool,
}

impl Profile {
    /// The defaults for a given profile kind.
    pub fn defaults(kind: ProfileKind) -> Self {
        match kind {
            ProfileKind::Edge => Profile {
                kind,
                store: StoreKind::Sqlite,
                history: HistoryTier::LocalParquet,
                supervise_drivers: true,
                auth_required: false,
            },
            ProfileKind::Cloud => Profile {
                kind,
                store: StoreKind::Postgres,
                history: HistoryTier::ObjectStore,
                supervise_drivers: false,
                auth_required: true,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_defaults_preserve_todays_behavior() {
        let p = Profile::defaults(ProfileKind::Edge);
        assert_eq!(p.store, StoreKind::Sqlite);
        assert_eq!(p.history, HistoryTier::LocalParquet);
        assert!(p.supervise_drivers);
        assert!(!p.auth_required);
    }

    #[test]
    fn cloud_defaults_select_cloud_backends() {
        let p = Profile::defaults(ProfileKind::Cloud);
        assert_eq!(p.store, StoreKind::Postgres);
        assert_eq!(p.history, HistoryTier::ObjectStore);
        assert!(!p.supervise_drivers);
        assert!(p.auth_required);
    }
}
