// Light/dark/system theme provider. Owns the `.light`/`.dark` class on <html>
// (styles/theme.css keys all color tokens off it) and persists the choice to
// localStorage. Adapted from satnaing/shadcn-admin, swapping its cookie helper
// for localStorage and resolving "system" against prefers-color-scheme.

import { createContext, useContext, useEffect, useMemo, useState } from 'react'

type Theme = 'dark' | 'light' | 'system'
type ResolvedTheme = Exclude<Theme, 'system'>

const DEFAULT_THEME: Theme = 'system'
const STORAGE_KEY = 'rubix-ui-theme'

type ThemeProviderState = {
  theme: Theme
  resolvedTheme: ResolvedTheme
  setTheme: (theme: Theme) => void
}

const ThemeContext = createContext<ThemeProviderState | null>(null)

function readStored(): Theme {
  if (typeof window === 'undefined') return DEFAULT_THEME
  const v = window.localStorage.getItem(STORAGE_KEY)
  return v === 'light' || v === 'dark' || v === 'system' ? v : DEFAULT_THEME
}

export function ThemeProvider({ children }: { children: React.ReactNode }) {
  const [theme, _setTheme] = useState<Theme>(readStored)

  const resolvedTheme = useMemo<ResolvedTheme>(() => {
    if (theme === 'system') {
      return window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light'
    }
    return theme
  }, [theme])

  useEffect(() => {
    const root = window.document.documentElement
    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')

    const apply = (t: ResolvedTheme) => {
      root.classList.remove('light', 'dark')
      root.classList.add(t)
    }

    apply(resolvedTheme)

    const handleChange = () => {
      if (theme === 'system') apply(mediaQuery.matches ? 'dark' : 'light')
    }
    mediaQuery.addEventListener('change', handleChange)
    return () => mediaQuery.removeEventListener('change', handleChange)
  }, [theme, resolvedTheme])

  const setTheme = (next: Theme) => {
    window.localStorage.setItem(STORAGE_KEY, next)
    _setTheme(next)
  }

  return <ThemeContext.Provider value={{ theme, resolvedTheme, setTheme }}>{children}</ThemeContext.Provider>
}

// eslint-disable-next-line react-refresh/only-export-components
export function useTheme() {
  const ctx = useContext(ThemeContext)
  if (!ctx) throw new Error('useTheme must be used within a ThemeProvider')
  return ctx
}
