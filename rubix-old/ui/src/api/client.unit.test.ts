import { afterEach, describe, expect, it, vi } from 'vitest'
import { ApiError, request } from './client'
import { useAuthStore } from '@/stores/auth-store'

function mockFetch(status: number, body: unknown) {
  return vi.fn(
    async (_url: string, _init?: RequestInit): Promise<Response> =>
      new Response(body === undefined ? null : JSON.stringify(body), {
        status,
        headers: { 'content-type': 'application/json' },
      })
  )
}

afterEach(() => {
  vi.restoreAllMocks()
  useAuthStore.getState().auth.resetAccessToken()
})

describe('request', () => {
  it('decodes a JSON body on success', async () => {
    vi.stubGlobal('fetch', mockFetch(200, [{ id: 'a' }]))
    await expect(request('/api/v1/sites')).resolves.toEqual([{ id: 'a' }])
  })

  it('builds a query string from defined params only', async () => {
    const fetchSpy = mockFetch(200, [])
    vi.stubGlobal('fetch', fetchSpy)
    await request('/api/v1/points', { query: { site_id: 's1', equip_id: undefined } })
    const url = fetchSpy.mock.calls[0]![0]
    expect(url).toContain('site_id=s1')
    expect(url).not.toContain('equip_id')
  })

  it('raises ApiError with the server error message on non-2xx', async () => {
    vi.stubGlobal('fetch', mockFetch(403, { error: 'priority too low' }))
    await expect(request('/api/v1/points/x/write', { method: 'POST', body: {} })).rejects.toThrow(
      ApiError
    )
    await expect(
      request('/api/v1/points/x/write', { method: 'POST', body: {} })
    ).rejects.toThrow('priority too low')
  })

  it('serialises a body and sets the content-type header', async () => {
    const fetchSpy = mockFetch(200, { ok: true })
    vi.stubGlobal('fetch', fetchSpy)
    await request('/api/v1/agent/chat', { method: 'POST', body: { message: 'hi' } })
    const init = fetchSpy.mock.calls[0]![1]!
    expect(init.body).toBe('{"message":"hi"}')
    expect((init.headers as Record<string, string>)['content-type']).toBe('application/json')
  })

  it('attaches a bearer Authorization header when a token is stored', async () => {
    const fetchSpy = mockFetch(200, [])
    vi.stubGlobal('fetch', fetchSpy)
    useAuthStore.getState().auth.setAccessToken('rbx_token')
    await request('/api/v1/sites')
    const init = fetchSpy.mock.calls[0]![1]!
    expect((init.headers as Record<string, string>)['authorization']).toBe('Bearer rbx_token')
  })

  it('sends no Authorization header when no token is stored', async () => {
    const fetchSpy = mockFetch(200, [])
    vi.stubGlobal('fetch', fetchSpy)
    await request('/api/v1/sites')
    const init = fetchSpy.mock.calls[0]![1]!
    expect(init.headers).toBeUndefined()
  })

  it('raises ApiError(401) so callers can clear the token', async () => {
    vi.stubGlobal('fetch', mockFetch(401, { error: 'invalid token' }))
    await expect(request('/api/v1/sites')).rejects.toMatchObject({ status: 401 })
  })
})
