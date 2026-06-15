// Pinned dashboards as records — `kind:"board"` over the generic record surface
// (§2, LAMINAR-BORROW.md). A board holds an ordered list of chart references with
// their grid layout; each referenced chart is a `kind:"chart"` record. Layout
// changes persist as a gate write (debounced by the caller), so every drag/resize
// is audited and undoable — resolving PRODUCT-UI's localStorage board gap.

import type { ApiClient } from './client'
import type { Record, RecordContent } from '../types/Record'
import { createRecord, deleteRecord, listRecords, updateRecord } from './records'

export const BOARD_KIND = 'board'

// One placed chart on a board: which chart record, and its grid rectangle.
export interface BoardPanel {
  chart_id: string
  x: number
  y: number
  w: number
  h: number
}

export interface BoardContent {
  kind: typeof BOARD_KIND
  name: string
  panels: BoardPanel[]
}

export interface SavedBoard {
  id: string
  name: string
  panels: BoardPanel[]
  updated: string
}

function boardContent(input: { name: string; panels: BoardPanel[] }): RecordContent {
  return { kind: BOARD_KIND, ...input } as unknown as RecordContent
}

function toSavedBoard(record: Record): SavedBoard {
  const c = record.content as Partial<BoardContent>
  return {
    id: record.id,
    name: typeof c.name === 'string' ? c.name : '(untitled)',
    panels: Array.isArray(c.panels) ? (c.panels as BoardPanel[]) : [],
    updated: record.updated,
  }
}

export async function listBoards(client: ApiClient): Promise<SavedBoard[]> {
  const records = await listRecords(client, { kind: BOARD_KIND })
  return records.map(toSavedBoard)
}

export async function createBoard(client: ApiClient, name: string): Promise<SavedBoard> {
  return toSavedBoard(await createRecord(client, { content: boardContent({ name, panels: [] }) }))
}

export async function updateBoard(
  client: ApiClient,
  id: string,
  input: { name: string; panels: BoardPanel[] },
): Promise<SavedBoard> {
  return toSavedBoard(await updateRecord(client, id, boardContent(input)))
}

export function deleteBoard(client: ApiClient, id: string): Promise<void> {
  return deleteRecord(client, id)
}
