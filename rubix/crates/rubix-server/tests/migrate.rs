//! Schema migration: an existing SQLite file on an older shape must evolve in
//! place when opened, never lose data, and never require deletion.

use rubix_server::store::Store;

/// Build a pre-dashboards database by hand (the v0 shape: `widgets` with no
/// `dashboard_id`, no `dashboards` table), seed a site and a widget, then open
/// it through `Store`. The open must add the column, create the dashboards
/// table, backfill the widget onto a default dashboard, and keep the row.
#[test]
fn opening_a_legacy_db_migrates_widgets_onto_a_default_dashboard() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("legacy.db");

    // UUIDs are bound through rusqlite's `uuid` type (stored as BLOB) so they
    // round-trip exactly as the real store reads them.
    let site_id = uuid::Uuid::new_v4();
    let widget_id = uuid::Uuid::new_v4();

    // v0 schema: the minimal old shape this migration targets (no dashboards
    // table, widgets without dashboard_id/query/settings).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE sites (
                id BLOB PRIMARY KEY, org TEXT NOT NULL, slug TEXT NOT NULL,
                display_name TEXT NOT NULL, tags TEXT NOT NULL, created_at TEXT NOT NULL,
                UNIQUE (org, slug)
            );
            CREATE TABLE widgets (
                id BLOB PRIMARY KEY,
                site_id BLOB NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
                kind TEXT NOT NULL, title TEXT NOT NULL, target TEXT NOT NULL,
                created_at TEXT NOT NULL
            );
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sites VALUES (?1,'kfc','hq','KFC HQ','{}','2026-01-01T00:00:00Z')",
            rusqlite::params![site_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO widgets VALUES (?1,?2,'point_value','Fan','kfc/hq/ahu-3/fan','2026-01-01T00:00:00Z')",
            rusqlite::params![widget_id, site_id],
        )
        .unwrap();
        // Explicitly v0 — the migration ladder must lift it.
        conn.execute_batch("PRAGMA user_version = 0").unwrap();
    }

    // Open through the store: base schema + migrations run here.
    let store = Store::open(&path).unwrap();

    // The widget survived and now hangs off a dashboard.
    let widgets = store.list_widgets(None, None).unwrap();
    assert_eq!(widgets.len(), 1, "the legacy widget must be preserved");
    let w = &widgets[0];
    assert_eq!(w.title, "Fan");
    assert_eq!(w.target, "kfc/hq/ahu-3/fan");
    // v2 added the `settings` column; a legacy tile reads back unset (the
    // canvas auto-flows it). If v2 had not run, decoding would have failed.
    assert!(w.settings.is_none(), "legacy tile has no settings");

    // A default dashboard was created for the site and owns the widget.
    let dashboards = store.list_dashboards("kfc", None).unwrap();
    assert_eq!(dashboards.len(), 1, "a default dashboard was backfilled");
    let d = &dashboards[0];
    assert_eq!(d.slug, "default");
    assert_eq!(d.site_id, Some(w.site_id));
    assert_eq!(w.dashboard_id, d.id, "widget points at the default dashboard");

    // Re-opening is idempotent: no duplicate dashboards, version stays stamped.
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.list_dashboards("kfc", None).unwrap().len(), 1);
    assert_eq!(store2.list_widgets(None, None).unwrap().len(), 1);
}

/// A fresh database (base schema already at the newest shape) opens, stamps the
/// version, and runs no destructive step.
#[test]
fn opening_a_fresh_db_is_clean() {
    let dir = tempfile::tempdir().unwrap();
    let store = Store::open(&dir.path().join("fresh.db")).unwrap();
    assert!(store.list_widgets(None, None).unwrap().is_empty());
    assert!(store.list_dashboards("anyorg", None).unwrap().is_empty());
}

/// v3 — flows and rules gain org+site scope. A legacy DB has global boards (no
/// scope) and org-only rules; opening must drop the unscopable boards, keep the
/// rules as org-level (site_id NULL), and let two scopes share a slug/name.
#[test]
fn opening_a_v2_db_scopes_flows_and_rules() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v2.db");

    // UUIDs bound through rusqlite's `uuid` type (BLOB) so they decode back.
    let board_id = uuid::Uuid::new_v4();
    let rule_id = uuid::Uuid::new_v4();

    // Build a v2-shape DB by hand: global boards table + org-only rules.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE boards (
                id BLOB PRIMARY KEY, slug TEXT NOT NULL, version INTEGER NOT NULL,
                display_name TEXT NOT NULL, enabled INTEGER NOT NULL DEFAULT 1,
                trigger TEXT NOT NULL, graph TEXT NOT NULL, created_at TEXT NOT NULL,
                UNIQUE (slug, version)
            );
            CREATE TABLE rules (
                id BLOB PRIMARY KEY, org TEXT NOT NULL, name TEXT NOT NULL,
                script TEXT NOT NULL, params TEXT NOT NULL, created_at TEXT NOT NULL,
                UNIQUE (org, name)
            );
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO boards VALUES (?1,'junk',1,'Junk',1,'{\"kind\":\"manual\"}','{\"nodes\":[],\"connections\":[]}','2026-01-01T00:00:00Z')",
            rusqlite::params![board_id],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO rules VALUES (?1,'kfc','temp-high','40','{}','2026-01-01T00:00:00Z')",
            rusqlite::params![rule_id],
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 2").unwrap();
    }

    let store = Store::open(&path).unwrap();

    // Boards: the global junk row was dropped.
    assert!(
        store.latest_boards_all().unwrap().is_empty(),
        "legacy global boards are dropped"
    );

    // Rules: preserved as org-level (site_id NULL), resolvable by org.
    let rules = store.list_rules("kfc", None).unwrap();
    assert_eq!(rules.len(), 1, "existing rule preserved");
    assert_eq!(rules[0].name, "temp-high");
    assert!(rules[0].site_id.is_none(), "migrated rule is org-level");
    assert_eq!(
        store.load_rule("kfc", None, "temp-high").unwrap().script,
        "40"
    );

    // Re-open is idempotent (version stays at latest, no error).
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.list_rules("kfc", None).unwrap().len(), 1);
}
