/**
 * The barcode round-trip contract (WS-09 Part A): encode → decode → resolve returns
 * the correct meter-type, the prefix rejects stray codes, and a backfilled type
 * (no stored `barcode`) still resolves from its `key`. A failure here means a
 * printed label would not resolve when scanned.
 */
import { describe, expect, it } from 'vitest'
import type { MeterTypeRecord } from '@/api/records'
import {
  BARCODE_PREFIX,
  barcodeFor,
  decodeBarcode,
  encodeBarcode,
  resolveBarcode,
} from './barcode'

function meterType(
  key: string,
  barcode?: string
): MeterTypeRecord {
  return {
    id: `rec-${key}`,
    namespace: 'acme',
    created: '',
    updated: '',
    tags: [],
    content: {
      kind: 'meter-type',
      key,
      name: key.toUpperCase(),
      version: 1,
      registers: [],
      ...(barcode ? { barcode } : {}),
    },
  } as MeterTypeRecord
}

// The two seed types (backfill case: no stored `barcode`) + one created with a
// stored value, to prove both resolve.
const types = [
  meterType('acme-pm5560'),
  meterType('acme-em24'),
  meterType('acme-new', encodeBarcode('acme-new')),
]

describe('barcode scheme', () => {
  it('encodes a key behind the fixed prefix', () => {
    expect(encodeBarcode('acme-pm5560')).toBe('nhp-mt:acme-pm5560')
    expect(encodeBarcode('acme-pm5560').startsWith(BARCODE_PREFIX)).toBe(true)
  })

  it('decodes a prefixed code back to the key (round-trip)', () => {
    const key = 'acme-pm5560'
    expect(decodeBarcode(encodeBarcode(key))).toBe(key)
    // tolerant of surrounding whitespace from a scanner
    expect(decodeBarcode(`  ${encodeBarcode(key)}  `)).toBe(key)
  })

  it('rejects a code without the NHP prefix (stray scan)', () => {
    expect(decodeBarcode('https://example.com/x')).toBeNull()
    expect(decodeBarcode('acme-pm5560')).toBeNull() // bare key, not a code
    expect(decodeBarcode('nhp-mt:')).toBeNull() // empty key
    expect(decodeBarcode('')).toBeNull()
  })

  it('encode → decode → resolve returns the correct meter-type', () => {
    const code = encodeBarcode('acme-pm5560')
    const resolved = resolveBarcode(code, types)
    expect(resolved?.content.key).toBe('acme-pm5560')
  })

  it('resolves a backfilled type (no stored barcode) from its key', () => {
    // acme-em24 has no `content.barcode` — derived code must still resolve it.
    const derived = barcodeFor(meterType('acme-em24'))
    expect(derived).toBe('nhp-mt:acme-em24')
    expect(resolveBarcode(derived, types)?.content.key).toBe('acme-em24')
  })

  it('resolves a type with a stored barcode', () => {
    const stored = barcodeFor(types[2]) // acme-new
    expect(stored).toBe('nhp-mt:acme-new')
    expect(resolveBarcode(stored, types)?.content.key).toBe('acme-new')
  })

  it('returns null for an unknown but well-formed code', () => {
    expect(resolveBarcode(encodeBarcode('not-a-type'), types)).toBeNull()
  })

  it('barcodeFor prefers a stored value over the derived one', () => {
    const t = meterType('acme-pm5560', 'nhp-mt:legacy-code')
    expect(barcodeFor(t)).toBe('nhp-mt:legacy-code')
  })
})
