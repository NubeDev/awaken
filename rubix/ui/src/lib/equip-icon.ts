import {
  CircuitBoard,
  Droplet,
  Fan,
  Flame,
  Wind,
  Zap,
  type LucideIcon,
} from 'lucide-react'

/** Icon for an equip, chosen from its Haystack-style tags. */
export function equipKindIcon(tags: string[]): LucideIcon {
  const t = new Set(tags)
  if (t.has('ahu')) return Fan
  if (t.has('chiller')) return Droplet
  if (t.has('boiler')) return Flame
  if (t.has('elec') || t.has('meter')) return Zap
  if (t.has('vav') || t.has('tower')) return Wind
  return CircuitBoard
}
