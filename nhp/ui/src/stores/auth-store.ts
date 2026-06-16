import { create } from 'zustand'
import { getCookie, setCookie, removeCookie } from '@/lib/cookies'

/**
 * Login-token auth store. The operator signs in with a subject + secret at
 * `POST /auth/login` (see api/auth.ts); the server mints an opaque bearer token
 * the UI carries thereafter. We persist the token AND the reflected identity
 * (subject/role/namespace from `GET /auth/me`) so the chrome can show who is
 * signed in and gate admin-only UI — no raw token is ever pasted by hand.
 */
const ACCESS_TOKEN = 'rubix.api.token'
const IDENTITY = 'rubix.api.identity'

/** The reflected principal a token authenticates as (a subset of /auth/me). */
export interface Identity {
  subject: string
  namespace: string
  role: string
}

interface AuthState {
  auth: {
    accessToken: string
    identity: Identity | null
    /** Persist the token and (optionally) the identity it resolved to. */
    setSession: (accessToken: string, identity: Identity | null) => void
    setAccessToken: (accessToken: string) => void
    resetAccessToken: () => void
  }
}

function readIdentity(): Identity | null {
  const raw = getCookie(IDENTITY)
  if (!raw) return null
  try {
    return JSON.parse(raw) as Identity
  } catch {
    return null
  }
}

export const useAuthStore = create<AuthState>()((set) => {
  const cookieState = getCookie(ACCESS_TOKEN)
  const initToken = cookieState ? JSON.parse(cookieState) : ''
  return {
    auth: {
      accessToken: initToken,
      identity: readIdentity(),
      setSession: (accessToken, identity) =>
        set((state) => {
          setCookie(ACCESS_TOKEN, JSON.stringify(accessToken))
          if (identity) setCookie(IDENTITY, JSON.stringify(identity))
          else removeCookie(IDENTITY)
          return { ...state, auth: { ...state.auth, accessToken, identity } }
        }),
      setAccessToken: (accessToken) =>
        set((state) => {
          setCookie(ACCESS_TOKEN, JSON.stringify(accessToken))
          return { ...state, auth: { ...state.auth, accessToken } }
        }),
      resetAccessToken: () =>
        set((state) => {
          removeCookie(ACCESS_TOKEN)
          removeCookie(IDENTITY)
          return {
            ...state,
            auth: { ...state.auth, accessToken: '', identity: null },
          }
        }),
    },
  }
})

/** Read the persisted token outside React (the fetch client uses this). */
export function currentAccessToken(): string {
  return useAuthStore.getState().auth.accessToken
}
