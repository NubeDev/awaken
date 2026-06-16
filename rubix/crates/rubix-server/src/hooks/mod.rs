//! After-write hooks — fire a rule when a watched record is written.
//!
//! PocketBase fires a hook on record events; rubix expresses the same as data: a
//! `kind:"hook"` record binds "on create/update of kind X, invoke rule Y"
//! (`rubix/docs/design/BACKEND-COLLECTIONS.md`, "Server-side hooks", build-order
//! step 5). The engine already exists (`rubix-rules`) but had no write trigger —
//! this module is that trigger.
//!
//! ## After-hooks, on the live-query data plane
//!
//! This is the **after-hook** model (the design's chosen execution model): the
//! rule fires *after* the write has committed, so it cannot reject the write. It
//! therefore rides the **live-query data plane** — a SurrealDB live query over the
//! `record` table — never the ungated in-process bus, exactly as the design
//! requires (so a hook only ever sees a committed, gate-audited record). A
//! before-hook that can veto a write is a different mechanism (it must run inside
//! the gate's `apply()`); it is deliberately out of scope here and left to its own
//! design note ([`HOOKS-AND-FILES.md`](../../../../docs/design/HOOKS-AND-FILES.md)).
//!
//! ## The rule still crosses the gate
//!
//! Firing a hook does not add a write path: the rule's insight is recorded through
//! the WS-05 gate as a `RuleInvoke` command, audited and correlated like any other
//! evaluation. The dispatcher therefore needs an identity that holds the
//! `RuleInvoke` grant in the record's namespace. It uses a per-namespace **system
//! principal** (`{namespace}_system`, an extension service account) provisioned the
//! same way as any other extension identity — its secret is rotated on each boot
//! and held only in memory, so there is no stored credential to leak
//! ([`reprovision_principal`](rubix_gate::reprovision_principal)).
//!
//! ## Cross-tenant routing
//!
//! One dispatcher serves every namespace. It subscribes to the `record` table on
//! the gate **owner** handle (so it sees every tenant's committed writes) and
//! routes each change to a system principal **scoped to that change's namespace**
//! before firing — so a hook in tenant A can never read or write tenant B's data.

use std::collections::{HashMap, HashSet};

use rubix_bus::{ControlBus, DataChangeKind, subscribe_table};
use rubix_core::{HOOK_KIND, Hook, HookEvent, Id, Principal, PrincipalKind, Role, find_hooks};
use rubix_gate::{
    Capability, PrincipalToken, ScopedSession, create_grant, issue_scoped_session,
    read_records_on_session_filtered, reprovision_principal,
};
use rubix_rules::{RuleRegistry, RuleRuntime, evaluate};
use rubix_trace::SampleRate;

use crate::dto::rule::{RULE_KIND, RuleDto, build_rule};
use crate::state::AppState;

/// How long to wait before re-subscribing after the live query ends or errors.
///
/// The live query ends only on a store/engine fault here (the subscription has no
/// natural end), so a tight reconnect loop would busy-spin against a broken store.
/// A short fixed backoff keeps the dispatcher self-healing without hammering.
const RESUBSCRIBE_BACKOFF: std::time::Duration = std::time::Duration::from_secs(1);

/// Spawn the hook dispatcher on its own thread.
///
/// Called once at boot, after the store and gate schema are ready. The dispatcher
/// owns a clone of [`AppState`] (an `Arc` bump on the store handle) and runs for
/// the life of the process. A failure inside the loop is logged and the loop
/// re-subscribes; it never aborts the server.
///
/// It runs on a **dedicated thread with a current-thread runtime** rather than
/// `tokio::spawn`: the Rhai rule engine's evaluation future is `!Send`, so it
/// cannot run on the multi-threaded request runtime. Driving it on a single-thread
/// runtime keeps the engine off the request workers entirely — the dispatcher is a
/// background plane, not part of any request.
pub fn spawn_hook_dispatcher(state: AppState) {
    let spawned = std::thread::Builder::new()
        .name("hook-dispatcher".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("hook dispatcher: build runtime failed: {error}; hooks disabled");
                    return;
                }
            };
            runtime.block_on(async move {
                Dispatcher::new(state).run().await;
            });
        });
    if let Err(error) = spawned {
        eprintln!("hook dispatcher: spawn thread failed: {error}; hooks disabled");
    }
}

