//! Stored-rule rows: org-scoped CRUD plus the referencing-rules listing the
//! rules-engine design calls for. Backend dispatch; SQLite body inline, Postgres
//! body in [`super::postgres::rules`].
//!
//! A rule is the emit unit a spark board composes: a named, parameterized Rhai
//! script that returns a verdict. Rules are **org-scoped** — `name` is unique
//! per org, so `rule("temp-high", …)` resolves one rule within a tenant. The
//! board node and composition resolve through [`Store::load_rule`], which backs
//! the [`rubix_rules::RuleStore`] trait (see [`super::super::flow`]).

use chrono::{DateTime, Utc};
use rubix_rules::ParamSchema;
use rusqlite::{params, OptionalExtension, Row};
use uuid::Uuid;

use super::backend::Backend;
use super::codec::{json_of, json_to, ts_of, ts_to};
use super::{Result, Store, StoreError};

/// A stored rule as persisted: identity, org scope, the Rhai script, and the
/// declared parameter schema. The domain type the API and the board store
/// exchange; the [`rubix_rules::StoredRule`] the engine loads is projected from
/// it ([`Self::into_stored`]).
#[derive(Debug, Clone, PartialEq)]
pub struct RuleRecord {
    pub id: Uuid,
    /// Tenant scope (the org namespace). Always set.
    pub org: String,
    /// The single site this rule is for; `None` makes it an org-level rule that
    /// applies across the org. A board run resolves a site-scoped rule first,
    /// then falls back to the org-level one of the same name.
    pub site_id: Option<Uuid>,
    /// Composition name (`rule("temp-high", …)`), unique per scope `(org, site_id)`.
    pub name: String,
    pub script: String,
    pub params: ParamSchema,
    pub created_at: DateTime<Utc>,
}

impl RuleRecord {
    /// Project onto the engine's [`rubix_rules::StoredRule`] (the org scope and
    /// timestamp are resolution metadata the engine does not need).
    pub fn into_stored(self) -> rubix_rules::StoredRule {
        rubix_rules::StoredRule {
            id: self.id.to_string(),
            name: self.name,
            script: self.script,
            params: self.params,
        }
    }
}

pub(crate) const RULE_COLS: &str = "id, org, site_id, name, script, params, created_at";

fn row_rule(row: &Row<'_>) -> rusqlite::Result<RuleRecord> {
    Ok(RuleRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        site_id: row.get(2)?,
        name: row.get(3)?,
        script: row.get(4)?,
        params: json_to(&row.get::<_, String>(5)?)?,
        created_at: ts_to(&row.get::<_, String>(6)?)?,
    })
}

