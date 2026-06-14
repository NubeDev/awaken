//! The tag graph: structure without a fixed ontology.
//!
//! A tag is a named graph node; records are connected to tags by a `tagged`
//! edge (`record→tagged→tag`). Tag-set traversal replaces a fixed
//! equipment/site/point schema — a record's meaning is the set of tags it
//! carries (`rubix/docs/SCOPE.md`, principle 4). Verbs execute SurrealQL
//! `RELATE`/`DELETE`/graph traversal over a SurrealDB connection borrowed from
//! the `rubix-store` durable handle (contract #6: graph + document, one engine).

mod attach;
mod create;
mod delete;
mod detach;
mod find_by_tags;
mod row;

pub use attach::attach_tag;
pub use create::create_tag;
pub use delete::delete_tag;
pub use detach::detach_tag;
pub use find_by_tags::find_records_by_tags;

pub(crate) use row::TagRow;

use crate::id::Id;

/// The SurrealDB table every tag node lives in.
pub(crate) const TAG_TABLE: &str = "tag";

/// The SurrealDB edge table linking a record to a tag (`record→tagged→tag`).
pub(crate) const TAGGED_EDGE: &str = "tagged";

/// A named tag node on the graph.
///
/// The `name` is the human-facing label; the `id` is the stable graph node
/// identity. A record carries meaning through the *set* of tags related to it,
/// not a fixed schema.
#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    /// Stable identifier for the tag node.
    pub id: Id,
    /// Human-facing label, e.g. `temperature` or `floor-2`.
    pub name: String,
}

impl Tag {
    /// Build a new tag with a freshly minted id.
    ///
    /// Callers persist with [`create_tag`].
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Id::new(),
            name: name.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Tag, TagRow};

    #[test]
    fn new_mints_an_id_and_keeps_the_name() {
        let tag = Tag::new("temperature");
        assert_eq!(tag.name, "temperature");
        assert!(!tag.id.as_str().is_empty());
    }

    #[test]
    fn tag_round_trips_through_the_persisted_row() {
        let tag = Tag::new("floor-2");
        let row = TagRow::from_tag(&tag);
        assert_eq!(row.into_tag(), tag);
    }
}
