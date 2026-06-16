/**
 * React Query hooks for the meter-type admin surface. Meter-types and meters are
 * `kind:"meter-type"` / `kind:"meter"` records on the rubix records API
 * (api/records.ts); registers are `kind:"register"`. Every mutation crosses the
 * gate and invalidates the relevant list so the UI re-reads.
 */
import {
  useMutation,
  useQuery,
  useQueryClient,
} from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createRecord,
  deleteRecord,
  listRecords,
  updateRecord,
  type Meter,
  type MeterType,
  type MeterTypeRecord,
  type MeterRecord,
  type RegisterRec,
  type RegisterRecord,
} from '@/api/records'

const keys = {
  meterTypes: ['meter-type'] as const,
  meters: ['meter'] as const,
  registers: ['register'] as const,
}

export function useMeterTypes() {
  return useQuery({
    queryKey: keys.meterTypes,
    queryFn: () => listRecords<MeterType>('meter-type'),
  })
}

export function useMeters() {
  return useQuery({
    queryKey: keys.meters,
    queryFn: () => listRecords<Meter>('meter'),
  })
}

export function useRegisters() {
  return useQuery({
    queryKey: keys.registers,
    queryFn: () => listRecords<RegisterRecord>('register'),
  })
}

export function useCreateMeterType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (content: MeterType) => createRecord<MeterType>(content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.meterTypes })
      toast.success('Meter-type created')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useUpdateMeterType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ id, content }: { id: string; content: MeterType }) =>
      updateRecord<MeterType>(id, content),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.meterTypes })
      toast.success('Meter-type saved (version bumped)')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeleteMeterType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: string) => deleteRecord(id),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.meterTypes })
      toast.success('Meter-type deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

/**
 * Re-apply a meter-type to one meter (DOMAIN-MODEL §versioning): replace the
 * meter's `kind:"register"` records with the type's current `registers[]` and
 * advance the meter's `meter_type_version`. Done as explicit gate writes — delete
 * the meter's old registers, create the new stamped set, then patch the meter.
 */
export function useReapplyMeterType() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: async (args: {
      meter: MeterRecord
      type: MeterTypeRecord
      meterRegisters: RegisterRec[]
    }) => {
      const { meter, type, meterRegisters } = args
      // Remove the meter's existing registers (owned by the meter post-stamp).
      for (const reg of meterRegisters) await deleteRecord(reg.id)
      // Stamp the type's current defs onto the meter (key unique per meter).
      for (const def of type.content.registers) {
        await createRecord<RegisterRecord>({
          ...def,
          key: `${meter.content.key}--${def.key}`,
          meter: meter.id,
          tags: meter.content.tags,
        })
      }
      // Advance the meter's stamped type version.
      await updateRecord<Meter>(meter.id, {
        ...meter.content,
        meter_type_version: type.content.version,
      })
    },
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.meters })
      qc.invalidateQueries({ queryKey: keys.registers })
      toast.success('Meter-type re-applied')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
