//! rubix-blob — the binary blob boundary beside the record store.
//!
//! File fields are the one genuinely new subsystem the collection design adds
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "File fields"): nothing else in the
//! tree stores bytes. A collection's `file` field holds a **reference**
//! ([`FileRef`]) — an id, filename, size, and content type — never the bytes. The
//! bytes live here, behind a pluggable [`BlobStore`].
//!
//! ## Two chokepoints preserved
//!
//! Upload and record-write stay two steps: a client uploads bytes to the blob
//! store (`POST /files`, gated on `FileUpload`) and gets back a [`FileRef`], then
//! stores that reference in a record's content through the **normal gated write**.
//! So the command gate still sees only JSON — blob bytes never flow through it,
//! exactly as the design requires.
//!
//! ## Namespace ownership
//!
//! A blob is owned by a namespace. The store keys every blob by
//! `(namespace, blob_id)`, so the download route enforces tenant isolation by
//! asking only for blobs under the caller's own namespace — a caller cannot read
//! another tenant's bytes even with a guessed id.
//!
//! ## Backends
//!
//! [`LocalFsBlobStore`] is the default, matching the embedded-engine edge profile.
//! A cloud object-store backend sits behind the `cloud` feature; until it is built,
//! requesting it fails closed ([`BlobError::BackendUnavailable`]) rather than
//! silently degrading — the same fail-closed pattern as the Postgres connector
//! (`BACKEND-COLLECTIONS.md`, open question 8: object-store backend, GC, and
//! blob↔cloud sync are the remaining deferred file work).

mod error;
mod local;
mod reference;
mod store;

pub use error::BlobError;
pub use local::LocalFsBlobStore;
pub use reference::FileRef;
pub use store::{BlobStore, Loaded};
