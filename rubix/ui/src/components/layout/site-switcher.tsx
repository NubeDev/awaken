import { Building2, ChevronsUpDown } from 'lucide-react'
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuLabel,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu'
import {
  SidebarMenu,
  SidebarMenuButton,
  SidebarMenuItem,
  useSidebar,
} from '@/components/ui/sidebar'
import { useActiveSite } from '@/hooks/use-active-site'
import { useSiteStore } from '@/stores/site-store'

/**
 * Sidebar header control: switch the active site. Sites are read live from the
 * API; the selection drives `useSiteStore`, which every page scopes to. Replaces
 * the template's static team switcher.
 */
export function SiteSwitcher() {
  const { isMobile } = useSidebar()
  const { site, sites } = useActiveSite()
  const setSite = useSiteStore((s) => s.setSite)

  return (
    <SidebarMenu>
      <SidebarMenuItem>
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <SidebarMenuButton
              size='lg'
              className='data-[state=open]:bg-sidebar-accent data-[state=open]:text-sidebar-accent-foreground'
            >
              <div className='bg-sidebar-primary text-sidebar-primary-foreground flex aspect-square size-8 items-center justify-center rounded-lg'>
                <Building2 className='size-4' />
              </div>
              <div className='grid flex-1 text-start text-sm leading-tight'>
                <span className='truncate font-semibold'>{site?.display_name ?? 'Rubix'}</span>
                <span className='text-muted-foreground truncate text-xs'>
                  {site ? `${site.org} · ${site.slug}` : 'Building Intelligence'}
                </span>
              </div>
              <ChevronsUpDown className='ms-auto' />
            </SidebarMenuButton>
          </DropdownMenuTrigger>
          <DropdownMenuContent
            className='w-(--radix-dropdown-menu-trigger-width) min-w-56 rounded-lg'
            align='start'
            side={isMobile ? 'bottom' : 'right'}
            sideOffset={4}
          >
            <DropdownMenuLabel className='text-muted-foreground text-xs'>Sites</DropdownMenuLabel>
            {sites.map((s) => (
              <DropdownMenuItem key={s.id} onClick={() => setSite(s.id)} className='gap-2 p-2'>
                <div className='flex size-6 items-center justify-center rounded-sm border'>
                  <Building2 className='size-4 shrink-0' />
                </div>
                <span className='truncate'>{s.display_name}</span>
              </DropdownMenuItem>
            ))}
          </DropdownMenuContent>
        </DropdownMenu>
      </SidebarMenuItem>
    </SidebarMenu>
  )
}
