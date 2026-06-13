//! Postgres backend bodies. One file per resource, mirroring the SQLite impls
//! in the parent module. STACK-DEISGN.md "Postgres (cloud), SQLite (edge)":
//! the cloud profile's relational store. Ids and timestamps are TEXT (shared
//! canonical-string / RFC 3339 codecs), so domain types round-trip identically
//! across both backends. Cloud-feature only.

use super::{Result, Store};

pub(super) mod boards;
pub(super) mod codec;
pub(super) mod command;
pub(super) mod dashboards;
pub(super) mod equips;
pub(super) mod grants;
pub(super) mod his;
pub(super) mod his_flush;
pub(super) mod keyexpr;
pub(super) mod points;
pub(super) mod rules;
pub(super) mod runs;
pub(super) mod sites;
pub(super) mod sparks;
pub(super) mod teams;
pub(super) mod tokens;
pub(super) mod users;
pub(super) mod widgets;

/// Wipe every table. Test-only: the shared store suite calls this to start each
/// Postgres pass from a clean slate (the SQLite pass uses a fresh temp file).
pub(super) fn truncate_all(store: &Store) -> Result<()> {
    store.postgres_conn()?.batch_execute(
        "TRUNCATE tokens, grants, memberships, teams, users, runs, widgets, his, sparks, boards, points, equips, sites RESTART IDENTITY CASCADE",
    )?;
    Ok(())
}
