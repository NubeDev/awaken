// App shell: the connection gate wraps everything. With no active connection we
// render the Connect screen; once connected, the tenant-scoped router takes over.
// TanStack Query is the server-state cache for the REST surface.

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import { ConnectionProvider, useConnection } from './api/ConnectionContext'
import { PreferencesProvider } from './context/PreferencesContext'
import { ThemeProvider } from './context/theme-provider'
import { ToastProvider } from './components/ui/toast'
import { Connect } from './pages/Connect'
import { router } from './router'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: true } },
})

function Gate() {
  const { connection } = useConnection()
  if (!connection) return <Connect />
  // An active connection exists here, so the preferences provider can fetch.
  return (
    <PreferencesProvider>
      <RouterProvider router={router} />
    </PreferencesProvider>
  )
}

export default function App() {
  return (
    <ThemeProvider>
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          <ConnectionProvider>
            <Gate />
          </ConnectionProvider>
        </ToastProvider>
      </QueryClientProvider>
    </ThemeProvider>
  )
}