impl Store {
    /// Insert a rule. The per-scope unique index rejects a duplicate name within
    /// `(org, site_id)` as a [`StoreError::Conflict`].
    pub fn create_rule(&self, rule: &RuleRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                if let Some(site_id) = rule.site_id {
                    Self::require_site(&conn, site_id)?;
                }
                conn.execute(
                    "INSERT INTO rules (id, org, site_id, name, script, params, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                    params![
                        rule.id,
                        rule.org,
                        rule.site_id,
                        rule.name,
                        rule.script,
                        json_of(&rule.params),
                        ts_of(&rule.created_at)
                    ],
                )?;
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::create_rule(self, rule),
        }
    }

    /// Rules in an org, newest first. With `site_id` `Some`, returns that site's
    /// rules plus the org-level ones (the set that resolves on that site); with
    /// `None`, returns every rule the org owns (org-level + all sites).
    pub fn list_rules(
        &self,
        org: &str,
        site_id: Option<Uuid>,
    ) -> Result<Vec<RuleRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let mut stmt = conn.prepare(&format!(
                    "SELECT {RULE_COLS} FROM rules \
                     WHERE org = ?1 AND (?2 IS NULL OR site_id IS NULL OR site_id = ?2) \
                     ORDER BY created_at DESC"
                ))?;
                let rows = stmt.query_map(params![org, site_id], row_rule)?;
                Ok(rows.collect::<rusqlite::Result<_>>()?)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::list_rules(self, org, site_id),
        }
    }

    /// Load one rule by name within a scope, applying the resolution precedence:
    /// a **site-scoped** rule (`site_id = Some`) wins, else the **org-level**
    /// (`site_id NULL`) rule of the same name. With `site_id` `None`, only the
    /// org-level rule matches. Fail-closed: a missing name is a
    /// [`StoreError::NotFound`] surfaced to the engine as a resolve error.
    pub fn load_rule(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        name: &str,
    ) -> Result<RuleRecord> {
        match &self.backend {
            Backend::Sqlite(_) => {
                if let Some(site_id) = site_id {
                    if let Some(rule) = self
                        .sqlite_conn()?
                        .query_row(
                            &format!(
                                "SELECT {RULE_COLS} FROM rules \
                                 WHERE org = ?1 AND site_id = ?2 AND name = ?3"
                            ),
                            params![org, site_id, name],
                            row_rule,
                        )
                        .optional()?
                    {
                        return Ok(rule);
                    }
                }
                // Fall back to the org-level rule.
                self.sqlite_conn()?
                    .query_row(
                        &format!(
                            "SELECT {RULE_COLS} FROM rules \
                             WHERE org = ?1 AND site_id IS NULL AND name = ?2"
                        ),
                        params![org, name],
                        row_rule,
                    )
                    .optional()?
                    .ok_or(StoreError::NotFound("rule"))
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::load_rule(self, org, site_id, name),
        }
    }

    /// Replace a rule's script and params (identified by org + site + name).
    /// Returns the updated record, or NotFound if the name does not exist in the
    /// scope.
    pub fn update_rule(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        name: &str,
        script: &str,
        params: &ParamSchema,
    ) -> Result<RuleRecord> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "UPDATE rules SET script = ?4, params = ?5 \
                     WHERE org = ?1 AND site_id IS ?2 AND name = ?3",
                    params![org, site_id, name, script, json_of(params)],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("rule"));
                }
                self.load_rule_exact(org, site_id, name)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::rules::update_rule(self, org, site_id, name, script, params)
            }
        }
    }

    /// Load one rule at an EXACT scope (no org fallback) — used after a scoped
    /// update/create to return the row that was written.
    pub fn load_rule_exact(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        name: &str,
    ) -> Result<RuleRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self
                .sqlite_conn()?
                .query_row(
                    &format!(
                        "SELECT {RULE_COLS} FROM rules \
                         WHERE org = ?1 AND site_id IS ?2 AND name = ?3"
                    ),
                    params![org, site_id, name],
                    row_rule,
                )
                .optional()?
                .ok_or(StoreError::NotFound("rule")),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => {
                super::postgres::rules::load_rule_exact(self, org, site_id, name)
            }
        }
    }

    /// Delete a rule at an exact scope. NotFound if it did not exist.
    pub fn delete_rule(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        name: &str,
    ) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "DELETE FROM rules WHERE org = ?1 AND site_id IS ?2 AND name = ?3",
                    params![org, site_id, name],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("rule"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::delete_rule(self, org, site_id, name),
        }
    }

    /// The rules in a scope whose scripts compose `name` — the design's
    /// change-impact listing. v1 resolves composition live by name, so editing a
    /// shared rule changes every rule built on it on the next tick; this listing
    /// makes that blast radius visible before an edit.
    ///
    /// Detection is a textual scan for a `rule("<name>", …)` call in each other
    /// rule's script (the composition primitive's only form). It can over-report
    /// a name that appears in a string literal but never under-reports a real
    /// call, which is the safe direction for a change-impact warning.
    pub fn referencing_rules(
        &self,
        org: &str,
        site_id: Option<Uuid>,
        name: &str,
    ) -> Result<Vec<RuleRecord>> {
        let all = self.list_rules(org, site_id)?;
        Ok(all
            .into_iter()
            .filter(|r| r.name != name && references(&r.script, name))
            .collect())
    }
}

/// Whether `script` contains a `rule("<name>", …)` composition call.
fn references(script: &str, name: &str) -> bool {
    for quote in ['"', '\''] {
        let needle = format!("rule({quote}{name}{quote}");
        // Allow optional whitespace after `(`: `rule( "name"`.
        if script.contains(&needle) || script.contains(&format!("rule( {quote}{name}{quote}")) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::references;

    #[test]
    fn detects_a_direct_composition_call() {
        assert!(references(
            r#"let x = rule("temp-high", df, #{});"#,
            "temp-high"
        ));
        assert!(references(r#"rule('temp-high', df, #{})"#, "temp-high"));
        assert!(references(r#"rule( "temp-high", df)"#, "temp-high"));
    }

    #[test]
    fn ignores_a_different_name() {
        assert!(!references(r#"rule("co2-stale", df)"#, "temp-high"));
        assert!(!references("finding(\"fault\", \"x\")", "temp-high"));
    }
}
