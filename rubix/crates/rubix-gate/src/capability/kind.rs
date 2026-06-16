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
    /// Open a bulk job — submit a `POST /records/bulk` (or other bulk op) that may
    /// promote to a long-running background job (`rubix/docs/design/BULK-AND-JOBS.md`,
    /// "Bulk record CRUD").
    ///
    /// This gates the **resource** (spawning a job), not the **data**: it
    /// authorizes only the act of opening a bulk job, never the underlying
    /// mutations or reads. Each item in the bulk envelope still flows through
    /// `apply()` and is checked against its own per-item capability + row-level
    /// perms, so a principal holding `bulk-submit` but not the per-item write
    /// capability gets a job whose every item fails authorization. The bulk cap and
    /// the per-item caps are deliberately separate authorities.
    BulkSubmit,
    /// Subscribe to the in-process control event bus.
    ///
    /// The control event bus (`rubix-bus` in-process plane) carries
    /// component-to-component coordination events — `record.created`,
    /// `rule.fired`, lifecycle transitions — inside the binary, threaded to their
    /// originating action by correlation id. A principal that holds this grant may
    /// **observe** that stream; the seam is checked **once at subscribe**, never
    /// re-taxed per event, the same shape as [`ZenohSubscribe`](Capability::ZenohSubscribe)
    /// on the data plane. It is a separate authority from
    /// [`EventPublish`](Capability::EventPublish): observing the bus and injecting
    /// onto it are distinct rights, so a read-only/analyst extension can watch the
    /// platform's events without being able to emit any (the same observe-vs-effect
    /// split as `rule-invoke` vs `rule-define`). The data-change (live-query) plane
    /// needs no capability — an extension reaches it through its own scoped session
    /// and SurrealDB row-level permissions, like a user (reads are native).
    EventSubscribe,
    /// Publish onto the in-process control event bus.
    ///
    /// The effect counterpart to [`EventSubscribe`](Capability::EventSubscribe): a
    /// principal that holds this grant may **emit** a control event onto the
    /// in-process plane, driving other components. Checked **once at the publish
    /// seam**, fail closed — an out-of-grant publisher reaches no subscriber. The
    /// event the bus carries already threads a gate-minted correlation id back to
    /// the action that produced it, so the audit trail is preserved through that
    /// thread rather than by auditing each ephemeral, non-persisted publish (the
    /// same once-checked, gate-bypassing shape as the data-plane stream
    /// capabilities). Kept apart from `EventSubscribe` so the authority to *affect*
    /// the platform's coordination is never an accidental side effect of the
    /// authority to *watch* it.
    EventPublish,
    /// Start, stop, or disable an extension's runtime lifecycle.
    ///
    /// The runtime half of the extension system (`rubix/docs/design/
    /// EXTENSION-RUNTIME.md`) turns a gated `lifecycle: start` write into a
    /// supervised child process. Driving that lifecycle is a *distinct*
    /// authority from registering a datasource: an operator should be able to
    /// grant "may start/stop extensions" without also granting
    /// [`DatasourceRegister`](Capability::DatasourceRegister). The lifecycle
    /// command is checked against this grant fail closed *before* any process is
    /// touched (the same before-effect discipline as every other command), so
    /// an out-of-grant start spawns nothing. It is the runtime counterpart to
    /// the provisioning authority a human admin holds: provisioning the
    /// extension *principal* is an owner write, while flipping a provisioned
    /// extension on and off is this capability — the one an operator extension
    /// or a tenant admin can be handed.
    ExtensionManage,
}

impl Capability {
    /// Every capability the platform knows about, in declaration order.
    ///
    /// The registry ([`is_registered`](crate::capability::is_registered)) and the
    /// wire round-trip both derive from this list, so adding a variant here is the
    /// single place a new capability becomes known.
    pub const ALL: [Capability; 15] = [
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
        Capability::BulkSubmit,
        Capability::EventSubscribe,
        Capability::EventPublish,
        Capability::ExtensionManage,
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
            Capability::BulkSubmit => "bulk-submit",
            Capability::EventSubscribe => "event-subscribe",
            Capability::EventPublish => "event-publish",
            Capability::ExtensionManage => "extension-manage",
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
        assert_eq!(
            Capability::DatasourceRegister.as_str(),
            "datasource-register"
        );
        assert_eq!(Capability::ZenohSubscribe.as_str(), "zenoh-subscribe");
        assert_eq!(Capability::AgentMemoryWrite.as_str(), "agent-memory-write");
        assert_eq!(Capability::DeviceActuate.as_str(), "device-actuate");
        assert_eq!(Capability::RuleDefine.as_str(), "rule-define");
        assert_eq!(Capability::DeviceManage.as_str(), "device-manage");
        assert_eq!(Capability::ReadingsAppend.as_str(), "readings-append");
        assert_eq!(Capability::BulkSubmit.as_str(), "bulk-submit");
        assert_eq!(Capability::EventSubscribe.as_str(), "event-subscribe");
        assert_eq!(Capability::EventPublish.as_str(), "event-publish");
        assert_eq!(Capability::ExtensionManage.as_str(), "extension-manage");
    }

    #[test]
    fn the_allow_set_lists_every_variant_once() {
        // `ALL` is the single source of truth the registry and wire round-trip
        // derive from; its length must track the variant count so a forgotten
        // entry cannot silently drop a capability out of the fail-closed set.
        assert_eq!(Capability::ALL.len(), 15);
        for capability in Capability::ALL {
            let occurrences = Capability::ALL
                .into_iter()
                .filter(|other| *other == capability)
                .count();
            assert_eq!(occurrences, 1, "{capability:?} appears more than once");
        }
    }
}
