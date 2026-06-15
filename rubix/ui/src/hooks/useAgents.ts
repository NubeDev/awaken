// TanStack Query hooks for the agent surface — list/provision agents and the
// ask bar. Scoped by tenant so switching tenant refetches with the right scope
// (mirrors hooks/useAdmin.ts + useAdminMutations.ts). Substrate-only.

import { useMutation, useQuery, useQueryClient } from '@tanstack/react-query'
import { useApi } from '../api/ConnectionContext'
import { askAgent, listAgents, provisionAgent } from '../api/agents'
import type {
  Agent,
  AskRequest,
  AskResponse,
  ProvisionAgentRequest,
  ProvisionedAgent,
} from '../types/Agent'

export function useAgents(tenant: string) {
  const api = useApi(tenant)
  return useQuery<Agent[]>({
    queryKey: ['agents', tenant],
    queryFn: () => listAgents(api),
    staleTime: 10_000,
  })
}

export function useAgentMutations(tenant: string) {
  const api = useApi(tenant)
  const qc = useQueryClient()
  const invalidate = () => qc.invalidateQueries({ queryKey: ['agents', tenant] })

  const provision = useMutation<ProvisionedAgent, Error, ProvisionAgentRequest>({
    mutationFn: (body) => provisionAgent(api, body),
    onSuccess: invalidate,
  })
  return { provision }
}

// The Copilot ask bar. A mutation (not a query) — each ask is an explicit action
// with its own result, not cached state.
export function useAskAgent(tenant: string) {
  const api = useApi(tenant)
  return useMutation<AskResponse, Error, AskRequest>({
    mutationFn: (body) => askAgent(api, body),
  })
}
