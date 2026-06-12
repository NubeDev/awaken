/**
 * Captured-wire decode tests. Each fixture under `__fixtures__/` is a real
 * response body captured from a running `rubix-server` (or, for spark/run where
 * no create route exists, built from the server's OpenAPI schema). The tests
 * push each through the live-mode endpoint functions and assert the decode
 * matches the corrected `types.ts` shapes — the contract UI-01 verified against
 * `/api-docs/openapi.json`. A drift here is a wire-shape regression.
 */
import { afterEach, describe, expect, it, vi } from 'vitest'
import siteFixture from './__fixtures__/site.json'
import writeFixture from './__fixtures__/write.json'
import queryFixture from './__fixtures__/query.json'
import runFixture from './__fixtures__/run.json'
import { tagNames } from './tags'
import type { PointEnvelope, QueryResult, RunRecord, Site } from './types'

function mockJson(body: unknown) {
  return vi.fn(
    async (): Promise<Response> =>
      new Response(JSON.stringify(body), {
        status: 200,
        headers: { 'content-type': 'application/json' },
      })
  )
}

afterEach(() => {
  vi.restoreAllMocks()
})

describe('wire contract: tags are a map, not an array', () => {
  it('decodes a site with a TagSet object and projects names', async () => {
    const body = siteFixture as Site
    vi.stubGlobal('fetch', mockJson([body]))
    const { sites } = await import('./endpoints')
    const [site] = await sites.list()
    expect(Array.isArray(site!.tags)).toBe(false)
    expect(typeof site!.tags).toBe('object')
    expect(tagNames(site!.tags)).toContain('campus')
  })
})

describe('wire contract: PriorityArray and PointResponse', () => {
  it('decodes a point write envelope with 16 slots and a keyexpr', async () => {
    const body = writeFixture as PointEnvelope
    vi.stubGlobal('fetch', mockJson(body))
    const { points } = await import('./endpoints')
    const env = await points.write('id', { value: 21.5 })
    expect(env.keyexpr).toBe('acme/hq/ahu-3/sat')
    expect(env.point.priority_array.slots).toHaveLength(16)
    expect(env.point.priority_array.relinquish_default).toBeNull()
    // operator write at priority 8 (slot index 7) lands its value.
    expect(env.point.priority_array.slots[7]).toBe(21.5)
  })
})

describe('wire contract: QueryResult is rows-of-objects (no columns)', () => {
  it('decodes query rows as column-keyed objects', async () => {
    const body = queryFixture as QueryResult
    expect('columns' in body).toBe(false)
    vi.stubGlobal('fetch', mockJson(body))
    const { query } = await import('./endpoints')
    const res = await query.run('SELECT 1')
    expect(Array.isArray(res.rows)).toBe(true)
    expect(res.rows[0]).toMatchObject({ slug: 'sat', kind: 'sp' })
  })
})

describe('wire contract: RunRecord shape', () => {
  it('decodes a suspended run with its pending write', async () => {
    const body = runFixture as RunRecord
    vi.stubGlobal('fetch', mockJson([body]))
    const { runs } = await import('./endpoints')
    const [run] = await runs.list()
    expect(run!.status).toBe('suspended')
    expect(run!.origin).toBe('chat')
    expect(run!.thread_id).toBe('thread_ops_1')
    expect(run!.pending_write?.point).toBe('acme/hq/ahu-3/sat')
    expect(run!.pending_write?.agent_min_priority).toBe(13)
  })
})
