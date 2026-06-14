/**
 * TypeScript mirrors of the `rubix-core` / `rubix-server` wire DTOs.
 * Field names and serde casings match the Rust types exactly — see
 * `crates/rubix-core/src/model.rs` and `crates/rubix-server/src/api/*`.
 */

export type Uuid = string
export type IsoTimestamp = string

/** `rubix_core::PointValue` — serde-untagged: a bool, number, or string. */
export type PointValue = boolean | number | string

export type PointKind = 'sensor' | 'cmd' | 'sp'
export type SparkSeverity = 'info' | 'warning' | 'fault'

/**
 * `rubix_core::TagSet` — `#[serde(transparent)] BTreeMap<String, Value>`. On the
 * wire it is a JSON object, not an array: marker tags map to `true`, value tags
 * to any JSON value (`{"ahu": true, "stage": 2}`). Read tag names via
 * `tagNames`/`hasTag` in `./tags`, never by treating this as `string[]`.
 */
export type TagSet = Record<string, unknown>

/** A single priority-array slot. `null` slots are unwritten. */
export type PrioritySlot = PointValue | null

/**
 * `rubix_core::PriorityArray` serialises as its 16 slots. The exact JSON shape
 * is the array of slots; `relinquish_default` is the fallback when all null.
 */
export interface PriorityArray {
  slots: PrioritySlot[]
  relinquish_default: PointValue | null
}

export interface Site {
  id: Uuid
  org: string
  slug: string
  display_name: string
  tags: TagSet
  created_at: IsoTimestamp
}

export interface Equip {
  id: Uuid
  site_id: Uuid
  path: string
  display_name: string
  tags: TagSet
  created_at: IsoTimestamp
}

export interface Point {
  id: Uuid
  equip_id: Uuid
  slug: string
  display_name: string
  kind: PointKind
  unit: string | null
  tags: TagSet
  priority_array: PriorityArray
  cur_value: PointValue | null
  cur_ts: IsoTimestamp | null
  created_at: IsoTimestamp
}

/** `PointResponse` returned by write/cur/get-by-id. */
export interface PointEnvelope {
  keyexpr: string
  point: Point
}

export interface Spark {
  id: Uuid
  site_id: Uuid
  rule: string
  severity: SparkSeverity
  message: string
  point_ids: Uuid[]
  ts: IsoTimestamp
  acknowledged: boolean
}

/** One history sample for a point. */
export interface HisSample {
  ts: IsoTimestamp
  value: PointValue
}

/** `rubix-server::WriteSource` — snake_case; only these two variants exist. */
export type WriteSource = 'operator' | 'agent'

export interface WriteRequest {
  value: PointValue
  priority?: number
  source?: WriteSource
}

export interface CurRequest {
  value: PointValue
}

export type ChatStatus = 'completed' | 'awaiting_approval'

export interface ChatRequest {
  thread_id: string
  message: string
}

export interface ChatResponse {
  response: string
  steps: number
  status: ChatStatus
  run_id?: string
}

/**
 * `rubix-server::AgentStatus` — read-only view of the process-global agent
 * config (env-configured at boot; not per-org and not editable from the UI).
 * Model fields are absent when `enabled` is false.
 */
export interface AgentStatus {
  enabled: boolean
  provider?: string
  model?: string
  max_rounds?: number
  min_priority: number
  escalation_floor: number
  dispatch_ready: boolean
}

/** `rubix-server::RunOrigin` — what raised a run. */
export type RunOrigin = 'chat' | 'dispatch' | 'mcp'

/** `rubix-server::RunStatus` — lifecycle of an agent run. `suspended` awaits approval. */
export type RunStatus = 'completed' | 'suspended' | 'resumed' | 'cancelled'

/** `rubix-server::PendingWrite` — the command a suspended run holds for approval. */
export interface PendingWrite {
  point: string
  priority: number
  value: PointValue
  agent_min_priority: number
}

/**
 * `rubix-server::RunRecord` — the persisted agent-run row backing the operator
 * surface. `pending_write` is present only while `status === 'suspended'`.
 */
export interface RunRecord {
  id: string
  thread_id: string
  origin: RunOrigin
  status: RunStatus
  response: string
  steps: number
  pending_write?: PendingWrite
  created_at: IsoTimestamp
  updated_at: IsoTimestamp
}

