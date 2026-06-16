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

// A scalar a variable can resolve to. Matches the wire `QueryVariable.value`
// element type.
export type VariableScalar = string | number | boolean

// How a variable's option list is sourced (VARIABLES-AND-TEMPLATING §1). A closed
// set; the resolver (`board-variables.ts`) has one arm per kind.
export type VariableKind = 'constant' | 'custom' | 'site' | 'query' | 'textbox'

// One dashboard variable, stored in the board record's JSON so it travels with the
// board on export/import. `current` is the persisted default selection; the live
// selection is URL/context-driven at view time.
export interface BoardVariable {
  /** Referenced in SQL as `$name` / `${name}` / `$__sqlIn(name)`. */
  name: string
  label?: string
  kind: VariableKind
  /** Per-kind config: `{ options }` for custom, `{ query }` for query, etc. */
  config?: {
    /** `custom`: the static option list. */
    options?: VariableScalar[]
    /** `query`: SQL whose first column becomes the option list. */
    query?: string
  }
  /** The persisted default selection (a scalar, or an array when `multi`). */
  current?: VariableScalar | VariableScalar[]
  /** Allow selecting several values (lowers to `${name:csv}` / `$__sqlIn`). */
  multi?: boolean
  /** Offer an "All" option that expands to every resolved value. */
  include_all?: boolean
  /** Hide from the bar (a `constant`, or a context-driven value). */
  hidden?: boolean
}

export interface BoardContent {
  kind: typeof BOARD_KIND
  name: string
  panels: BoardPanel[]
  variables?: BoardVariable[]
}

export interface SavedBoard {
  id: string
  name: string
  panels: BoardPanel[]
  variables: BoardVariable[]
  updated: string
}

type BoardInput = {
  name: string
  panels: BoardPanel[]
  variables?: BoardVariable[]
}

function boardContent(input: BoardInput): RecordContent {
  return { kind: BOARD_KIND, ...input } as unknown as RecordContent
}

function toSavedBoard(record: Record): SavedBoard {
  const c = record.content as Partial<BoardContent>
  return {
    id: record.id,
    name: typeof c.name === 'string' ? c.name : '(untitled)',
    panels: Array.isArray(c.panels) ? (c.panels as BoardPanel[]) : [],
    variables: Array.isArray(c.variables) ? (c.variables as BoardVariable[]) : [],
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
  input: BoardInput,
): Promise<SavedBoard> {
  return toSavedBoard(await updateRecord(client, id, boardContent(input)))
}

export function deleteBoard(client: ApiClient, id: string): Promise<void> {
  return deleteRecord(client, id)
}
