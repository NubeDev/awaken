/**
 * Point keyexpr construction and resolution. The server addresses a point by
 * the dotted/slashed path `org/slug/equip.path/point.slug` — the same string
 * `PointEnvelope.keyexpr` carries (see `__fixtures__/point.json` and
 * `crates/rubix-server/src/api/points`). A `point_value` / `point_history`
 * widget stores this keyexpr as its `target`; rendering a live tile means
 * resolving that keyexpr back to a `Point`, so both directions live here.
 */
import type { Equip, Point, Site } from './types'

/** The canonical point keyexpr the server stores and a widget target carries. */
export function pointKeyexpr(site: Site, equip: Equip, point: Point): string {
  return `${site.org}/${site.slug}/${equip.path}/${point.slug}`
}

/** Index a site's points by keyexpr for widget-target resolution. */
export function keyexprIndex(
  site: Site,
  equips: Equip[],
  points: Point[]
): Map<string, Point> {
  const equipById = new Map(equips.map((e) => [e.id, e]))
  const index = new Map<string, Point>()
  for (const point of points) {
    const equip = equipById.get(point.equip_id)
    if (equip) index.set(pointKeyexpr(site, equip, point), point)
  }
  return index
}

/**
 * Index points across many sites by keyexpr. An org-overview dashboard mixes
 * tiles from several sites, so its canvas resolves targets against the union.
 * `equips` and `points` may span every site; each point is keyed under the
 * site its equip belongs to.
 */
export function keyexprIndexMulti(
  sites: Site[],
  equips: Equip[],
  points: Point[]
): Map<string, Point> {
  const siteById = new Map(sites.map((s) => [s.id, s]))
  const equipById = new Map(equips.map((e) => [e.id, e]))
  const index = new Map<string, Point>()
  for (const point of points) {
    const equip = equipById.get(point.equip_id)
    const site = equip && siteById.get(equip.site_id)
    if (equip && site) index.set(pointKeyexpr(site, equip, point), point)
  }
  return index
}
