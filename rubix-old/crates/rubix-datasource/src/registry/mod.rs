//! The registry that owns one pool per datasource and the only place a
//! decrypted credential lives.

mod run;

pub use run::DatasourceRegistry;
