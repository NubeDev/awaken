/**
 * Thin fetch wrapper for the rubix-server API. One job: issue a request to an
 * `/api/v1/*` path and decode JSON, raising `ApiError` on a non-2xx status with
 * the server's `ErrorBody` message when present. Endpoint functions live in
 * `endpoints.ts`; React Query hooks in `hooks/`.
 */
import { currentAccessToken } from '@/stores/auth-store'

export const API_BASE = import.meta.env.VITE_API_BASE ?? ''

export class ApiError extends Error {
  constructor(
    readonly status: number,
    message: string
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

/** A service-account credential pair for the `x-rubix-subject`/`-secret` headers. */
export interface ServiceAuth {
  subject: string
  secret: string
}

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'
  body?: unknown
  query?: Record<string, string | number | undefined>
  signal?: AbortSignal
  /** Extra request headers (e.g. `Accept-Units` for unit negotiation). */
  headers?: Record<string, string>
  /**
   * Override the default service-account auth for THIS request. The rubix admin
   * surface (`/principals`) requires `Role::Admin`, which the default operator
   * credential is not, so user CRUD passes the seeded admin here (see api/admin.ts).
   */
  auth?: ServiceAuth
}

function buildUrl(path: string, query?: RequestOptions['query']): string {
  const url = `${API_BASE}${path}`
  if (!query) return url
  const params = new URLSearchParams()
  for (const [key, value] of Object.entries(query)) {
    if (value !== undefined && value !== '') params.set(key, String(value))
  }
  const qs = params.toString()
  return qs ? `${url}?${qs}` : url
}

async function readError(res: Response): Promise<string> {
  try {
    const data = (await res.json()) as { error?: string; message?: string }
    return data.error ?? data.message ?? res.statusText
  } catch {
    return res.statusText || `HTTP ${res.status}`
  }
}

/**
 * Service-account credentials for the rubix records API. rubix accepts EITHER a
 * `Bearer` login token OR the `x-rubix-subject` / `x-rubix-secret` header pair
 * (rubix/crates/rubix-server/src/auth.rs). The NHP collections/seed use the header
 * pair as the seeded `acme_operator` (WS-02/03); the admin UI talks to the same
 * `/records` surface, so it sends the same pair when configured. Set via Vite env
 * (`VITE_RUBIX_SUBJECT` / `VITE_RUBIX_SECRET`) for a `--seed-dev` backend; when
 * unset the Bearer token (sign-in screen) is used instead.
 */
const SERVICE_SUBJECT = import.meta.env.VITE_RUBIX_SUBJECT as string | undefined
const SERVICE_SECRET = import.meta.env.VITE_RUBIX_SECRET as string | undefined

function buildHeaders(
  hasBody: boolean,
  extra?: Record<string, string>,
  auth?: ServiceAuth
): HeadersInit | undefined {
  const headers: Record<string, string> = { ...extra }
  if (hasBody) headers['content-type'] = 'application/json'
  // An explicit per-request credential (admin surface) wins over the defaults.
  if (auth) {
    headers['x-rubix-subject'] = auth.subject
    headers['x-rubix-secret'] = auth.secret
    return headers
  }
  const token = currentAccessToken()
  if (token) headers['authorization'] = `Bearer ${token}`
  else if (SERVICE_SUBJECT && SERVICE_SECRET) {
    headers['x-rubix-subject'] = SERVICE_SUBJECT
    headers['x-rubix-secret'] = SERVICE_SECRET
  }
  return Object.keys(headers).length > 0 ? headers : undefined
}

export async function request<T>(
  path: string,
  options: RequestOptions = {}
): Promise<T> {
  const { method = 'GET', body, query, signal, headers, auth } = options
  const res = await fetch(buildUrl(path, query), {
    method,
    signal,
    headers: buildHeaders(body !== undefined, headers, auth),
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) throw new ApiError(res.status, await readError(res))
  if (res.status === 204) return undefined as T
  return (await res.json()) as T
}
