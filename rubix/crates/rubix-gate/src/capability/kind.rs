//! The capabilities a grant can attach to a principal.
//!
//! Capability grants are the **second authz layer** (`rubix/docs/SCOPE.md`,
//! "Two authz layers"): app-enforced authority over non-record / cross-plane
//! actions, distinct from the SurrealDB-native row-level read scope. Each
//! variant names one cross-plane action the gate guards. A capability that is
//! not one of these variants is unknown and must fail closed when checked.

/// A cross-plane action the gate guards with an app-enforced grant.
///
/// These are deliberately *not* SurrealDB record permissions: they govern
/// actions that touch another plane (datasources, rules, ingest, external
/// queries, Zenoh key-spaces), which SurrealDB's permission engine does not see
/// (`rubix/docs/SCOPE.md`, "Two authz layers"). Both layers key off the same
/// [`Principal`](rubix_core::Principal). The stable wire/storage form is the
/// kebab-case string from [`Capability::as_str`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Capability {
    /// Register a datasource connector.
    DatasourceRegister,
    /// Invoke a rule evaluation.
    RuleInvoke,
    /// Publish data into the ingest plane.
    IngestPublish,
    /// Query an external (non-SurrealDB) datasource.
    ExternalQuery,
    /// Subscribe to a Zenoh key-space.
    ZenohSubscribe,
}

impl Capability {
    /// Every capability the platform knows about, in declaration order.
    ///
    /// The registry ([`is_registered`](crate::capability::is_registered)) and the
    /// wire round-trip both derive from this list, so adding a variant here is the
    /// single place a new capability becomes known.
    pub const ALL: [Capability; 5] = [
        Capability::DatasourceRegister,
        Capability::RuleInvoke,
        Capability::IngestPublish,
        Capability::ExternalQuery,
        Capability::ZenohSubscribe,
    ];

    /// The stable wire/storage string for this capability.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Capability::DatasourceRegister => "datasource-register",
            Capability::RuleInvoke => "rule-invoke",
            Capability::IngestPublish => "ingest-publish",
            Capability::ExternalQuery => "external-query",
            Capability::ZenohSubscribe => "zenoh-subscribe",
        }
    }

    /// Resolve a stored/wire string to a known capability.
    ///
    /// Returns `None` for any string that is not a registered capability — the
    /// caller turns that into a fail-closed deny rather than a guess. Named
    /// `parse` (not `from_str`) so it is not confused with `FromStr`, which would
    /// imply a fallible-but-domainless conversion.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Capability> {
        Capability::ALL
            .into_iter()
            .find(|capability| capability.as_str() == raw)
    }
}

#[cfg(test)]
mod tests {
    use super::Capability;

    #[test]
    fn every_capability_round_trips_through_its_string() {
        for capability in Capability::ALL {
            assert_eq!(Capability::parse(capability.as_str()), Some(capability));
        }
    }

    #[test]
    fn an_unknown_string_resolves_to_none() {
        assert_eq!(Capability::parse("not-a-capability"), None);
        assert_eq!(Capability::parse(""), None);
    }

    #[test]
    fn capability_strings_are_kebab_case() {
        assert_eq!(Capability::DatasourceRegister.as_str(), "datasource-register");
        assert_eq!(Capability::ZenohSubscribe.as_str(), "zenoh-subscribe");
    }
}