/// The running dispatcher: a live-query subscriber plus the per-namespace caches it
/// builds lazily as it sees each tenant's first hookable write.
struct Dispatcher {
    /// Shared state — the owner store handle and the SurrealDB namespace/database.
    state: AppState,
    /// The in-process bus a fired rule publishes its insight on. The dispatcher has
    /// no in-process subscriber, so the firing's reach is 0; the durable insight is
    /// the record written through the gate, not this event.
    bus: ControlBus,
    /// Span sampling for the fired evaluation. Hooks fire on the hot write path, so
    /// they do not persist a span per evaluation (rate 0.0) — the insight + audit
    /// row are the durable trail.
    sample: SampleRate,
    /// One cached scoped session per namespace, keyed by the tenant namespace. Each
    /// is the system principal signed in and scoped to that namespace; built once
    /// per namespace per process lifetime.
    sessions: HashMap<String, ScopedSession>,
    /// Cached hook bindings per namespace. Refreshed lazily and invalidated when a
    /// `kind:"hook"` record changes in that namespace (seen on the same stream), so
    /// a newly defined hook takes effect without a per-write reload.
    hooks: HashMap<String, Vec<Hook>>,
    /// The set of insight kinds (rule `output`s) per namespace — the
    /// **recursion guard**. A fired rule writes an insight record whose `kind` is
    /// the rule's output; that write reappears on the stream. Treating an insight as
    /// a hookable event is what would let a hook fire on the insight it produced and
    /// loop, so the dispatcher skips any change whose `kind` is a rule output. The
    /// set is refreshed lazily and invalidated when a `kind:"rule"` record changes.
    outputs: HashMap<String, HashSet<String>>,
}

