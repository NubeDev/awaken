//! Boot reconciler wiring: bring supervised extensions back to their persisted
//! desired state on server startup.
//!
//! The gated `lifecycle` records persist each extension's desired state across
//! reboots, but on a fresh boot nothing is running and the live HTTP lifecycle
//! path (`http/extensions/lifecycle.rs`) only fires when an operator calls it.
//! Without this, an extension last left in `start` never respawns after a reboot
//! (`rubix/docs/design/EXTENSION-RUNTIME.md`, "boot-time reconciler"). This is the
//! durability half of runtime phase 2.
//!
//! It runs on its **own thread** (like the hook dispatcher and job sweeper), so a
//! slow record read or a child spawn never delays binding the socket, and a
//! failure is logged-and-continue — never fatal to boot. The live HTTP path still
//! works without it.
//!
//! ## Identity handoff at boot (Open question 2, boot half)
//!
//! Starting a process-flavour child needs the child's run secret. In the HTTP path
//! the operator supplies it per request and the server never stores it; at boot
//! there is no operator. We resolve this the same way the hook dispatcher resolves
//! the system principal: **rotate each `start` extension's secret at boot** —
//! `reprovision_principal` mints a fresh in-memory secret, handed to the child via
//! its [`Identity`], with nothing persisted. The boot-minted secret is strictly
//! shorter-lived than any stored credential would be, and there is no stored
//! secret to leak. The `resolve_identity` closure the reconciler calls is sync, so
//! the secrets are provisioned ahead of time and the closure only looks them up.

use std::collections::HashMap;

use rubix_core::{Id, Principal, PrincipalKind, Role};
use rubix_ext::runtime::reconcile_on_session;
use rubix_ext::supervisor::Identity;
use rubix_gate::{
    Capability, PrincipalToken, ScopedSession, create_grant, issue_scoped_session,
    read_records_on_session, reprovision_principal,
};

use crate::state::AppState;

/// Spawn the boot reconciler on its own thread.
///
/// Called once at boot, after the store and gate schema are ready, alongside
/// `spawn_hook_dispatcher` / `spawn_job_sweeper`. It reconciles the configured
/// namespace once and the thread exits — there is no stream to watch, unlike the
/// other two background planes. A failure is logged and the server serves anyway.
pub fn spawn_extension_reconciler(state: AppState) {
    let spawned = std::thread::Builder::new()
        .name("ext-reconciler".to_owned())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(runtime) => runtime,
                Err(error) => {
                    eprintln!("ext reconciler: build runtime failed: {error}; skipping reconcile");
                    return;
                }
            };
            runtime.block_on(async move {
                if let Err(error) = reconcile(&state).await {
                    eprintln!("ext reconciler: {error}; extensions not reconciled");
                }
            });
        });
    if let Err(error) = spawned {
        eprintln!("ext reconciler: spawn thread failed: {error}; extensions not reconciled");
    }
}

/// Reconcile the configured namespace's extensions against their control records.
///
/// Edge is single-tenant, so we reconcile only the configured namespace — the only
/// one with control records. (A cloud/multi-tenant fan-out is a later follow-up;
/// the reconciler itself is already per-session, so it generalises by iterating
/// onboarded namespaces.) Reads run on a system-principal scoped session, so
/// row-level perms naturally limit them to that namespace.
async fn reconcile(state: &AppState) -> Result<(), String> {
    let namespace = state.namespace.clone();
    let session = system_session(state, &namespace).await?;

    // Pre-provision a fresh secret for every process-flavour `start` we are about
    // to bring up, so the (sync) `resolve_identity` closure can hand it over
    // without awaiting. We don't know which records are `start` ahead of the read,
    // so we provision lazily inside the closure via a pre-built map: read the
    // control records, mint a secret per candidate, then reconcile.
    let records = read_records_on_session(&session)
        .await
        .map_err(|e| format!("reading control records in `{namespace}`: {e}"))?;

    // Mint and grant a fresh run secret for each extension subject present, keyed by
    // subject. Reconcile skips non-`start`/non-process records, so an unused entry
    // here is harmless (the principal's secret is simply rotated and never used).
    // The subject derivation mirrors the reconciler's own `parse`: the `extension`
    // field if present, else the record id. A record that is not a control record is
    // harmless here — its secret is rotated but reconcile never spawns it.
    let mut secrets: HashMap<String, String> = HashMap::new();
    for record in &records {
        let subject = record
            .content
            .get("extension")
            .and_then(serde_json::Value::as_str)
            .map_or_else(|| record.id.as_str().to_owned(), str::to_owned);
        if secrets.contains_key(&subject) {
            continue;
        }
        let secret = provision_extension(state, &namespace, &subject).await?;
        secrets.insert(subject, secret);
    }

    let resolve_identity = |id: &rubix_ext::supervisor::ExtensionId| {
        secrets.get(id.subject()).map(|secret| Identity {
            namespace: id.namespace().to_owned(),
            subject: id.subject().to_owned(),
            secret: secret.clone(),
        })
    };

    let report = reconcile_on_session(&state.extensions, &session, resolve_identity)
        .await
        .map_err(|e| format!("reconciling `{namespace}`: {e}"))?;

    if !report.started.is_empty() || !report.stopped.is_empty() || !report.skipped.is_empty() {
        println!(
            "ext reconciler [{namespace}]: {} started, {} stopped, {} skipped",
            report.started.len(),
            report.stopped.len(),
            report.skipped.len(),
        );
        for (id, reason) in &report.skipped {
            println!("ext reconciler [{namespace}]: skipped {id}: {reason}");
        }
    }
    Ok(())
}

/// The system principal's scoped session for `namespace`, provisioned with the
/// `ExtensionManage` grant the reconcile path crosses.
///
/// Same pattern as the hook dispatcher's system principal (`hooks/mod.rs`): a
/// `{namespace}_system` extension service account whose secret is rotated on each
/// boot and held only in memory, granted by the in-memory `{namespace}_admin`.
async fn system_session(state: &AppState, namespace: &str) -> Result<ScopedSession, String> {
    let db = state.store.raw();
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
        .map_err(|e| format!("provisioning system principal: {e}"))?;

    let admin = Principal::new(
        Id::from_raw(format!("{namespace}_admin")),
        namespace.to_owned(),
        PrincipalKind::User,
        Role::Admin,
    );
    create_grant(db, &admin, &principal, Capability::ExtensionManage)
        .await
        .map_err(|e| format!("granting ExtensionManage: {e}"))?;

    let token = PrincipalToken::new(subject, secret);
    issue_scoped_session(db, &state.namespace, &state.database, principal, &token)
        .await
        .map_err(|e| format!("issuing system session: {e}"))
}

/// Rotate one extension principal's run secret in-memory and return it for handoff
/// to the spawned child. Nothing is persisted; the secret lives only long enough to
/// reach the child's environment.
async fn provision_extension(
    state: &AppState,
    namespace: &str,
    subject: &str,
) -> Result<String, String> {
    let db = state.store.raw();
    let secret = uuid::Uuid::new_v4().to_string();
    let principal = Principal::new(
        Id::from_raw(subject.to_owned()),
        namespace.to_owned(),
        PrincipalKind::Extension,
        Role::Operator,
    );
    reprovision_principal(db, &principal, secret.clone())
        .await
        .map_err(|e| format!("provisioning extension `{subject}`: {e}"))?;
    Ok(secret)
}
