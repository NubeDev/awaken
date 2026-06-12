/**
 * Demo-mode endpoint shims. Mirror the real endpoint functions but resolve from
 * the in-memory fixture set, so the whole UI is populated with the sample
 * building without a backend. Mutations (ack, write, relinquish) update the
 * fixtures in place so interactions feel live within the session.
 */
import type {
  ChatResponse,
  CurRequest,
  Equip,
  HisSample,
  Point,
  PointEnvelope,
  QueryResult,
  RunSummary,
  Site,
  Spark,
  Uuid,
  WriteRequest,
} from '../types'
import { EQUIPS, POINTS, SITES, SPARKS, historyFor } from './fixtures'

/** Demo mode is on when explicitly enabled. Defaults on until a backend ships. */
export function isDemo(): boolean {
  return (import.meta.env.VITE_DEMO ?? '1') !== '0'
}

const delay = <T>(value: T): Promise<T> => Promise.resolve(value)
const clone = <T>(v: T): T => JSON.parse(JSON.stringify(v))

const points = clone(POINTS) as Point[]
const sparks = clone(SPARKS) as Spark[]

function envelope(p: Point): PointEnvelope {
  const equip = EQUIPS.find((e) => e.id === p.equip_id)
  const site = SITES.find((s) => s.id === equip?.site_id)
  return { keyexpr: `${site?.org}/${site?.slug}/${equip?.path}/${p.slug}`, point: clone(p) }
}

export const demo = {
  sites: { list: (): Promise<Site[]> => delay(clone(SITES)) },
  equips: {
    list: (siteId?: Uuid): Promise<Equip[]> =>
      delay(clone(siteId ? EQUIPS.filter((e) => e.site_id === siteId) : EQUIPS)),
  },
  points: {
    list: (params: { equipId?: Uuid; siteId?: Uuid }): Promise<Point[]> => {
      let out = points
      if (params.equipId) out = out.filter((p) => p.equip_id === params.equipId)
      if (params.siteId) {
        const equipIds = new Set(EQUIPS.filter((e) => e.site_id === params.siteId).map((e) => e.id))
        out = out.filter((p) => equipIds.has(p.equip_id))
      }
      return delay(clone(out))
    },
    history: (id: Uuid): Promise<HisSample[]> => delay(historyFor(id)),
    write: (id: Uuid, body: WriteRequest): Promise<PointEnvelope> => {
      const p = points.find((x) => x.id === id)
      if (p) {
        const lvl = (body.priority ?? 16) - 1
        p.priority_array.slots[lvl] = body.value
        const win = p.priority_array.slots.find((s) => s !== null)
        if (win !== undefined) p.cur_value = win
      }
      return delay(envelope(p!))
    },
    relinquish: (id: Uuid, priority: number): Promise<PointEnvelope> => {
      const p = points.find((x) => x.id === id)
      if (p) {
        p.priority_array.slots[priority - 1] = null
        const win = p.priority_array.slots.find((s) => s !== null)
        p.cur_value = win ?? p.priority_array.relinquish_default
      }
      return delay(envelope(p!))
    },
    ingest: (id: Uuid, body: CurRequest): Promise<PointEnvelope> => {
      const p = points.find((x) => x.id === id)
      if (p) p.cur_value = body.value
      return delay(envelope(p!))
    },
  },
  sparks: {
    list: (siteId?: Uuid): Promise<Spark[]> =>
      delay(clone(siteId ? sparks.filter((s) => s.site_id === siteId) : sparks)),
    ack: (id: Uuid): Promise<Spark> => {
      const s = sparks.find((x) => x.id === id)
      if (s) s.acknowledged = true
      return delay(clone(s!))
    },
  },
  runs: {
    list: (): Promise<RunSummary[]> =>
      delay([
        {
          id: 'run_8fa2',
          status: 'awaiting_approval',
          title: 'Diagnose AHU-3 simultaneous heat/cool',
          started_at: new Date(Date.now() - 6 * 60_000).toISOString(),
        },
      ]),
  },
  agent: {
    chat: (): Promise<ChatResponse> =>
      delay({ response: 'Demo mode: connect a backend to run the agent.', steps: 0, status: 'completed' }),
  },
  query: {
    run: (): Promise<QueryResult> =>
      delay({
        columns: ['slug', 'display_name', 'kind'],
        rows: POINTS.map((p) => [p.slug, p.display_name, p.kind]),
      }),
  },
}
