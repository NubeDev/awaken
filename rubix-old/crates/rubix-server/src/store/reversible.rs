//! The reverser registry (docs/design/audit-and-undo.md "The reverser"): the one
//! extension point for undo/redo. Per resource kind, a [`Reversible`] says how to
//! undo (`apply_inverse`) and redo (`apply_forward`) a [`Change`]. Because every
//! row carries a full snapshot, the inverse/forward are mechanical:
//!
//! - undo `Create` → delete `resource_id`; undo `Delete` → re-insert `before`;
//!   undo `Update` → write `before` back.
//! - redo is the forward of each (insert `after`, delete, write `after`).
//!
//! A [`SnapshotReverser`] makes that mechanical default reusable: a kind that maps
//! cleanly onto a model + store create/update/delete adopts undo/redo by declaring
//! a [`SnapshotKind`] — "register one reverser", never a new feature. A kind only
//! needs a bespoke `Reversible` when undo has a side effect a snapshot cannot
//! express (e.g. a site cascade; WS-08 wires that path).

use std::collections::BTreeMap;

use rubix_core::{Change, Op};

use super::{Result, Store, StoreError};

/// How to undo/redo one resource kind. The registry holds one boxed impl per
/// `kind()`; the grouped dispatch routes each change row to its kind's reverser.
pub trait Reversible: Send + Sync {
    /// The resource-kind token this reverser owns (matches `Change::kind`).
    fn kind(&self) -> &'static str;
    /// Undo: revert the entity to its `before` state (or remove it for a Create).
    fn apply_inverse(&self, store: &Store, change: &Change) -> Result<()>;
    /// Redo: re-apply the change's forward effect (insert `after`, delete, or
    /// write `after`).
    fn apply_forward(&self, store: &Store, change: &Change) -> Result<()>;
}

/// A resource kind whose undo/redo is fully captured by snapshots: a model that
/// (de)serializes from `before`/`after`, plus the three store verbs. Implementors
/// get a free [`Reversible`] via [`SnapshotReverser`].
pub trait SnapshotKind: Send + Sync + 'static {
    /// The kind token.
    const KIND: &'static str;
    /// The owned model the snapshot deserializes into.
    type Model: serde::de::DeserializeOwned;

    /// Re-insert a removed/created row (undo Delete, redo Create).
    fn insert(store: &Store, model: &Self::Model) -> Result<()>;
    /// Write a snapshot back over the live row (undo/redo Update).
    fn replace(store: &Store, model: &Self::Model) -> Result<()>;
    /// Remove the row (undo Create, redo Delete). Keyed by the change's
    /// `resource_id`, supplied by the dispatcher.
    fn remove(store: &Store, model: &Self::Model) -> Result<()>;
}

/// Bridges a [`SnapshotKind`] to [`Reversible`] with the mechanical inverse/forward
/// rules. Stateless; one instance per kind in the registry.
pub struct SnapshotReverser<K: SnapshotKind>(std::marker::PhantomData<K>);

impl<K: SnapshotKind> Default for SnapshotReverser<K> {
    fn default() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<K: SnapshotKind> SnapshotReverser<K> {
    fn decode(snapshot: Option<&serde_json::Value>, what: &'static str) -> Result<K::Model> {
        let value = snapshot.ok_or_else(|| {
            StoreError::Invalid(format!("{} change for {} has no {what} snapshot", K::KIND, what))
        })?;
        serde_json::from_value(value.clone())
            .map_err(|e| StoreError::Db(anyhow::anyhow!("decode {} {what} snapshot: {e}", K::KIND)))
    }
}

impl<K: SnapshotKind> Reversible for SnapshotReverser<K> {
    fn kind(&self) -> &'static str {
        K::KIND
    }

    fn apply_inverse(&self, store: &Store, change: &Change) -> Result<()> {
        match change.op {
            // Created → remove it.
            Op::Create => K::remove(store, &Self::decode(change.after.as_ref(), "after")?),
            // Deleted → re-insert the prior row.
            Op::Delete => K::insert(store, &Self::decode(change.before.as_ref(), "before")?),
            // Updated → write the prior snapshot back.
            Op::Update => K::replace(store, &Self::decode(change.before.as_ref(), "before")?),
        }
    }

