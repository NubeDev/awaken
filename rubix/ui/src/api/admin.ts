// Admin & management calls — principals and grants. HTTP only, no state, no
// React (mirrors api/records.ts). Backs the principals/grants admin screen onto
// the control-plane surface in crates/rubix-server/src/http/admin/*.

import type { ApiClient } from './client'
import type {
  CreatePrincipalRequest,
  CreatedPrincipal,
  CreateTeamRequest,
  Grant,
  Principal,
  Team,
  TeamMember,
} from '../types/Admin'

export function listPrincipals(client: ApiClient): Promise<Principal[]> {
  return client.get<Principal[]>('principals')
}

export function createPrincipal(
  client: ApiClient,
  body: CreatePrincipalRequest,
): Promise<CreatedPrincipal> {
  return client.post<CreatedPrincipal>('principals', body)
}

export function updatePrincipalRole(
  client: ApiClient,
  subject: string,
  role: string,
): Promise<Principal> {
  return client.patch<Principal>(`principals/${encodeURIComponent(subject)}`, { role })
}

export function deletePrincipal(client: ApiClient, subject: string): Promise<void> {
  return client.del(`principals/${encodeURIComponent(subject)}`)
}

export function listGrants(client: ApiClient, subject: string): Promise<Grant[]> {
  return client.get<Grant[]>(`principals/${encodeURIComponent(subject)}/grants`)
}

// PUT (grant) is idempotent; DELETE (revoke) on an absent grant is a no-op 204.
export function grantCapability(
  client: ApiClient,
  subject: string,
  capability: string,
): Promise<Grant> {
  return client.put<Grant>(
    `principals/${encodeURIComponent(subject)}/grants/${encodeURIComponent(capability)}`,
    {},
  )
}

export function revokeCapability(
  client: ApiClient,
  subject: string,
  capability: string,
): Promise<void> {
  return client.del(
    `principals/${encodeURIComponent(subject)}/grants/${encodeURIComponent(capability)}`,
  )
}

// — Teams & memberships —————————————————————————————————————————————————————
// Backs the teams admin screen onto /teams in crates/rubix-server/src/http/admin/
// teams.rs. A team groups principals; a grant on a team (below) flows to every
// member.

export function listTeams(client: ApiClient): Promise<Team[]> {
  return client.get<Team[]>('teams')
}

export function createTeam(client: ApiClient, body: CreateTeamRequest): Promise<Team> {
  return client.post<Team>('teams', body)
}

export function deleteTeam(client: ApiClient, slug: string): Promise<void> {
  return client.del(`teams/${encodeURIComponent(slug)}`)
}

export function listTeamMembers(client: ApiClient, slug: string): Promise<TeamMember[]> {
  return client.get<TeamMember[]>(`teams/${encodeURIComponent(slug)}/members`)
}

export function addTeamMember(
  client: ApiClient,
  slug: string,
  subject: string,
): Promise<TeamMember> {
  return client.post<TeamMember>(`teams/${encodeURIComponent(slug)}/members`, { subject })
}

export function removeTeamMember(
  client: ApiClient,
  slug: string,
  subject: string,
): Promise<void> {
  return client.del(
    `teams/${encodeURIComponent(slug)}/members/${encodeURIComponent(subject)}`,
  )
}

export function listTeamGrants(client: ApiClient, slug: string): Promise<Grant[]> {
  return client.get<Grant[]>(`teams/${encodeURIComponent(slug)}/grants`)
}

// PUT (grant) is idempotent; DELETE (revoke) on an absent grant is a no-op 204.
export function grantTeamCapability(
  client: ApiClient,
  slug: string,
  capability: string,
): Promise<Grant> {
  return client.put<Grant>(
    `teams/${encodeURIComponent(slug)}/grants/${encodeURIComponent(capability)}`,
    {},
  )
}

export function revokeTeamCapability(
  client: ApiClient,
  slug: string,
  capability: string,
): Promise<void> {
  return client.del(
    `teams/${encodeURIComponent(slug)}/grants/${encodeURIComponent(capability)}`,
  )
}
