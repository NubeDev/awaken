import { createFileRoute, Link, Outlet } from '@tanstack/react-router'
import { Main } from '@/components/layout/main'

/**
 * Admin layout (back-of-house). Hosts a small section nav and an Outlet for the
 * sub-routes: meter-types (WS-04); gateways/networks/users land under here too
 * (WS-05). NHP nav is flat, so the sections are tabs inside the page, not URL
 * scope. See nhp/docs/ADMIN.md.
 */
const SECTIONS = [
  { to: '/admin/meter-types', label: 'Meter-types' },
] as const

function AdminLayout() {
  return (
    <Main>
      <div className='mb-4 flex gap-1 border-b'>
        {SECTIONS.map((s) => (
          <Link
            key={s.to}
            to={s.to}
            className='data-[status=active]:border-primary data-[status=active]:text-foreground text-muted-foreground -mb-px border-b-2 border-transparent px-3 py-2 text-sm'
            activeProps={{ 'data-status': 'active' }}
          >
            {s.label}
          </Link>
        ))}
      </div>
      <Outlet />
    </Main>
  )
}

export const Route = createFileRoute('/_authenticated/admin')({
  component: AdminLayout,
})
