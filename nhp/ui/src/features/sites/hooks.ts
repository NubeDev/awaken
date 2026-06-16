/**
 * React Query hooks for the site admin surface. Sites are `kind:"site"` records
 * on the rubix records API (api/records.ts) — owned by a tenant (the REQUIRED
 * parent relation) and owning gateways. Tenants (`kind:"tenant"`) are read-only
 * here, for the parent picker; gateways (`kind:"gateway"`) drive the per-site
 * gateway-count rollup. Every mutation crosses the gate and invalidates the list.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createRecord,
  deleteRecord,
  listRecords,
  updateRecord,
  type Gateway,
  type Site,
  type Tenant,
} from '@/api/records'

const keys = {
  sites: ['site'] as const,
  tenants: ['tenant'] as const,
  gateways: ['gateway'] as const,
}

export function useSites() {
  return useQuery({
    queryKey: keys.sites,
    queryFn: () => listRecords<Site>('site'),
  })
}

/** Tenants, read-only — for the site's required parent `tenant` relation picker. */
export function useTenants() {
  return useQuery({
    queryKey: keys.tenants,
    queryFn: () => listRecords<Tenant>('tenant'),
  })
}

/** Gateways, read-only — for the per-site gateway-count rollup. */
export function useGateways() {
  return useQuery({
    queryKey: keys.gateways,
    queryFn: () => listRecords<Gateway>('gateway'),
  })
}

export function useCreateSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (content: Site) => createRecord<Site>(content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.sites })
      toast.success('Site created')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useUpdateSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: Site }) =>
      updateRecord<Site>(id, content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.sites })
      toast.success('Site saved')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeleteSite() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.sites })
      toast.success('Site deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
