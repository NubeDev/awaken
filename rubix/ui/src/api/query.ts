// Query surface — POST /query (DataFusion). Read-only, gated on external-query.

import type { ApiClient } from './client'

export interface QueryResponse {
  rows: Record<string, unknown>[]
}

export function runQuery(client: ApiClient, sql: string): Promise<QueryResponse> {
  return client.post<QueryResponse>('query', { sql })
}