/**
 * `rubix-server::ResumeResponse` — the result of approving a suspended run. The
 * resume endpoint re-applies the held write and returns the commanded point and
 * its effective value; it does NOT echo back a `RunRecord`.
 */
export interface ResumeResponse {
  run_id: string
  point: string
  priority: number
  effective?: PointValue | null
}

/**
 * `scheduler::Trigger` — what fires a stored board. serde-tagged on `kind`
 * (snake_case). `manual` boards run only via `POST /boards/{slug}/run`.
 */
export type Trigger =
  | { kind: 'manual' }
  | { kind: 'interval'; seconds: number }
  | { kind: 'subscription'; key: string }

/** `rubix_flow::BoardNode` — a graph node naming a registered actor component. */
export interface BoardNode {
  id: string
  component: string
  config: Record<string, unknown>
}

/** `rubix_flow::BoardConnection` — a directed wire between two node ports. */
export interface BoardConnection {
  from_node: string
  from_port: string
  to_node: string
  to_port: string
}

/**
 * `rubix_flow::BoardGraph` — the stored wiresheet. Nodes plus connections; there
 * is no canvas geometry on the wire, so the UI lays nodes out deterministically.
 */
export interface BoardGraph {
  nodes: BoardNode[]
  connections: BoardConnection[]
}

/**
 * `rubix-server::BoardView` — a stored flow. Scoped to an `org` and optionally a
 * `site_id` (null = org-level, applying across the org) — the same model as
 * dashboards/rules.
 */
export interface BoardView {
  id: Uuid
  org: string
  site_id?: Uuid | null
  slug: string
  version: number
  display_name: string
  enabled: boolean
  trigger: Trigger
  graph: BoardGraph
  created_at: IsoTimestamp
}

/**
 * `rubix-server::ComponentView` — one board component's editor schema, from
 * `GET /api/v1/boards/components`. The flow editor's palette and per-node config
 * form are driven entirely by this; nothing about a node's ports or config is
 * hardcoded in the client.
 */
export interface ComponentView {
  component: string
  label: string
  description: string
  kind: 'source' | 'logic' | 'sink' | 'agent'
  inports: PortView[]
  outports: PortView[]
  config: ConfigFieldView[]
}

/** The semantic value a port carries; drives connection validation. */
export type PortType = 'flow' | 'scalar' | 'object' | 'error'

/** A component port; `id` matches the wire port name on a `BoardConnection`. */
export interface PortView {
  id: string
  label: string
  port_type: PortType
}

export type ConfigFieldType =
  | 'string'
  | 'keyexpr'
  | 'integer'
  | 'number'
  | 'boolean'
  | 'enum'
  | 'json'

/** One configurable field on a node's `config` map. */
export interface ConfigFieldView {
  name: string
  label: string
  field_type: ConfigFieldType
  required: boolean
  default?: unknown
  options?: string[]
  min?: number
  max?: number
  help?: string
}

/**
 * `rubix-server::CreateBoard` — body for `POST /api/v1/boards`. A slug that
 * exists creates a new version (the editor saves graph edits this way).
 */
export interface CreateBoard {
  org: string
  /** Omit for an org-level flow; set to scope to one site. */
  site_id?: Uuid | null
  slug: string
  display_name: string
  enabled?: boolean
  trigger: Trigger
  board: BoardGraph
}

/** `rubix_flow::NodeOutput` — one outport packet from a board run. */
export interface NodeOutput {
  node: string
  port: string
  value: unknown
}

/** `rubix-server::RunBoardResponse` — every outport packet from one board run. */
export interface RunBoardResponse {
  outputs: NodeOutput[]
}

/**
 * `rubix-server::PortOutput` — one node's latest value on one port, from the
 * scheduler's in-memory cache (`GET /boards/{slug}/outputs`). `at` is the
 * RFC3339 capture time, so the UI can show freshness of a running board.
 */
export interface PortOutput {
  node: string
  port: string
  value: unknown
  /** Link quality (`ok`/`fault`/`null`) so a value is self-describing. */
  quality?: string
  at: IsoTimestamp
}

/**
 * `rubix_core::WidgetKind` — what a pinned dashboard tile renders. serde
 * snake_case. `point_*` kinds carry a point keyexpr in `target`; `board_output`
 * carries a board slug; `datasource` carries a datasource id in `target` and
 * native SQL in `query`.
 */
