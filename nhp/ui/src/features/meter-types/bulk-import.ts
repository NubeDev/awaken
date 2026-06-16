/**
 * Bulk-paste import for big register maps (ADMIN.md §1 "bulk-paste a register
 * table (CSV/JSON)"). Accepts either a JSON array of register-defs or a CSV with
 * a header row whose columns are register-def field names. Returns parsed defs or
 * throws a human-readable error the dialog surfaces. Enum/type validity is left to
 * the row editors + the client enforce layer — this only parses the table.
 */
import { blankRegister } from './register-defaults'
import type { RegisterDef } from '@/api/records'

const NUMBER_FIELDS = new Set([
  'address',
  'word_count',
  'scale',
  'offset',
  'precision',
])
const BOOL_FIELDS = new Set(['signed', 'history'])

function coerce(field: string, raw: string): unknown {
  if (NUMBER_FIELDS.has(field)) return raw === '' ? 0 : Number(raw)
  if (BOOL_FIELDS.has(field)) return /^(true|1|yes|y)$/i.test(raw.trim())
  return raw
}

/** Parse one CSV line, honouring double-quoted fields with embedded commas. */
function splitCsvLine(line: string): string[] {
  const out: string[] = []
  let cur = ''
  let inQuotes = false
  for (let i = 0; i < line.length; i++) {
    const ch = line[i]
    if (inQuotes) {
      if (ch === '"' && line[i + 1] === '"') {
        cur += '"'
        i++
      } else if (ch === '"') inQuotes = false
      else cur += ch
    } else if (ch === '"') inQuotes = true
    else if (ch === ',') {
      out.push(cur)
      cur = ''
    } else cur += ch
  }
  out.push(cur)
  return out.map((s) => s.trim())
}

function parseCsv(text: string, startIndex: number): RegisterDef[] {
  const lines = text
    .split(/\r?\n/)
    .map((l) => l.trim())
    .filter((l) => l.length > 0)
  if (lines.length < 2) throw new Error('CSV needs a header row and ≥1 data row')
  const header = splitCsvLine(lines[0])
  return lines.slice(1).map((line, i) => {
    const cells = splitCsvLine(line)
    const base = blankRegister(startIndex + i) as unknown as Record<
      string,
      unknown
    >
    header.forEach((field, col) => {
      if (cells[col] !== undefined && field in base) {
        base[field] = coerce(field, cells[col])
      }
    })
    return base as unknown as RegisterDef
  })
}

function parseJson(text: string, startIndex: number): RegisterDef[] {
  const data: unknown = JSON.parse(text)
  const arr = Array.isArray(data) ? data : [data]
  return arr.map((row, i) => {
    if (typeof row !== 'object' || row === null) {
      throw new Error('Each JSON entry must be a register object')
    }
    return { ...blankRegister(startIndex + i), ...(row as object) } as RegisterDef
  })
}

/** Parse pasted text as JSON (if it looks like JSON) else CSV. */
export function parseRegisterTable(
  text: string,
  startIndex: number
): RegisterDef[] {
  const trimmed = text.trim()
  if (!trimmed) throw new Error('Nothing to import')
  if (trimmed.startsWith('[') || trimmed.startsWith('{')) {
    return parseJson(trimmed, startIndex)
  }
  return parseCsv(trimmed, startIndex)
}
