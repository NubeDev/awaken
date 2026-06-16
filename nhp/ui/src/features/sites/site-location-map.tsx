/**
 * Small interactive map for the site form: shows the chosen coordinates with a
 * draggable pin so the admin can confirm the geocode landed in the right spot and
 * nudge it. Dragging (or clicking the map) reports new coordinates upward, which
 * the form writes to `geo` and re-derives the timezone from. Token-free CARTO
 * tiles, same basemap as the portfolio SiteMap.
 */
import { useEffect, useRef } from 'react'
import MapGL, {
  Marker,
  NavigationControl,
  type MapRef,
  type MapLayerMouseEvent,
} from 'react-map-gl/maplibre'
import { MapPin } from 'lucide-react'
import 'maplibre-gl/dist/maplibre-gl.css'
import { useTheme } from '@/context/theme-provider'
import { cartoStyle, type LatLng } from './geo'

const DARK_STYLE = cartoStyle('dark_all')
const LIGHT_STYLE = cartoStyle('light_all')

type SiteLocationMapProps = {
  value: LatLng | null
  onChange: (coord: LatLng) => void
  height?: number
}

export function SiteLocationMap({
  value,
  onChange,
  height = 200,
}: SiteLocationMapProps) {
  const { resolvedTheme } = useTheme()
  const mapRef = useRef<MapRef>(null)
  const mapStyle = resolvedTheme === 'dark' ? DARK_STYLE : LIGHT_STYLE

  // Recentre when the coordinate changes from outside (e.g. an address pick).
  useEffect(() => {
    if (value && mapRef.current) {
      mapRef.current.flyTo({
        center: [value.lng, value.lat],
        zoom: 14,
        duration: 600,
      })
    }
  }, [value])

  const place = (e: MapLayerMouseEvent) =>
    onChange({ lat: e.lngLat.lat, lng: e.lngLat.lng })

  return (
    <div
      className='relative overflow-hidden rounded-md border'
      style={{ height }}
    >
      <MapGL
        ref={mapRef}
        initialViewState={{
          latitude: value?.lat ?? 39.5,
          longitude: value?.lng ?? -98.35,
          zoom: value ? 14 : 3,
        }}
        mapStyle={mapStyle}
        attributionControl={{ compact: true }}
        onClick={place}
      >
        <NavigationControl position='top-right' showCompass={false} />
        {value ? (
          <Marker
            latitude={value.lat}
            longitude={value.lng}
            anchor='bottom'
            draggable
            onDragEnd={(e) =>
              onChange({ lat: e.lngLat.lat, lng: e.lngLat.lng })
            }
          >
            <MapPin
              className='text-primary size-8 cursor-grab drop-shadow-md active:cursor-grabbing'
              fill='currentColor'
              stroke='white'
              strokeWidth={1.5}
            />
          </Marker>
        ) : null}
      </MapGL>
      {!value ? (
        <div className='text-muted-foreground bg-background/70 pointer-events-none absolute inset-x-0 bottom-0 p-1.5 text-center text-xs backdrop-blur-sm'>
          Search an address or click the map to drop a pin.
        </div>
      ) : null}
    </div>
  )
}
