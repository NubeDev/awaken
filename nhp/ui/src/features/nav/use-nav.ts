/**
 * React Query hooks over the nav-tree endpoints (docs/design/page-context-and-
 * nav.md §§4,6). The list is server-filtered to the nodes the caller holds
 * `view` on, so the sidebar renders only granted nodes without a client-side
 * gate. Mutations invalidate the tree so the sidebar and builder stay in sync.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import * as api from '@/api/endpoints'
import { qk } from '@/api/keys'
import type { CreateNavNode, PatchNavNode, Uuid } from '@/api/types'

/** The org's nav tree (flat, in `parent_id`/`sort_order` order). Empty while
 *  `org` is undefined. */
export function useNavTree(org: string | undefined) {
  return useQuery({
    queryKey: qk.nav(org),
    queryFn: ({ signal }) => api.nav.list(org as string, signal),
    enabled: Boolean(org),
  })
}

export function useNavNode(id: Uuid | undefined) {
  return useQuery({
    queryKey: qk.navNode(id as Uuid),
    queryFn: ({ signal }) => api.nav.get(id as Uuid, signal),
    enabled: Boolean(id),
  })
}

export function useCreateNavNode() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (body: CreateNavNode) => api.nav.create(body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['nav'] }),
  })
}

export function usePatchNavNode() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (args: { id: Uuid; body: PatchNavNode }) =>
      api.nav.patch(args.id, args.body),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['nav'] }),
  })
}

export function useDeleteNavNode() {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (id: Uuid) => api.nav.remove(id),
    onSuccess: () => qc.invalidateQueries({ queryKey: ['nav'] }),
  })
}
