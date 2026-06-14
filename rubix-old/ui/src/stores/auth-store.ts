import { create } from 'zustand'
import { getCookie, setCookie, removeCookie } from '@/lib/cookies'

/**
 * API-token auth store. The rubix-server accepts a bearer token on `/api/v1/*`;
 * until a deployment issuer (OIDC) is configured the operator pastes a raw token
 * on the sign-in screen. We persist only the opaque token — no decoded identity,
 * because no `whoami` endpoint is exposed on the wire (see ../TODOs.md). The
 * displayed principal stays a neutral "Operator".
 */
const ACCESS_TOKEN = 'rubix.api.token'

interface AuthState {
  auth: {
    accessToken: string
    setAccessToken: (accessToken: string) => void
    resetAccessToken: () => void
  }
}

export const useAuthStore = create<AuthState>()((set) => {
  const cookieState = getCookie(ACCESS_TOKEN)
  const initToken = cookieState ? JSON.parse(cookieState) : ''
  return {
    auth: {
      accessToken: initToken,
      setAccessToken: (accessToken) =>
        set((state) => {
          setCookie(ACCESS_TOKEN, JSON.stringify(accessToken))
          return { ...state, auth: { ...state.auth, accessToken } }
        }),
      resetAccessToken: () =>
        set((state) => {
          removeCookie(ACCESS_TOKEN)
          return { ...state, auth: { ...state.auth, accessToken: '' } }
        }),
    },
  }
})

/** Read the persisted token outside React (the fetch client uses this). */
export function currentAccessToken(): string {
  return useAuthStore.getState().auth.accessToken
}
