// Record resource calls — GET/POST /records. HTTP only, no state, no React.

import type { ApiClient } from './client'
import type { CreateRecordRequest, Record } from '../types/Record'

// Optional kind/tag narrowing — the backend filters on the scoped session
// (GET /records?kind=&tag=), so a grid reading one collection asks for just it.
export function listRecords(
  client: ApiClient,
  filter?: { kind?: string; tag?: string },
): Promise<Record[]> {
  const params = new URLSearchParams()
  if (filter?.kind) params.set('kind', filter.kind)
  if (filter?.tag) params.set('tag', filter.tag)
  const qs = params.toString()
  return client.get<Record[]>(qs ? `records?${qs}` : 'records')
}

export function updateRecord(
  client: ApiClient,
  id: string,
  content: Record['content'],
): Promise<Record> {
  return client.patch<Record>(`records/${encodeURIComponent(id)}`, { content })
}

export function deleteRecord(client: ApiClient, id: string): Promise<void> {
  return client.del(`records/${encodeURIComponent(id)}`)
}

export function getRecord(client: ApiClient, id: string): Promise<Record> {
  return client.get<Record>(`records/${encodeURIComponent(id)}`)
}

export function createRecord(client: ApiClient, body: CreateRecordRequest): Promise<Record> {
  return client.post<Record>('records', body)
}
