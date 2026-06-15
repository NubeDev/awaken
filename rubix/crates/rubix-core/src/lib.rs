//! Domain model and shared contracts for the rubix platform.
//!
//! Scope authority: `rubix/docs/SCOPE.md`. Crate role and contracts:
//! `rubix/STACK-DEISGN.md` (`rubix-core` row + load-bearing contracts #3, #6).

mod collection;
mod configure;
mod correlate;
mod error;
mod id;
mod principal;
mod record;
mod tag;

pub use collection::{
    COLLECTION_KIND, CollectionDef, CollectionParseError, FieldDef, FieldFailure, FieldType,
    NAMESPACE_SETTINGS_KIND, ValidationError, bootstrap_meta_collection, find_collection,
    namespace_strict,
};
pub use configure::{Profile, RuntimeConfig, StoreEngine};
pub use correlate::CorrelationId;
pub use error::{Error, Result, ResultExt};
pub use id::Id;
pub use principal::{Principal, PrincipalKind, Role};
pub use record::{
    Record, create_record, decode_record, delete_record, list_records, read_record,
    update_record,
};
pub use tag::{
    Tag, attach_tag, create_tag, delete_tag, detach_tag, find_records_by_tags,
};
