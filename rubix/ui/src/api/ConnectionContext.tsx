// Ambient connection + per-tenant API client. The connection (endpoint +
// credential) is app-wide; the tenant is the first URL segment, so a scoped
// client is derived per tenant via useApi(tenant) — there is no global mutable
// "current tenant" outside the router (PRODUCT-UI "Routing").

import { createContext, useCallback, useContext, useMemo, useState, type ReactNode } from 'react'
import { ApiClient } from './client'
import { clearConnection, loadConnection, saveConnection, type Connection } from './connection'

interface ConnectionCtx {
  connection: Connection | null
  connect: (c: Connection) => void
  disconnect: () => void
}

const Ctx = createContext<ConnectionCtx | null>(null)

export function ConnectionProvider({ children }: { children: ReactNode }) {
  const [connection, setConnection] = useState<Connection | null>(() => loadConnection())

  const connect = useCallback((c: Connection) => {
    saveConnection(c)
    setConnection(c)
  }, [])

  const disconnect = useCallback(() => {
    clearConnection()
    setConnection(null)
  }, [])

  const value = useMemo(() => ({ connection, connect, disconnect }), [connection, connect, disconnect])
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

export function useConnection(): ConnectionCtx {
  const ctx = useContext(Ctx)
  if (!ctx) throw new Error('useConnection must be used within ConnectionProvider')
  return ctx
}

// A tenant-scoped API client. Pass the tenant route param; falls back to the
// connection's default tenant for the portfolio screen (pre-tenant).
export function useApi(tenant?: string): ApiClient {
  const { connection } = useConnection()
  if (!connection) throw new Error('useApi requires an active connection')
  const t = tenant || connection.tenant
  return useMemo(() => new ApiClient(connection, t), [connection, t])
}
