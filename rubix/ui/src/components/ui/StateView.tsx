// Shared loading / error / empty states so every data screen handles the three
// non-happy paths consistently (no silent blank screens).

import { Orb } from './Orb'

export function LoadingView({ label = 'Loading…' }: { label?: string }) {
  return (
    <div className="flex-1 grid place-items-center">
      <div className="flex items-center gap-3 text-muted text-[13px]">
        <Orb size={26} />
        {label}
      </div>
    </div>
  )
}

export function ErrorView({ error }: { error: unknown }) {
  const msg = error instanceof Error ? error.message : String(error)
  return (
    <div className="flex-1 grid place-items-center px-8">
      <div className="max-w-[460px] rounded-2xl border border-crit/30 bg-crit/[.06] p-6 text-center">
        <div className="text-[15px] font-semibold text-crit">Couldn’t reach the backend</div>
        <div className="text-[12.5px] text-muted mt-2 break-words">{msg}</div>
      </div>
    </div>
  )
}

export function EmptyView({ title, hint }: { title: string; hint?: string }) {
  return (
    <div className="rounded-2xl border border-dashed border-border p-8 text-center">
      <div className="text-[14px] font-semibold">{title}</div>
      {hint && <div className="text-[12.5px] text-muted mt-1">{hint}</div>}
    </div>
  )
}