impl Dispatcher {
    fn new(state: AppState) -> Self {
        Self {
            state,
            bus: ControlBus::new(),
            sample: SampleRate::new(0.0),
            sessions: HashMap::new(),
            hooks: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    /// Subscribe to record changes and dispatch hooks until the process ends.
    ///
    /// The outer loop re-subscribes if the live query ends or faults, so a transient
    /// store error does not silently stop hooks for the rest of the process.
    async fn run(&mut self) {
        loop {
            let mut stream = match subscribe_table(self.state.store.raw(), "record").await {
                Ok(stream) => stream,
                Err(error) => {
                    eprintln!("hook dispatcher: subscribe failed: {error}; retrying");
                    tokio::time::sleep(RESUBSCRIBE_BACKOFF).await;
                    continue;
                }
            };
            while let Some(next) = stream.next().await {
                match next {
                    Ok(change) => self.dispatch(&change).await,
                    Err(error) => {
                        eprintln!("hook dispatcher: stream error: {error}; re-subscribing");
                        break;
                    }
                }
            }
            tokio::time::sleep(RESUBSCRIBE_BACKOFF).await;
        }
    }

    /// Match one committed change against the namespace's hooks and fire each.
    ///
    /// A failure firing one hook is logged and never stops the others or the loop —
    /// a hook is a side effect on a write that already committed, so it must not be
    /// able to wedge the dispatcher.
    async fn dispatch(&mut self, change: &rubix_bus::DataChange) {
        let record = change.record();
        let namespace = record.namespace.clone();
        let kind = record.content.get("kind").and_then(|v| v.as_str());

        // Config writes invalidate the matching cache and fire no rule themselves:
        // a hook binding changed → reload the namespace's hooks; a rule changed →
        // reload the namespace's output set (the recursion guard below). A `rule`/
        // `hook` record is config, not a hookable domain event.
        match kind {
            Some(HOOK_KIND) => {
                self.hooks.remove(&namespace);
                return;
            }
            Some(RULE_KIND) => {
                self.outputs.remove(&namespace);
                return;
            }
            _ => {}
        }

        // Recursion guard: an insight a rule emitted is not a hookable domain write.
        // Skipping any record whose kind is a rule's output makes a hook→rule→insight
        // loop impossible — the dispatcher only ever creates insight records, and it
        // never re-triggers on them. (A record with no kind matches no hook anyway.)
        let Some(kind_str) = kind else { return };
        let is_insight = match self.outputs_for(&namespace).await {
            Ok(outputs) => outputs.contains(kind_str),
            Err(error) => {
                // A transient load failure is logged and the write proceeds: missing
                // the guard once cannot loop (the next pass reloads and stops it), and
                // silently dropping every hook on a blip is the worse failure.
                eprintln!("hook dispatcher: load outputs for `{namespace}`: {error}");
                false
            }
        };
        if is_insight {
            return;
        }

        let event = match change.kind() {
            DataChangeKind::Created => HookEvent::Create,
            DataChangeKind::Updated => HookEvent::Update,
            DataChangeKind::Deleted => HookEvent::Delete,
        };

        // Snapshot the matching rule ids before any await that borrows `self` again.
        let rules: Vec<String> = match self.hooks_for(&namespace).await {
            Ok(hooks) => hooks
                .iter()
                .filter(|hook| hook.matches(event, kind))
                .map(|hook| hook.rule.clone())
                .collect(),
            Err(error) => {
                eprintln!("hook dispatcher: load hooks for `{namespace}`: {error}");
                return;
            }
        };
        if rules.is_empty() {
            return;
        }

        for rule in rules {
            if let Err(error) = self.fire(&namespace, &rule).await {
                eprintln!("hook dispatcher: fire rule `{rule}` in `{namespace}`: {error}");
            }
        }
    }

    /// The cached hook bindings for `namespace`, loading them on first use.
    async fn hooks_for(&mut self, namespace: &str) -> Result<&[Hook], String> {
        if !self.hooks.contains_key(namespace) {
            let hooks = find_hooks(self.state.store.raw(), namespace)
                .await
                .map_err(|e| e.to_string())?;
            self.hooks.insert(namespace.to_owned(), hooks);
        }
        Ok(self.hooks.get(namespace).map_or(&[][..], Vec::as_slice))
    }

    /// The cached set of rule output kinds for `namespace` (the recursion guard),
    /// loading it on first use.
    async fn outputs_for(&mut self, namespace: &str) -> Result<&HashSet<String>, String> {
        if !self.outputs.contains_key(namespace) {
            let outputs = load_rule_outputs(self.state.store.raw(), namespace).await?;
            self.outputs.insert(namespace.to_owned(), outputs);
        }
        Ok(self
            .outputs
            .get(namespace)
            .expect("outputs were just inserted"))
    }

    /// Invoke `rule` in `namespace` through the gate as the system principal.
    async fn fire(&mut self, namespace: &str, rule: &str) -> Result<(), String> {
        let session = self.session_for(namespace).await?;
        let registry = self.registry_for(&session).await?;

        let runtime = RuleRuntime {
            gate_db: self.state.store.raw(),
            session: session.connection(),
            trace_db: self.state.store.raw(),
            bus: &self.bus,
            sample: self.sample,
        };
        evaluate(
            &runtime,
            &registry,
            session.principal(),
            &Id::from_raw(rule),
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    /// Build a rule registry from every `kind:"rule"` record visible to `session`.
    ///
    /// The registry holds the whole namespace's rules so the fired rule resolves its
    /// composed sub-rules; a rule whose stored definition is malformed is skipped
    /// rather than failing the whole firing (the bad rule is the author's problem,
    /// surfaced when they dry-run it). The registry id of each rule is its name, so
    /// a hook's `rule` field (a rule name) resolves directly.
    async fn registry_for(&self, session: &ScopedSession) -> Result<RuleRegistry, String> {
        let records = read_records_on_session_filtered(session, Some(RULE_KIND), &[])
            .await
            .map_err(|e| e.to_string())?;
        let mut registry = RuleRegistry::new();
        for record in records {
            let Some(dto) = RuleDto::from_record(record) else {
                continue;
            };
            let RuleDto {
                name,
                script,
                inputs,
                subrules,
                output,
                ..
            } = dto;
            if let Ok(rule) = build_rule(&name, &script, &inputs, &subrules, &output) {
                registry.insert(rule);
            }
        }
        Ok(registry)
    }

    /// The system principal's scoped session for `namespace`, provisioning it on
    /// first use.
    ///
    /// The session is cached for the process lifetime. Provisioning rotates the
    /// system principal's secret (upsert) and re-grants `RuleInvoke` (the gate the
    /// insight write crosses) and `ExternalQuery` (the rule's window reads run on
    /// the scoped session); both grants are idempotent. The grantor is an in-memory
    /// admin in the same namespace — the no-escalation authority the gate checks,
    /// the same pattern the dev seed uses to confer grants.
    async fn session_for(&mut self, namespace: &str) -> Result<ScopedSession, String> {
        if let Some(session) = self.sessions.get(namespace) {
            return Ok(session.clone());
        }

        let db = self.state.store.raw();
        let subject = format!("{namespace}_system");
        let secret = uuid::Uuid::new_v4().to_string();
        let principal = Principal::new(
            Id::from_raw(subject.clone()),
            namespace.to_owned(),
            PrincipalKind::Extension,
            Role::Operator,
        );
        reprovision_principal(db, &principal, secret.clone())
            .await
            .map_err(|e| e.to_string())?;

        let admin = Principal::new(
            Id::from_raw(format!("{namespace}_admin")),
            namespace.to_owned(),
            PrincipalKind::User,
            Role::Admin,
        );
        for capability in [Capability::RuleInvoke, Capability::ExternalQuery] {
            create_grant(db, &admin, &principal, capability)
                .await
                .map_err(|e| e.to_string())?;
        }

        // Sign into the SurrealDB namespace/database the server runs on; the
        // principal's `namespace` field (not the SurrealDB NS) is what row-perms
        // scope reads to.
        let token = PrincipalToken::new(subject, secret);
        let session = issue_scoped_session(
            db,
            &self.state.namespace,
            &self.state.database,
            principal,
            &token,
        )
        .await
        .map_err(|e| e.to_string())?;

        self.sessions.insert(namespace.to_owned(), session.clone());
        Ok(session)
    }
}

/// Load the set of insight kinds a namespace's rules emit (their `output`s).
///
/// Read directly on the owner handle filtered by namespace (like `find_hooks`),
/// not on a scoped session — the dispatcher consults this on the hot dispatch path
/// before deciding whether a write is a hookable event, ahead of any session. A
/// rule whose `output` is missing or non-string is skipped; it cannot be an insight
/// kind a downstream write would carry.
async fn load_rule_outputs(
    db: &surrealdb::Surreal<surrealdb::engine::local::Db>,
    namespace: &str,
) -> Result<HashSet<String>, String> {
    let outputs: Vec<serde_json::Value> = db
        .query(
            "SELECT VALUE content.output FROM record \
             WHERE namespace = $namespace AND content.kind = $rule_kind",
        )
        .bind(("namespace", namespace.to_owned()))
        .bind(("rule_kind", RULE_KIND))
        .await
        .map_err(|e| e.to_string())?
        .take(0)
        .map_err(|e| e.to_string())?;

    Ok(outputs
        .into_iter()
        .filter_map(|value| value.as_str().map(str::to_owned))
        .collect())
}
