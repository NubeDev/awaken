/**
 * The meter-type barcode scheme (WS-09 Part A) — the single round-trip contract
 * the admin RENDERS and the scan wizard DECODES. A drift here means a printed
 * label no longer resolves, so encode/decode/resolve live in ONE module.
 *
 * Scheme (nhp/docs/sessions/WS-09.md §Barcode scheme): a meter-type's `key` is
 * already its stable unique id (DOMAIN-MODEL §meter-type). The barcode encodes that
 * key behind a fixed prefix so a stray scanned code (a shipping label, a URL) does
 * NOT accidentally resolve:
 *   encode(key) = `nhp-mt:${key}`        e.g. `nhp-mt:acme-pm5560`
 *   decode(code) = key | null            (null when the prefix is absent)
 *   resolve(code, types) = the meter-type whose `content.key === decode(code)`
 *
 * BACKFILL-SAFE. Existing meter-types carry no stored `barcode`; the derived code is
 * a pure function of `key`, so `barcodeFor` falls back to it and old types resolve
 * with no data migration.
 */
import type { MeterTypeRecord } from '@/api/records'

/** Fixed namespace prefix — see module doc / WS-09. */
export const BARCODE_PREFIX = 'nhp-mt:'

/** The canonical barcode value for a meter-type key. */
export function encodeBarcode(key: string): string {
  return `${BARCODE_PREFIX}${key}`
}

/**
 * The meter-type key a scanned/typed code carries, or `null` when the code is not
 * an NHP meter-type code (wrong/absent prefix, or empty key).
 */
export function decodeBarcode(code: string): string | null {
  const trimmed = code.trim()
  if (!trimmed.startsWith(BARCODE_PREFIX)) return null
  const key = trimmed.slice(BARCODE_PREFIX.length)
  return key.length > 0 ? key : null
}

/**
 * The barcode value to RENDER for a meter-type: its stored `barcode` if set, else
 * the value derived from `key` (backfill — existing types have no stored field).
 */
export function barcodeFor(type: MeterTypeRecord): string {
  const stored = type.content.barcode?.trim()
  return stored && stored.length > 0 ? stored : encodeBarcode(type.content.key)
}

/**
 * Resolve a scanned/typed code to the matching meter-type, or `null` when the code
 * does not decode or no meter-type has that key. Matches on `key` (the scheme's
 * payload) so a backfilled type with no stored `barcode` still resolves.
 */
export function resolveBarcode(
  code: string,
  types: MeterTypeRecord[]
): MeterTypeRecord | null {
  const key = decodeBarcode(code)
  if (!key) return null
  return types.find((t) => t.content.key === key) ?? null
}
