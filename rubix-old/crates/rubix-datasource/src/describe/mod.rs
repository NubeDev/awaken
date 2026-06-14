//! Schema discovery: operator-declared blob or live `information_schema`
//! introspection, both shaped into one [`crate::manifest::SchemaBlob`].

mod run;

pub use run::describe;
