// Mutations for the admin console — record CRUD, principal CRUD, grant toggles.
// Each invalidates the matching query key on success so the table reflects the
// write (the WS live plane will replace polling later). Substrate-only.

import { useMutation, useQueryClient } from '@tanstack/react-query'
import { useApi } from '../api/ConnectionContext'
import { createRecord, updateRecord, deleteRecord } from '../api/records'
import {
  createPrincipal,
  updatePrincipalRole,
  deletePrincipal,
  grantCapability,
  revokeCapability,
} from '../api/admin'
import type { Record, RecordContent } from '../types/Record'
import type { CreatePrincipalRequest, CreatedPrincipal } from '../types/Admin'

export function useRecordMutations(tenant: string) {
  const api = useApi(tenant)
  const qc = useQueryClient()
  const invalidate = () => qc.invalidateQueries({ queryKey: ['records', tenant] })

  const create = useMutation({
    mutationFn: (content: RecordContent) => createRecord(api, { content }),
    onSuccess: invalidate,
  })
  const update = useMutation({
    mutationFn: ({ id, content }: { id: string; content: RecordContent }) =>
      updateRecord(api, id, content),
    onSuccess: invalidate,
  })
  const remove = useMutation({
    mutationFn: (id: string) => deleteRecord(api, id),
    onSuccess: invalidate,
  })
  return { create, update, remove }
}

export function usePrincipalMutations(tenant: string) {
  const api = useApi(tenant)
  const qc = useQueryClient()
  const invalidate = () => qc.invalidateQueries({ queryKey: ['principals', tenant] })

  const create = useMutation<CreatedPrincipal, Error, CreatePrincipalRequest>({
    mutationFn: (body) => createPrincipal(api, body),
    onSuccess: invalidate,
  })
  const setRole = useMutation({
    mutationFn: ({ subject, role }: { subject: string; role: string }) =>
      updatePrincipalRole(api, subject, role),
    onSuccess: invalidate,
  })
  const remove = useMutation({
    mutationFn: (subject: string) => deletePrincipal(api, subject),
    onSuccess: invalidate,
  })
  return { create, setRole, remove }
}

export function useGrantMutations(tenant: string, subject: string) {
  const api = useApi(tenant)
  const qc = useQueryClient()
  const invalidate = () => qc.invalidateQueries({ queryKey: ['grants', tenant, subject] })

  const grant = useMutation({
    mutationFn: (capability: string) => grantCapability(api, subject, capability),
    onSuccess: invalidate,
  })
  const revoke = useMutation({
    mutationFn: (capability: string) => revokeCapability(api, subject, capability),
    onSuccess: invalidate,
  })
  return { grant, revoke }
}

export type { Record }
