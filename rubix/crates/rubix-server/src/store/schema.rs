pub(crate) const SCHEMA: &str = "
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
    slug        TEXT NOT NULL,
    version     INTEGER NOT NULL,
    display_name TEXT NOT NULL,
    enabled     INTEGER NOT NULL DEFAULT 1,
    trigger     TEXT NOT NULL,
    graph       TEXT NOT NULL,
    created_at  TEXT NOT NULL,
    UNIQUE (slug, version)
);
CREATE TABLE IF NOT EXISTS widgets (
    id         TEXT PRIMARY KEY,
    site_id    TEXT NOT NULL REFERENCES sites(id) ON DELETE CASCADE,
    kind       TEXT NOT NULL,
    title      TEXT NOT NULL,
    target     TEXT NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_his_point_ts ON his (point_id, ts);
CREATE INDEX IF NOT EXISTS idx_sparks_site ON sparks (site_id, ts);
CREATE INDEX IF NOT EXISTS idx_boards_slug ON boards (slug, version DESC);
CREATE INDEX IF NOT EXISTS idx_widgets_site ON widgets (site_id, created_at DESC);
";
