/**
 * React Query hooks for the meters admin surface. Meters are `kind:"meter"`
 * records on the rubix records API (api/records.ts) — stamped from a meter-type
 * onto a network (creating one is the meters-wizard's job, WS-06). This surface is
 * read + delete only. Networks (`kind:"network"`) and meter-types
 * (`kind:"meter-type"`) are read-only here, to resolve a meter's relations to
 * display labels.
 *
 * status/last_seen on a meter are POLLER-OWNED (DOMAIN-MODEL) — shown read-only.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  deleteRecord,
  listRecords,
  type Meter,
  type MeterType,
  type Network,
} from '@/api/records'

const keys = {
  meters: ['meter'] as const,
  networks: ['network'] as const,
  meterTypes: ['meter-type'] as const,
}

export function useMeters() {
  return useQuery({
    queryKey: keys.meters,
    queryFn: () => listRecords<Meter>('meter'),
  })
}

/** Networks, read-only — to resolve a meter's parent `network` to a label. */
export function useNetworks() {
  return useQuery({
    queryKey: keys.networks,
    queryFn: () => listRecords<Network>('network'),
  })
}

/** Meter-types, read-only — to resolve a meter's `meter_type` to a label. */
export function useMeterTypes() {
  return useQuery({
    queryKey: keys.meterTypes,
    queryFn: () => listRecords<MeterType>('meter-type'),
  })
}

export function useDeleteMeter() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.meters })
      toast.success('Meter deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
