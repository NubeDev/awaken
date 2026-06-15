// A minimal toast surface for the admin console — success/error feedback after a
// mutation. Deliberately dependency-free (no sonner/radix-toast): a context holds
// a short-lived queue and renders a fixed stack. Themed via theme.css tokens.

import * as React from 'react'

import { cn } from '@/lib/cn'

type ToastTone = 'default' | 'error'

interface Toast {
  id: number
  message: string
  tone: ToastTone
}

interface ToastCtx {
  toast: (message: string, tone?: ToastTone) => void
}

const Ctx = React.createContext<ToastCtx | null>(null)

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = React.useState<Toast[]>([])
  const seq = React.useRef(0)

  const toast = React.useCallback((message: string, tone: ToastTone = 'default') => {
    const id = ++seq.current
    setToasts((t) => [...t, { id, message, tone }])
    setTimeout(() => setToasts((t) => t.filter((x) => x.id !== id)), 4000)
  }, [])

  const value = React.useMemo(() => ({ toast }), [toast])

  return (
    <Ctx.Provider value={value}>
      {children}
      <div className="fixed bottom-4 right-4 z-[100] flex flex-col gap-2">
        {toasts.map((t) => (
          <div
            key={t.id}
            className={cn(
              'fade rounded-lg border px-4 py-2.5 text-sm shadow-lg',
              t.tone === 'error'
                ? 'border-destructive/40 bg-destructive/15 text-destructive'
                : 'border-border bg-popover text-popover-foreground',
            )}
          >
            {t.message}
          </div>
        ))}
      </div>
    </Ctx.Provider>
  )
}

export function useToast(): ToastCtx {
  const ctx = React.useContext(Ctx)
  if (!ctx) throw new Error('useToast must be used within ToastProvider')
  return ctx
}
