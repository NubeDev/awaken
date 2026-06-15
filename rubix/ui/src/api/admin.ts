// Admin & management calls — principals and grants. HTTP only, no state, no
// React (mirrors api/records.ts). Backs the principals/grants admin screen onto
// the control-plane surface in crates/rubix-server/src/http/admin/*.

import type { ApiClient } from './client'
import type {
  CreatePrincipalRequest,
  CreatedPrincipal,
  Grant,
  Principal,
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
