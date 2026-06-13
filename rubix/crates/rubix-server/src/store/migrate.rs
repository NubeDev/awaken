//! Forward-only SQLite schema migrations.
//!
//! The base schema ([`super::schema::SCHEMA_SQLITE`]) is `CREATE TABLE IF NOT
//! EXISTS`, which establishes a *fresh* database but cannot evolve an existing
//! one — `IF NOT EXISTS` is a no-op against a table that already exists, so a
//! new column never lands on an old file. That is what silently dropped data on
//! schema changes: the only way to pick up a new column was to delete the file.
//!
//! This module closes that gap. Each migration is an ordered, idempotent step
//! guarded by `PRAGMA user_version`; on open we run every step whose version is
//! above the file's current `user_version`, then stamp the new version. A fresh
//! database (created by the base schema at the latest shape) is simply stamped
//! to `LATEST` with no steps to run.

use rusqlite::Connection;

/// One ordered migration: a version stamp and the step that takes the schema to
/// it. The step is a closure (not raw SQL) so it can inspect the live schema and
/// stay idempotent — e.g. add a column only when it is actually missing, which
/// lets the same step be a no-op on a database the base schema already created
/// at the newest shape.
struct Migration {
    version: i64,
    step: fn(&rusqlite::Transaction<'_>) -> rusqlite::Result<()>,
}

/// The migration ladder, in ascending version order. Append new steps; never
/// edit or reorder a shipped one.
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        step: migrate_v1_dashboards,
    },
    Migration {
        version: 2,
        step: migrate_v2_widget_settings,
    },
    Migration {
        version: 3,
        step: migrate_v3_board_rule_scope,
    },
    Migration {
        version: 4,
        step: migrate_v4_rbac_identity,
    },
];

/// v1 — dashboards as a first-class entity. Widgets gain `dashboard_id` and hang
/// off a dashboard rather than directly off a site. The base schema already
/// creates the `dashboards` table; this adds the widget column when an older
/// file lacks it and backfills every orphaned widget into a per-site "default"
/// dashboard so nothing is lost. A no-op on a fresh database (column present).
fn migrate_v1_dashboards(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    if !column_exists(tx, "widgets", "dashboard_id")? {
        tx.execute_batch(
            "ALTER TABLE widgets \
             ADD COLUMN dashboard_id TEXT REFERENCES dashboards(id) ON DELETE CASCADE",
        )?;
    }
    // `query` carries operator-authored SQL for datasource widgets; an older
    // file predates it.
    if !column_exists(tx, "widgets", "query")? {
        tx.execute_batch("ALTER TABLE widgets ADD COLUMN query TEXT")?;
    }
    tx.execute_batch(
        "
        -- One default dashboard per site that still has orphaned widgets. The id
        -- is a 16-byte blob, matching how the store binds `Uuid` columns: the
        -- rusqlite `uuid` feature stores UUIDs as BLOB, so a text id would not
        -- decode back through `row_dashboard`/`row_widget`.
        INSERT INTO dashboards (id, org, site_id, slug, title, created_at)
        SELECT
            randomblob(16),
            s.org, s.id, 'default', 'Default', strftime('%Y-%m-%dT%H:%M:%fZ','now')
        FROM sites s
        WHERE EXISTS (SELECT 1 FROM widgets w WHERE w.site_id = s.id AND w.dashboard_id IS NULL)
          AND NOT EXISTS (SELECT 1 FROM dashboards d WHERE d.site_id = s.id AND d.slug = 'default');

        -- Point every orphaned widget at its site's default dashboard.
        UPDATE widgets
        SET dashboard_id = (
            SELECT d.id FROM dashboards d
            WHERE d.site_id = widgets.site_id AND d.slug = 'default'
        )
        WHERE dashboard_id IS NULL;

        -- The dashboard_id index lives here (not the base schema) because it
        -- references a column a legacy file only gains in this step.
        CREATE INDEX IF NOT EXISTS idx_widgets_dashboard ON widgets (dashboard_id, created_at DESC);
        ",
    )
}

/// v2 — widgets gain a `settings` JSON column carrying grid layout + chart
/// config. Nullable, so every existing tile reads back as `settings: None` and
/// the canvas falls back to auto-flow + default rendering. A no-op on a fresh
/// database (column present in the base schema).
fn migrate_v2_widget_settings(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    if !column_exists(tx, "widgets", "settings")? {
        tx.execute_batch("ALTER TABLE widgets ADD COLUMN settings TEXT")?;
    }
    Ok(())
}