    fn apply_forward(&self, store: &Store, change: &Change) -> Result<()> {
        match change.op {
            // Re-create from the after snapshot.
            Op::Create => K::insert(store, &Self::decode(change.after.as_ref(), "after")?),
            // Re-delete.
            Op::Delete => K::remove(store, &Self::decode(change.before.as_ref(), "before")?),
            // Re-apply the new snapshot.
            Op::Update => K::replace(store, &Self::decode(change.after.as_ref(), "after")?),
        }
    }
}

/// The set of reversers, keyed by kind. Built once (it is dependency-free and
/// cheap) and consulted by the grouped undo/redo dispatch.
pub struct ReverserRegistry {
    by_kind: BTreeMap<&'static str, Box<dyn Reversible>>,
}

impl ReverserRegistry {
    /// The registry every undo/redo dispatch uses. Each registered kind is a kind
    /// the coverage guard then requires a recording path for.
    pub fn new() -> Self {
        let mut by_kind: BTreeMap<&'static str, Box<dyn Reversible>> = BTreeMap::new();
        Self::register(&mut by_kind, kinds::DashboardKind::reverser());
        Self { by_kind }
    }

    fn register(
        map: &mut BTreeMap<&'static str, Box<dyn Reversible>>,
        reverser: Box<dyn Reversible>,
    ) {
        map.insert(reverser.kind(), reverser);
    }

    /// The reverser for a kind, or [`StoreError::Invalid`] when no kind is
    /// registered (undo of an unknown kind fails closed — it never silently
    /// no-ops).
    pub fn get(&self, kind: &str) -> Result<&dyn Reversible> {
        self.by_kind
            .get(kind)
            .map(|b| b.as_ref())
            .ok_or_else(|| StoreError::Invalid(format!("no reverser registered for kind `{kind}`")))
    }

    /// Every registered kind token (drives the coverage guard).
    pub fn kinds(&self) -> Vec<&'static str> {
        self.by_kind.keys().copied().collect()
    }
}

impl Default for ReverserRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Every reversible kind the build ships. The coverage guard enumerates this and
/// asserts each has a recording path (WS-08 wires the handlers).
pub fn registered_kinds() -> Vec<&'static str> {
    ReverserRegistry::new().kinds()
}

/// Undo a whole change group as one step (docs/design/audit-and-undo.md
/// "group_id groups a transaction"): apply each row's inverse. Rows arrive
/// newest-first, which is the correct inverse order (the last write is undone
/// first). Fails closed if any kind is unregistered.
pub fn apply_group_inverse(
    store: &Store,
    registry: &ReverserRegistry,
    changes: &[Change],
) -> Result<()> {
    for change in changes {
        registry.get(&change.kind)?.apply_inverse(store, change)?;
    }
    Ok(())
}

/// Redo a whole change group: apply each row's forward in the original order
/// (oldest-first), the reverse of the inverse pass.
pub fn apply_group_forward(
    store: &Store,
    registry: &ReverserRegistry,
    changes: &[Change],
) -> Result<()> {
    for change in changes.iter().rev() {
        registry.get(&change.kind)?.apply_forward(store, change)?;
    }
    Ok(())
}

/// The concrete snapshot kinds. Each is a thin map from a domain model onto the
/// store's create/update/delete; adding a kind here (plus its WS-08 recording
/// path) is the whole cost of making it undoable.
mod kinds {
    use rubix_core::Dashboard;

    use super::{Reversible, SnapshotKind, SnapshotReverser};
    use crate::store::{Result, Store};

    /// The dashboard kind — the design's proof that "for everything" rides the
    /// registry, not per-feature code. Snapshot is the full [`Dashboard`] row.
    pub struct DashboardKind;

    impl DashboardKind {
        pub fn reverser() -> Box<dyn Reversible> {
            Box::<SnapshotReverser<DashboardKind>>::default()
        }
    }

    impl SnapshotKind for DashboardKind {
        const KIND: &'static str = "dashboard";
        type Model = Dashboard;

        fn insert(store: &Store, model: &Dashboard) -> Result<()> {
            store.create_dashboard(model)
        }

        fn replace(store: &Store, model: &Dashboard) -> Result<()> {
            // The mutable metadata is title + variables; identity columns are
            // immutable, so writing the snapshot back is title/variables only.
            store
                .update_dashboard(model.id, Some(&model.title), Some(&model.variables))
                .map(|_| ())
        }

        fn remove(store: &Store, model: &Dashboard) -> Result<()> {
            store.delete_dashboard(model.id)
        }
    }
}
