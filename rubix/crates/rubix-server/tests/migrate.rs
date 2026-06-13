//! Schema migration: an existing SQLite file on an older shape must evolve in
//! place when opened, never lose data, and never require deletion.

use chrono::Utc;
use rubix_server::auth::AdminLevel;
use rubix_server::store::{Store, TeamRecord, UserRecord};

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

/// v6 — dashboards gain a `variables` JSON column. A v5-shape DB has a
/// `dashboards` table without it; opening must add the column, keep the existing
/// board, and read it back with an empty variable list. Re-opening is a no-op.
#[test]
fn opening_a_v5_db_adds_dashboard_variables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v5.db");
    let dash_id = uuid::Uuid::new_v4();

    // A v5-shape dashboards table: the pre-v6 column set (no `variables`).
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE dashboards (
                id BLOB PRIMARY KEY, org TEXT NOT NULL, site_id BLOB,
                slug TEXT NOT NULL, title TEXT NOT NULL, created_at TEXT NOT NULL
            );
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dashboards VALUES (?1,'acme',NULL,'overview','Overview','2026-01-01T00:00:00Z')",
            rusqlite::params![dash_id],
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 5").unwrap();
    }

    let store = Store::open(&path).unwrap();
    let dashboards = store.list_dashboards("acme", None).unwrap();
    assert_eq!(dashboards.len(), 1, "the legacy dashboard is preserved");
    assert_eq!(dashboards[0].slug, "overview");
    assert!(
        dashboards[0].variables.is_empty(),
        "a legacy board reads back with no variables"
    );

    // Re-open is idempotent.
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.list_dashboards("acme", None).unwrap().len(), 1);
}

/// v7+v8 — a v6-shape DB (no `entity_tags` / `nav_nodes` tables) must gain them
/// on open, keep existing data, and let both new tables round-trip
/// (docs/design/page-context-and-nav.md §§3,4). Re-opening is a no-op.
#[test]
fn opening_a_v6_db_adds_entity_tags_and_nav_nodes() {
    use rubix_core::{EntityTags, NavNode, NavTarget};

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v6.db");
    let dash_id = uuid::Uuid::new_v4();

    // A v6-shape DB: dashboards with `variables`, but neither new table.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE dashboards (
                id BLOB PRIMARY KEY, org TEXT NOT NULL, site_id BLOB,
                slug TEXT NOT NULL, title TEXT NOT NULL, variables TEXT,
                created_at TEXT NOT NULL
            );
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO dashboards VALUES (?1,'acme',NULL,'overview','Overview',NULL,'2026-01-01T00:00:00Z')",
            rusqlite::params![dash_id],
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 6").unwrap();
    }

    let store = Store::open(&path).unwrap();
    // Pre-existing data survived.
    assert_eq!(store.list_dashboards("acme", None).unwrap().len(), 1);

    // entity_tags round-trips (an injection-shaped value binds, never executes).
    let mut tags = EntityTags::default();
    tags.0
        .insert("building".into(), Some("'); DROP TABLE dashboards; --".into()));
    store
        .replace_entity_tags("acme", "dashboard", dash_id, &tags)
        .unwrap();
    let read = store.entity_tags("acme", "dashboard", dash_id).unwrap();
    assert_eq!(read.0.get("building").unwrap().as_deref(), Some("'); DROP TABLE dashboards; --"));
    // The injected value was inert: the dashboard is still there.
    assert_eq!(store.list_dashboards("acme", None).unwrap().len(), 1);

    // nav_nodes round-trips.
    let node = NavNode {
        id: uuid::Uuid::new_v4(),
        org: "acme".into(),
        parent_id: None,
        title: "Buildings".into(),
        sort_order: 0,
        target: NavTarget::Group,
        context: None,
        icon: None,
        accent: None,
    };
    store.create_nav_node(&node).unwrap();
    assert_eq!(store.list_nav_nodes("acme").unwrap().len(), 1);

    // Re-open is idempotent.
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.list_nav_nodes("acme").unwrap().len(), 1);
    assert_eq!(store2.entity_tags("acme", "dashboard", dash_id).unwrap().0.len(), 1);
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

/// v4 — RBAC identity. A v3-shape DB (no users/teams/memberships/grants tables)
/// must gain them on open, keep existing data, stamp `user_version = 4`, and let
/// the new tables round-trip. Re-opening is a no-op.
#[test]
fn opening_a_v3_db_adds_rbac_identity_tables() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v3.db");
    let site_id = uuid::Uuid::new_v4();

    // A v3-shape DB: the base tables exist (sites here stands in for prior data)
    // but none of the v4 RBAC tables do. Stamp it explicitly at v3.
    {
        let conn = rusqlite::Connection::open(&path).unwrap();
        conn.execute_batch(
            "
            CREATE TABLE sites (
                id BLOB PRIMARY KEY, org TEXT NOT NULL, slug TEXT NOT NULL,
                display_name TEXT NOT NULL, tags TEXT NOT NULL, created_at TEXT NOT NULL,
                UNIQUE (org, slug)
            );
            ",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO sites VALUES (?1,'acme','hq','Acme HQ','{}','2026-01-01T00:00:00Z')",
            rusqlite::params![site_id],
        )
        .unwrap();
        conn.execute_batch("PRAGMA user_version = 3").unwrap();
    }

    let store = Store::open(&path).unwrap();

    // The pre-existing site survived (no destructive step).
    assert_eq!(store.list_sites(None).unwrap().len(), 1, "site preserved");

    // The new tables exist and round-trip. Seed a user + team + membership.
    let user_id = uuid::Uuid::new_v4();
    store
        .create_user(&UserRecord {
            id: user_id,
            org: "acme".into(),
            subject: "sub-1".into(),
            email: "a@acme.test".into(),
            display_name: "Admin".into(),
            admin_level: AdminLevel::OrgAdmin,
            created_at: Utc::now(),
        })
        .unwrap();
    let team_id = uuid::Uuid::new_v4();
    store
        .create_team(&TeamRecord {
            id: team_id,
            org: "acme".into(),
            slug: "ops".into(),
            name: "Ops".into(),
            created_at: Utc::now(),
        })
        .unwrap();
    store.add_team_member(team_id, user_id).unwrap();

    assert_eq!(store.list_users("acme").unwrap().len(), 1);
    assert_eq!(store.team_ids_for_user(user_id).unwrap(), vec![team_id]);
    assert_eq!(
        store.user_by_subject("sub-1").unwrap().unwrap().admin_level,
        AdminLevel::OrgAdmin
    );

    // `user_version` was bumped to the latest (4) — re-open is a clean no-op and
    // the seeded rows persist.
    let store2 = Store::open(&path).unwrap();
    assert_eq!(store2.list_users("acme").unwrap().len(), 1);
    assert_eq!(store2.list_teams("acme").unwrap().len(), 1);
}
