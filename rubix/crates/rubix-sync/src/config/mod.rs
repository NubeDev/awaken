//! The config-plane reconciler: ownership first, then LWW with audit tiebreak.
//!
//! Config (dashboards, rules, tags, datasource defs) is the only sync surface
//! that ever reconciles (`rubix/docs/SCOPE.md`, "Sync and conflict model"); the
//! data plane is conflict-free by partition ([`crate::data`]). Reconciliation is
//! orchestrated by [`reconcile`] in a fixed order: ownership ([`own`]) settles the
//! conflict outright wherever a scope names an owner — cloud owns shared/tenant
//! config, edge owns local-only — and only an unavoidable overlap falls back to
//! last-write-wins ([`last_write_wins`]), breaking a write-instant tie on the
//! WS-05 audit timestamp ([`tiebreak`]).

pub mod last_write_wins;
pub mod own;
pub mod reconcile;
pub mod tiebreak;

pub use last_write_wins::{ConfigVersion, LwwOutcome, last_write_wins};
pub use own::{ConfigScope, Owner, owner_of};
pub use reconcile::{reconcile, reconcile_ambiguous};
pub use tiebreak::break_tie;
