//! The SQL-execution seam and its sqlx Postgres implementation. Everything
//! above this module is unit-testable against a fake [`SqlBackend`].

mod postgres;
mod rows;
mod run;

pub use postgres::{PostgresBackend, PostgresConn};
pub use rows::{Column, ResultSet, Row};
pub use run::{RawResult, SqlBackend};
