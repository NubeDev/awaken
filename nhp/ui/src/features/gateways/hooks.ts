/**
 * React Query hooks for the gateway + network admin surface. Gateways and
 * networks are `kind:"gateway"` / `kind:"network"` records on the rubix records
 * API (api/records.ts). Meters (`kind:"meter"`) are read-only here — used only to
 * compute device-count vs each network's cap. Every mutation crosses the gate and
 * invalidates the relevant list.
 *
 * status/last_seen on a gateway are POLLER-OWNED (DOMAIN-MODEL) — the create/edit
 * mutations below never send them; see gateway-form.tsx.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createRecord,
  deleteRecord,
  listRecords,
  updateRecord,
  type Gateway,
  type Meter,
  type Network,
  type Site,
} from '@/api/records'

const keys = {
  gateways: ['gateway'] as const,
  networks: ['network'] as const,
  meters: ['meter'] as const,
  sites: ['site'] as const,
}

/** Sites, read-only — for the gateway's required parent `site` relation picker. */
export function useSites() {
  return useQuery({
    queryKey: keys.sites,
    queryFn: () => listRecords<Site>('site'),
  })
}

export function useGateways() {
  return useQuery({
    queryKey: keys.gateways,
    queryFn: () => listRecords<Gateway>('gateway'),
  })
}

export function useNetworks() {
  return useQuery({
    queryKey: keys.networks,
    queryFn: () => listRecords<Network>('network'),
  })
}

/** Meters, read-only — for device-count vs `max_devices` (capacity.ts). */
export function useMeters() {
  return useQuery({
    queryKey: keys.meters,
    queryFn: () => listRecords<Meter>('meter'),
  })
}

export function useCreateGateway() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (content: Gateway) => createRecord<Gateway>(content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.gateways })
      toast.success('Gateway created')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useUpdateGateway() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: Gateway }) =>
      updateRecord<Gateway>(id, content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.gateways })
      toast.success('Gateway saved')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeleteGateway() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.gateways })
      qc.invalidateQueries({ queryKey: keys.networks })
      toast.success('Gateway deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useCreateNetwork() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (content: Network) => createRecord<Network>(content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.networks })
      toast.success('Network created')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useUpdateNetwork() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: Network }) =>
      updateRecord<Network>(id, content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.networks })
      toast.success('Network saved')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeleteNetwork() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.networks })
      toast.success('Network deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