/// v3 — flows (boards) and rules gain the uniform `org` + optional `site_id`
/// scope (matching dashboards): `site_id` set → that site, NULL → org-level
/// applying across the org. Boards were previously global; their existing rows
/// carry no derivable scope, so they are dropped (the operator re-creates flows
/// under a scope). Rules already had `org`; they gain `site_id` defaulting to
/// NULL (org-level), so no rule is lost. Idempotent via `column_exists`.
fn migrate_v3_board_rule_scope(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    // Boards: the old table carries a table-level `UNIQUE(slug, version)` that
    // SQLite cannot drop via ALTER, and it would block two scopes sharing a
    // slug. Existing rows are unscopable global junk, so recreate the table at
    // the new shape outright (drop + create) rather than alter in place.
    if !column_exists(tx, "boards", "org")? {
        tx.execute_batch(
            "
            DROP TABLE IF EXISTS boards;
            CREATE TABLE boards (
                id           TEXT PRIMARY KEY,
                org          TEXT NOT NULL,
                site_id      TEXT REFERENCES sites(id) ON DELETE CASCADE,
                slug         TEXT NOT NULL,
                version      INTEGER NOT NULL,
                display_name TEXT NOT NULL,
                enabled      INTEGER NOT NULL DEFAULT 1,
                trigger      TEXT NOT NULL,
                graph        TEXT NOT NULL,
                created_at   TEXT NOT NULL
            );
            ",
        )?;
    }
    // Rules: gain site_id (NULL = org-level) while PRESERVING existing rows. The
    // old `UNIQUE(org, name)` table constraint spans all rows regardless of
    // site, which would block a site rule sharing a name with an org rule —
    // exactly the override the new model wants. SQLite can't drop a table
    // constraint via ALTER, so rebuild the table and copy the rows over (each
    // existing rule becomes org-level, site_id NULL). Detect the legacy shape by
    // the absent column.
    if !column_exists(tx, "rules", "site_id")? {
        tx.execute_batch(
            "
            ALTER TABLE rules RENAME TO rules_old;
            CREATE TABLE rules (
                id          TEXT PRIMARY KEY,
                org         TEXT NOT NULL,
                site_id     TEXT REFERENCES sites(id) ON DELETE CASCADE,
                name        TEXT NOT NULL,
                script      TEXT NOT NULL,
                params      TEXT NOT NULL,
                created_at  TEXT NOT NULL
            );
            INSERT INTO rules (id, org, site_id, name, script, params, created_at)
                SELECT id, org, NULL, name, script, params, created_at FROM rules_old;
            DROP TABLE rules_old;
            ",
        )?;
    }
    // Scope indexes (idempotent).
    tx.execute_batch(
        "
        CREATE UNIQUE INDEX IF NOT EXISTS idx_boards_org_slug_ver
            ON boards (org, slug, version) WHERE site_id IS NULL;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_boards_site_slug_ver
            ON boards (org, site_id, slug, version) WHERE site_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_boards_org ON boards (org, slug, version DESC);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_rules_org_name
            ON rules (org, name) WHERE site_id IS NULL;
        CREATE UNIQUE INDEX IF NOT EXISTS idx_rules_site_name
            ON rules (org, site_id, name) WHERE site_id IS NOT NULL;
        CREATE INDEX IF NOT EXISTS idx_rules_org ON rules (org, name);
        ",
    )
}

/// v4 — RBAC identity + ACL: `users`, `teams`, `memberships`, and `grants`.
/// These are new tables only (no existing data to migrate), so the step is pure
/// `CREATE TABLE IF NOT EXISTS` — a no-op on a fresh database (the base schema
/// already created them) and additive on a legacy one. The shapes mirror
/// [`super::schema::SCHEMA_SQLITE`] exactly; keep them in sync. See
/// `docs/design/authz-rbac.md`.
fn migrate_v4_rbac_identity(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS users (
            id           TEXT PRIMARY KEY,
            org          TEXT NOT NULL,
            subject      TEXT NOT NULL,
            email        TEXT NOT NULL,
            display_name TEXT NOT NULL,
            admin_level  TEXT NOT NULL DEFAULT 'none',
            created_at   TEXT NOT NULL,
            UNIQUE (subject),
            UNIQUE (org, email)
        );
        CREATE TABLE IF NOT EXISTS teams (
            id         TEXT PRIMARY KEY,
            org        TEXT NOT NULL,
            slug       TEXT NOT NULL,
            name       TEXT NOT NULL,
            created_at TEXT NOT NULL,
            UNIQUE (org, slug)
        );
        CREATE TABLE IF NOT EXISTS memberships (
            user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
            team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
            PRIMARY KEY (user_id, team_id)
        );
        CREATE TABLE IF NOT EXISTS grants (
            id            TEXT PRIMARY KEY,
            org           TEXT NOT NULL,
            subject_kind  TEXT NOT NULL,
            subject_id    TEXT NOT NULL,
            resource_kind TEXT NOT NULL,
            resource_ref  TEXT NOT NULL,
            permission    TEXT NOT NULL,
            created_at    TEXT NOT NULL,
            UNIQUE (org, subject_kind, subject_id, resource_kind, resource_ref, permission)
        );
        CREATE INDEX IF NOT EXISTS idx_users_org ON users (org, email);
        CREATE INDEX IF NOT EXISTS idx_teams_org ON teams (org, slug);
        CREATE INDEX IF NOT EXISTS idx_memberships_team ON memberships (team_id);
        CREATE INDEX IF NOT EXISTS idx_grants_subject ON grants (org, subject_kind, subject_id);
        ",
    )
}

/// True when `table` has a column named `column`, read from `PRAGMA
/// table_info` — the portable way to make a column-add migration idempotent.
fn column_exists(
    conn: &Connection,
    table: &str,
    column: &str,
) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

/// The highest known schema version — the shape the base schema and the code
/// expect.
fn latest() -> i64 {
    MIGRATIONS.last().map(|m| m.version).unwrap_or(0)
}

/// Bring `conn` up to [`latest`]. Runs every migration above the file's
/// `user_version`, each in its own transaction, stamping the version as it goes.
/// Idempotent: a database already at or above latest does nothing, and each step
/// is written to be safe to re-run (so a fresh DB at the newest shape just
/// stamps the version without changing anything).
pub(crate) fn run(conn: &mut Connection) -> rusqlite::Result<()> {
    let current: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if current >= latest() {
        return Ok(());
    }
    for m in MIGRATIONS.iter().filter(|m| m.version > current) {
        let tx = conn.transaction()?;
        (m.step)(&tx)?;
        // PRAGMA cannot bind parameters; the version is a trusted constant.
        tx.execute_batch(&format!("PRAGMA user_version = {}", m.version))?;
        tx.commit()?;
    }
    Ok(())
}