export type WidgetKind =
  | 'point_value'
  | 'point_history'
  | 'board_output'
  | 'datasource'

/** `react-grid-layout` cell for a tile (`rubix_core::GridLayout`). */
export interface GridLayout {
  x: number
  y: number
  w: number
  h: number
}

/**
 * Chart-type config for a `point_history`/`datasource` tile — a discriminated
 * union mirroring the recharts wrappers the canvas renders. The server treats
 * this as opaque JSON (`WidgetSettings.config`), so the shape lives here.
 */
export type ChartType = 'area' | 'line' | 'bar' | 'table'
export interface ChartConfig {
  type: ChartType
}

/**
 * `rubix_core::WidgetSettings` — a tile's presentation state: grid placement
 * and chart config. Both halves optional; absent → auto-flow + default render.
 */
export interface WidgetSettings {
  layout?: GridLayout
  config?: ChartConfig
  /**
   * Per-column quantity declarations the server reads to convert a result
   * column into the viewer's preferred unit (WS-11). Absent/empty → every
   * column passes through as a bare number.
   */
  fields?: SeriesField[]
}

/**
 * `rubix_core::SeriesField` — declares what a result column *is*, so the server
 * can convert it at the response edge. `quantity`/`stored_unit` are wire codes
 * from `GET /api/v1/units`; a field with no `quantity` is not convertible.
 */
export interface SeriesField {
  column: string
  quantity?: string
  stored_unit?: string
}

/** `rubix_core::Widget` — a pinned dashboard tile row (`GET /api/v1/widgets`). */
export interface Widget {
  id: Uuid
  dashboard_id: Uuid
  site_id: Uuid
  kind: WidgetKind
  title: string
  target: string
  /** Native SQL for a `datasource` tile; absent for every other kind. */
  query?: string
  /** Grid layout + chart config; absent until the builder sets it. */
  settings?: WidgetSettings
  created_at: IsoTimestamp
}

/** `rubix-server::CreateWidget` — body for `POST /api/v1/widgets`. */
export interface CreateWidget {
  /** Dashboard to pin onto; omit to use the site's default board. */
  dashboard_id?: Uuid
  site_id: Uuid
  kind: WidgetKind
  title: string
  target: string
  /** Native SQL — required for `datasource`, rejected for every other kind. */
  query?: string
}

/**
 * `rubix-server::PatchWidget` — body for `PATCH /api/v1/widgets/{id}`. Only
 * `settings` is mutable: an object sets it, `null` clears it, omitting it is a
 * no-op.
 */
export interface PatchWidget {
  settings?: WidgetSettings | null
}

/**
 * `rubix_core::Dashboard` — a named board of widgets. `site_id` null makes it an
 * **org overview** spanning every site under the org; set makes it site-scoped.
 */
export interface Dashboard {
  id: Uuid
  org: string
  site_id?: Uuid | null
  slug: string
  title: string
  /** Dashboard variables (docs/design/variables-and-templating.md §1). Absent /
   *  empty for a board with no parameterisation. */
  variables?: Variable[]
  created_at: IsoTimestamp
}

/**
 * `rubix_core::VariableKind` — the closed set of variable kinds (serde
 * snake_case). Built-ins (`$__org`/`$__site`/`$__user`/`$__from`/`$__to`) are
 * not authored variables and are not members of this union.
 */
export type VariableKind =
  | 'constant'
  | 'custom'
  | 'query'
  | 'datasource'
  | 'site'
  | 'interval'
  | 'textbox'
  | 'context'

/**
 * `rubix_core::ContextSource` — which page-context layer a `context` variable
 * reads (docs/design/page-context-and-nav.md §2). `nav` reads the open node
 * (`slug`/`name`/`path[n]`), `url` a bare `?key=…` param, `tag` the board's
 * tag, `values` a nav node's `context.values[key]`.
 */
export type ContextSource = 'nav' | 'url' | 'tag' | 'values'

/**
 * `rubix_core::VariableConfig` — per-kind config, tagged on `kind` (matching the
 * serde `#[serde(tag = "kind")]` wire shape).
 */
export type VariableConfig =
  | { kind: 'constant'; value: PointValue | null }
  | { kind: 'custom'; options: string[] }
  | { kind: 'query'; sql: string; datasource_id?: string | null }
  | { kind: 'datasource'; datasource_kind?: string | null }
  | { kind: 'site' }
  | { kind: 'interval'; options: string[] }
  | { kind: 'textbox' }
  | { kind: 'context'; source: ContextSource; key: string }

