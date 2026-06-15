//! Transport DTOs — the wire shapes routes map domain types to and from.
//!
//! One file per resource (`rubix/docs/FILE-LAYOUT.md`): records, the query
//! surface, and datasources. DTOs are deliberately separate from the domain types
//! so the response layer (timestamps as strings, prefs-formatted) does not leak
//! engine shapes onto the wire.

pub mod auth;
pub mod datasource;
pub mod query;
pub mod record;

pub use auth::{LoginRequest, LoginResponse, MeResponse};
pub use datasource::{DatasourceDto, RegisterDatasourceRequest, UpdateDatasourceRequest};
pub use query::{QueryRequest, QueryResponse};
pub use record::{CreateRecordRequest, RecordDto, UpdateRecordRequest};
