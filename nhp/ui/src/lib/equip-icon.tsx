import { CircuitBoard, Droplet, Fan, Flame, Wind, Zap } from 'lucide-react'
import { hasTag } from '@/api/tags'
import type { TagSet } from '@/api/types'

type EquipKind = 'ahu' | 'chiller' | 'boiler' | 'elec' | 'vav' | 'other'

/** Equip kind derived from its Haystack-style marker tags. */
function equipKind(tags: TagSet): EquipKind {
  if (hasTag(tags, 'ahu')) return 'ahu'
  if (hasTag(tags, 'chiller')) return 'chiller'
  if (hasTag(tags, 'boiler')) return 'boiler'
  if (hasTag(tags, 'elec') || hasTag(tags, 'meter')) return 'elec'
  if (hasTag(tags, 'vav') || hasTag(tags, 'tower')) return 'vav'
  return 'other'
}

/**
 * Render the tag-derived equip icon. Dispatching on a plain kind string (rather
 * than returning a `LucideIcon` reference) keeps the icon component static, which
 * the React compiler requires — a component must not be computed during render.
 */
export function EquipKindIcon({
  tags,
  className,
}: {
  tags: TagSet
  className?: string
}) {
  switch (equipKind(tags)) {
    case 'ahu':
      return <Fan className={className} />
    case 'chiller':
      return <Droplet className={className} />
    case 'boiler':
      return <Flame className={className} />
    case 'elec':
      return <Zap className={className} />
    case 'vav':
      return <Wind className={className} />
    default:
      return <CircuitBoard className={className} />
  }
}
