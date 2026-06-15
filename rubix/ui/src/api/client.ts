// The tenant-scoped HTTP client. Every read/write is bound to one tenant by
// construction (PRODUCT-UI "Multi-tenancy"): the UI only ever asks for the
// current tenant and relies on the gate to enforce isolation.
//
// The contract target is `/api/v1/t/<tenant>/...`. The backend today serves
// flat, single-namespace routes (`/records`, `/query`) — so this is the ONE
// hand-maintained adapter seam (PRODUCT-UI "Backend gap"): `path()` knows how to
// map the tenant-scoped shape onto whatever the backend currently exposes. When
// the backend versions + nests its routes, flip TENANT_ROUTES_LIVE and delete
// nothing else.

import type { Connection } from './connection'

// The backend has not yet landed `/api/v1/t/:tenant`. Until it does, the tenant
// is carried implicitly by the credential's scoped session and we hit flat
// routes. Kept as a single switch so the UI never encodes the old shape twice.
const TENANT_ROUTES_LIVE = false

export class ApiError extends Error {
  constructor(
    public status: number,
    message: string,
  ) {
    super(message)
    this.name = 'ApiError'
  }
}

export class ApiClient {
  constructor(
    private conn: Connection,
    private tenant: string,
  ) {}

  private path(resource: string): string {
    const base = this.conn.endpoint.replace(/\/$/, '')
    if (TENANT_ROUTES_LIVE) {
      return `${base}/api/v1/t/${encodeURIComponent(this.tenant)}/${resource}`
    }
    return `${base}/${resource}`
  }

  private headers(): HeadersInit {
    return {
      'content-type': 'application/json',
      'x-rubix-subject': this.conn.subject,
      'x-rubix-secret': this.conn.secret,
    }
  }

  async get<T>(resource: string): Promise<T> {
    return this.request<T>('GET', resource)
  }

  async post<T>(resource: string, body: unknown): Promise<T> {
    return this.request<T>('POST', resource, body)
  }

  async patch<T>(resource: string, body: unknown): Promise<T> {
    return this.request<T>('PATCH', resource, body)
  }

  async del(resource: string): Promise<void> {
    await this.request<unknown>('DELETE', resource)
  }

  private async request<T>(method: string, resource: string, body?: unknown): Promise<T> {
    const res = await fetch(this.path(resource), {
      method,
      headers: this.headers(),
      body: body === undefined ? undefined : JSON.stringify(body),
    })
    if (!res.ok) {
      const text = await res.text().catch(() => res.statusText)
      throw new ApiError(res.status, text || res.statusText)
    }
    if (res.status === 204) return undefined as T
    const text = await res.text()
    return (text ? JSON.parse(text) : undefined) as T
  }
}
