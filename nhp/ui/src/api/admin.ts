/**
 * Typed access to the rubix admin surface — principals (users + service accounts)
 * and their capability grants. This is the SAME identity model the docs call
 * "user management" and "service-account management" (ADMIN.md §6); the only
 * distinction is `kind` (`user` vs `extension`).
 *
 * Routes (bare, NOT under `/api/v1`; rubix/crates/rubix-server/src/http/admin/mod.rs):
 *   POST   /principals                 { subject, kind, role, secret? } -> CreatedPrincipal
 *   GET    /principals                                                  -> Principal[]
 *   PATCH  /principals/:subject        { role }                         -> Principal
 *   DELETE /principals/:subject
 *   GET    /principals/:subject/grants                                  -> Grant[]
 *   PUT    /principals/:subject/grants/:capability                      -> Grant
 *   DELETE /principals/:subject/grants/:capability
 *
 * AUTH — admin-only. Every handler calls `require_admin` (http/admin/guard.rs), so
 * the default operator records-credential is rejected (403). These calls send the
 * seeded ADMIN credential (`VITE_RUBIX_ADMIN_SUBJECT`/`_SECRET`, default
 * `acme_admin` / `admin-demo`) via the per-request `auth` override on `request`.
 * See WS-05.md "principal-CRUD reachability decision".
 *
 * SUBJECT is namespace-LOCAL on the wire: send `alice`, the server stores
 * `acme_alice` and strips the `{namespace}_` prefix off every response — so the UI
 * always works in unprefixed subjects.
 */
import { request, type ServiceAuth } from './client'

export type PrincipalKind = 'user' | 'extension'
/** NHP role set on the rubix principal surface (ADMIN.md §6). */
export type PrincipalRole = 'viewer' | 'operator' | 'admin'

export interface Principal {
  subject: string
  namespace: string
  kind: PrincipalKind
  role: PrincipalRole
}

/** Create echoes a minted secret exactly once (only when the server generated it). */
export interface CreatedPrincipal extends Principal {
  secret?: string
}

export interface Grant {
  subject: string
  namespace: string
  capability: string
}

export interface CreatePrincipalBody {
  subject: string
  kind: PrincipalKind
  role: PrincipalRole
  /** Omit to have the server mint and return one once. */
  secret?: string
}

/**
 * The admin credential for the rubix admin surface. Defaults to the seeded
 * `acme_admin` / `admin-demo` (rubix/crates/rubix-server/src/seed/cast.rs) so the
 * POC works against a `--seed-dev` backend without extra config; override via
 * `VITE_RUBIX_ADMIN_SUBJECT` / `VITE_RUBIX_ADMIN_SECRET`.
 */
export const adminAuth: ServiceAuth = {
  subject:
    (import.meta.env.VITE_RUBIX_ADMIN_SUBJECT as string | undefined) ??
    'acme_admin',
  secret:
    (import.meta.env.VITE_RUBIX_ADMIN_SECRET as string | undefined) ??
    'admin-demo',
}

export async function listPrincipals(): Promise<Principal[]> {
  return request<Principal[]>('/principals', { auth: adminAuth })
}

export async function createPrincipal(
  body: CreatePrincipalBody
): Promise<CreatedPrincipal> {
  return request<CreatedPrincipal>('/principals', {
    method: 'POST',
    body,
    auth: adminAuth,
  })
}

export async function setPrincipalRole(
  subject: string,
  role: PrincipalRole
): Promise<Principal> {
  return request<Principal>(`/principals/${subject}`, {
    method: 'PATCH',
    body: { role },
    auth: adminAuth,
  })
}

export async function deletePrincipal(subject: string): Promise<void> {
  await request<void>(`/principals/${subject}`, {
    method: 'DELETE',
    auth: adminAuth,
  })
}

export async function listGrants(subject: string): Promise<Grant[]> {
  return request<Grant[]>(`/principals/${subject}/grants`, { auth: adminAuth })
}
