// Rule resource calls — CRUD plus the studio's two surfaces: a side-effect-free
// dry-run (run a draft against real history without firing) and the referencing
// (blast-radius) read. HTTP only, no state, no React (mirrors api/records.ts);
// backs the rules studio onto crates/rubix-server/src/http/rules/*.
//
// A rule is authored as a Rhai script over time-window bindings (table/field/
// grain/aggregate) and emits an insight kind — the rubix-rules model. Rules
// persist server-side as `kind:"rule"` records, but the UI talks to the dedicated
// `/rules` surface (gated on rule-define, validated, with dry-run/referencing),
// never to raw records.

import type { ApiClient } from './client'

/** One declared input a rule's script reads as a time-window value. */
export interface Binding {
  /** The script variable name this value binds to. */
  name: string
  /** The canonical table the series is read from. */
  table: CanonicalTable
  /** The numeric `content.<field>` series rolled up. */
  field: string
  /** The bucket width. */
  grain: Grain
  /** The bucket aggregate the rule decides on. */
  aggregate: Aggregate
}

export type CanonicalTable = 'records' | 'tags' | 'audit' | 'insights' | 'trace_summary'
export type Grain = 'minute' | 'hour' | 'day' | 'week'
export type Aggregate = 'avg' | 'min' | 'max' | 'sum' | 'count' | 'first' | 'last'

export const TABLES: CanonicalTable[] = ['records', 'tags', 'audit', 'insights', 'trace_summary']
export const GRAINS: Grain[] = ['minute', 'hour', 'day', 'week']
export const AGGREGATES: Aggregate[] = ['avg', 'min', 'max', 'sum', 'count', 'first', 'last']

/** A rule as the server returns it — the full definition plus storage metadata. */
export interface Rule {
  /** The record id the rule is stored under (its delete handle). */
  id: string
  /** The stable name — the composition handle and the audited target. */
  name: string
  /** The Rhai script. */
  script: string
  /** The window-value inputs the script reads. */
  inputs: Binding[]
  /** The names of the sub-rules this script may `invoke`. */
  subrules: string[]
  /** The insight kind this rule's decision is recorded under. */
  output: string
  created: string
  updated: string
}

/** The body of a create-rule request. */
export interface CreateRuleRequest {
  name: string
  script: string
  inputs: Binding[]
  subrules: string[]
  output: string
}

/** The body of an update-rule request — the name is immutable, so it is omitted. */
export interface UpdateRuleRequest {
  script: string
  inputs: Binding[]
  subrules: string[]
  output: string
}

/** The body of a dry-run — the on-screen (possibly unsaved) draft. */
export interface DryRunRequest {
  script: string
  inputs: Binding[]
  subrules: string[]
}

/** One window bucket the debugger charts — the frame a binding saw. */
export interface Bucket {
  bucket_start: number
  avg: number
  min: number
  max: number
  sum: number
  count: number
  first: number
  last: number
}

/** One resolved input: the buckets it saw and the value it selected. */
export interface ResolvedInput {
  name: string
  buckets: Bucket[]
  value: number
}

/** The verdict of a dry-run: the decision and the frame it decided on. */
export interface DryRunResponse {
  fired: boolean
  value: number
  reason: string
  inputs: ResolvedInput[]
}

export function listRules(client: ApiClient): Promise<Rule[]> {
  return client.get<Rule[]>('rules')
}

export function createRule(client: ApiClient, body: CreateRuleRequest): Promise<Rule> {
  return client.post<Rule>('rules', body)
}

export function updateRule(client: ApiClient, name: string, body: UpdateRuleRequest): Promise<Rule> {
  return client.patch<Rule>(`rules/${encodeURIComponent(name)}`, body)
}

export function deleteRule(client: ApiClient, name: string): Promise<void> {
  return client.del(`rules/${encodeURIComponent(name)}`)
}

// Dry-run is a read in effect (a verdict over the caller's own visible history),
// so it needs no rule-define grant. The draft travels in the body; `name` is just
// the route label for the rule being debugged.
export function dryRunRule(
  client: ApiClient,
  name: string,
  body: DryRunRequest,
): Promise<DryRunResponse> {
  return client.post<DryRunResponse>(`rules/${encodeURIComponent(name)}/dryrun`, body)
}

// The rules that compose `name` via `invoke` — the blast radius shown before an
// edit or delete changes them on the next tick.
export function referencingRules(client: ApiClient, name: string): Promise<Rule[]> {
  return client.get<Rule[]>(`rules/${encodeURIComponent(name)}/referencing`)
}
