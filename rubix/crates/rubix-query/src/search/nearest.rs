//! k-nearest-neighbour search over a SurrealDB vector column.
//!
//! Vectors live beside the same records the rules and dashboards use, so semantic
//! search needs no separate store (`rubix/docs/SCOPE.md`, principle 6). Contract
//! #6 keeps SurrealQL first: the nearest-neighbour search runs in SurrealDB on
//! the principal's scoped session — SurrealDB owns the vector data and the
//! distance functions — and DataFusion is not involved. The query computes the
//! Euclidean distance from a probe vector to each record's vector column,
//! orders ascending, and returns the closest `k`. Because it runs on the scoped
//! session, SurrealDB row-level permissions bound which records can match
//! (contract #1).

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{QueryError, Result};

/// One nearest-neighbour hit: the record id and its distance from the probe.
#[derive(Debug, Clone, PartialEq)]
pub struct Neighbour {
    /// The matched record's id (`table:key`).
    pub id: String,
    /// The Euclidean distance from the probe vector (smaller is nearer).
    pub distance: f64,
}

/// Find the `k` records in `table` whose `vector_field` is nearest `probe`.
///
/// Runs on `session` (a gate-issued scoped connection) so only records the
/// principal may read are candidates. Distance is Euclidean over the vector
/// column; hits come back nearest-first. A `k` of zero, an empty `probe`, or an
/// empty table yields no hits. Rows whose `vector_field` is absent or the wrong
/// dimension are excluded by SurrealDB's distance function rather than matched at
/// a bogus distance.
///
/// # Errors
/// Returns [`QueryError::Scan`] if the SurrealDB query fails, or
/// [`QueryError::Rejected`] if `table` is not a bare identifier or
/// `vector_field` is not a dotted identifier path (guarding the interpolated
/// identifiers against injection).
pub async fn nearest(
    session: &Surreal<Db>,
    table: &str,
    vector_field: &str,
    probe: &[f64],
    k: usize,
) -> Result<Vec<Neighbour>> {
    if k == 0 || probe.is_empty() {
        return Ok(Vec::new());
    }
    let table = ensure_identifier(table)?;
    let field = ensure_field_path(vector_field)?;

    // The field/table are validated identifiers; the probe and limit bind as
    // parameters so no value is interpolated into the statement text. The
    // distance is projected as a named field so `ORDER BY` sorts on the alias —
    // SurrealDB's `ORDER BY` takes a field/idiom, not a bare function call.
    let sql = format!(
        "SELECT record::id(id) AS id, vector::distance::euclidean({field}, $probe) AS distance \
         FROM {table} \
         WHERE {field} IS NOT NONE \
         ORDER BY distance ASC \
         LIMIT $k"
    );
    let mut response = session
        .query(sql)
        .bind(("probe", probe.to_vec()))
        .bind(("k", k as i64))
        .await
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    let hits: Vec<serde_json::Value> = response
        .take(0)
        .map_err(|e| QueryError::Scan(e.to_string()))?;
    Ok(hits.iter().filter_map(neighbour_of).collect())
}

/// Project one hit row into a [`Neighbour`], skipping a row that lacks either
/// field (the projection always supplies both, so this only drops a malformed
/// engine response rather than silently fabricating a zero distance).
fn neighbour_of(row: &serde_json::Value) -> Option<Neighbour> {
    let id = row.get("id")?.as_str()?.to_owned();
    let distance = row.get("distance")?.as_f64()?;
    Some(Neighbour { id, distance })
}

/// Validate that `name` is a bare SurrealQL identifier safe to interpolate.
///
/// Table names cannot bind as query parameters, so they are validated to
/// `[A-Za-z_][A-Za-z0-9_]*` and otherwise rejected — closing that injection
/// surface.
fn ensure_identifier(name: &str) -> Result<&str> {
    if is_identifier(name) {
        Ok(name)
    } else {
        Err(QueryError::Rejected(format!(
            "not a valid identifier: {name:?}"
        )))
    }
}

/// Validate that `path` is a dotted identifier path (e.g. `content.embedding`).
///
/// A vector may live at the top level or inside a record's free-form `content`
/// (`rubix/docs/SCOPE.md`, principle 4), so the field is a dotted path. Each
/// segment is validated as a bare identifier, rejecting any path that could
/// inject SurrealQL through the interpolated field.
fn ensure_field_path(path: &str) -> Result<&str> {
    if !path.is_empty() && path.split('.').all(is_identifier) {
        Ok(path)
    } else {
        Err(QueryError::Rejected(format!(
            "not a valid field path: {path:?}"
        )))
    }
}

/// Whether `name` is a bare `[A-Za-z_][A-Za-z0-9_]*` identifier.
fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let valid_start = chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_');
    valid_start && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

#[cfg(test)]
mod tests {
    use super::{ensure_field_path, ensure_identifier};

    #[test]
    fn accepts_a_bare_identifier() {
        assert!(ensure_identifier("embedding").is_ok());
        assert!(ensure_identifier("_vec1").is_ok());
        assert!(ensure_identifier("record").is_ok());
    }

    #[test]
    fn rejects_an_identifier_with_punctuation() {
        assert!(ensure_identifier("embedding; DROP").is_err());
        assert!(ensure_identifier("a.b").is_err());
        assert!(ensure_identifier("").is_err());
        assert!(ensure_identifier("1bad").is_err());
    }

    #[test]
    fn accepts_a_dotted_field_path() {
        assert!(ensure_field_path("embedding").is_ok());
        assert!(ensure_field_path("content.embedding").is_ok());
        assert!(ensure_field_path("content.vec.value").is_ok());
    }

    #[test]
    fn rejects_an_injected_field_path() {
        assert!(ensure_field_path("content.embedding; DROP TABLE record").is_err());
        assert!(ensure_field_path("content..embedding").is_err());
        assert!(ensure_field_path(".embedding").is_err());
        assert!(ensure_field_path("").is_err());
    }
}
