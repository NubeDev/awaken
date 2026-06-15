// Record resource calls — GET/POST /records. HTTP only, no state, no React.

import type { ApiClient } from './client'
import type { CreateRecordRequest, Record } from '../types/Record'

export function listRecords(client: ApiClient): Promise<Record[]> {
  return client.get<Record[]>('records')
}

export function getRecord(client: ApiClient, id: string): Promise<Record> {
  return client.get<Record>(`records/${encodeURIComponent(id)}`)
}

export function createRecord(client: ApiClient, body: CreateRecordRequest): Promise<Record> {
  return client.post<Record>('records', body)
}
