/**
 * Server-Sent Events client for the board live-value stream. The browser's
 * native `EventSource` can't send an `Authorization` header, so we read the SSE
 * response over `fetch` (which carries the bearer token, same as `request`) and
 * parse the frames ourselves. Each frame's `data:` payload is a JSON array of
 * `PortOutput` — the board's full latest snapshot.
 */
import { currentAccessToken } from '@/stores/auth-store'

import { API_BASE } from './client'
import type { PortOutput } from './types'

/** A board scope, mirroring the REST endpoints' `?org=&site_id=` query. */
export type StreamScope = { org: string; siteId?: string }

/**
 * Split an accumulating SSE buffer into complete event blocks plus the trailing
 * partial block. Events are separated by a blank line (`\n\n`). Pure, so the
 * framing is unit-testable without a network.
 */
export function splitSseEvents(buffer: string): { events: string[]; rest: string } {
  const parts = buffer.split('\n\n')
  const rest = parts.pop() ?? ''
  return { events: parts, rest }
}

/**
 * Extract the joined `data:` payload from one SSE event block, or `null` when
 * the block carries no data line (e.g. a keep-alive comment). Pure.
 */
export function sseEventData(block: string): string | null {
  const data = block
    .split('\n')
    .filter((line) => line.startsWith('data:'))
    .map((line) => line.slice('data:'.length).replace(/^ /, ''))
    .join('\n')
  return data.length > 0 ? data : null
}

/**
 * Open the board outputs stream and invoke `onSnapshot` for every snapshot
 * frame until `signal` aborts or the connection closes. Rejects on a non-OK
 * response or a network error so the caller can back off and reconnect.
 */
export async function streamBoardOutputs(
  slug: string,
  scope: StreamScope,
  onSnapshot: (snapshot: PortOutput[]) => void,
  signal: AbortSignal
): Promise<void> {
  const params = new URLSearchParams({ org: scope.org })
  if (scope.siteId) params.set('site_id', scope.siteId)
  const url = `${API_BASE}/api/v1/boards/${slug}/outputs/stream?${params.toString()}`

  const token = currentAccessToken()
  const res = await fetch(url, {
    headers: token ? { authorization: `Bearer ${token}` } : {},
    signal,
  })
  if (!res.ok || !res.body) {
    throw new Error(`board outputs stream failed: ${res.status}`)
  }

  const reader = res.body.getReader()
  const decoder = new TextDecoder()
  let buffer = ''
  for (;;) {
    const { value, done } = await reader.read()
    if (done) break
    buffer += decoder.decode(value, { stream: true })
    const { events, rest } = splitSseEvents(buffer)
    buffer = rest
    for (const event of events) {
      const data = sseEventData(event)
      if (data === null) continue
      try {
        onSnapshot(JSON.parse(data) as PortOutput[])
      } catch {
        // Skip a malformed frame rather than tearing down the stream.
      }
    }
  }
}