/** One option / single selected value: always a scalar (never nested). */
export type ScalarValue = PointValue | null

/** A variable's selected value(s): a scalar (single) or an array (multi). */
export type VariableValue = ScalarValue | ScalarValue[]

/** `rubix_core::Variable` — one dashboard variable. */
export interface Variable {
  name: string
  label?: string | null
  kind: VariableKind
  config: VariableConfig
  /** Selected value(s); maintained by the resolution layer. */
  current?: VariableValue
  multi?: boolean
  include_all?: boolean
  hidden?: boolean
}

/** `rubix-server::CreateDashboard` — body for `POST /api/v1/dashboards`. */
export interface CreateDashboard {
  org: string
  /** Omit for an org overview. */
  site_id?: Uuid | null
  slug: string
  title: string
  variables?: Variable[]
}

/** `rubix-server::PatchDashboard` — body for `PATCH /api/v1/dashboards/{id}`. */
export interface PatchDashboard {
  title?: string
  /** Replace the variable list wholesale; omit to leave unchanged, `[]` clears. */
  variables?: Variable[]
}

/**
 * `rubix_query::QueryVariable` — a resolved variable sent on a query request so
 * the server interpolation engine binds it (never splices) into SQL. `name` is
 * the SQL reference without the leading `$`; `value` is a scalar or array.
 */
export interface QueryVariable {
  name: string
  value: VariableValue
}

/**
 * `rubix-server::TimeRangeBody` — the dashboard time range a query's time macros
 * (`$__from`/`$__to`/`$__timeFilter`/`$__timeGroup`/`$__interval`) bind against
 * (docs/design/time-range-and-refresh.md §4). `from`/`to` are absolute RFC 3339
 * instants or relative tokens (`now`, `now-6h`, `now/d`); the server resolves
 * them against one frozen `now` so every widget in a refresh shares an instant.
 * The bounds bind as parameters — they never splice into SQL.
 */
export interface TimeRangeBody {
  from: string
  to: string
}

/** A single DataFusion `/query` result row: column name -> value. */
export type QueryRow = Record<string, PointValue | null>

/** `rubix-server::QueryResponse` — rows as JSON objects; no separate columns. */
export interface QueryResult {
  rows: QueryRow[]
}

// --- Create / Patch request bodies (the CRUD management surface) ---------------
// Each mirrors the matching `rubix-server::Create*` / `Patch*` struct. PATCH is
// partial: an omitted field is left unchanged. Identity fields (`org`/`slug`/
// `equip.path`/`point.slug`) are immutable and absent from the Patch bodies; the
// backend returns 400 if one is sent.

/** Body for `POST /api/v1/sites`. */
export interface CreateSite {
  org: string
  slug: string
  display_name: string
  tags?: TagSet
}

/** Body for `PATCH /api/v1/sites/{id}` — metadata only. */
export interface PatchSite {
  display_name?: string
  tags?: TagSet
}

/** Body for `POST /api/v1/equips`. `path` is a slash-separated keyexpr path. */
export interface CreateEquip {
  site_id: Uuid
  path: string
  display_name: string
  tags?: TagSet
}

/** Body for `PATCH /api/v1/equips/{id}` — metadata only (`path` immutable). */
export interface PatchEquip {
  display_name?: string
  tags?: TagSet
}

/** Body for `POST /api/v1/points`. */
export interface CreatePoint {
  equip_id: Uuid
  slug: string
  display_name: string
  kind: PointKind
  unit?: string | null
  tags?: TagSet
  relinquish_default?: PointValue | null
}

/** Body for `PATCH /api/v1/points/{id}` — metadata only (`slug` immutable). */
export interface PatchPoint {
  display_name?: string
  tags?: TagSet
  unit?: string
  kind?: PointKind
}

/** Body for `PATCH /api/v1/boards/{slug}` — latest-version metadata. */
export interface PatchBoard {
  display_name?: string
  enabled?: boolean
}

/**
 * `rubix-server::OrgSummary` — a derived tenant: one distinct `org` with the
 * sites visible under it (`GET /api/v1/orgs`). There is no org table; this is
 * grouped from the sites the principal may see.
 */
