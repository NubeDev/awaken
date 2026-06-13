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

interface RequestOptions {
  method?: 'GET' | 'POST' | 'PUT' | 'PATCH' | 'DELETE'
  body?: unknown
  query?: Record<string, string | number | undefined>
  signal?: AbortSignal
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

function buildHeaders(hasBody: boolean): HeadersInit | undefined {
  const headers: Record<string, string> = {}
  if (hasBody) headers['content-type'] = 'application/json'
  const token = currentAccessToken()
  if (token) headers['authorization'] = `Bearer ${token}`
  return Object.keys(headers).length > 0 ? headers : undefined
}

export async function request<T>(
  path: string,
  options: RequestOptions = {}
): Promise<T> {
  const { method = 'GET', body, query, signal } = options
  const res = await fetch(buildUrl(path, query), {
    method,
    signal,
    headers: buildHeaders(body !== undefined),
    body: body !== undefined ? JSON.stringify(body) : undefined,
  })
  if (!res.ok) throw new ApiError(res.status, await readError(res))
  if (res.status === 204) return undefined as T
  return (await res.json()) as T
}
