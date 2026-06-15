// Fetch the tenant's records via TanStack Query. The query key is scoped by
// tenant so switching tenant refetches with the right scope. The live plane
// (/ws/records) would invalidate this key on each event — wired when the WS
// hook lands; for now records refetch on focus.

import { useQuery } from '@tanstack/react-query'
import { useApi } from '../api/ConnectionContext'
import { listRecords } from '../api/records'
import type { Record } from '../types/Record'

export function useRecords(tenant: string) {
  const api = useApi(tenant)
  return useQuery<Record[]>({
    queryKey: ['records', tenant],
    queryFn: () => listRecords(api),
    staleTime: 10_000,
  })
}