export interface OrgSummary {
  org: string
  site_count: number
  sites: string[]
  tags: TagSet
}

/** `rubix-server::ProvisionOrg` — body for `POST /api/v1/orgs` (onboard a tenant). */
export interface ProvisionOrg {
  org: string
  slug: string
  display_name: string
  tags?: TagSet
}

// --- Rules engine (Rules Studio) ----------------------------------------------
// Mirror the `rubix-server` stored-rule routes and the dry-run debugger spine.
// A rule is a Rhai script operating on a `Frame` (the `df` variable) built from
// a point's history (`ts` + `value` columns); it returns a verdict.

/** `rubix-rules::ParamSpec` — one declared parameter of a rule. */
export interface ParamSpec {
  required: boolean
  description?: string
}

/** `rubix-rules::ParamSchema` — a rule's declared parameter map. */
export interface ParamSchema {
  params: Record<string, ParamSpec>
}

/**
 * `rubix-server::RuleView` — a stored rule. Owned by an `org` and optionally a
 * `site_id` (null = org-level, applying across the org); a site rule overrides
 * the org-level one of the same name during a board run.
 */
export interface RuleView {
  id: Uuid
  org: string
  site_id?: Uuid | null
  name: string
  script: string
  params: ParamSchema
  created_at: IsoTimestamp
}

/** Body for `POST /api/v1/orgs/{org}/rules`. */
export interface CreateRule {
  /** Omit for an org-level rule; set to scope to one site. */
  site_id?: Uuid | null
  name: string
  script: string
  params?: ParamSchema
}

/** Body for `PUT /api/v1/orgs/{org}/rules/{name}` — script + params only. */
export interface UpdateRule {
  script: string
  params?: ParamSchema
}

/**
 * `rubix-rules::RuleResult` — a rule's verdict over the frame it ran on.
 * `severity` is meaningful when `flagged`; `value` is the optional score a
 * composing rule reads.
 */
export interface RuleResult {
  flagged: boolean
  severity: SparkSeverity
  message: string
  value: number | null
}

/** One resolved input row the dry-run returns so the UI can chart the frame. */
export interface FrameRow {
  ts: IsoTimestamp
  value: number | null
}

/** The dry-run input frame summary: row count plus the rows the rule saw. */
export interface FrameSummary {
  row_count: number
  rows: FrameRow[]
}

/**
 * Body for `POST /api/v1/orgs/{org}/rules/dry-run`. Exactly one of `script`
 * (inline) or `rule` (stored id-or-name) is the source; `point` selects the
 * input window by keyexpr (omit to run against an empty frame).
 */
export interface DryRunRequest {
  script?: string
  rule?: string
  params?: Record<string, unknown>
  point?: string
  limit?: number
}

/** `rubix-server::DryRunResponse` — the verdict plus the frame it ran over. */
export interface DryRunResponse {
  result: RuleResult
  frame: FrameSummary
}

// --- Authorization (RBAC) -----------------------------------------------------

/** `rubix-server::Role` — the caller's coarse role. `admin` (super- or org-admin
 *  by scope) additionally unlocks the identity/authorization management surfaces. */
export type Role = 'admin' | 'operator' | 'service' | 'viewer'

/** `rubix-server::Scope` — the org/team/site a principal is confined to; omitted
 *  levels are global (an unset `org` is a global/super-admin principal). */
export interface Scope {
  org?: string
  team?: string
  site?: string
}

/**
 * `rubix-server::Whoami` — the resolved identity of the caller (`GET
 * /api/v1/whoami`). The UI reads this once at boot to render permission-aware
 * chrome. `auth_enabled` is false on the open dev server (then `subject` is
 * `"dev"` and the principal is a synthetic global operator).
 */
export interface Whoami {
  subject: string
  scope: Scope
  role: Role
  can_write: boolean
  /** True when the caller may manage users/teams/grants (org- or super-admin).
   *  The Members/Teams/Access surfaces gate on this. */
  can_admin: boolean
  auth_enabled: boolean
}

// --- RBAC: users, teams, memberships, grants (authz-rbac.md increments B–E) ---

/** `rubix-server::AdminLevel` — a user's admin tier. */
export type AdminLevel = 'none' | 'org_admin' | 'super_admin'

