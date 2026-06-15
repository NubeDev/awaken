import { clearCookies } from '@/test-utils/cookies'
import { beforeEach, describe, expect, it, vi } from 'vitest'

async function importAuthStore() {
  const mod = await import('./auth-store')
  return mod
}

describe('useAuthStore', () => {
  beforeEach(() => {
    clearCookies()
    vi.resetModules()
  })

  it('starts with an empty access token when nothing is persisted', async () => {
    const { useAuthStore } = await importAuthStore()
    expect(useAuthStore.getState().auth.accessToken).toBe('')
  })

  it('persists the token so a fresh store instance reads it back', async () => {
    const { useAuthStore } = await importAuthStore()
    useAuthStore.getState().auth.setAccessToken('rbx_session')

    vi.resetModules()
    const { useAuthStore: reloaded } = await importAuthStore()
    expect(reloaded.getState().auth.accessToken).toBe('rbx_session')
  })

  it('resetAccessToken clears the persisted token', async () => {
    const { useAuthStore } = await importAuthStore()
    useAuthStore.getState().auth.setAccessToken('rbx_drop')
    useAuthStore.getState().auth.resetAccessToken()

    vi.resetModules()
    const { useAuthStore: reloaded } = await importAuthStore()
    expect(reloaded.getState().auth.accessToken).toBe('')
  })

  it('currentAccessToken reads the live token outside React', async () => {
    const { useAuthStore, currentAccessToken } = await importAuthStore()
    expect(currentAccessToken()).toBe('')
    useAuthStore.getState().auth.setAccessToken('rbx_live')
    expect(currentAccessToken()).toBe('rbx_live')
  })
})
