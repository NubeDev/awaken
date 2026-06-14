import { createContext, useContext, useState } from 'react'

type AskAwakenContextType = {
  open: boolean
  setOpen: (open: boolean) => void
}

const AskAwakenContext = createContext<AskAwakenContextType | null>(null)

/** Holds the Ask-awaken sheet's open state so the command palette can open it. */
export function AskAwakenProvider({ children }: { children: React.ReactNode }) {
  const [open, setOpen] = useState(false)
  return <AskAwakenContext value={{ open, setOpen }}>{children}</AskAwakenContext>
}

// eslint-disable-next-line react-refresh/only-export-components
export function useAskAwaken() {
  const ctx = useContext(AskAwakenContext)
  if (!ctx) throw new Error('useAskAwaken must be used within AskAwakenProvider')
  return ctx
}
