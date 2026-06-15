// Chart colour resolution — ported from ui-demo/copilot/viz.js. A key like
// 'crit' resolves to the themed HSL; anything else passes through unchanged so
// callers can hand in a raw colour.

const C: Record<string, string> = {
  crit: 'hsl(var(--crit))',
  amber: 'hsl(var(--amber))',
  green: 'hsl(var(--green))',
  cool: 'hsl(var(--cool))',
  r1: 'hsl(var(--r1))',
  r2: 'hsl(var(--r2))',
  muted: 'hsl(var(--muted))',
  grid: 'hsl(230 10% 18%)',
  axis: 'hsl(220 8% 56%)',
}

export const col = (k: string): string => C[k] || k

// Temperature → heat colour, interpolated across the demo's stop table.
export function heat(t: number | null): string {
  if (t == null) return 'hsl(38 10% 30%)'
  const st: [number, number, number, number][] = [
    [16, 196, 62, 52],
    [20, 150, 58, 45],
    [23, 150, 55, 46],
    [24.5, 40, 80, 52],
    [26, 18, 82, 52],
    [27, 6, 70, 50],
  ]
  if (t <= st[0][0]) return `hsl(${st[0][1]} ${st[0][2]}% ${st[0][3]}%)`
  const last = st[st.length - 1]
  if (t >= last[0]) return `hsl(${last[1]} ${last[2]}% ${last[3]}%)`
  for (let i = 0; i < st.length - 1; i++) {
    if (t >= st[i][0] && t <= st[i + 1][0]) {
      const f = (t - st[i][0]) / (st[i + 1][0] - st[i][0])
      const h = (st[i][1] + (st[i + 1][1] - st[i][1]) * f).toFixed(0)
      const s = (st[i][2] + (st[i + 1][2] - st[i][2]) * f).toFixed(0)
      const l = (st[i][3] + (st[i + 1][3] - st[i][3]) * f).toFixed(0)
      return `hsl(${h} ${s}% ${l}%)`
    }
  }
  return `hsl(${last[1]} ${last[2]}% ${last[3]}%)`
}
