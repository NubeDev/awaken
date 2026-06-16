/**
 * Typed access to the rubix generic records API — the surface every NHP domain
 * entity lives on (WS-02/03). A record is `{ id, namespace, content, tags, … }`
 * (rubix/crates/rubix-server/src/dto/record.rs); `content.kind` selects the
 * collection. The routes are bare (NOT under `/api/v1`):
 *   POST   /records              { content }      → RecordDto
 *   GET    /records?kind=…&tag=…                  → RecordDto[]
 *   PATCH  /records/:id          { content }      → RecordDto
 *   DELETE /records/:id
 * Writes cross the gate (audited/undoable). Auth: see api/client.ts.
 */
import { request } from './client'
import type {
  ByteOrder,
  ChartType,
  Datatype,
  FnCode,
  NetType,
  Protocol,
  Status,
} from '@/enums/options'

/** A record as the rubix list/get read returns it. `content` is free-form JSON. */
export interface RecordDto<C = Record<string, unknown>> {
  id: string
  namespace: string
  content: C
  /** Tag names; only the LIST read joins the tag graph (single reads return []). */
  tags: string[]
  created: string
  updated: string
}

/**
 * Alarm threshold ramp (DOMAIN-MODEL §Alarms): ordered steps the chart colours by
 * and the rule engine fires on. The baseline step carries `value: null`.
 */
export type AlarmSeverity = 'ok' | 'warning' | 'critical'
export interface AlarmThreshold {
  value: number | null
  severity: AlarmSeverity
}
export interface Alarm {
  thresholds: AlarmThreshold[]
  /** Optional dwell before firing (hysteresis), e.g. "5m". */
  for?: string
}

/**
 * One register definition inside a meter-type's `registers[]` (DOMAIN-MODEL
 * §register). The same shape is stamped onto a meter's `kind:"register"` records.
 */
export interface RegisterDef {
  key: string
  name: string
  // protocol metadata (consumed by the external poller)
  address: number
  fn_code: FnCode
  datatype: Datatype
  word_count: number
  byte_order: ByteOrder
  scale: number
  offset: number
  signed: boolean
  // semantics & presentation (consumed by NHP/dashboards)
  unit: string
  quantity: string
  history: boolean
  chart_type: ChartType
  chart_group: string
  precision: number
  // alarms (optional)
  alarm?: Alarm
}

/** `kind:"meter-type"` content — the admin template (DOMAIN-MODEL §meter-type). */
export interface MeterType {
  kind: 'meter-type'
  key: string
  name: string
  manufacturer?: string
  version: number
  registers: RegisterDef[]
  /**
   * The printable scan code "on the box" (WS-09). Optional: existing types have no
   * stored value and fall back to one derived from `key` — see enums/barcode.ts.
   */
  barcode?: string
  tags?: string[]
}

/** `kind:"meter"` content — a stamped meter (DOMAIN-MODEL §meter). */
export interface Meter {
  kind: 'meter'
  key: string
  name: string
  network: string
  meter_type: string
  meter_type_version: number
  address: number
  status?: string
  last_seen?: string
  tags?: string[]
}

/** `kind:"register"` content — a meter's concrete register, stamped from a def. */
export interface RegisterRecord extends RegisterDef {
  meter: string
  tags?: string[]
}

/**
 * `kind:"tenant"` content (DOMAIN-MODEL §tenant) — the portfolio root. A tenant
 * owns sites; `namespace` is the rubix namespace its data lives under (seed
 * portfolio.mjs). Has no parent relation.
 */
export interface Tenant {
  kind: 'tenant'
  key: string
  name: string
  namespace?: string
  tags?: string[]
}

/** `kind:"site"` content (DOMAIN-MODEL §site) — read-only here, for the gateway's
 * required parent `site` relation picker. */
export interface Site {
  kind: 'site'
  key: string
  name: string
  tenant?: string
  address?: string
  /** IANA tz; dashboards render site-local time (DOMAIN-MODEL §site). */
  timezone?: string
  geo?: string
  tags?: string[]
}

/**
 * `kind:"gateway"` content (DOMAIN-MODEL §gateway). `status`/`last_seen` are
 * written by the external poller and are READ-ONLY in NHP — the admin UI never
 * sets them (DOMAIN-MODEL "Status fields are poller-owned").
 */
export interface Gateway {
  kind: 'gateway'
  key: string
  name: string
  site?: string
  model?: string
  /** Address the poller uses; NHP only stores it. */
  host?: string
  /** Poller-written. Do not set from the UI. */
  status?: Status
  /** Poller-written. Do not set from the UI. */
  last_seen?: string
  tags?: string[]
}

/** Serial params for a `485` network (DOMAIN-MODEL §network `params`). */
export interface Net485Params {
  baud: number
  parity: 'none' | 'even' | 'odd'
  stop_bits: number
  data_bits: number
}

/** TCP params for an `ethernet` network. */
export interface NetEthernetParams {
  ip: string
  port: number
}

export type NetParams = Net485Params | NetEthernetParams

/**
 * `kind:"network"` content (DOMAIN-MODEL §network). `net_type` selects the
 * `params` shape (serial vs tcp). `max_devices` is the per-network device cap
 * enforced client-side (DOMAIN-MODEL "Device limit"; rubix gate can't count).
 */
export interface Network {
  kind: 'network'
  key: string
  name?: string
  gateway: string
  net_type: NetType
  protocol: Protocol
  max_devices: number
  params?: NetParams
  tags?: string[]
}

export type TenantRecord = RecordDto<Tenant>
export type SiteRecord = RecordDto<Site>
export type GatewayRecord = RecordDto<Gateway>
export type NetworkRecord = RecordDto<Network>
export type MeterTypeRecord = RecordDto<MeterType>
export type MeterRecord = RecordDto<Meter>
export type RegisterRec = RecordDto<RegisterRecord>

// --- CRUD over /records, narrowed by content.kind --------------------------------

export async function listRecords<C>(kind: string): Promise<RecordDto<C>[]> {
  return request<RecordDto<C>[]>('/records', { query: { kind } })
}

export async function createRecord<C>(content: C): Promise<RecordDto<C>> {
  return request<RecordDto<C>>('/records', { method: 'POST', body: { content } })
}

export async function updateRecord<C>(
  id: string,
  content: C
): Promise<RecordDto<C>> {
  return request<RecordDto<C>>(`/records/${id}`, {
    method: 'PATCH',
    body: { content },
  })
}

export async function deleteRecord(id: string): Promise<void> {
  await request<void>(`/records/${id}`, { method: 'DELETE' })
}
