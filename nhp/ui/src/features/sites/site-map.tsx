/**
 * Sites map (DOMAIN-MODEL §site): plots every site that carries a `geo`
 * coordinate ("lat,lng") as a marker, and deep-links each marker to that site's
 * dashboard — /dashboards?tenant=<tenantKey>&site=<siteKey>, the same scope the
 * portfolio tree links to. Token-free: react-map-gl over MapLibre's open demo
 * tile style. Alarms / per-site status overlays are deferred (future work).
 */
import { useMemo, useState } from 'react'
import { useNavigate } from '@tanstack/react-router'
import MapGL, { Marker, Popup, NavigationControl } from 'react-map-gl/maplibre'
import { MapPin } from 'lucide-react'
import 'maplibre-gl/dist/maplibre-gl.css'
import type { SiteRecord } from '@/api/records'
import { Card } from '@/components/ui/card'
import { useSites, useTenants } from './hooks'

/** MapLibre's hosted demo style — raster OSM tiles, no token required. */
const MAP_STYLE = 'https://demotiles.maplibre.org/style.json'

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

export function SiteMap() {
  const sites = useSites()
  const tenants = useTenants()
  const navigate = useNavigate()
  const [active, setActive] = useState<Pin | null>(null)

  const tenantKeyById = useMemo(() => {
    const m = new Map<string, string>()
    for (const t of tenants.data ?? []) m.set(t.id, t.content.key)
    return m
  }, [tenants.data])

  const pins = useMemo<Pin[]>(() => {
    return (sites.data ?? []).flatMap((site) => {
      const coord = parseGeo(site.content.geo)
      if (!coord) return []
      return [
        {
          site,
          ...coord,
          tenantKey: site.content.tenant
            ? tenantKeyById.get(site.content.tenant)
            : undefined,
        },
      ]
    })
  }, [sites.data, tenantKeyById])

  // Centre on the mean of the plotted pins so the first paint frames them;
  // fall back to a continental US view when nothing is plotted yet.
  const initial = useMemo(() => {
    if (pins.length === 0) return { latitude: 39.5, longitude: -98.35, zoom: 3 }
    const lat = pins.reduce((s, p) => s + p.lat, 0) / pins.length
    const lng = pins.reduce((s, p) => s + p.lng, 0) / pins.length
    return { latitude: lat, longitude: lng, zoom: pins.length === 1 ? 9 : 4 }
  }, [pins])

  const openSite = (p: Pin) =>
    navigate({
      to: '/dashboards',
      search: { tenant: p.tenantKey, site: p.site.content.key },
    })

  return (
    <Card className='overflow-hidden p-0'>
      <div className='relative h-[420px] w-full'>
        <MapGL
          initialViewState={initial}
          mapStyle={MAP_STYLE}
          attributionControl={false}
        >
          <NavigationControl position='top-right' showCompass={false} />
          {pins.map((p) => (
            <Marker
              key={p.site.id}
              latitude={p.lat}
              longitude={p.lng}
              anchor='bottom'
              onClick={(e) => {
                // Stop the map from swallowing the click as a pan.
                e.originalEvent.stopPropagation()
                setActive(p)
              }}
            >
              <button
                type='button'
                title={p.site.content.name}
                className='text-primary hover:text-primary/80 cursor-pointer drop-shadow'
                aria-label={`Open ${p.site.content.name} dashboard`}
              >
                <MapPin className='size-7' fill='currentColor' strokeWidth={1.5} />
              </button>
            </Marker>
          ))}

          {active ? (
            <Popup
              latitude={active.lat}
              longitude={active.lng}
              anchor='top'
              offset={8}
              closeButton
              closeOnClick={false}
              onClose={() => setActive(null)}
            >
              <div className='space-y-1'>
                <div className='font-medium'>{active.site.content.name}</div>
                {active.site.content.address ? (
                  <div className='text-muted-foreground text-xs'>
                    {active.site.content.address}
                  </div>
                ) : null}
                <button
                  type='button'
                  onClick={() => openSite(active)}
                  className='text-primary text-xs font-medium hover:underline'
                >
                  Open dashboard →
                </button>
              </div>
            </Popup>
          ) : null}
        </MapGL>

        {sites.isLoading ? (
          <div className='text-muted-foreground bg-background/60 absolute inset-0 flex items-center justify-center text-sm'>
            Loading map…
          </div>
        ) : pins.length === 0 ? (
          <div className='text-muted-foreground bg-background/60 pointer-events-none absolute inset-0 flex items-center justify-center text-sm'>
            No sites have coordinates yet. Add latitude,longitude to a site to
            plot it.
          </div>
        ) : null}
      </div>
    </Card>
  )
}
