// The connection a client holds to a rubix backend: where it is and who we are.
//
// Browser mode is same-origin (apiBase ''), desktop/Tauri points at a configured
// endpoint — see ADMIN-UI "Delivery targets". Auth is the header credential pair
// the gate verifies natively (x-rubix-subject / x-rubix-secret in
// crates/rubix-server/src/auth.rs); no JWT/session layer exists yet, so we hold
// the credential and send it per request.

const STORAGE_KEY = 'rubix.connection'

export interface Connection {
  /** Origin of the backend, e.g. '' (same-origin) or 'http://edge.local:8088'. */
  endpoint: string
  /** Credential subject — the seed names these `<tenant>_<role>`, e.g. acme_operator. */
  subject: string
  /** Credential secret. */
  secret: string
  /** Default tenant (namespace) to enter; the portfolio can switch it. */
  tenant: string
}

export function loadConnection(): Connection | null {
  try {
    const raw = localStorage.getItem(STORAGE_KEY)
    if (!raw) return null
    const c = JSON.parse(raw) as Connection
    if (!c.subject || !c.secret) return null
    return { endpoint: c.endpoint ?? '', tenant: c.tenant || 'acme', subject: c.subject, secret: c.secret }
  } catch {
    return null
  }
}

export function saveConnection(c: Connection): void {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(c))
}

export function clearConnection(): void {
  localStorage.removeItem(STORAGE_KEY)
}
