// Wire shapes for the admin & management surface, mirroring the Rust DTOs in
// crates/rubix-server/src/dto/admin.rs and dto/auth.rs. These are substrate
// types — principals, grants, capabilities — never a domain entity. The admin
// console is built entirely on these plus the generic Record.

/** A principal as returned by the API — identity only, never a secret. */
export interface Principal {
  subject: string
  namespace: string
  /** 'user' | 'extension' */
  kind: string
  /** 'viewer' | 'operator' | 'admin' */
  role: string
}

/** The create-principal request body. Omit `secret` to have the server mint one. */
export interface CreatePrincipalRequest {
  subject: string
  kind: string
  role: string
  secret?: string
}

/** The create-principal response — the only response that ever carries a secret. */
export interface CreatedPrincipal extends Principal {
  /** Present only when the server minted the secret. */
  secret?: string
}

/** A capability grant attached to a principal. */
export interface Grant {
  subject: string
  namespace: string
  capability: string
}

/** The current principal plus the capabilities it holds (GET /auth/me). */
export interface Me {
  subject: string
  namespace: string
  kind: string
  role: string
  capabilities: string[]
}

/** The login response (POST /auth/login). */
export interface LoginResponse {
  token: string
  expires: string
}

export type Role = 'viewer' | 'operator' | 'admin'
export const ROLES: Role[] = ['viewer', 'operator', 'admin']

export type PrincipalKind = 'user' | 'extension'
export const PRINCIPAL_KINDS: PrincipalKind[] = ['user', 'extension']

/** The capability strings the gate recognizes — crates/rubix-gate/src/capability/
 *  kind.rs. The grants screen toggles a principal's membership in this set. */
export const CAPABILITIES = [
  'datasource-register',
  'rule-invoke',
  'ingest-publish',
  'external-query',
  'zenoh-subscribe',
  'agent-memory-write',
  'device-actuate',
  'rule-define',
  'device-manage',
] as const

export type Capability = (typeof CAPABILITIES)[number]
