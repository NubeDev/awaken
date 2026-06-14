//! The data-change event delivered by the live-query plane.
//!
//! The data-change plane is SurrealDB live queries: "a record appeared/changed"
//! pushed to a subscriber (`rubix/docs/SCOPE.md`, "Event bus" — data-change). A
//! [`DataChange`] carries the change kind and the record it concerns. The record
//! is already filtered by the scoped session's row-level permissions, so a
//! subscriber only ever receives changes to records its principal may read
//! (contract #1, `rubix/STACK-DEISGN.md`).

use rubix_core::Record;

/// What happened to the record the [`DataChange`] carries.
///
/// Mirrors the SurrealDB live-query actions that denote a record change; the
/// stream maps the engine's `Action` onto this kind (the `Killed`/`Error`
/// actions are handled by the stream itself, not surfaced as a change).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataChangeKind {
    /// A record was created.
    Created,
    /// A record's content was updated.
    Updated,
    /// A record was deleted.
    Deleted,
}

/// A record change observed on the data-change plane.
#[derive(Debug, Clone, PartialEq)]
pub struct DataChange {
    kind: DataChangeKind,
    record: Record,
}

impl DataChange {
    /// Build a data-change event of `kind` carrying `record`.
    #[must_use]
    pub fn new(kind: DataChangeKind, record: Record) -> Self {
        Self { kind, record }
    }

    /// What happened to the record.
    #[must_use]
    pub fn kind(&self) -> DataChangeKind {
        self.kind
    }

    /// The record the change concerns.
    #[must_use]
    pub fn record(&self) -> &Record {
        &self.record
    }
}
