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
    /// Tenant scope. `name` is unique within an org.
    pub org: String,
    /// Composition name (`rule("temp-high", …)`), unique per org.
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

pub(crate) const RULE_COLS: &str = "id, org, name, script, params, created_at";

fn row_rule(row: &Row<'_>) -> rusqlite::Result<RuleRecord> {
    Ok(RuleRecord {
        id: row.get(0)?,
        org: row.get(1)?,
        name: row.get(2)?,
        script: row.get(3)?,
        params: json_to(&row.get::<_, String>(4)?)?,
        created_at: ts_to(&row.get::<_, String>(5)?)?,
    })
}

impl Store {
    /// Insert a rule. The `(org, name)` UNIQUE constraint rejects a duplicate
    /// name within the org as a [`StoreError::Conflict`].
    pub fn create_rule(&self, rule: &RuleRecord) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                self.sqlite_conn()?.execute(
                    "INSERT INTO rules (id, org, name, script, params, created_at) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                    params![
                        rule.id,
                        rule.org,
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

    /// Every rule in an org, newest first.
    pub fn list_rules(&self, org: &str) -> Result<Vec<RuleRecord>> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let conn = self.sqlite_conn()?;
                let mut stmt = conn.prepare(&format!(
                    "SELECT {RULE_COLS} FROM rules WHERE org = ?1 ORDER BY created_at DESC"
                ))?;
                let rows = stmt.query_map(params![org], row_rule)?;
                Ok(rows.collect::<rusqlite::Result<_>>()?)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::list_rules(self, org),
        }
    }

    /// Load one rule by org + name. The fail-closed resolution path: a missing
    /// name is a [`StoreError::NotFound`], surfaced to the engine as a resolve
    /// error.
    pub fn load_rule(&self, org: &str, name: &str) -> Result<RuleRecord> {
        match &self.backend {
            Backend::Sqlite(_) => self
                .sqlite_conn()?
                .query_row(
                    &format!("SELECT {RULE_COLS} FROM rules WHERE org = ?1 AND name = ?2"),
                    params![org, name],
                    row_rule,
                )
                .optional()?
                .ok_or(StoreError::NotFound("rule")),
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::load_rule(self, org, name),
        }
    }

    /// Replace a rule's script and params (identified by org + name). Returns the
    /// updated record, or NotFound if the name does not exist in the org.
    pub fn update_rule(
        &self,
        org: &str,
        name: &str,
        script: &str,
        params: &ParamSchema,
    ) -> Result<RuleRecord> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self.sqlite_conn()?.execute(
                    "UPDATE rules SET script = ?3, params = ?4 WHERE org = ?1 AND name = ?2",
                    params![org, name, script, json_of(params)],
                )?;
                if n == 0 {
                    return Err(StoreError::NotFound("rule"));
                }
                self.load_rule(org, name)
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::update_rule(self, org, name, script, params),
        }
    }

    /// Delete a rule by org + name. NotFound if it did not exist.
    pub fn delete_rule(&self, org: &str, name: &str) -> Result<()> {
        match &self.backend {
            Backend::Sqlite(_) => {
                let n = self
                    .sqlite_conn()?
                    .execute("DELETE FROM rules WHERE org = ?1 AND name = ?2", params![org, name])?;
                if n == 0 {
                    return Err(StoreError::NotFound("rule"));
                }
                Ok(())
            }
            #[cfg(feature = "cloud")]
            Backend::Postgres(_) => super::postgres::rules::delete_rule(self, org, name),
        }
    }

    /// The rules in `org` whose scripts compose `name` — the design's
    /// change-impact listing. v1 resolves composition live by name, so editing a
    /// shared rule changes every rule built on it on the next tick; this listing
    /// makes that blast radius visible before an edit.
    ///
    /// Detection is a textual scan for a `rule("<name>", …)` call in each other
    /// rule's script (the composition primitive's only form). It can over-report
    /// a name that appears in a string literal but never under-reports a real
    /// call, which is the safe direction for a change-impact warning.
    pub fn referencing_rules(&self, org: &str, name: &str) -> Result<Vec<RuleRecord>> {
        let all = self.list_rules(org)?;
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
        assert!(references(r#"let x = rule("temp-high", df, #{});"#, "temp-high"));
        assert!(references(r#"rule('temp-high', df, #{})"#, "temp-high"));
        assert!(references(r#"rule( "temp-high", df)"#, "temp-high"));
    }

    #[test]
    fn ignores_a_different_name() {
        assert!(!references(r#"rule("co2-stale", df)"#, "temp-high"));
        assert!(!references("finding(\"fault\", \"x\")", "temp-high"));
    }
}
