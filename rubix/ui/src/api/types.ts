/**
 * TypeScript mirrors of the `rubix-core` / `rubix-server` wire DTOs.
 * Field names and serde casings match the Rust types exactly — see
 * `crates/rubix-core/src/model.rs` and `crates/rubix-server/src/api/*`.
 */

export type Uuid = string;
export type IsoTimestamp = string;

/** `rubix_core::PointValue` — serde-untagged: a bool, number, or string. */
export type PointValue = boolean | number | string;

export type PointKind = 'sensor' | 'cmd' | 'sp';
export type SparkSeverity = 'info' | 'warning' | 'fault';

/**
 * `rubix_core::TagSet` — `#[serde(transparent)] BTreeMap<String, Value>`. On the
 * wire it is a JSON object, not an array: marker tags map to `true`, value tags
 * to any JSON value (`{"ahu": true, "stage": 2}`). Read tag names via
 * `tagNames`/`hasTag` in `./tags`, never by treating this as `string[]`.
 */
export type TagSet = Record<string, unknown>;

/** A single priority-array slot. `null` slots are unwritten. */
export type PrioritySlot = PointValue | null;

/**
 * `rubix_core::PriorityArray` serialises as its 16 slots. The exact JSON shape
 * is the array of slots; `relinquish_default` is the fallback when all null.
 */
export interface PriorityArray {
  slots: PrioritySlot[];
  relinquish_default: PointValue | null;
}

export interface Site {
  id: Uuid;
  org: string;
  slug: string;
  display_name: string;
  tags: TagSet;
  created_at: IsoTimestamp;
}

export interface Equip {
  id: Uuid;
  site_id: Uuid;
  path: string;
  display_name: string;
  tags: TagSet;
  created_at: IsoTimestamp;
}

export interface Point {
  id: Uuid;
  equip_id: Uuid;
  slug: string;
  display_name: string;
  kind: PointKind;
  unit: string | null;
  tags: TagSet;
  priority_array: PriorityArray;
  cur_value: PointValue | null;
  cur_ts: IsoTimestamp | null;
  created_at: IsoTimestamp;
}

/** `PointResponse` returned by write/cur/get-by-id. */
export interface PointEnvelope {
  keyexpr: string;
  point: Point;
}

export interface Spark {
  id: Uuid;
  site_id: Uuid;
  rule: string;
  severity: SparkSeverity;
  message: string;
  point_ids: Uuid[];
  ts: IsoTimestamp;
  acknowledged: boolean;
}

/** One history sample for a point. */
export interface HisSample {
  ts: IsoTimestamp;
  value: PointValue;
}

/** `rubix-server::WriteSource` — snake_case; only these two variants exist. */
export type WriteSource = 'operator' | 'agent';

export interface WriteRequest {
  value: PointValue;
  priority?: number;
  source?: WriteSource;
}

export interface CurRequest {
  value: PointValue;
}

export type ChatStatus = 'completed' | 'awaiting_approval';

export interface ChatRequest {
  thread_id: string;
  message: string;
}

export interface ChatResponse {
  response: string;
  steps: number;
  status: ChatStatus;
  run_id?: string;
}

/** `rubix-server::RunOrigin` — what raised a run. */
export type RunOrigin = 'chat' | 'dispatch' | 'mcp';

/** `rubix-server::RunStatus` — lifecycle of an agent run. `suspended` awaits approval. */
export type RunStatus = 'completed' | 'suspended' | 'resumed' | 'cancelled';

/** `rubix-server::PendingWrite` — the command a suspended run holds for approval. */
export interface PendingWrite {
  point: string;
  priority: number;
  value: PointValue;
  agent_min_priority: number;
}

/**
 * `rubix-server::RunRecord` — the persisted agent-run row backing the operator
 * surface. `pending_write` is present only while `status === 'suspended'`.
 */
export interface RunRecord {
  id: string;
  thread_id: string;
  origin: RunOrigin;
  status: RunStatus;
  response: string;
  steps: number;
  pending_write?: PendingWrite;
  created_at: IsoTimestamp;
  updated_at: IsoTimestamp;
}

/**
 * `rubix-server::ResumeResponse` — the result of approving a suspended run. The
 * resume endpoint re-applies the held write and returns the commanded point and
 * its effective value; it does NOT echo back a `RunRecord`.
 */
export interface ResumeResponse {
  run_id: string;
  point: string;
  priority: number;
  effective?: PointValue | null;
}

/**
 * `scheduler::Trigger` — what fires a stored board. serde-tagged on `kind`
 * (snake_case). `manual` boards run only via `POST /boards/{slug}/run`.
 */
export type Trigger =
  | { kind: 'manual' }
  | { kind: 'interval'; seconds: number }
  | { kind: 'subscription'; key: string };

/** `rubix_flow::BoardNode` — a graph node naming a registered actor component. */
export interface BoardNode {
  id: string;
  component: string;
  config: Record<string, unknown>;
}

/** `rubix_flow::BoardConnection` — a directed wire between two node ports. */
export interface BoardConnection {
  from_node: string;
  from_port: string;
  to_node: string;
  to_port: string;
}

/**
 * `rubix_flow::BoardGraph` — the stored wiresheet. Nodes plus connections; there
 * is no canvas geometry on the wire, so the UI lays nodes out deterministically.
 */
export interface BoardGraph {
  nodes: BoardNode[];
  connections: BoardConnection[];
}

/** `rubix-server::BoardView` — a stored board as returned by `/api/v1/boards`. */
export interface BoardView {
  id: Uuid;
  slug: string;
  version: number;
  display_name: string;
  enabled: boolean;
  trigger: Trigger;
  graph: BoardGraph;
  created_at: IsoTimestamp;
}

/** `rubix_flow::NodeOutput` — one outport packet from a board run. */
export interface NodeOutput {
  node: string;
  port: string;
  value: unknown;
}

/** `rubix-server::RunBoardResponse` — every outport packet from one board run. */
export interface RunBoardResponse {
  outputs: NodeOutput[];
}

/**
 * `rubix_core::WidgetKind` — what a pinned dashboard tile renders. serde
 * snake_case. `point_*` kinds carry a point keyexpr in `target`; `board_output`
 * carries a board slug.
 */
export type WidgetKind = 'point_value' | 'point_history' | 'board_output';

/** `rubix_core::Widget` — a pinned dashboard tile row (`GET /api/v1/widgets`). */
export interface Widget {
  id: Uuid;
  site_id: Uuid;
  kind: WidgetKind;
  title: string;
  target: string;
  created_at: IsoTimestamp;
}

/** `rubix-server::CreateWidget` — body for `POST /api/v1/widgets`. */
export interface CreateWidget {
  site_id: Uuid;
  kind: WidgetKind;
  title: string;
  target: string;
}

/** A single DataFusion `/query` result row: column name -> value. */
export type QueryRow = Record<string, PointValue | null>;

/** `rubix-server::QueryResponse` — rows as JSON objects; no separate columns. */
export interface QueryResult {
  rows: QueryRow[];
}
