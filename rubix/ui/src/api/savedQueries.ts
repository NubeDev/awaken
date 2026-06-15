// Saved queries as records — `kind:"query"` over the generic record surface
// (§1a/§2, LAMINAR-BORROW.md). No new table: a saved query is a tagged record
// holding its SQL and an optional chart config, so it rides the gate, audit,
// scoped-session, and live-query machinery like every other artifact.

import type { ApiClient } from './client'
import type { Record, RecordContent } from '../types/Record'
import { createRecord, deleteRecord, listRecords, updateRecord } from './records'
import type { ChartConfig } from '../components/chart-builder/types'

export const QUERY_KIND = 'query'

// The content a `kind:"query"` record carries. `chart` is the chart-builder
// config the result was last visualised with (optional — a query may be
// table-only). Mirrors Laminar's saved-query shape minus the structured spec
// (the no-SQL builder is deferred, §1c).
export interface SavedQueryContent {
  kind: typeof QUERY_KIND
  name: string
  sql: string
  chart?: ChartConfig
}

// Build the wire content for a saved query. The cast bridges the typed shape to
// the generic, index-signed `RecordContent` the record surface accepts.
function queryContent(input: { name: string; sql: string; chart?: ChartConfig }): RecordContent {
  return { kind: QUERY_KIND, ...input } as unknown as RecordContent
}

export interface SavedQuery {
  id: string
  name: string
  sql: string
  chart?: ChartConfig
  updated: string
}

function toSavedQuery(record: Record): SavedQuery {
  const c = record.content as Partial<SavedQueryContent>
  return {
    id: record.id,
    name: typeof c.name === 'string' ? c.name : '(untitled)',
    sql: typeof c.sql === 'string' ? c.sql : '',
    chart: c.chart,
    updated: record.updated,
  }
}

export async function listSavedQueries(client: ApiClient): Promise<SavedQuery[]> {
  const records = await listRecords(client, { kind: QUERY_KIND })
  return records.map(toSavedQuery)
}

export async function createSavedQuery(
  client: ApiClient,
  input: { name: string; sql: string; chart?: ChartConfig },
): Promise<SavedQuery> {
  return toSavedQuery(await createRecord(client, { content: queryContent(input) }))
}

export async function updateSavedQuery(
  client: ApiClient,
  id: string,
  input: { name: string; sql: string; chart?: ChartConfig },
): Promise<SavedQuery> {
  return toSavedQuery(await updateRecord(client, id, queryContent(input)))
}

export function deleteSavedQuery(client: ApiClient, id: string): Promise<void> {
  return deleteRecord(client, id)
}
