/**
 * Shared geo helpers for sites: the token-free CARTO/MapLibre raster style, the
 * `geo` ("lat,lng") parser/formatter, and OpenStreetMap Nominatim geocoding for
 * address autocomplete. Coordinates live as one `geo` string on the Site record
 * (DOMAIN-MODEL §site), so parse/format keep that single source of truth.
 *
 * Nominatim is the free OSM geocoder — no API key, but it asks callers to send a
 * descriptive User-Agent/Referer and stay under ~1 req/sec, so the search is
 * debounced by the caller. Network-optional: if the server/browser has no
 * internet the search simply returns nothing and the user types coords by hand.
 */
import type { StyleSpecification } from 'maplibre-gl'

export type LatLng = { lat: number; lng: number }

/** Parse a `geo` string ("lat,lng") to a finite {lat,lng}, or null if unusable. */
export function parseGeo(geo?: string): LatLng | null {
  if (!geo) return null
  const [a, b] = geo.split(',').map((s) => Number(s.trim()))
  if (!Number.isFinite(a) || !Number.isFinite(b)) return null
  if (a < -90 || a > 90 || b < -180 || b > 180) return null
  return { lat: a, lng: b }
}

/** Format coordinates back into the stored `geo` string, ~5 dp (≈1 m). */
export function formatGeo({ lat, lng }: LatLng): string {
  return `${lat.toFixed(5)},${lng.toFixed(5)}`
}

/**
 * CARTO basemaps as MapLibre raster styles — token-free tiles. Muted dark in dark
 * mode, Positron (light) otherwise. © OpenStreetMap contributors © CARTO.
 */
export function cartoStyle(
  variant: 'dark_all' | 'light_all'
): StyleSpecification {
  return {
    version: 8,
    sources: {
      carto: {
        type: 'raster',
        tiles: [
          `https://a.basemaps.cartocdn.com/${variant}/{z}/{x}/{y}@2x.png`,
          `https://b.basemaps.cartocdn.com/${variant}/{z}/{x}/{y}@2x.png`,
          `https://c.basemaps.cartocdn.com/${variant}/{z}/{x}/{y}@2x.png`,
        ],
        tileSize: 256,
        attribution:
          '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> © <a href="https://carto.com/attributions">CARTO</a>',
      },
    },
    layers: [{ id: 'carto', type: 'raster', source: 'carto' }],
  }
}

export type GeocodeResult = {
  /** Full human-readable address (Nominatim `display_name`). */
  label: string
  lat: number
  lng: number
}

/**
 * Geocode a free-text address via OSM Nominatim. Returns up to `limit` matches
 * ranked by Nominatim's importance. Resolves to [] on any failure (offline, rate
 * limit, abort) — callers fall back to manual entry. Pass an AbortSignal so a
 * superseded keystroke's request can be cancelled.
 */
export async function geocodeAddress(
  query: string,
  signal?: AbortSignal,
  limit = 5
): Promise<GeocodeResult[]> {
  const q = query.trim()
  if (q.length < 3) return []
  const url =
    'https://nominatim.openstreetmap.org/search?format=jsonv2' +
    `&addressdetails=0&limit=${limit}&q=${encodeURIComponent(q)}`
  try {
    const res = await fetch(url, {
      signal,
      headers: { Accept: 'application/json' },
    })
    if (!res.ok) return []
    const data = (await res.json()) as Array<{
      display_name: string
      lat: string
      lon: string
    }>
    return data
      .map((d) => ({
        label: d.display_name,
        lat: Number(d.lat),
        lng: Number(d.lon),
      }))
      .filter((r) => Number.isFinite(r.lat) && Number.isFinite(r.lng))
  } catch {
    // Aborted, offline, or blocked — let the caller fall back to manual entry.
    return []
  }
}
