//! Build a Haystack marker [`TagSet`] from a list of tag names. Markers map to
//! JSON `true` (`{"ahu": true}`), matching the wire shape the UI reads.

use rubix_core::TagSet;

pub fn markers(names: &[&str]) -> TagSet {
    TagSet(
        names
            .iter()
            .map(|n| ((*n).to_string(), serde_json::Value::Bool(true)))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_become_true_markers() {
        let tags = markers(&["ahu", "hvac"]);
        assert!(tags.has_all(["ahu", "hvac"]));
        tags.validate().expect("marker names are valid tags");
    }
}
