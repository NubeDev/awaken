//! Collections — an opt-in typed contract over the schemaless record store.
//!
//! PocketBase's value is that an admin defines a *collection* (a named shape with
//! typed fields) at runtime and immediately gets typed CRUD, with zero backend
//! code (`rubix/docs/design/BACKEND-COLLECTIONS.md`). Rubix preserves SCOPE's
//! "the domain is not baked in" by making a collection a **record**, not a table:
//! a `kind: "collection"` record whose content declares the fields records of its
//! `name` must carry. This module owns the domain model of that contract — the
//! field types ([`FieldType`]), the parsed definition ([`CollectionDef`]), the
//! content validation it performs ([`ValidationError`]), and the loaders that
//! resolve a write's `kind` to its collection and read the per-namespace
//! strict-mode switch. The gate's command path invokes them; enforcement is
//! orchestrated there, on the one mutation chokepoint.
//!
//! Validation is **fail-open by default**: a record whose `kind` matches no
//! collection writes unconstrained, so today's unkinded records are unaffected
//! and a tenant adopts collections incrementally. Strict mode flips a namespace
//! fail-closed once its collections exist (open question 1).

mod bootstrap;
mod def;
mod field;
mod load;
mod validate;

pub use bootstrap::bootstrap_meta_collection;
pub use def::{COLLECTION_KIND, CollectionDef, CollectionParseError};
pub use field::{FieldDef, FieldType};
pub use load::{NAMESPACE_SETTINGS_KIND, find_collection, namespace_strict};
pub use validate::{FieldFailure, ValidationError};
