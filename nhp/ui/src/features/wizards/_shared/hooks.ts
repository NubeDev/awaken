/**
 * Read-only data the wizards need to thread parent ids and pick templates:
 * existing tenants, sites, gateways, networks, meter-types, and the meters used
 * for device-limit math (capacity.ts). All on the generic records API — the same
 * lists the admin features read; the wizard only ORCHESTRATES writes over them.
 */
import { useQuery } from '@tanstack/react-query'
import {
  listRecords,
  type Gateway,
  type Meter,
  type MeterType,
  type Network,
  type Site,
} from '@/api/records'

interface TenantContent {
  kind: 'tenant'
  key: string
  name: string
}

export function useTenants() {
  return useQuery({
    queryKey: ['tenant'],
    queryFn: () => listRecords<TenantContent>('tenant'),
  })
}

export function useSites() {
  return useQuery({
    queryKey: ['site'],
    queryFn: () => listRecords<Site>('site'),
  })
}

export function useGateways() {
  return useQuery({
    queryKey: ['gateway'],
    queryFn: () => listRecords<Gateway>('gateway'),
  })
}

export function useNetworks() {
  return useQuery({
    queryKey: ['network'],
    queryFn: () => listRecords<Network>('network'),
  })
}

export function useMeterTypes() {
  return useQuery({
    queryKey: ['meter-type'],
    queryFn: () => listRecords<MeterType>('meter-type'),
  })
}

export function useMeters() {
  return useQuery({
    queryKey: ['meter'],
    queryFn: () => listRecords<Meter>('meter'),
  })
}

export type { TenantContent }
