//! Comma-separated `?tags=` query parsing shared by list endpoints.

pub(crate) fn parse_tags(raw: Option<&str>) -> Vec<String> {
    raw.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}
