/**
 * Ambient declaration for `tz-lookup` (no bundled types / no `@types/tz-lookup`).
 * The package's default export maps a lat/lng to an IANA timezone name; site-form
 * uses it to default a site's timezone from its map pin.
 */
declare module 'tz-lookup' {
  export default function tzLookup(lat: number, lng: number): string
}
