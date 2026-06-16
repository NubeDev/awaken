//! Demo dashboards, variables, and a navigation tree per tenant.
//!
//! Makes [VARIABLES-AND-TEMPLATING.md](../../../docs/design/VARIABLES-AND-TEMPLATING.md)
//! and [PAGE-CONTEXT-AND-NAV.md](../../../docs/design/PAGE-CONTEXT-AND-NAV.md)
//! concrete: one **templated** chart (`$site` in its SQL), one board that carries a
//! `site` variable and mounts the chart, and a navigation tree that mounts that one
//! board under each of the tenant's sites with a different `context.values.site` —
//! the fleet story (one board, many places) in seed data. A small default tree of
//! static `route` nodes ships too, so the sidebar has a nav to build from out of the
//! box.
//!
//! All three are ordinary `kind:` records written through the gate as the tenant
//! operator (`IngestPublish`), exactly as the UI would write them over
//! `POST /records`; nothing here needs a bespoke table or route.

use rubix_core::{Id, Principal};
use rubix_gate::{Capability, Change, Command, apply};
use serde_json::{Value, json};
use surrealdb::Surreal;
use surrealdb::engine::local::Db;

use super::SeedError;

/// The static pages mounted as a default `route` tree (a slice of the router's
/// built-in pages). Kept small but representative; the allow-list itself is the
/// UI's router table (`PAGE-CONTEXT-AND-NAV.md` §4).
const DEFAULT_ROUTES: &[(&str, &str)] = &[
    ("home", "Home"),
    ("devices", "Devices"),
    ("dashboards", "Dashboards"),
    ("datasources", "Datasources"),
    ("rules", "Rules"),
    ("audit", "Audit"),
];

/// Seed the demo board, its templated chart, and the navigation tree for one
/// tenant, returning the number of records written.
///
/// `sites` is the tenant's `(key, name)` list; the board is mounted once per site
/// under a "Buildings" group so the same board serves each site through its
/// `context.values.site`.
pub async fn seed_dashboards(
    db: &Surreal<Db>,
    namespace: &str,
    operator: &Principal,
    sites: &[(&str, &str)],
) -> Result<usize, SeedError> {
    let mut count = 0;

    // A templated chart: "points per equip at the selected $site". The `$site`
    // reference is lowered server-side into an escaped literal at query time — the
    // injection boundary — so one chart serves every site.
    let chart_id = nav_id(namespace, "chart", "points-by-equip");
    put(
        db,
        operator,
        &chart_id,
        json!({
            "kind": "chart",
            "name": "Points by equipment",
            "sql": "SELECT json_get(json_get(content, 'content'), 'equip') AS equip, \
                    count(*) AS n FROM record \
                    WHERE json_get(json_get(content, 'content'), 'kind') = 'point' \
                    AND json_get(json_get(content, 'content'), 'site') = $site \
                    GROUP BY equip ORDER BY n DESC",
            "config": { "type": "horizontalBar", "x": "n", "y": "equip", "displayMode": "total" },
        }),
    )
    .await?;
    count += 1;

    // The board carries a `site` variable (its default selection is the first site)
    // and mounts the chart in one panel. `content.variables` is the board-JSON model
    // from VARIABLES-AND-TEMPLATING §1; a node's context overrides the default.
    let first_site = sites.first().map_or("", |(key, _)| *key);
    let board_id = nav_id(namespace, "board", "site-overview");
    put(
        db,
        operator,
        &board_id,
        json!({
            "kind": "board",
            "name": "Site Overview",
            "variables": [
                {
                    "name": "site",
                    "label": "Site",
                    "kind": "site",
                    "config": {},
                    "current": first_site,
                    "multi": false,
                    "include_all": false,
                    "hidden": false,
                }
            ],
            "panels": [ { "chart_id": board_id_ref(&chart_id), "x": 0, "y": 0, "w": 12, "h": 8 } ],
        }),
    )
    .await?;
    count += 1;

    // Default tree: a "Console" group over the static route nodes, so a fresh tenant
    // has a navigation to render and extend (per-node access is a documented
    // follow-up — every node is namespace-visible for now).
    let console_id = nav_id(namespace, "nav", "console");
    put(db, operator, &console_id, group_node("Console", 0)).await?;
    count += 1;
    for (order, (route, title)) in DEFAULT_ROUTES.iter().enumerate() {
        let node_id = nav_id(namespace, "nav", &format!("route-{route}"));
        put(
            db,
            operator,
            &node_id,
            route_node(&console_id, title, route, order as i64),
        )
        .await?;
        count += 1;
    }

    // The fleet story: a "Buildings" group, then one board mount per site, each
    // binding the *same* board to that site via context.values.site.
    let buildings_id = nav_id(namespace, "nav", "buildings");
    put(db, operator, &buildings_id, group_node("Buildings", 1)).await?;
    count += 1;
    for (order, (site_key, site_name)) in sites.iter().enumerate() {
        let node_id = nav_id(namespace, "nav", &format!("site-{site_key}"));
        put(
            db,
            operator,
            &node_id,
            board_mount(&buildings_id, site_name, &board_id, site_key, order as i64),
        )
        .await?;
        count += 1;
    }

    Ok(count)
}

/// A `group` nav node (a non-clickable header).
fn group_node(title: &str, sort_order: i64) -> Value {
    json!({
        "kind": "nav_node",
        "parent": Value::Null,
        "title": title,
        "sort_order": sort_order,
        "target": { "kind": "group" },
    })
}

/// A `route` nav node mounting a static built-in page under `parent`.
fn route_node(parent: &Id, title: &str, route: &str, sort_order: i64) -> Value {
    json!({
        "kind": "nav_node",
        "parent": parent.to_string(),
        "title": title,
        "sort_order": sort_order,
        "target": { "kind": "route", "route": route },
    })
}

/// A `board` nav node mounting `board` under `parent`, bound to `site` via context.
fn board_mount(parent: &Id, title: &str, board: &Id, site: &str, sort_order: i64) -> Value {
    json!({
        "kind": "nav_node",
        "parent": parent.to_string(),
        "title": title,
        "sort_order": sort_order,
        "target": { "kind": "board", "board": board.to_string() },
        "context": { "values": { "site": site }, "tags": {} },
    })
}

/// A deterministic, namespace-prefixed record id (`{ns}--{kind}--{slug}`).
fn nav_id(namespace: &str, kind: &str, slug: &str) -> Id {
    Id::from_raw(format!("{namespace}--{kind}--{slug}"))
}

/// A board panel references a chart by its bare record id string.
fn board_id_ref(chart: &Id) -> String {
    chart.to_string()
}

/// Create `content` at `target` through the gate as the tenant operator.
async fn put(
    db: &Surreal<Db>,
    operator: &Principal,
    target: &Id,
    content: Value,
) -> Result<(), SeedError> {
    let command = Command::new(
        operator.clone(),
        Capability::IngestPublish,
        target.clone(),
        Change::Create(content),
    );
    apply(db, &command, None)
        .await
        .map(|_| ())
        .map_err(|e| SeedError::new("write dashboard record", e))
}
