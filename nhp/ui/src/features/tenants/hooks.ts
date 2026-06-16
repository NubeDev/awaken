/**
 * React Query hooks for the tenant admin surface. Tenants are `kind:"tenant"`
 * records on the rubix records API (api/records.ts) — the portfolio root that
 * owns sites. Sites (`kind:"site"`) are read-only here, used only to roll up the
 * site-count per tenant. Every mutation crosses the gate and invalidates the list.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createRecord,
  deleteRecord,
  listRecords,
  updateRecord,
  type Site,
  type Tenant,
} from '@/api/records'

const keys = {
  tenants: ['tenant'] as const,
  sites: ['site'] as const,
}

export function useTenants() {
  return useQuery({
    queryKey: keys.tenants,
    queryFn: () => listRecords<Tenant>('tenant'),
  })
}

/** Sites, read-only — for the per-tenant site-count rollup. */
export function useSites() {
  return useQuery({
    queryKey: keys.sites,
    queryFn: () => listRecords<Site>('site'),
  })
}

export function useCreateTenant() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (content: Tenant) => createRecord<Tenant>(content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.tenants })
      toast.success('Tenant created')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useUpdateTenant() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: Tenant }) =>
      updateRecord<Tenant>(id, content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.tenants })
      toast.success('Tenant saved')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeleteTenant() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.tenants })
      toast.success('Tenant deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
