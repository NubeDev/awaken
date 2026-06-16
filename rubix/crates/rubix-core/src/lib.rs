//! Domain model and shared contracts for the rubix platform.
//!
//! Scope authority: `rubix/docs/SCOPE.md`. Crate role and contracts:
//! `rubix/STACK-DEISGN.md` (`rubix-core` row + load-bearing contracts #3, #6).

mod collection;
mod configure;
mod correlate;
mod error;
mod hook;
mod id;
mod principal;
mod reading;
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
pub use hook::{HOOK_KIND, Hook, HookEvent, HookParseError, find_hooks};
pub use id::Id;
pub use principal::{Principal, PrincipalKind, Role};
pub use reading::{
    Reading, append_readings, list_readings, read_reading, read_readings_window, reading_id,
};
pub use record::{
    Record, RecordTags, create_record, decode_record, delete_record, list_record_tags,
    list_records, list_records_filtered, read_record, update_record,
};
pub use tag::{
    Tag, attach_tag, create_tag, delete_tag, detach_tag, find_records_by_tags,
};
