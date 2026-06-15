// Saved charts as records — `kind:"chart"` over the generic record surface
// (§2, LAMINAR-BORROW.md). A chart is self-contained: its SQL plus the
// chart-builder config, so a dashboard panel can re-run and render it on its own.
// No new table — it rides the gate/audit/scoped-session like every artifact.

import type { ApiClient } from './client'
import type { Record, RecordContent } from '../types/Record'
import { createRecord, deleteRecord, listRecords, updateRecord } from './records'
import type { ChartConfig } from '../components/chart-builder/types'

export const CHART_KIND = 'chart'

export interface ChartContent {
  kind: typeof CHART_KIND
  name: string
  sql: string
  config: ChartConfig
}

export interface SavedChart {
  id: string
  name: string
  sql: string
  config: ChartConfig
  updated: string
}

function chartContent(input: { name: string; sql: string; config: ChartConfig }): RecordContent {
  return { kind: CHART_KIND, ...input } as unknown as RecordContent
}

function toSavedChart(record: Record): SavedChart {
  const c = record.content as Partial<ChartContent>
  return {
    id: record.id,
    name: typeof c.name === 'string' ? c.name : '(untitled)',
    sql: typeof c.sql === 'string' ? c.sql : '',
    config: (c.config ?? {}) as ChartConfig,
    updated: record.updated,
  }
}

export async function listCharts(client: ApiClient): Promise<SavedChart[]> {
  const records = await listRecords(client, { kind: CHART_KIND })
  return records.map(toSavedChart)
}

export async function createChart(
  client: ApiClient,
  input: { name: string; sql: string; config: ChartConfig },
): Promise<SavedChart> {
  return toSavedChart(await createRecord(client, { content: chartContent(input) }))
}

export async function updateChart(
  client: ApiClient,
  id: string,
  input: { name: string; sql: string; config: ChartConfig },
): Promise<SavedChart> {
  return toSavedChart(await updateRecord(client, id, chartContent(input)))
}

export function deleteChart(client: ApiClient, id: string): Promise<void> {
  return deleteRecord(client, id)
}
