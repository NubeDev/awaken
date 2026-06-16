//! Shared test fixture: an in-memory SurrealDB connection on a fresh database.
//!
//! Integration tests exercise the real SurrealQL the record/tag verbs emit, so
//! they need a live engine. kv-mem keeps each test isolated and fast
//! (`rubix/STACK-DEISGN.md`, "Key decisions": kv-mem for tests).

use surrealdb::Surreal;
use surrealdb::engine::local::{Db, Mem};

/// Open an in-memory engine, select a fresh namespace/database, and define the
/// record/tag/tagged tables.
///
/// `rubix-store::init_schema` defines these in a running node; rubix-core cannot
/// depend on the store crate (it sits below it), so the fixture mirrors the same
/// definitions to keep reads against an empty database returning no rows instead
/// of erroring on a missing table.
pub async fn open_memory_db() -> Surreal<Db> {
    let db = Surreal::new::<Mem>(())
        .await
        .expect("open in-memory engine");
    db.use_ns("rubix")
        .use_db("test")
        .await
        .expect("select namespace/database");
    db.query(
        "DEFINE TABLE IF NOT EXISTS record SCHEMALESS;\n\
         DEFINE TABLE IF NOT EXISTS tag SCHEMALESS;\n\
         DEFINE TABLE IF NOT EXISTS tagged TYPE RELATION SCHEMALESS;\n\
         DEFINE TABLE IF NOT EXISTS reading SCHEMALESS;\n\
         DEFINE FIELD IF NOT EXISTS series ON reading TYPE record;\n\
         DEFINE FIELD IF NOT EXISTS at ON reading TYPE datetime;\n\
         DEFINE FIELD IF NOT EXISTS value ON reading TYPE number;\n\
         DEFINE FIELD IF NOT EXISTS namespace ON reading TYPE string;\n\
         DEFINE FIELD IF NOT EXISTS created ON reading TYPE datetime DEFAULT time::now();\n\
         DEFINE INDEX IF NOT EXISTS reading_ns_series_at ON reading FIELDS namespace, series, at;\n\
         DEFINE INDEX IF NOT EXISTS reading_ns_at ON reading FIELDS namespace, at;",
    )
    .await
    .expect("define tables")
    .check()
    .expect("schema applied");
    db
}
