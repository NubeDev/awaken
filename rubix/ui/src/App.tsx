// App shell: the connection gate wraps everything. With no active connection we
// render the Connect screen; once connected, the tenant-scoped router takes over.
// TanStack Query is the server-state cache for the REST surface.

import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { RouterProvider } from '@tanstack/react-router'
import { ConnectionProvider, useConnection } from './api/ConnectionContext'
import { ToastProvider } from './components/ui/toast'
import { Connect } from './pages/Connect'
import { router } from './router'

const queryClient = new QueryClient({
  defaultOptions: { queries: { retry: 1, refetchOnWindowFocus: true } },
})

function Gate() {
  const { connection } = useConnection()
  if (!connection) return <Connect />
  return <RouterProvider router={router} />
}

export default function App() {
  return (
    <QueryClientProvider client={queryClient}>
      <ToastProvider>
        <ConnectionProvider>
          <Gate />
        </ConnectionProvider>
      </ToastProvider>
    </QueryClientProvider>
  )
}
