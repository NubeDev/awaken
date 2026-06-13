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
    Migration {
        version: 5,
        step: migrate_v5_prefs,
    },
    Migration {
        version: 6,
        step: migrate_v6_dashboard_variables,
    },
    Migration {
        version: 7,
        step: migrate_v7_entity_tags,
    },
    Migration {
        version: 8,
        step: migrate_v8_nav_nodes,
    },
    Migration {
        version: 9,
        step: migrate_v9_change_ledger,
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

/// v5 — units & datetime preferences (WS-11): `prefs_org` + `prefs_user`. New
/// tables only (no existing data to migrate), so pure `CREATE TABLE IF NOT
/// EXISTS` — a no-op on a fresh database (the base schema already created them)
/// and additive on a legacy one. Shapes mirror [`super::schema::SCHEMA_SQLITE`]
/// exactly; keep them in sync.
fn migrate_v5_prefs(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS prefs_org (
            org              TEXT PRIMARY KEY,
            timezone         TEXT,
            locale           TEXT,
            language         TEXT,
            unit_system      TEXT,
            temperature_unit TEXT,
            pressure_unit    TEXT,
            speed_unit       TEXT,
            length_unit      TEXT,
            mass_unit        TEXT,
            date_format      TEXT,
            time_format      TEXT,
            week_start       TEXT,
            number_format    TEXT,
            currency         TEXT,
            updated_at       INTEGER NOT NULL
        );
        CREATE TABLE IF NOT EXISTS prefs_user (
            user_id          TEXT NOT NULL,
            org              TEXT NOT NULL,
            timezone         TEXT,
            locale           TEXT,
            language         TEXT,
            unit_system      TEXT,
            temperature_unit TEXT,
            pressure_unit    TEXT,
            speed_unit       TEXT,
            length_unit      TEXT,
            mass_unit        TEXT,
            date_format      TEXT,
            time_format      TEXT,
            week_start       TEXT,
            number_format    TEXT,
            currency         TEXT,
            theme            TEXT,
            updated_at       INTEGER NOT NULL,
            PRIMARY KEY (user_id, org)
        );
        ",
    )
}

/// v6 — dashboards gain a `variables` JSON column carrying the dashboard's
/// variable model (docs/design/variables-and-templating.md §1). Nullable, so an
/// existing board reads back with an empty variable list and behaves exactly as
/// before. A no-op on a fresh database (column present in the base schema).
fn migrate_v6_dashboard_variables(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    if !column_exists(tx, "dashboards", "variables")? {
        tx.execute_batch("ALTER TABLE dashboards ADD COLUMN variables TEXT")?;
    }
    Ok(())
}

/// v7 — behaviour-affecting entity tags (docs/design/page-context-and-nav.md §3):
/// the org-scoped `entity_tags` table. A new table only (no existing data to
/// migrate), so pure `CREATE TABLE IF NOT EXISTS` — a no-op on a fresh database
/// (the base schema already created it) and additive on a legacy one. Shape
/// mirrors [`super::schema::SCHEMA_SQLITE`] exactly; keep them in sync.
fn migrate_v7_entity_tags(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS entity_tags (
            org       TEXT NOT NULL,
            kind      TEXT NOT NULL,
            entity_id TEXT NOT NULL,
            key       TEXT NOT NULL,
            value     TEXT,
            PRIMARY KEY (org, kind, entity_id, key)
        );
        CREATE INDEX IF NOT EXISTS idx_entity_tags_reverse
            ON entity_tags (org, kind, key, value);
        ",
    )
}

/// v8 — the navigation tree (docs/design/page-context-and-nav.md §4): the
/// org-scoped, nestable `nav_nodes` table. A new table only, so pure `CREATE
/// TABLE IF NOT EXISTS` — a no-op on a fresh database and additive on a legacy
/// one. Shape mirrors [`super::schema::SCHEMA_SQLITE`] exactly; keep them in sync.
fn migrate_v8_nav_nodes(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS nav_nodes (
            id         TEXT PRIMARY KEY,
            org        TEXT NOT NULL,
            parent_id  TEXT REFERENCES nav_nodes(id) ON DELETE CASCADE,
            title      TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            target     TEXT NOT NULL,
            context    TEXT,
            icon       TEXT,
            accent     TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_nav_nodes_tree
            ON nav_nodes (org, parent_id, sort_order);
        ",
    )
}

/// v9 — the append-only change ledger (docs/design/audit-and-undo.md "The
/// substrate"): the org-scoped `changes` table and the per-actor `undo_cursors`
/// table. New tables only (no existing data to migrate), so pure `CREATE TABLE IF
/// NOT EXISTS` — a no-op on a fresh database (the base schema already created them)
/// and additive on a legacy one. Shapes mirror [`super::schema::SCHEMA_SQLITE`]
/// exactly; keep them in sync.
fn migrate_v9_change_ledger(tx: &rusqlite::Transaction<'_>) -> rusqlite::Result<()> {
    tx.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS changes (
            id          TEXT PRIMARY KEY,
            at          TEXT NOT NULL,
            org         TEXT NOT NULL,
            site_id     TEXT,
            actor       TEXT NOT NULL,
            kind        TEXT NOT NULL,
            resource_id TEXT NOT NULL,
            op          TEXT NOT NULL,
            before      TEXT,
            after       TEXT,
            group_id    TEXT NOT NULL,
            correlation TEXT
        );
        CREATE TABLE IF NOT EXISTS undo_cursors (
            org        TEXT NOT NULL,
            subject    TEXT NOT NULL,
            redo_stack TEXT NOT NULL DEFAULT '[]',
            epoch      INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (org, subject)
        );
        CREATE INDEX IF NOT EXISTS idx_changes_org_at ON changes (org, at DESC, id DESC);
        CREATE INDEX IF NOT EXISTS idx_changes_resource
            ON changes (org, kind, resource_id, at DESC);
        CREATE INDEX IF NOT EXISTS idx_changes_group ON changes (group_id);
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
