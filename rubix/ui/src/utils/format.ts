// Display formatting — numbers, temperatures, relative times.

export function fmtNum(n: number | null | undefined, digits = 0): string {
  if (n == null) return '—'
  return n.toLocaleString('en-US', { minimumFractionDigits: digits, maximumFractionDigits: digits })
}

export function fmtTemp(n: number | null | undefined): string {
  return n == null ? '—' : `${n.toFixed(1)}°`
}

export function fmtDeviation(temp: number | null, sp: number | null): string {
  if (temp == null || sp == null) return '—'
  const d = temp - sp
  return `${d >= 0 ? '+' : ''}${d.toFixed(1)}`
}

export function relTime(iso: string | null): string {
  if (!iso) return '—'
  const t = new Date(iso).getTime()
  if (Number.isNaN(t)) return '—'
  const mins = Math.round((Date.now() - t) / 60000)
  if (mins < 1) return 'now'
  if (mins < 60) return `${mins}m ago`
  const hrs = Math.round(mins / 60)
  if (hrs < 24) return `${hrs}h ago`
  return `${Math.round(hrs / 24)}d ago`
}
