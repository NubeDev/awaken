//! CREATE a tag node.

use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use crate::error::{Error, Result};

use super::{TAG_TABLE, Tag, TagRow};

/// Persist `tag` into the `tag` table, keyed by its own id.
///
/// Returns the stored tag as SurrealDB decoded it.
///
/// # Errors
/// Returns [`Error::Store`] if the write fails or the row is not returned.
pub async fn create_tag(db: &Surreal<Db>, tag: &Tag) -> Result<Tag> {
    let created: Option<TagRow> = db
        .create((TAG_TABLE, tag.id.as_str()))
        .content(TagRow::from_tag(tag))
        .await
        .map_err(|e| Error::Store(e.to_string()))?;
    created
        .map(TagRow::into_tag)
        .ok_or_else(|| Error::Store("tag create returned no row".to_owned()))
}
