/**
 * The rubix auth surface — sign in with a subject + secret, reflect the current
 * principal, and revoke a session on sign-out. This replaces the
 * paste-a-raw-API-token flow: the operator types a username (subject) and
 * password (secret), the server verifies them once at `POST /auth/login` and
 * mints an opaque, revocable bearer token the UI carries thereafter.
 *
 * Routes (bare, NOT under `/api/v1`; rubix/crates/rubix-server/src/http/auth/mod.rs):
 *   POST /auth/login    { subject, secret } -> { token, expires }
 *   GET  /auth/me                           -> { subject, namespace, kind, role, capabilities }
 *   POST /auth/logout   (Bearer)            -> 204
 *
 * SUBJECT is the full stored subject (`acme_admin`), not the bare role — the gate
 * matches on the whole principal key (rubix/crates/rubix-gate authenticate). The
 * seeded `--seed-dev` cast lives in namespace `acme`; see DEMO_PRINCIPALS.
 */
import { request } from './client'

/** The body of a sign-in request. */
export interface LoginBody {
  subject: string
  secret: string
}

/** A freshly minted login token and its expiry (RFC 3339, UTC). */
export interface LoginResponse {
  token: string
  expires: string
}

/** The authenticated principal and what it may do. */
export interface MeResponse {
  subject: string
  namespace: string
  kind: string
  role: string
  capabilities: string[]
}

/** Exchange a subject + secret for an opaque bearer token. */
export async function login(body: LoginBody): Promise<LoginResponse> {
  return request<LoginResponse>('/auth/login', { method: 'POST', body })
}

/** Reflect the principal a bearer token authenticates as. */
export async function fetchMe(token: string): Promise<MeResponse> {
  return request<MeResponse>('/auth/me', {
    headers: { authorization: `Bearer ${token}` },
  })
}

/**
 * Revoke the current session token. Idempotent server-side; we swallow errors so
 * a sign-out always clears the client even if the network call fails.
 */
export async function logout(token: string): Promise<void> {
  try {
    await request<void>('/auth/logout', {
      method: 'POST',
      headers: { authorization: `Bearer ${token}` },
    })
  } catch {
    // A best-effort revoke; the client token is cleared regardless.
  }
}

/** A seeded demo principal: a one-click sign-in for the `--seed-dev` cast. */
export interface DemoPrincipal {
  /** The display label on the quick-login button. */
  label: string
  /** A one-line description of the role's authority. */
  blurb: string
  /** The full stored subject the gate authenticates (`{namespace}_{role}`). */
  subject: string
  /** The demo secret seeded for this principal (rubix seed/cast.rs). */
  secret: string
}

/**
 * The `--seed-dev` principal cast for namespace `acme`
 * (rubix/crates/rubix-server/src/seed/cast.rs). These are DEMO credentials baked
 * into a `--seed-dev` backend so an operator never has to paste a raw token; a
 * real deployment provisions its own principals and these buttons go away.
 */
export const DEMO_PRINCIPALS: DemoPrincipal[] = [
  {
    label: 'Admin',
    blurb: 'Full access — user & service-account management.',
    subject: 'acme_admin',
    secret: 'admin-demo',
  },
  {
    label: 'Operator',
    blurb: 'Read & write records, run queries.',
    subject: 'acme_operator',
    secret: 'operator-demo',
  },
  {
    label: 'Viewer',
    blurb: 'Read-only access to the portfolio.',
    subject: 'acme_viewer',
    secret: 'viewer-demo',
  },
]
