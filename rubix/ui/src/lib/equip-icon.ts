import {
  CircuitBoard,
  Droplet,
  Fan,
  Flame,
  Wind,
  Zap,
  type LucideIcon,
} from 'lucide-react'
import { hasTag } from '@/api/tags'
import type { TagSet } from '@/api/types'

/** Icon for an equip, chosen from its Haystack-style marker tags. */
export function equipKindIcon(tags: TagSet): LucideIcon {
  if (hasTag(tags, 'ahu')) return Fan
  if (hasTag(tags, 'chiller')) return Droplet
  if (hasTag(tags, 'boiler')) return Flame
  if (hasTag(tags, 'elec') || hasTag(tags, 'meter')) return Zap
  if (hasTag(tags, 'vav') || hasTag(tags, 'tower')) return Wind
  return CircuitBoard
}