/** `rubix-server::store::UserRecord`. */
export interface User {
  id: Uuid
  org: string
  subject: string
  email: string
  display_name: string
  admin_level: AdminLevel
  created_at: IsoTimestamp
}

export interface CreateUser {
  subject: string
  email: string
  display_name: string
  admin_level?: AdminLevel
}

export interface PatchUser {
  email?: string
  display_name?: string
  admin_level?: AdminLevel
}

/** `rubix-server::store::TeamRecord`. */
export interface Team {
  id: Uuid
  org: string
  slug: string
  name: string
  created_at: IsoTimestamp
}

export interface CreateTeam {
  slug: string
  name: string
}

export interface PatchTeam {
  name?: string
}

/** Grant subject + permission, mirroring `rubix-server::store`. */
export type SubjectKind = 'user' | 'team'
export type Permission = 'read' | 'write' | 'admin'

/** `rubix-server::store::GrantRecord` — a Layer-2 ACL row. `resource_ref` is
 *  `dashboard:<id>` / `board:<org>/<site?>/<slug>` / `rule:<org>/<site?>/<name>`
 *  or `*` (all-of-kind within the org). */
export interface Grant {
  id: Uuid
  org: string
  subject_kind: SubjectKind
  subject_id: string
  resource_kind: string
  resource_ref: string
  permission: Permission
  created_at: IsoTimestamp
}

export interface CreateGrant {
  subject_kind: SubjectKind
  subject_id: string
  resource_kind: string
  resource_ref: string
  permission: Permission
}

/** Grant body addressed at a dashboard in the path (kind/ref implied). */
export interface CreateDashboardGrant {
  subject_kind: SubjectKind
  subject_id: string
  permission: Permission
}

// --- Units & datetime preferences (WS-11) -------------------------------------

/**
 * `rubix_prefs::ResolvedPreferences` — the fully-resolved view returned by
 * `GET /api/v1/me/preferences`. Every field is concrete (the server collapsed
 * user → org → system default and the `"auto"` derivations), so the UI can
 * format/convert without re-deriving anything. Enum fields carry the wire
 * tokens the backend serialises.
 */
export interface ResolvedPreferences {
  timezone: string
  locale: string
  language: string
  unit_system: 'metric' | 'imperial'
  temperature_unit: string
  pressure_unit: string
  speed_unit: string
  length_unit: string
  mass_unit: string
  /** e.g. `"YYYY-MM-DD"`, `"DD/MM/YYYY"`, `"MM/DD/YYYY"`. */
  date_format: string
  /** `"24h"` | `"12h"`. */
  time_format: string
  week_start: 'monday' | 'sunday' | 'saturday' | string
  /** e.g. `"1,234.56"`, `"1.234,56"`, `"1 234,56"`. */
  number_format: string
  currency: string
  theme: 'light' | 'dark' | 'system'
}

/**
 * `rubix_prefs::PreferencesPatch` — the `PATCH` body. Every field is optional;
 * a present `null` reverts that field to inherit, an omitted key leaves it. A
 * per-unit field accepts a concrete unit code or the `"auto"` sentinel.
 */
export interface PreferencesPatch {
  timezone?: string | null
  locale?: string | null
  language?: string | null
  unit_system?: 'metric' | 'imperial' | null
  temperature_unit?: string | null
  pressure_unit?: string | null
  speed_unit?: string | null
  length_unit?: string | null
  mass_unit?: string | null
  date_format?: string | null
  time_format?: string | null
  week_start?: string | null
  number_format?: string | null
  currency?: string | null
  theme?: 'light' | 'dark' | 'system' | null
}

/** One quantity's registry entry from `GET /api/v1/units`. */
export interface QuantityEntry {
  quantity: string
  canonical: string
  allowed: string[]
}

/** `GET /api/v1/units` payload — the closed unit registry. */
export interface UnitsDocument {
  quantities: QuantityEntry[]
}

// ── Nav tree + entity tags (docs/design/page-context-and-nav.md) ──────────────

/**
 * `rubix_core::NavRoute` — the closed allow-list of built-in static pages a
 * `route` nav target may point at. The server rejects any other value.
 */
export type NavRoute =
  | 'sites'
  | 'equips'
  | 'points'
  | 'dashboards'
  | 'datasources'
  | 'rules'
  | 'boards'
  | 'sparks'
  | 'runs'
  | 'audit'
  | 'access'

