// The Rubix orb — the agent's identity mark. A conic-gradient core that slowly
// rotates, optionally with a sparkle. Ported from the demo's `orb()`.

import { Sparkles } from 'lucide-react'

interface OrbProps {
  size?: number
  sparkle?: boolean
  /** Soft halo variant used in headers/banners. */
  blur?: boolean
}

export function Orb({ size = 32, sparkle = false, blur = false }: OrbProps) {
  return (
    <div className="relative grid place-items-center shrink-0" style={{ width: size, height: size }}>
      <div className={`absolute inset-0 rounded-full orb-core${blur ? ' blur-[1px]' : ''}`} />
      <div className={`absolute inset-[3px] rounded-full ${blur ? 'bg-bg/55' : 'bg-bg'}`} />
      {sparkle && <Sparkles className="relative text-white" size={size * 0.5} />}
    </div>
  )
}
