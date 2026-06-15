// Auth surface — login/logout/me (crates/rubix-server/src/http/auth/*). The UI
// reflects what a principal may do from `me()`; login/logout exist for the token
// path (today the connection still holds the header credential, so `me()` is the
// load-bearing call — it returns the current identity and its capabilities).

import type { ApiClient } from './client'
import type { LoginResponse, Me } from '../types/Admin'

export function me(client: ApiClient): Promise<Me> {
  return client.get<Me>('auth/me')
}

export function login(
  client: ApiClient,
  subject: string,
  secret: string,
): Promise<LoginResponse> {
  return client.post<LoginResponse>('auth/login', { subject, secret })
}

export function logout(client: ApiClient): Promise<void> {
  return client.post<void>('auth/logout', {})
}
