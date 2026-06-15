// One zone row in the live floor plan — heat-mapped cells, setpoint, deviation
// and a load bar. Ported from screens.js `building()` row markup.

import { heat } from '../viz/colors'
import { fmtDeviation, fmtTemp } from '../../utils/format'
import { sevMap } from '../ui/severity'
import type { Zone } from '../../types/Domain'

// Six heat cells spanning a small spread around the zone temperature, so the row
// reads as a strip rather than a single swatch (matches the demo).
function cells(zone: Zone) {
  if (zone.temp == null) {
    return Array.from({ length: 6 }, (_, i) => (
      <span key={i} className="flex-1 h-7 rounded" style={{ background: 'hsl(38 10% 22%)' }} />
    ))
  }
  return Array.from({ length: 6 }, (_, i) => {
    const t = zone.temp! + Math.sin(i * 1.7) * 0.5
    return <span key={i} className="flex-1 h-7 rounded" style={{ background: heat(t) }} />
  })
}

export function ZoneRow({ zone, maxLoad }: { zone: Zone; maxLoad: number }) {
  const s = sevMap[zone.severity]
  const shadow =
    zone.severity === 'crit'
      ? 'inset 3px 0 0 hsl(var(--crit))'
      : zone.severity === 'amber'
        ? 'inset 3px 0 0 hsl(var(--amber))'
        : undefined
  return (
    <div className="flex items-center gap-4 px-5 py-3 hover:bg-panel3/50 transition" style={{ boxShadow: shadow }}>
      <div className="w-[150px] shrink-0">
        <div className="text-[13px] font-semibold truncate">{zone.name}</div>
        <div className="text-[11px] text-muted mono">
          SP {zone.sp != null ? `${zone.sp.toFixed(1)}°` : '—'}
          {zone.note ? ` · ${zone.note}` : ''}
        </div>
      </div>
      <div className="flex-1 flex gap-1">{cells(zone)}</div>
      <div className="w-[96px] text-right shrink-0">
        <div className={`mono text-[15px] font-semibold ${zone.severity === 'crit' ? 'text-crit' : ''}`}>
          {fmtTemp(zone.temp)}
        </div>
        <div className="text-[11px] text-muted mono">{fmtDeviation(zone.temp, zone.sp)}</div>
        <div className="h-1.5 rounded-full bg-border mt-1 overflow-hidden">
          <span
            className="block h-full rounded-full"
            style={{ width: `${maxLoad ? (zone.load / maxLoad) * 100 : 0}%`, background: `hsl(var(--${s.c}))` }}
          />
        </div>
      </div>
    </div>
  )
}
