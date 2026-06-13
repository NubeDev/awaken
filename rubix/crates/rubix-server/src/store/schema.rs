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
CREATE INDEX IF NOT EXISTS idx_his_point_ts ON his (point_id, ts);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sparks_site ON sparks (site_id, ts);
-- idx_boards_org is created by migration v3 (references the org column).
CREATE INDEX IF NOT EXISTS idx_widgets_site ON widgets (site_id, created_at DESC);
-- idx_widgets_dashboard is created by migration v1 (column added there for legacy files).
CREATE INDEX IF NOT EXISTS idx_dashboards_org ON dashboards (org, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rules_org ON rules (org, name);
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
CREATE INDEX IF NOT EXISTS idx_his_point_ts ON his (point_id, ts);
CREATE INDEX IF NOT EXISTS idx_runs_status ON runs (status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_sparks_site ON sparks (site_id, ts);
CREATE INDEX IF NOT EXISTS idx_boards_org ON boards (org, slug, version DESC);
CREATE INDEX IF NOT EXISTS idx_widgets_site ON widgets (site_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_widgets_dashboard ON widgets (dashboard_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_dashboards_org ON dashboards (org, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_rules_org ON rules (org, name);
";
