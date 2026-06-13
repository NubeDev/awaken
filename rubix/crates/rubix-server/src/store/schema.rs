pub(crate) const SCHEMA_SQLITE: &str = "
CREATE TABLE IF NOT EXISTS sites (
    id           TEXT PRIMARY KEY,
    org          TEXT NOT NULL,
    slug         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    tags         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (org, slug)
);
CREATE TABLE IF NOT EXISTS equips (
    id           TEXT PRIMARY KEY,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    tags         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (site_id, path)
);
CREATE TABLE IF NOT EXISTS points (
    id             TEXT PRIMARY KEY,
    equip_id       TEXT NOT NULL REFERENCES equips(id) ON DELETE CASCADE,
    slug           TEXT NOT NULL,
    display_name   TEXT NOT NULL,
    kind           TEXT NOT NULL,
    unit           TEXT,
    tags           TEXT NOT NULL,
    priority_array TEXT NOT NULL,
    cur_value      TEXT,
    cur_ts         TEXT,
    created_at     TEXT NOT NULL,
    UNIQUE (equip_id, slug)
);
CREATE TABLE IF NOT EXISTS his (
    point_id TEXT NOT NULL REFERENCES points(id) ON DELETE CASCADE,
    ts       TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (point_id, ts)
);
CREATE TABLE IF NOT EXISTS sparks (
    id           TEXT PRIMARY KEY,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    rule         TEXT NOT NULL,
    severity     TEXT NOT NULL,
    message      TEXT NOT NULL,
    point_ids    TEXT NOT NULL,
    ts           TEXT NOT NULL,
    acknowledged INTEGER NOT NULL DEFAULT 0
);
CREATE TABLE IF NOT EXISTS boards (
    id          TEXT PRIMARY KEY,
    org         TEXT NOT NULL,
    site_id     TEXT REFERENCES sites(id) ON DELETE CASCADE,
    slug        TEXT NOT NULL,
    version     INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    trigger     TEXT NOT NULL,
    graph       TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
-- The board scope partial indexes (idx_boards_org_slug_ver / _site_slug_ver)
-- are created by migration v3: they reference the org/site_id columns a legacy
-- file only gains during that migration.
CREATE TABLE IF NOT EXISTS dashboards (
    id         TEXT PRIMARY KEY,
    org        TEXT NOT NULL,
    site_id    TEXT REFERENCES sites(id) ON DELETE CASCADE,
    slug       TEXT NOT NULL,
    title      TEXT NOT NULL,
    -- Dashboard variables as a JSON array (docs/design/variables-and-templating.md
    -- §1). Travels with the dashboard snapshot; NULL/absent decodes to an empty
    -- list. Legacy files gain this column in migration v6.
    variables  TEXT,
    created_at TEXT NOT NULL
);
-- A slug is unique within its scope. Two partial indexes: site-scoped boards
-- are unique per (org, site); org overviews (NULL site_id) per (org).
CREATE UNIQUE INDEX IF NOT EXISTS idx_dashboards_site_slug
    ON dashboards (org, site_id, slug) WHERE site_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_dashboards_overview_slug
    ON dashboards (org, slug) WHERE site_id IS NULL;
CREATE TABLE IF NOT EXISTS widgets (
    id           TEXT PRIMARY KEY,
    dashboard_id TEXT NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    title        TEXT NOT NULL,
    target       TEXT NOT NULL,
    query        TEXT,
    settings     TEXT,
    created_at   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    org         TEXT NOT NULL,
    site_id     TEXT REFERENCES sites(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    script      TEXT NOT NULL,
    params      TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
-- The rule scope partial indexes (idx_rules_org_name / _site_name) are created
-- by migration v3 (they reference the site_id column legacy files gain there).
CREATE TABLE IF NOT EXISTS runs (
    id            TEXT PRIMARY KEY,
    thread_id     TEXT NOT NULL,
    origin        TEXT NOT NULL,
    status        TEXT NOT NULL,
    response      TEXT NOT NULL,
    steps         INTEGER NOT NULL,
    pending_write TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tokens (
    id          TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL,
    scope_org   TEXT,
    scope_team  TEXT,
    scope_site  TEXT,
    created_at  TEXT NOT NULL,
    revoked_at  TEXT,
    UNIQUE (secret_hash)
);
-- RBAC identity: a user is an account keyed by its verified token `subject`
-- (OIDC `sub` or PAT id). `admin_level` is the user's admin tier (none/org_admin/
-- super_admin). `org` is the home org. See docs/design/authz-rbac.md.
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
-- Many-to-many user↔team. Rows vanish with either side (cascade).
CREATE TABLE IF NOT EXISTS memberships (
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    team_id TEXT NOT NULL REFERENCES teams(id) ON DELETE CASCADE,
    PRIMARY KEY (user_id, team_id)
);
-- Layer-2 ACL: a grant pins a permission on a resource (or `*`) for a subject
-- (user or team). Grants ADD access; they never subtract. `resource_ref` is a
-- textual address (`dashboard:<uuid>`, `board:<org>/<site?>/<slug>`,
-- `rule:<org>/<site?>/<name>`, or `*`). See docs/design/authz-rbac.md.
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
-- Units & datetime preferences (WS-11). Two layers — org and user — both
-- all-nullable (NULL = inherit); the resolver collapses user -> org -> system
-- default per column. `org` is the tenant key (rubix's workspace). Columns are
-- TEXT carrying enum wire tokens, the 'auto' sentinel, or a concrete unit code.
-- See rubix-prefs::resolver and docs/scope WS-11.
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
-- Behaviour-affecting entity tags (docs/design/page-context-and-nav.md §3).
-- Org-scoped key/value tags on a domain entity (today only `dashboard`). One
-- row per (org, kind, entity_id, key); a NULL value is a marker tag. Tags drive
-- queries via PageContext, so the routes enforce the entity's own authz. Legacy
-- files gain this table in migration v7.
CREATE TABLE IF NOT EXISTS entity_tags (
    org       TEXT NOT NULL,
    kind      TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     TEXT,
    PRIMARY KEY (org, kind, entity_id, key)
);
-- Navigation tree (docs/design/page-context-and-nav.md §4). Org-scoped, nestable
-- (parent_id self-ref, NULL = root). `target` is a JSON tagged union
-- (group/dashboard/route); `context` is JSON, dashboard targets only. Legacy
-- files gain this table in migration v8.
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
-- The append-only change ledger (docs/design/audit-and-undo.md, the substrate).
-- One row per logical mutation: `before`/`after` are full JSON snapshots (NULL by
-- op — Create has no before, Delete no after); `group_id` joins the rows of one
-- logical operation so a cascade undoes as one step. Org-scoped: every audit read
-- filters by `org`. Legacy files gain this table in migration v9.
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
-- Per-actor undo cursor (docs/design/audit-and-undo.md, undo/redo): one row per
-- `(org, subject)`. `redo_stack` is a JSON array of undone `group_id`s (LIFO);
-- `epoch` is the CAS guard so two concurrent undos cannot double-pop. Legacy files
-- gain this table in migration v9.
CREATE TABLE IF NOT EXISTS undo_cursors (
    org        TEXT NOT NULL,
    subject    TEXT NOT NULL,
    redo_stack TEXT NOT NULL DEFAULT '[]',
    epoch      INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (org, subject)
);
CREATE INDEX IF NOT EXISTS idx_entity_tags_reverse ON entity_tags (org, kind, key, value);
CREATE INDEX IF NOT EXISTS idx_nav_nodes_tree ON nav_nodes (org, parent_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_changes_org_at ON changes (org, at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_changes_resource ON changes (org, kind, resource_id, at DESC);
CREATE INDEX IF NOT EXISTS idx_changes_group ON changes (group_id);
CREATE INDEX IF NOT EXISTS idx_his_point_ts ON his (point_id, ts);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sparks_site ON sparks (site_id, ts);
-- idx_boards_org is created by migration v3 (references the org column).
CREATE INDEX IF NOT EXISTS idx_widgets_site ON widgets (site_id, created_at DESC);
-- idx_widgets_dashboard is created by migration v1 (column added there for legacy files).
CREATE INDEX IF NOT EXISTS idx_dashboards_org ON dashboards (org, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rules_org ON rules (org, name);
CREATE INDEX IF NOT EXISTS idx_users_org ON users (org, email);
CREATE INDEX IF NOT EXISTS idx_teams_org ON teams (org, slug);
CREATE INDEX IF NOT EXISTS idx_memberships_team ON memberships (team_id);
CREATE INDEX IF NOT EXISTS idx_grants_subject ON grants (org, subject_kind, subject_id);
";

/// Postgres dialect of the same schema. Identifiers and shapes mirror
/// [`SCHEMA_SQLITE`]; ids and timestamps are TEXT (the canonical-string and
/// RFC 3339 codecs are shared with the SQLite path), counters are BIGINT, and
/// the two flag columns are BOOLEAN. Cloud-feature only.
#[cfg(feature = "cloud")]
pub(crate) const SCHEMA_POSTGRES: &str = "
CREATE TABLE IF NOT EXISTS sites (
    id           TEXT PRIMARY KEY,
    org          TEXT NOT NULL,
    slug         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    tags         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (org, slug)
);
CREATE TABLE IF NOT EXISTS equips (
    id           TEXT PRIMARY KEY,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    path         TEXT NOT NULL,
    display_name TEXT NOT NULL,
    tags         TEXT NOT NULL,
    created_at   TEXT NOT NULL,
    UNIQUE (site_id, path)
);
CREATE TABLE IF NOT EXISTS points (
    id             TEXT PRIMARY KEY,
    equip_id       TEXT NOT NULL REFERENCES equips(id) ON DELETE CASCADE,
    slug           TEXT NOT NULL,
    display_name   TEXT NOT NULL,
    kind           TEXT NOT NULL,
    unit           TEXT,
    tags           TEXT NOT NULL,
    priority_array TEXT NOT NULL,
    cur_value      TEXT,
    cur_ts         TEXT,
    created_at     TEXT NOT NULL,
    UNIQUE (equip_id, slug)
);
CREATE TABLE IF NOT EXISTS his (
    point_id TEXT NOT NULL REFERENCES points(id) ON DELETE CASCADE,
    ts       TEXT NOT NULL,
    value    TEXT NOT NULL,
    PRIMARY KEY (point_id, ts)
);
CREATE TABLE IF NOT EXISTS sparks (
    id           TEXT PRIMARY KEY,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    rule         TEXT NOT NULL,
    severity     TEXT NOT NULL,
    message      TEXT NOT NULL,
    point_ids    TEXT NOT NULL,
    ts           TEXT NOT NULL,
    acknowledged BOOLEAN NOT NULL DEFAULT FALSE
);
CREATE TABLE IF NOT EXISTS boards (
    id           TEXT PRIMARY KEY,
    org          TEXT NOT NULL,
    site_id      TEXT REFERENCES sites(id) ON DELETE CASCADE,
    slug         TEXT NOT NULL,
    version      BIGINT NOT NULL,
    display_name TEXT NOT NULL,
    enabled      BOOLEAN NOT NULL DEFAULT TRUE,
    trigger      TEXT NOT NULL,
    graph        TEXT NOT NULL,
    created_at   TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS idx_boards_org_slug_ver
    ON boards (org, slug, version) WHERE site_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_boards_site_slug_ver
    ON boards (org, site_id, slug, version) WHERE site_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS dashboards (
    id         TEXT PRIMARY KEY,
    org        TEXT NOT NULL,
    site_id    TEXT REFERENCES sites(id) ON DELETE CASCADE,
    slug       TEXT NOT NULL,
    title      TEXT NOT NULL,
    -- Dashboard variables as a JSON array (docs/design/variables-and-templating.md
    -- §1). Travels with the dashboard snapshot; NULL/absent decodes to an empty
    -- list. Legacy files gain this column in migration v6.
    variables  TEXT,
    created_at TEXT NOT NULL
);
-- A slug is unique within its scope. Two partial indexes: site-scoped boards
-- are unique per (org, site); org overviews (NULL site_id) per (org).
CREATE UNIQUE INDEX IF NOT EXISTS idx_dashboards_site_slug
    ON dashboards (org, site_id, slug) WHERE site_id IS NOT NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_dashboards_overview_slug
    ON dashboards (org, slug) WHERE site_id IS NULL;
CREATE TABLE IF NOT EXISTS widgets (
    id           TEXT PRIMARY KEY,
    dashboard_id TEXT NOT NULL REFERENCES dashboards(id) ON DELETE CASCADE,
    site_id      TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    kind         TEXT NOT NULL,
    title        TEXT NOT NULL,
    target       TEXT NOT NULL,
    query        TEXT,
    settings     TEXT,
    created_at   TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS rules (
    id          TEXT PRIMARY KEY,
    org         TEXT NOT NULL,
    site_id     TEXT REFERENCES sites(id) ON DELETE CASCADE,
    name        TEXT NOT NULL,
    script      TEXT NOT NULL,
    params      TEXT NOT NULL,
    created_at  TEXT NOT NULL
);
-- A rule name is unique per scope: org-level (NULL site) per org, site rules per
-- (org, site). A board run resolves the site rule first, else the org-level one.
CREATE UNIQUE INDEX IF NOT EXISTS idx_rules_org_name
    ON rules (org, name) WHERE site_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS idx_rules_site_name
    ON rules (org, site_id, name) WHERE site_id IS NOT NULL;
CREATE TABLE IF NOT EXISTS runs (
    id            TEXT PRIMARY KEY,
    thread_id     TEXT NOT NULL,
    origin        TEXT NOT NULL,
    status        TEXT NOT NULL,
    response      TEXT NOT NULL,
    steps         BIGINT NOT NULL,
    pending_write TEXT,
    created_at    TEXT NOT NULL,
    updated_at    TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS tokens (
    id          TEXT PRIMARY KEY,
    secret_hash TEXT NOT NULL,
    name        TEXT NOT NULL,
    role        TEXT NOT NULL,
    scope_org   TEXT,
    scope_team  TEXT,
    scope_site  TEXT,
    created_at  TEXT NOT NULL,
    revoked_at  TEXT,
    UNIQUE (secret_hash)
);
-- RBAC identity + ACL (mirrors SCHEMA_SQLITE; see docs/design/authz-rbac.md).
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
-- Units & datetime preferences (WS-11); mirrors SCHEMA_SQLITE, updated_at BIGINT.
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
    updated_at       BIGINT NOT NULL
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
    updated_at       BIGINT NOT NULL,
    PRIMARY KEY (user_id, org)
);
-- Behaviour-affecting entity tags + nav tree (mirrors SCHEMA_SQLITE; see
-- docs/design/page-context-and-nav.md §§3,4).
CREATE TABLE IF NOT EXISTS entity_tags (
    org       TEXT NOT NULL,
    kind      TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    key       TEXT NOT NULL,
    value     TEXT,
    PRIMARY KEY (org, kind, entity_id, key)
);
CREATE TABLE IF NOT EXISTS nav_nodes (
    id         TEXT PRIMARY KEY,
    org        TEXT NOT NULL,
    parent_id  TEXT REFERENCES nav_nodes(id) ON DELETE CASCADE,
    title      TEXT NOT NULL,
    sort_order BIGINT NOT NULL DEFAULT 0,
    target     TEXT NOT NULL,
    context    TEXT,
    icon       TEXT,
    accent     TEXT
);
-- The append-only change ledger + per-actor undo cursor (mirrors SCHEMA_SQLITE;
-- see docs/design/audit-and-undo.md). `epoch` is BIGINT here.
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
    epoch      BIGINT NOT NULL DEFAULT 0,
    PRIMARY KEY (org, subject)
);
CREATE INDEX IF NOT EXISTS idx_entity_tags_reverse ON entity_tags (org, kind, key, value);
CREATE INDEX IF NOT EXISTS idx_nav_nodes_tree ON nav_nodes (org, parent_id, sort_order);
CREATE INDEX IF NOT EXISTS idx_changes_org_at ON changes (org, at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_changes_resource ON changes (org, kind, resource_id, at DESC);
CREATE INDEX IF NOT EXISTS idx_changes_group ON changes (group_id);
CREATE INDEX IF NOT EXISTS idx_his_point_ts ON his (point_id, ts);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sparks_site ON sparks (site_id, ts);
CREATE INDEX IF NOT EXISTS idx_boards_org ON boards (org, slug, version DESC);
CREATE INDEX IF NOT EXISTS idx_widgets_site ON widgets (site_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_widgets_dashboard ON widgets (dashboard_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_dashboards_org ON dashboards (org, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rules_org ON rules (org, name);
CREATE INDEX IF NOT EXISTS idx_users_org ON users (org, email);
CREATE INDEX IF NOT EXISTS idx_teams_org ON teams (org, slug);
CREATE INDEX IF NOT EXISTS idx_memberships_team ON memberships (team_id);
CREATE INDEX IF NOT EXISTS idx_grants_subject ON grants (org, subject_kind, subject_id);
";
