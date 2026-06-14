//! Reject any statement that is not a single read-only query.
//!
//! The unified query surface is read-only by contract: it exposes the canonical
//! tables for `SELECT`/`WITH` only (`rubix/docs/sessions/WS-09.md`). A principal
//! cannot mutate through the query path — writes cross the gate as a `Command`
//! (`rubix/STACK-DEISGN.md`, contract #1), never DataFusion. This guard runs
//! before planning: it accepts a single `SELECT` or `WITH` statement and refuses
//! everything else (writes, DDL, or several statements chained with `;`), so an
//! injected second statement cannot ride in behind a leading `SELECT`.

use datafusion::sql::parser::{DFParser, Statement};
use datafusion::sql::sqlparser::ast::Statement as SqlStatement;
use datafusion::sql::sqlparser::dialect::GenericDialect;

use crate::error::{QueryError, Result};

/// Ensure `sql` is exactly one read-only statement.
///
/// Parses `sql` with DataFusion's own SQL parser — the same parser that will
/// plan the query — so the guard sees precisely what the engine would execute,
/// never a divergent lexer's view. A `SELECT` (optionally with a leading `WITH`
/// CTE) passes; a write, DDL, an empty input, or more than one statement is
/// rejected. Returning the parsed statement count to the caller is deliberate:
/// the guard is the single place statement shape is decided.
///
/// # Errors
/// Returns [`QueryError::Rejected`] if the input does not parse, is empty,
/// contains more than one statement, or is not a `SELECT`/`WITH` query.
pub fn ensure_read_only(sql: &str) -> Result<()> {
    let statements = DFParser::parse_sql_with_dialect(sql, &GenericDialect {})
        .map_err(|e| QueryError::Rejected(format!("could not parse SQL: {e}")))?;

    let mut iter = statements.into_iter();
    let first = iter
        .next()
        .ok_or_else(|| QueryError::Rejected("no statement to run".to_owned()))?;
    if iter.next().is_some() {
        return Err(QueryError::Rejected(
            "only a single statement may be submitted".to_owned(),
        ));
    }

    match first {
        Statement::Statement(boxed) => match *boxed {
            SqlStatement::Query(_) => Ok(()),
            other => Err(QueryError::Rejected(format!(
                "only SELECT/WITH queries are allowed, got: {}",
                statement_kind(&other)
            ))),
        },
        // Every non-`Statement` arm of the DataFusion parser (CREATE EXTERNAL
        // TABLE, COPY, EXPLAIN, SET, RESET, …) is a non-read statement.
        _ => Err(QueryError::Rejected(
            "only SELECT/WITH queries are allowed".to_owned(),
        )),
    }
}

/// A short, human-readable kind name for a rejected statement.
///
/// Kept terse on purpose: the message names what was refused without echoing the
/// principal's full statement back into the error.
fn statement_kind(statement: &SqlStatement) -> &'static str {
    match statement {
        SqlStatement::Insert(_) => "INSERT",
        SqlStatement::Update { .. } => "UPDATE",
        SqlStatement::Delete(_) => "DELETE",
        SqlStatement::CreateTable(_) => "CREATE TABLE",
        SqlStatement::Drop { .. } => "DROP",
        SqlStatement::Truncate { .. } => "TRUNCATE",
        SqlStatement::AlterTable { .. } => "ALTER TABLE",
        _ => "a non-read statement",
    }
}

#[cfg(test)]
mod tests {
    use super::ensure_read_only;

    #[test]
    fn accepts_a_plain_select() {
        ensure_read_only("SELECT id, content FROM record").unwrap();
    }

    #[test]
    fn accepts_a_with_cte() {
        ensure_read_only(
            "WITH recent AS (SELECT * FROM record) SELECT count(*) FROM recent",
        )
        .unwrap();
    }

    #[test]
    fn rejects_insert() {
        let err = ensure_read_only("INSERT INTO record (id) VALUES ('x')").unwrap_err();
        assert!(err.to_string().contains("INSERT"), "{err}");
    }

    #[test]
    fn rejects_update() {
        assert!(ensure_read_only("UPDATE record SET content = '{}'").is_err());
    }

    #[test]
    fn rejects_delete() {
        assert!(ensure_read_only("DELETE FROM record WHERE id = 'x'").is_err());
    }

    #[test]
    fn rejects_a_trailing_second_statement() {
        let err = ensure_read_only("SELECT * FROM record; DROP TABLE record").unwrap_err();
        assert!(err.to_string().contains("single statement"), "{err}");
    }

    #[test]
    fn rejects_empty_input() {
        assert!(ensure_read_only("   ").is_err());
        assert!(ensure_read_only("").is_err());
    }

    #[test]
    fn rejects_unparseable_input() {
        assert!(ensure_read_only("SELEKT oops FROM").is_err());
    }
}
