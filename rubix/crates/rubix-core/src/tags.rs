use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::CoreError;

/// Haystack-flavoured tag set: marker tags map to `true`, value tags to a
/// JSON value (`{"ahu": true, "stage": 2}`).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(transparent)]
pub struct TagSet(pub BTreeMap<String, serde_json::Value>);

impl TagSet {
    pub fn validate(&self) -> Result<(), CoreError> {
        for key in self.0.keys() {
            if key.is_empty() || !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                return Err(CoreError::InvalidTag(key.clone()));
            }
        }
        Ok(())
    }

    /// True when every named tag is present (marker semantics: presence, not value).
    pub fn has_all<'a>(&self, tags: impl IntoIterator<Item = &'a str>) -> bool {
        tags.into_iter().all(|t| self.0.contains_key(t))
    }
}

/// Validate a path segment used in zenoh-style keyexprs (`org/site/equip/point`).
pub fn validate_slug(slug: &str) -> Result<(), CoreError> {
    if slug.is_empty()
        || !slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
    {
        return Err(CoreError::InvalidSlug(slug.to_string()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn marker_and_value_tags_match_on_presence() {
        let tags: TagSet =
            serde_json::from_value(json!({"ahu": true, "discharge": true, "stage": 2})).unwrap();
        assert!(tags.has_all(["ahu", "stage"]));
        assert!(!tags.has_all(["ahu", "vav"]));
        tags.validate().unwrap();
    }

    #[test]
    fn rejects_bad_tag_names() {
        let tags: TagSet = serde_json::from_value(json!({"bad tag": true})).unwrap();
        assert!(tags.validate().is_err());
    }

    #[test]
    fn slug_validation() {
        assert!(validate_slug("ahu-3").is_ok());
        assert!(validate_slug("AHU_3").is_err());
        assert!(validate_slug("").is_err());
    }
}
