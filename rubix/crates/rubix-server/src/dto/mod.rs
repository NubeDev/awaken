//! Transport DTOs — the wire shapes routes map domain types to and from.
//!
//! One file per resource (`rubix/docs/FILE-LAYOUT.md`): records, the query
//! surface, and datasources. DTOs are deliberately separate from the domain types
//! so the response layer (timestamps as strings, prefs-formatted) does not leak
//! engine shapes onto the wire.

pub mod admin;
pub mod agent;
pub mod auth;
pub mod datasource;
pub mod prefs;
pub mod query;
pub mod record;

pub use admin::{
    CreateDeviceRequest, CreatePrincipalRequest, CreateTenantRequest, CreatedPrincipalDto,
    DeviceDto, GrantDto, PrincipalDto, TenantDto, UpdateDeviceRequest, UpdatePrincipalRequest,
};
pub use agent::{
    AgentDto, AskRequest, AskResponse, PersistRequest, PersistedDto, ProvisionAgentRequest,
    ProvisionedAgentDto, RecallRequest, RecalledDto,
};
pub use auth::{LoginRequest, LoginResponse, MeResponse};
pub use datasource::{DatasourceDto, RegisterDatasourceRequest, UpdateDatasourceRequest};
pub use prefs::{PreferencesDto, UpdatePreferencesRequest};
pub use query::{
    BatchQueryItem, BatchQueryRequest, BatchQueryResponse, BatchQueryResult, ColumnDto, QueryRequest,
    QueryResponse, TimeBoundDto, TimeScopeDto,
};
pub use record::{CreateRecordRequest, RecordDto, UpdateRecordRequest};
