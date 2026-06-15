// TanStack Query hooks for the admin console — records, collections, principals.
// All scoped by tenant so switching tenant refetches with the right scope. These
// read the substrate only; no hook here knows a domain type.

import { useQuery } from '@tanstack/react-query'
import { useApi } from '../api/ConnectionContext'
import { listRecords } from '../api/records'
import { listCollections } from '../api/collections'
import { listPrincipals } from '../api/admin'
import { me } from '../api/auth'
import type { Record } from '../types/Record'
import type { CollectionDef } from '../api/collections'
import type { Me, Principal } from '../types/Admin'

export function useAllRecords(tenant: string, kind?: string) {
  const api = useApi(tenant)
  return useQuery<Record[]>({
    queryKey: ['records', tenant, kind ?? 'all'],
    queryFn: () => listRecords(api, kind ? { kind } : undefined),
    staleTime: 10_000,
  })
}

export function useCollections(tenant: string) {
  const api = useApi(tenant)
  return useQuery<CollectionDef[]>({
    queryKey: ['collections', tenant],
    queryFn: () => listCollections(api),
    staleTime: 30_000,
  })
}

export function usePrincipals(tenant: string) {
  const api = useApi(tenant)
  return useQuery<Principal[]>({
    queryKey: ['principals', tenant],
    queryFn: () => listPrincipals(api),
    staleTime: 10_000,
  })
}

export function useMe(tenant: string) {
  const api = useApi(tenant)
  return useQuery<Me>({
    queryKey: ['me', tenant],
    queryFn: () => me(api),
    staleTime: 60_000,
  })
}
