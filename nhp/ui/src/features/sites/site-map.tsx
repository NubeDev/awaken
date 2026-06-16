/**
 * Sites map: plots every site that carries a `geo` coordinate ("lat,lng") on a
 * real street map and deep-links each marker to that site's dashboard —
 * /dashboards?tenant=<tenantKey>&site=<siteKey>, the same scope the portfolio
 * tree links to. Token-free: react-map-gl over MapLibre with OpenStreetMap
 * raster tiles (no Mapbox/MapTiler account). Pass `tenantKey` to show only one
 * tenant's sites (the dashboard home does this); omit it to show all.
 *
 * Alarms / per-site status colouring on the markers is deferred (future work).
 */
import { useEffect, useMemo, useRef, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import MapGL, {
  Marker,
  Popup,
  NavigationControl,
  type MapRef,
} from 'react-map-gl/maplibre'
import type { StyleSpecification } from 'maplibre-gl'
import { MapPin } from 'lucide-react'
import 'maplibre-gl/dist/maplibre-gl.css'
import type { SiteRecord } from '@/api/records'
import { useSites, useTenants } from './hooks'

/**
 * OpenStreetMap raster tiles as a MapLibre style — a real street map, no token.
 * Defined inline (not a hosted style.json) so the only network dep is the tile
 * CDN itself. © OpenStreetMap contributors.
 */
const OSM_STYLE: StyleSpecification = {
  version: 8,
  sources: {
    osm: {
      type: 'raster',
      tiles: ['https://tile.openstreetmap.org/{z}/{x}/{y}.png'],
      tileSize: 256,
      attribution: '© OpenStreetMap contributors',
    },
  },
  layers: [{ id: 'osm', type: 'raster', source: 'osm' }],
}

type Pin = {
  site: SiteRecord
  lat: number
  lng: number
  /** Parent tenant key — the dashboard deep-link is keyed, not by record id. */
  tenantKey?: string
}

/** Parse a `geo` string ("lat,lng") to a finite {lat,lng}, or null if unusable. */
function parseGeo(geo?: string): { lat: number; lng: number } | null {
  if (!geo) return null
  const [a, b] = geo.split(',').map((s) => Number(s.trim()))
  if (!Number.isFinite(a) || !Number.isFinite(b)) return null
  if (a < -90 || a > 90 || b < -180 || b > 180) return null
  return { lat: a, lng: b }
}

export function SiteMap({
  tenantKey,
  className,
  height = 360,
}: {
  /** Show only this tenant's sites. Omit to show every tenant's. */
  tenantKey?: string
  className?: string
  height?: number
}) {
  const sites = useSites()
  const tenants = useTenants()
  const navigate = useNavigate()
  const mapRef = useRef<MapRef>(null)
  const [active, setActive] = useState<Pin | null>(null)

  const tenantById = useMemo(() => {
    const m = new Map<string, string>()
    for (const t of tenants.data ?? []) m.set(t.id, t.content.key)
    return m
  }, [tenants.data])

  // The tenant record id for the requested key, so we can filter sites by parent.
  const tenantId = useMemo(() => {
    if (!tenantKey) return undefined
    return (tenants.data ?? []).find((t) => t.content.key === tenantKey)?.id
  }, [tenants.data, tenantKey])

  const pins = useMemo<Pin[]>(() => {
    return (sites.data ?? []).flatMap((site) => {
      if (tenantId && site.content.tenant !== tenantId) return []
      const coord = parseGeo(site.content.geo)
      if (!coord) return []
      return [
        {
          site,
          ...coord,
          tenantKey: site.content.tenant
            ? tenantById.get(site.content.tenant)
            : undefined,
        },
      ]
    })
  }, [sites.data, tenantById, tenantId])

  // Frame the markers: fit the map to their bounding box once they're known.
  useEffect(() => {
    const map = mapRef.current
    if (!map || pins.length === 0) return
    if (pins.length === 1) {
      map.flyTo({ center: [pins[0].lng, pins[0].lat], zoom: 11, duration: 0 })
      return
    }
    let minLng = Infinity,
      minLat = Infinity,
      maxLng = -Infinity,
      maxLat = -Infinity
    for (const p of pins) {
      minLng = Math.min(minLng, p.lng)
      minLat = Math.min(minLat, p.lat)
      maxLng = Math.max(maxLng, p.lng)
      maxLat = Math.max(maxLat, p.lat)
    }
    map.fitBounds(
      [
        [minLng, minLat],
        [maxLng, maxLat],
      ],
      { padding: 64, maxZoom: 12, duration: 0 }
    )
  }, [pins])

  const openSite = (p: Pin) =>
    navigate({
      to: '/dashboards',
      search: { tenant: p.tenantKey, site: p.site.content.key },
    })

  return (
    <div
      className={className}
      style={{ height }}
    >
      <div className='relative h-full w-full overflow-hidden rounded-xl border shadow-sm'>
        <MapGL
          ref={mapRef}
          initialViewState={{ latitude: 39.5, longitude: -98.35, zoom: 3 }}
          mapStyle={OSM_STYLE}
          attributionControl={{ compact: true }}
        >
          <NavigationControl position='top-right' showCompass={false} />
          {pins.map((p) => {
            const isActive = active?.site.id === p.site.id
            return (
              <Marker
                key={p.site.id}
                latitude={p.lat}
                longitude={p.lng}
                anchor='bottom'
                onClick={(e) => {
                  e.originalEvent.stopPropagation()
                  setActive(p)
                }}
              >
                <button
                  type='button'
                  title={p.site.content.name}
                  aria-label={`Open ${p.site.content.name} dashboard`}
                  className='group relative grid place-items-center'
                >
                  {/* Soft pulse halo behind the pin. */}
                  <span className='bg-primary/25 absolute size-7 rounded-full blur-[2px] transition-transform group-hover:scale-125' />
                  <MapPin
                    className={
                      'relative size-8 cursor-pointer drop-shadow-md transition-transform group-hover:-translate-y-0.5 ' +
                      (isActive ? 'text-primary' : 'text-primary/90')
                    }
                    fill='currentColor'
                    stroke='white'
                    strokeWidth={1.5}
                  />
                </button>
              </Marker>
            )
          })}

          {active ? (
            <Popup
              latitude={active.lat}
              longitude={active.lng}
              anchor='bottom'
              offset={30}
              closeButton={false}
              closeOnClick={false}
              onClose={() => setActive(null)}
              className='site-map-popup'
            >
              <button
                type='button'
                onClick={() => openSite(active)}
                className='block w-full cursor-pointer text-left'
              >
                <div className='text-sm font-semibold'>
                  {active.site.content.name}
                </div>
                {active.site.content.address ? (
                  <div className='text-muted-foreground text-xs'>
                    {active.site.content.address}
                  </div>
                ) : null}
                <div className='text-primary mt-1 text-xs font-medium'>
                  Open dashboard →
                </div>
              </button>
            </Popup>
          ) : null}
        </MapGL>

        {sites.isLoading ? (
          <div className='text-muted-foreground bg-background/70 absolute inset-0 flex items-center justify-center text-sm backdrop-blur-sm'>
            Loading map…
          </div>
        ) : pins.length === 0 ? (
          <div className='text-muted-foreground bg-background/70 pointer-events-none absolute inset-0 flex items-center justify-center text-sm backdrop-blur-sm'>
            No sites have coordinates yet.
          </div>
        ) : null}
      </div>
    </div>
  )
}
