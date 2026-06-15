// Collections — read the registered collection definitions. A collection is a
// `kind: "collection"` record (crates/rubix-core/src/collection/def.rs), not a
// table, so the UI reads them through the same generic GET /records?kind=collection
// path as any other record (ADMIN-UI: "until the backend serves collection
// definitions, the registry is read as records; the screens don't change when it
// moves server-side"). This stays domain-agnostic — it knows the `collection`
// meta-kind, never a domain kind like a site or a point.

import type { ApiClient } from './client'
import { listRecords } from './records'
import type { Record } from '../types/Record'

/** A collection field type, mirroring FieldType in rubix-core. */
export type FieldType = 'text' | 'number' | 'bool' | 'date' | 'file' | 'relation'

/** One typed field a collection declares over record content (FieldDef). */
export interface FieldDef {
  name: string
  type: FieldType
  required: boolean
  unique: boolean
}

/** A parsed collection definition (CollectionDef). */
export interface CollectionDef {
  /** The kind value records of this collection carry. */
  name: string
  schema: FieldDef[]
  /** The id of the backing collection record, for drill-through. */
  recordId: string
}

export const COLLECTION_KIND = 'collection'

const FIELD_TYPES: FieldType[] = ['text', 'number', 'bool', 'date', 'file', 'relation']

/** Parse a `kind: "collection"` record's content into a CollectionDef, or null
 *  if it is malformed (defensive — a half-shaped collection record never throws). */
export function parseCollection(record: Record): CollectionDef | null {
  const content = record.content as { name?: unknown; schema?: unknown }
  const name = typeof content.name === 'string' && content.name ? content.name : null
  if (!name) return null

  const schema: FieldDef[] = Array.isArray(content.schema)
    ? content.schema.flatMap((entry) => {
        const f = entry as { name?: unknown; type?: unknown; required?: unknown; unique?: unknown }
        if (typeof f.name !== 'string' || !f.name) return []
        const type = FIELD_TYPES.includes(f.type as FieldType) ? (f.type as FieldType) : 'text'
        return [{ name: f.name, type, required: f.required === true, unique: f.unique === true }]
      })
    : []

  return { name, schema, recordId: record.id }
}

export async function listCollections(client: ApiClient): Promise<CollectionDef[]> {
  const records = await listRecords(client, { kind: COLLECTION_KIND })
  return records.map(parseCollection).filter((c): c is CollectionDef => c !== null)
}
