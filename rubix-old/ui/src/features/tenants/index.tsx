import { useMemo, useState } from 'react'
import { Building2, Pencil, Plus, Trash2 } from 'lucide-react'
import { useDeleteSite, useOrgs, useSites } from '@/api/hooks'
import { tagNames } from '@/api/tags'
import type { OrgSummary, Site } from '@/api/types'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Card } from '@/components/ui/card'
import { ConfirmDialog } from '@/components/confirm-dialog'
import { Main } from '@/components/layout/main'
import { PageHeader } from '@/components/layout/page-header'
import { ProvisionTenantDialog } from './components/provision-tenant-dialog'
import { SiteFormDialog } from './components/site-form-dialog'

/**
 * Tenants admin: the org/site management surface. Orgs are derived from the
 * sites the principal can see (`GET /orgs`); each org expands to its sites with
 * full create/edit/delete. Provisioning a tenant creates its first site under a
 * new org. There is no org row to delete — an org disappears when its last site
 * is removed.
 */
export function Tenants() {
  const { data: orgs = [] } = useOrgs()
  const { data: sites = [] } = useSites()
  const [provisionOpen, setProvisionOpen] = useState(false)

  const sitesByOrg = useMemo(() => {
    const map = new Map<string, Site[]>()
    for (const s of sites) {
      const list = map.get(s.org) ?? []
      list.push(s)
      map.set(s.org, list)
    }
    for (const list of map.values())
      list.sort((a, b) => a.slug.localeCompare(b.slug))
    return map
  }, [sites])

  return (
    <>
      <PageHeader title='Tenants' sub='Orgs and sites across your portfolio' />
      <Main fluid fixed className='flex min-h-0 flex-col'>
        <div className='mb-3 flex items-center justify-between'>
          <p className='text-sm text-muted-foreground'>
            {orgs.length} {orgs.length === 1 ? 'tenant' : 'tenants'}
          </p>
          <Button size='sm' onClick={() => setProvisionOpen(true)}>
            <Plus className='size-4' /> New tenant
          </Button>
        </div>

        <div className='scroll min-h-0 flex-1 space-y-4 overflow-y-auto pe-1'>
          {orgs.length === 0 ? (
            <Card className='grid h-40 place-items-center'>
              <p className='text-sm text-muted-foreground'>
                No tenants yet. Provision one to get started.
              </p>
            </Card>
          ) : (
            orgs.map((org) => (
              <OrgSection
                key={org.org}
                org={org}
                sites={sitesByOrg.get(org.org) ?? []}
              />
            ))
          )}
        </div>
      </Main>

      <ProvisionTenantDialog
        open={provisionOpen}
        onOpenChange={setProvisionOpen}
      />
    </>
  )
}

function OrgSection({ org, sites }: { org: OrgSummary; sites: Site[] }) {
  const [createOpen, setCreateOpen] = useState(false)

  return (
    <Card className='p-3'>
      <div className='mb-2.5 flex items-center justify-between'>
        <div className='flex items-center gap-2'>
          <Building2 className='size-4 text-primary' />
          <span className='text-sm font-medium'>{org.org}</span>
          <Badge variant='muted' className='h-4 px-1.5 text-[10px]'>
            {org.site_count} {org.site_count === 1 ? 'site' : 'sites'}
          </Badge>
        </div>
        <Button size='sm' variant='ghost' onClick={() => setCreateOpen(true)}>
          <Plus className='size-3.5' /> Add site
        </Button>
      </div>
      <div className='space-y-1.5'>
        {sites.map((site) => (
          <SiteRow key={site.id} site={site} />
        ))}
      </div>

      <SiteFormDialog
        mode='create'
        org={org.org}
        open={createOpen}
        onOpenChange={setCreateOpen}
      />
    </Card>
  )
}

function SiteRow({ site }: { site: Site }) {
  const del = useDeleteSite()
  const [editOpen, setEditOpen] = useState(false)
  const [confirmOpen, setConfirmOpen] = useState(false)
  const markers = tagNames(site.tags)

  return (
    <div className='flex items-center justify-between rounded-md bg-muted/40 px-3 py-2'>
      <div className='min-w-0'>
        <div className='flex items-center gap-2'>
          <span className='truncate text-sm'>{site.display_name}</span>
          <code className='text-[11px] text-muted-foreground'>
            {site.org}/{site.slug}
          </code>
        </div>
        {markers.length > 0 ? (
          <div className='mt-1 flex flex-wrap gap-1'>
            {markers.map((m) => (
              <Badge key={m} variant='muted' className='h-4 px-1 text-[9.5px]'>
                {m}
              </Badge>
            ))}
          </div>
        ) : null}
      </div>
      <div className='flex shrink-0 items-center gap-1'>
        <Button
          size='icon'
          variant='ghost'
          className='size-7'
          onClick={() => setEditOpen(true)}
        >
          <Pencil className='size-3.5' />
        </Button>
        <Button
          size='icon'
          variant='ghost'
          className='size-7 text-sev-fault'
          onClick={() => setConfirmOpen(true)}
        >
          <Trash2 className='size-3.5' />
        </Button>
      </div>

      <SiteFormDialog
        mode='edit'
        site={site}
        open={editOpen}
        onOpenChange={setEditOpen}
      />

      <ConfirmDialog
        open={confirmOpen}
        onOpenChange={setConfirmOpen}
        destructive
        title={`Delete ${site.display_name}?`}
        desc={
          <>
            This permanently deletes{' '}
            <code>
              {site.org}/{site.slug}
            </code>{' '}
            and <strong>cascades</strong> to every equip, point, history sample,
            and spark beneath it. This cannot be undone.
          </>
        }
        confirmText='Delete site'
        isLoading={del.isPending}
        handleConfirm={() =>
          del.mutate(site.id, { onSuccess: () => setConfirmOpen(false) })
        }
      />
    </div>
  )
}
