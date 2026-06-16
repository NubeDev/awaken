/**
 * React Query hooks for the users & service-accounts admin surface — the rubix
 * principal + grant API (api/admin.ts), NOT the records API. Users (`kind:"user"`)
 * and service accounts (`kind:"extension"`, e.g. the poller `agent`) are one
 * identity model (ADMIN.md §6); the list shows both.
 *
 * AUTH: these routes are admin-only (require_admin). api/admin.ts sends the seeded
 * admin credential; if it is wrong/missing every call 403s and the error surfaces
 * via the toast — no silent failure (WS-05.md reachability decision).
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { toast } from 'sonner'
import {
  createPrincipal,
  deletePrincipal,
  listGrants,
  listPrincipals,
  setPrincipalRole,
  type CreatePrincipalBody,
  type PrincipalRole,
} from '@/api/admin'

const keys = {
  principals: ['principals'] as const,
  grants: (subject: string) => ['grants', subject] as const,
}

export function usePrincipals() {
  return useQuery({
    queryKey: keys.principals,
    queryFn: listPrincipals,
  })
}

/** A principal's capability grants (read-only display in the POC). */
export function useGrants(subject: string, enabled: boolean) {
  return useQuery({
    queryKey: keys.grants(subject),
    queryFn: () => listGrants(subject),
    enabled,
  })
}

export function useCreatePrincipal() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreatePrincipalBody) => createPrincipal(body),
    onSuccess: (created) => {
      qc.invalidateQueries({ queryKey: keys.principals })
      // The minted secret is returned exactly once — surface it so it isn't lost.
      if (created.secret) {
        toast.success(
          `Created ${created.subject}. Secret (shown once): ${created.secret}`,
          { duration: 30_000 }
        )
      } else {
        toast.success(`Created ${created.subject}`)
      }
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useSetRole() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: ({ subject, role }: { subject: string; role: PrincipalRole }) =>
      setPrincipalRole(subject, role),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.principals })
      toast.success('Role updated')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}

export function useDeletePrincipal() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (subject: string) => deletePrincipal(subject),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: keys.principals })
      toast.success('Principal deleted')
    },
    onError: (e: Error) => toast.error(e.message),
  })
}
