//! Development seed: a demo portfolio behind the `--seed-dev` flag.
//!
//! The Makefile's `SEED=1` switch boots the server with a populated store so UI
//! and API work has something real to read. There is no HTTP path to create
//! principals or grants (identity precedes any scoped session), so the seed runs
//! the gate library directly, then writes a building portfolio through the gate
//! as the tenant operator (real audit, undo, and live-query events).
//!
//! Two tenants, two sites each, three domains (HVAC, energy, water). The data is
//! deterministic, so a fresh store seeds identically every run. The gate/audit
//! schema must already be defined on `db` (the binary does this at boot).

mod cast;
mod dashboards;
mod history;
mod portfolio;
mod rules;

use std::collections::HashSet;
use std::fmt;

use chrono::Utc;
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use cast::provision_cast;
use portfolio::{TenantTally, seed_tenant};

/// The tenants seeded, in order. Kept here so the orchestrator and the per-tenant
/// topology in [`portfolio`] agree on the set without a cross-module constant.
const TENANTS: &[&str] = &["acme", "globex"];

/// A seed failure, naming the step that failed and the underlying cause.
#[derive(Debug)]
pub struct SeedError {
    /// What the seed was doing when it failed.
    context: &'static str,
    /// The underlying error, rendered.
    cause: String,
}

impl SeedError {
    /// Wrap `cause` with the step `context` it failed in.
    fn new(context: &'static str, cause: impl fmt::Display) -> Self {
        Self {
            context,
            cause: cause.to_string(),
        }
    }
}

impl fmt::Display for SeedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "seed failed while {}: {}", self.context, self.cause)
    }
}

impl std::error::Error for SeedError {}

/// Provision the cast and write the demo portfolio for every tenant.
///
/// Idempotent on a fresh store; on a non-fresh store the identity/record creates
/// may fail (deterministic ids already exist) — delete the data dir to re-seed.
/// Prints a login table and per-tenant tallies to stdout.
///
/// # Errors
/// Returns a [`SeedError`] if provisioning or any gate write fails.
pub async fn seed_dev(db: &Surreal<Db>) -> Result<(), SeedError> {
    let now = Utc::now();
    let mut tags = HashSet::new();
    let mut tallies = Vec::with_capacity(TENANTS.len());

    println!("seeding demo portfolio ({} tenants)", TENANTS.len());
    println!("{:<20} {:<14} grants", "subject", "secret");

    for namespace in TENANTS {
        let (operator, credentials) = provision_cast(db, namespace).await?;
        // Each tenant namespace gets the bootstrap meta-collection so the demo
        // store exposes a collection registry from the first read.
        rubix_core::bootstrap_meta_collection(db, namespace)
            .await
            .map_err(|e| SeedError::new("seeding meta-collection", e))?;
        for cred in &credentials {
            let grants = cred
                .grants
                .iter()
                .map(|c| c.as_str())
                .collect::<Vec<_>>()
                .join(",");
            let grants = if grants.is_empty() {
                "—".to_owned()
            } else {
                grants
            };
            println!("{:<20} {:<14} {}", cred.subject, cred.secret, grants);
        }
        let mut tally = seed_tenant(db, namespace, &operator, now, &mut tags).await?;
        // The worked set of demo rules over that tenant's readings — what the
        // rules studio opens onto (simple thresholds → composed → scored).
        tally.rules = rules::seed_rules(db, namespace, &operator).await?;
        // One write-triggered hook so the portfolio demonstrates step 5: editing the
        // site re-fires the temperature rule (`BACKEND-COLLECTIONS.md`, hooks).
        rules::seed_hooks(db, namespace, &operator).await?;
        tallies.push(tally);
    }

    report(&tallies);
    Ok(())
}

/// Print the per-tenant record tallies and the grand total.
fn report(tallies: &[TenantTally]) {
    let mut total = 0usize;
    for t in tallies {
        let count = t.sites + t.nodes + t.readings + t.rules + t.dashboards;
        total += count;
        println!(
            "  {}: {} sites, {} nodes, {} readings, {} rules, {} dashboards ({} records)",
            t.namespace, t.sites, t.nodes, t.readings, t.rules, t.dashboards, count
        );
    }
    println!(
        "seed complete: {total} records across {} tenants",
        tallies.len()
    );
}
