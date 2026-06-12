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
  tags: string[];
  created_at: IsoTimestamp;
}

export interface Equip {
  id: Uuid;
  site_id: Uuid;
  path: string;
  display_name: string;
  tags: string[];
  created_at: IsoTimestamp;
}

export interface Point {
  id: Uuid;
  equip_id: Uuid;
  slug: string;
  display_name: string;
  kind: PointKind;
  unit: string | null;
  tags: string[];
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

export type WriteSource = 'operator' | 'agent' | 'schedule';

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

export interface RunSummary {
  id: string;
  status: string;
  title?: string;
  started_at?: IsoTimestamp;
}

/** A DataFusion `/query` result: column names plus row arrays. */
export interface QueryResult {
  columns: string[];
  rows: PointValue[][];
}
