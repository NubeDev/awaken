//! Build the SurrealQL mutation a command applies, with `RETURN BEFORE`.
//!
//! The durable write boundary for a command's record change. Each [`Change`]
//! variant maps to one SurrealQL statement that mutates the generic `record`
//! table and returns the row's prior state in the same statement via
//! `RETURN BEFORE` — so the before-image is taken atomically with the write, no
//! separate read-before-write round trip (`rubix/docs/SCOPE.md`, "Audit log").
//! Execution and before/after decoding live in [`capture`](super::capture); this
//! file owns only the statement and its bound parameters.

use super::action::Change;

/// The table generic records live in (mirrors `rubix-core`'s `record` table).
pub(super) const RECORD_TABLE: &str = "record";

/// A SurrealQL mutation statement plus the content it writes (if any).
///
/// `statement` is bound against `$record` (the target thing), `$namespace`, and
/// — for create/update — `$content`. `writes` is the after-image the command
/// sets, used both as the `$content` binding and as the audit after-summary; a
/// delete writes nothing, so it is `None`.
pub(super) struct Mutation {
    pub(super) statement: String,
    pub(super) writes: Option<serde_json::Value>,
}

/// Build the `RETURN BEFORE` mutation statement for `change`.
///
/// `CREATE`/`UPDATE` carry the new content as `$content`; `DELETE` carries none.
/// Every statement projects `RETURN BEFORE`, so executing it yields the prior
/// row state atomically with the write.
pub(super) fn mutation_for(change: &Change) -> Mutation {
    match change {
        Change::Create(content) => Mutation {
            statement: "CREATE $record CONTENT \
                 { namespace: $namespace, content: $content, \
                   created: time::now(), updated: time::now() } \
                 RETURN BEFORE"
                .to_owned(),
            writes: Some(content.clone()),
        },
        Change::Update(content) => Mutation {
            statement: "UPDATE $record MERGE \
                 { content: $content, updated: time::now() } \
                 RETURN BEFORE"
                .to_owned(),
            writes: Some(content.clone()),
        },
        Change::Delete => Mutation {
            statement: "DELETE $record RETURN BEFORE".to_owned(),
            writes: None,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Change, mutation_for};

    #[test]
    fn every_statement_returns_the_before_image() {
        for change in [
            Change::Create(serde_json::json!({ "k": "v" })),
            Change::Update(serde_json::json!({ "k": "v" })),
            Change::Delete,
        ] {
            assert!(
                mutation_for(&change).statement.contains("RETURN BEFORE"),
                "{} must capture before atomically",
                change.action()
            );
        }
    }

    #[test]
    fn only_create_and_update_carry_content() {
        assert!(
            mutation_for(&Change::Create(serde_json::json!({})))
                .writes
                .is_some()
        );
        assert!(
            mutation_for(&Change::Update(serde_json::json!({})))
                .writes
                .is_some()
        );
        assert!(mutation_for(&Change::Delete).writes.is_none());
    }
}
