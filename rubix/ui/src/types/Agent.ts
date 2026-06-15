// Wire shapes for the agent surface, mirroring the Rust DTOs in
// crates/rubix-server/src/dto/agent.rs. The agent is a substrate concept — a
// scoped service-account principal at a tier — not a domain entity. The admin
// agent screen and the Copilot ask bar are built on these.

/** A provisioned agent as returned by the API — identity + tier, never a secret. */
export interface Agent {
  subject: string
  namespace: string
  /** 'analyst' | 'operator' | 'actuator' */
  tier: string
}

/** The provision-agent request body. Omit `secret` to have the server mint one. */
export interface ProvisionAgentRequest {
  subject: string
  tier: string
  secret?: string
}

/** The provision response — the only response that ever carries an agent secret. */
export interface ProvisionedAgent extends Agent {
  /** Present only when the server minted the secret. */
  secret?: string
}

/** The ask request: a question plus optional grounding assembled by the caller. */
export interface AskRequest {
  question: string
  context?: string
}

/** The ask response. `grounded:false` marks a model-free fallback answer. */
export interface AskResponse {
  answer: string
  /** true ⇒ the cloud brain answered; false ⇒ a degraded, model-free fallback. */
  grounded: boolean
}

/** One recalled memory: record id and distance from the probe (smaller = nearer). */
export interface Recalled {
  id: string
  distance: number
}

/** The persist response: the new memory id and the gate's correlation id. */
export interface Persisted {
  memory_id: string
  correlation_id: string
}

/** The agent tiers, strictly layered analyst ⊂ operator ⊂ actuator (AGENT.md). */
export type AgentTier = 'analyst' | 'operator' | 'actuator'
export const AGENT_TIERS: AgentTier[] = ['analyst', 'operator', 'actuator']

/** A one-line description of what each tier may do, for the provisioning UI. */
export const TIER_SUMMARY: Record<AgentTier, string> = {
  analyst: 'Read-only “ask your data” + records memory of what it read.',
  operator: 'Analyst + records insights and writes rule definitions/schedules.',
  actuator: 'Operator + commands registered physical points (setpoints, relays).',
}
