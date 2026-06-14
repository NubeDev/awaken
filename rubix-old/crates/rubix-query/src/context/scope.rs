//! The tenant a scoped query session is confined to.
//!
//! A [`QueryScope`] names one `{org}/{site}` pair. A session built for a scope
//! exposes the canonical tables as views already filtered to that org/site, so
//! the SQL a scoped agent runs can only ever read its own tenant's rows — the
//! confinement is structural, not a rewrite of the caller's SQL.

use crate::error::QueryError;

/// One `{org}/{site}` tenant a scoped query session is confined to.
///
/// `org` is the `sites.org` value and `site` the `sites.slug` value; together
/// they are the `UNIQUE (org, slug)` tenant key. Both are embedded as SQL
/// string literals in the filtered views, so they are rejected up front if they
/// contain a quote or NUL — a defense-in-depth guard even though the values
/// originate from a validated keyexpr prefix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryScope {
    org: String,
    site: String,
}

impl QueryScope {
    /// Build a scope from an `org` and `site` (the `sites.org`/`sites.slug`
    /// pair). Rejects empty parts and any value carrying a single quote or NUL,
    /// which could break out of the SQL string literal the filtered views embed.
    pub fn new(org: impl Into<String>, site: impl Into<String>) -> Result<Self, QueryError> {
        let org = org.into();
        let site = site.into();
        Self::check("org", &org)?;
        Self::check("site", &site)?;
        Ok(Self { org, site })
    }

    fn check(part: &str, value: &str) -> Result<(), QueryError> {
        if value.is_empty() {
            return Err(QueryError::Scope(format!("{part} is empty")));
        }
        if value.contains('\'') || value.contains('\0') {
            return Err(QueryError::Scope(format!(
                "{part} carries an illegal character: {value:?}"
            )));
        }
        Ok(())
    }

    /// The org (`sites.org`) this scope is confined to.
    pub(crate) fn org(&self) -> &str {
        &self.org
    }

    /// The site slug (`sites.slug`) this scope is confined to.
    pub(crate) fn site(&self) -> &str {
        &self.site
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_and_exposes_parts() {
        let s = QueryScope::new("nube", "hq").unwrap();
        assert_eq!(s.org(), "nube");
        assert_eq!(s.site(), "hq");
    }

    #[test]
    fn rejects_quote_and_empty() {
        assert!(QueryScope::new("nu'be", "hq").is_err());
        assert!(QueryScope::new("nube", "h\0q").is_err());
        assert!(QueryScope::new("", "hq").is_err());
        assert!(QueryScope::new("nube", "").is_err());
    }
}
