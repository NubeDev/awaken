// The generic record as the server returns it (see RecordDto in
// crates/rubix-server/src/dto/record.rs). Everything in the product is a record
// distinguished by `content.kind` — no domain type is baked into the wire.

export interface Record {
  id: string
  namespace: string
  content: RecordContent
  created: string
  updated: string
}

// Content is free-form JSON; `kind` is the only convention every seeded record
// carries. The rest is read defensively per kind.
export interface RecordContent {
  kind?: string
  [key: string]: unknown
}

export interface CreateRecordRequest {
  content: Record['content']
}
