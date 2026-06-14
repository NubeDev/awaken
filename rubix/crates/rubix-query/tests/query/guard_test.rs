//! Integration: the statement guard rejects non-read SQL on the run path.
//!
//! The unified surface is read-only (`rubix/docs/sessions/WS-09.md`): a write,
//! DDL, or a second chained statement is refused before any scan runs, so a
//! mutation cannot ride in through the query plane (contract #1 keeps writes on
//! the gate). Asserted through the public `run` entry, not just the guard unit,
//! to prove the rejection happens end to end.

#[path = "../fixture/mod.rs"]
mod fixture;

use rubix_core::Role;
use rubix_query::{QueryError, run};

use fixture::open::{open_query_store, scoped_session_for};

#[tokio::test]
async fn a_write_statement_is_rejected_before_execution() {
    let database = "guard_write";
    let handle = open_query_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    for sql in [
        "INSERT INTO record (id) VALUES ('x')",
        "UPDATE record SET content = '{}'",
        "DELETE FROM record",
        "SELECT * FROM record; DROP TABLE record",
    ] {
        let err = run(session.connection(), sql)
            .await
            .expect_err("non-read statement must be rejected");
        assert!(
            matches!(err, QueryError::Rejected(_)),
            "expected a rejection for {sql:?}, got {err:?}"
        );
    }
}

#[tokio::test]
async fn a_select_is_accepted_by_the_guard() {
    let database = "guard_select";
    let handle = open_query_store(database).await;
    let (_principal, session) =
        scoped_session_for(&handle, database, "alice", "rubix", Role::Viewer).await;

    run(session.connection(), "SELECT id FROM record")
        .await
        .expect("a SELECT must pass the guard and run");
}
