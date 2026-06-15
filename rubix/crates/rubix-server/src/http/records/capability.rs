//! The capability a record mutation through the transport requires.
//!
//! Every record write crosses the WS-05 gate, which checks an app-enforced
//! capability grant before applying (`rubix/docs/SCOPE.md`, "Two authz layers").
//! The committed capability set (`rubix-gate::Capability`) has no dedicated
//! record-write variant; an HTTP record write *is* publishing data into the
//! platform, so it requires the [`IngestPublish`](rubix_gate::Capability::IngestPublish)
//! grant — the same capability the WS-12 ingest path is gated by. This is the one
//! place that choice is named, so create/update/delete cannot drift apart. The
//! WS-16 session log records this as a deliberate assumption pending a dedicated
//! record-write capability if the gate's enum gains one.

use rubix_gate::Capability;

/// The capability grant a record mutation requires.
pub(crate) const RECORD_WRITE: Capability = Capability::IngestPublish;