/**
 * `rubix_core::NavTarget` — what a nav node mounts. A tagged union on `kind`: a
 * `group` is a header with no destination, a `dashboard` mounts a board by id
 * (validated to live in the node's org), a `route` opens a built-in page.
 */
export type NavTarget =
  | { kind: 'group' }
  | { kind: 'dashboard'; dashboard_id: string }
  | { kind: 'route'; route: NavRoute }

/**
 * `rubix_core::NavContext` — the page context a `dashboard` node injects:
 * free-form variable `values` and entity `tags`. Only meaningful on a
 * `dashboard` target.
 */
export interface NavContext {
  values?: Record<string, unknown>
  tags?: Record<string, string>
}

/**
 * `rubix_core::NavNode` — one org-scoped, nestable nav-tree row. Returned flat
 * (in `parent_id` / `sort_order` order); the client assembles the nesting.
 */
export interface NavNode {
  id: string
  org: string
  parent_id: string | null
  title: string
  sort_order: number
  target: NavTarget
  context?: NavContext | null
  icon?: string | null
  accent?: string | null
}

/** `POST /api/v1/nav` body. Identity (`id`) is server-assigned. */
export interface CreateNavNode {
  org: string
  parent_id?: string | null
  title: string
  sort_order?: number
  target: NavTarget
  context?: NavContext | null
  icon?: string | null
  accent?: string | null
}

/**
 * `PATCH /api/v1/nav/{id}` body. Every field optional; an absent field is left
 * unchanged. A present `parent_id: null` moves the node to root; a present
 * `context: null` clears it. `org` is immutable identity and not patchable.
 */
export interface PatchNavNode {
  parent_id?: string | null
  title?: string
  sort_order?: number
  target?: NavTarget
  context?: NavContext | null
  icon?: string | null
  accent?: string | null
}

/**
 * `rubix_core::EntityTags` — an entity's full tag set (`PUT`/`GET
 * /api/v1/tags/{kind}/{id}`). A map of key → value; a `null` value is a
 * present-but-unset key. The `PUT` replaces the set wholesale.
 */
export type EntityTags = Record<string, string | null>

/**
 * `rubix_core::TagEntityKind` — the entity kinds that carry behaviour-affecting
 * tags (docs/design/page-context-and-nav.md §3). A closed set; the server
 * rejects any other kind path segment.
 */
export type TagEntityKind = 'dashboard'

// --- Audit & undo/redo (docs/design/audit-and-undo.md) ------------------------
// Hand-authored mirror of `rubix_core::{Op, Actor, Change}` and the audit/undo
// route DTOs. Rubix has no codegen; keep in sync with the Rust types.

/** `rubix_core::Op` — the kind of mutation a change records. */
export type Op = 'create' | 'update' | 'delete'

/**
 * `rubix_core::Actor` — who made a change. The tagged union mirrors the Rust
 * `#[serde(tag = "kind")]` shape. `agent` is the AI runtime writing the same
 * ledger; `system` is the scheduler/provisioning path.
 */
export type Actor =
  | { kind: 'user'; subject: string }
  | { kind: 'agent'; run_id: string; model: string }
  | { kind: 'system' }

/**
 * `rubix_core::Change` — one immutable change-ledger row. `before`/`after` are
 * full JSON snapshots (absent per op: no `before` on create, no `after` on
 * delete). `group_id` joins the rows of one logical operation so they undo as a
 * single step.
 */
export interface Change {
  id: string
  at: string
  org: string
  site_id?: string | null
  actor: Actor
  kind: string
  resource_id: string
  op: Op
  before?: unknown
  after?: unknown
  group_id: string
  correlation?: string | null
}

/**
 * Query params for `GET /api/v1/audit`. `org` is required and always enforced
 * (a cross-org read is impossible); the rest narrow the result.
 */
export interface AuditQuery {
  org: string
  kind?: string
  resource_id?: string
  actor?: string
  op?: Op
  limit?: number
}

/** Body for `POST /api/v1/undo` and `/api/v1/redo`. */
export interface UndoRequest {
  org: string
}

/**
 * Result of an undo/redo: the group that moved and the resource ids it touched,
 * so the UI invalidates exactly the matching query keys. `group` is absent when
 * there was nothing to undo/redo.
 */
export interface UndoResult {
  group?: string
  touched: string[]
}
