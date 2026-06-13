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

/** `rubix-server::BoardView` — a stored board as returned by `/api/v1/boards`. */
export interface BoardView {
  id: Uuid
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
  at: IsoTimestamp
}

/**
 * `rubix_core::WidgetKind` — what a pinned dashboard tile renders. serde
 * snake_case. `point_*` kinds carry a point keyexpr in `target`; `board_output`
 * carries a board slug.
 */
export type WidgetKind = 'point_value' | 'point_history' | 'board_output'

/** `rubix_core::Widget` — a pinned dashboard tile row (`GET /api/v1/widgets`). */
export interface Widget {
  id: Uuid
  dashboard_id: Uuid
  site_id: Uuid
  kind: WidgetKind
  title: string
  target: string
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
  created_at: IsoTimestamp
}

/** `rubix-server::CreateDashboard` — body for `POST /api/v1/dashboards`. */
export interface CreateDashboard {
  org: string
  /** Omit for an org overview. */
  site_id?: Uuid | null
  slug: string
  title: string
}

/** `rubix-server::PatchDashboard` — body for `PATCH /api/v1/dashboards/{id}`. */
export interface PatchDashboard {
  title?: string
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
