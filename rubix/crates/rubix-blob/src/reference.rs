//! The file reference stored in a record's content.

use serde::{Deserialize, Serialize};

/// A reference to a stored blob — what a `file` field holds in record content.
///
/// This is the reference shape the collection field type documents and validates
/// (`rubix_core`'s `FieldType::File` accepts an object carrying a string `id`).
/// The bytes live in the [`BlobStore`](crate::BlobStore); this carries only the
/// metadata a client renders (filename, size, content type) plus the id the
/// download route resolves. Serialised camelCase (`contentType`) to match the
/// documented wire shape `{ id, filename, size, contentType }`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileRef {
    /// The server-minted blob id — the download handle and the store key.
    pub id: String,
    /// The original filename the client uploaded (display only).
    pub filename: String,
    /// The blob's size in bytes.
    pub size: u64,
    /// The blob's MIME content type, for the download response and rendering.
    pub content_type: String,
}

impl FileRef {
    /// Build a reference for a freshly stored blob.
    #[must_use]
    pub fn new(
        id: impl Into<String>,
        filename: impl Into<String>,
        size: u64,
        content_type: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            filename: filename.into(),
            size,
            content_type: content_type.into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FileRef;

    #[test]
    fn serialises_to_the_documented_camel_case_shape() {
        let json = serde_json::to_value(FileRef::new("f-1", "plan.pdf", 42, "application/pdf"))
            .expect("serialise");
        assert_eq!(json["id"], "f-1");
        assert_eq!(json["filename"], "plan.pdf");
        assert_eq!(json["size"], 42);
        assert_eq!(json["contentType"], "application/pdf");
    }

    #[test]
    fn round_trips_through_json() {
        let reference = FileRef::new("f-2", "a.png", 7, "image/png");
        let json = serde_json::to_string(&reference).expect("serialise");
        let back: FileRef = serde_json::from_str(&json).expect("deserialise");
        assert_eq!(reference, back);
    }
}
