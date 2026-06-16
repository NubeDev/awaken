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
    /// Persist agent working/episodic memory and its embedding through the gate.
    ///
    /// Memory persistence is a mutation, so it must cross the gate (contract #1),
    /// but none of the data-plane capabilities fit: `rule-invoke` records a *rule
    /// decision*, not arbitrary agent memory. This is the deliberate, fail-closed
    /// grant the agent's `VectorStoreIndex` write path authorizes
    /// (`rubix/docs/design/AGENT.md`, "Memory writes cross the gate").
    AgentMemoryWrite,
    /// Command a registered physical point (setpoint offset, relay state, mode).
    ///
    /// Actuation is a *third plane* the gate did not previously guard: it leaves
    /// SurrealDB and reaches a device. It is **not** `rule-invoke` (which only
    /// records an append-only insight) — pre-cooling a floor and recording "the
    /// floor is warm" must be two grants. The grant writes a desired-effect record
    /// through the gate; a device-egress worker performs the physical I/O
    /// (`rubix/docs/design/AGENT.md`, "Actuator").
    DeviceActuate,
    /// Create, enable, or schedule a rule definition/binding.
    ///
    /// Distinct from `rule-invoke`, which only *evaluates* a rule and records its
    /// decision. Mutating a rule definition, binding, or schedule (e.g. "roll out
    /// the night profile to Level 5") is a separate authority
    /// (`rubix/docs/design/AGENT.md`, demo manifest).
    RuleDefine,
    /// Create, update, or delete a device *registry* entry.
    ///
    /// Governs the control-plane registration of a device, **not** commanding the
    /// hardware — that is [`DeviceActuate`](Capability::DeviceActuate), a distinct
    /// authority over a distinct plane. Managing the registry (who is a device,
    /// its label/class/metadata) and actuating a registered device must be two
    /// grants (`rubix/docs/design/ADMIN-API.md`, Surface 4).
    DeviceManage,
    /// Bulk-append readings into the time-series data plane.
    ///
    /// The fail-closed grant the `POST /readings` bulk-append endpoint checks
    /// **once per request** before writing on the root/owner handle
    /// (`rubix/docs/design/READINGS-TIMESERIES.md`, "Bulk append endpoint"). It is
    /// **not** `ingest-publish`: that authorizes the Zenoh ingest stream (where the
    /// once-per-stream check lives at subscribe), while this authorizes a
    /// non-Zenoh batch writer (the seed, backfills). Both are gate-bypassing
    /// data-plane writes — distinct authorities so a batch backfill and a live
    /// stream can be granted apart. Readings never enter the command gate, audit,
    /// or undo (SCOPE: "readings … never undone").
    ReadingsAppend,
    /// Upload a file's bytes into the blob store.
    ///
    /// The fail-closed grant the `POST /files` upload endpoint checks before
    /// writing bytes to the blob store (`rubix/docs/design/BACKEND-COLLECTIONS.md`,
    /// "File fields"; build-order step 6). Blob bytes are a **fourth plane** the
    /// gate did not previously guard: they live outside SurrealDB (local FS on
    /// edge, object store on cloud), so a record write capability does not cover
    /// them. Upload is deliberately its own authority — the returned file
    /// *reference* is later stored in a record's content through the normal gated
    /// write, so the command gate still sees only JSON.
    FileUpload,
}

impl Capability {
    /// Every capability the platform knows about, in declaration order.
    ///
    /// The registry ([`is_registered`](crate::capability::is_registered)) and the
    /// wire round-trip both derive from this list, so adding a variant here is the
    /// single place a new capability becomes known.
    pub const ALL: [Capability; 11] = [
        Capability::DatasourceRegister,
        Capability::RuleInvoke,
        Capability::IngestPublish,
        Capability::ExternalQuery,
        Capability::ZenohSubscribe,
        Capability::AgentMemoryWrite,
        Capability::DeviceActuate,
        Capability::RuleDefine,
        Capability::DeviceManage,
        Capability::ReadingsAppend,
        Capability::FileUpload,
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
            Capability::AgentMemoryWrite => "agent-memory-write",
            Capability::DeviceActuate => "device-actuate",
            Capability::RuleDefine => "rule-define",
            Capability::DeviceManage => "device-manage",
            Capability::ReadingsAppend => "readings-append",
            Capability::FileUpload => "file-upload",
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
        assert_eq!(Capability::AgentMemoryWrite.as_str(), "agent-memory-write");
        assert_eq!(Capability::DeviceActuate.as_str(), "device-actuate");
        assert_eq!(Capability::RuleDefine.as_str(), "rule-define");
        assert_eq!(Capability::DeviceManage.as_str(), "device-manage");
        assert_eq!(Capability::ReadingsAppend.as_str(), "readings-append");
    }

    #[test]
    fn the_allow_set_lists_every_variant_once() {
        // `ALL` is the single source of truth the registry and wire round-trip
        // derive from; its length must track the variant count so a forgotten
        // entry cannot silently drop a capability out of the fail-closed set.
        assert_eq!(Capability::ALL.len(), 11);
        for capability in Capability::ALL {
            let occurrences = Capability::ALL
                .into_iter()
                .filter(|other| *other == capability)
                .count();
            assert_eq!(occurrences, 1, "{capability:?} appears more than once");
        }
    }
}
