/**
 * React Query hooks over the entity-tag endpoints (docs/design/page-context-and-
 * nav.md §3). Tags are behaviour-affecting (they feed `PageContext.tags`), so
 * the write enforces the entity's own `edit` authz server-side; this client just
 * round-trips the full set. A successful `PUT` invalidates the entity's set so a
 * dependent board re-reads its tags.
 */
import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import * as api from '@/api/endpoints'
import { qk } from '@/api/keys'
import type { EntityTags, TagEntityKind, Uuid } from '@/api/types'

/** An entity's full tag set. Enabled only when an id is present. */
export function useEntityTags(kind: TagEntityKind, id: Uuid | undefined) {
  return useQuery({
    queryKey: qk.entityTags(kind, id as Uuid),
    queryFn: ({ signal }) => api.tags.get(kind, id as Uuid, signal),
    enabled: Boolean(id),
  })
}

/** Distinct tag keys in use for a kind (authoring autocomplete). */
export function useTagKeys(kind: TagEntityKind, org: string | undefined) {
  return useQuery({
    queryKey: qk.tagKeys(kind, org ?? 'all'),
    queryFn: ({ signal }) => api.tags.keys(kind, org as string, signal),
    enabled: Boolean(org),
  })
}

/** Full-replace an entity's tag set. */
export function useReplaceEntityTags(kind: TagEntityKind) {
  const qc = useQueryClient()
  return useMutation({
    mutationFn: (args: { id: Uuid; tags: EntityTags }) =>
      api.tags.put(kind, args.id, args.tags),
    onSuccess: (_data, args) => {
      qc.invalidateQueries({ queryKey: qk.entityTags(kind, args.id) })
      qc.invalidateQueries({ queryKey: ['tags'] })
    },
  })
}
