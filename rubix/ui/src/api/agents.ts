// Agent calls — provision/list agents, the memory seam, and the ask surface.
// HTTP only, no state, no React (mirrors api/admin.ts). Backs the admin agent
// screen and the Copilot ask bar onto crates/rubix-server/src/http/agent/*.

import type { ApiClient } from './client'
import type {
  Agent,
  AskRequest,
  AskResponse,
  Persisted,
  ProvisionAgentRequest,
  ProvisionedAgent,
  Recalled,
} from '../types/Agent'

export function listAgents(client: ApiClient): Promise<Agent[]> {
  return client.get<Agent[]>('agent')
}

export function provisionAgent(
  client: ApiClient,
  body: ProvisionAgentRequest,
): Promise<ProvisionedAgent> {
  return client.post<ProvisionedAgent>('agent', body)
}

// Recall runs on the caller's scoped session — a read, no capability checked.
export function recallMemory(
  client: ApiClient,
  probe: number[],
  k: number,
): Promise<Recalled[]> {
  return client.post<Recalled[]>('agent/memory/recall', { probe, k })
}

// Persist crosses the gate as an `agent-memory-write` command — fail closed if
// the caller lacks the grant.
export function persistMemory(
  client: ApiClient,
  kind: string,
  text: string,
  embedding: number[],
): Promise<Persisted> {
  return client.post<Persisted>('agent/memory/persist', { kind, text, embedding })
}

// Ask the brain. The server answers with the LLM when configured, else returns a
// grounded, model-free fallback (`grounded:false`) — never a fabricated answer.
export function askAgent(client: ApiClient, body: AskRequest): Promise<AskResponse> {
  return client.post<AskResponse>('agent/ask', body)
}
