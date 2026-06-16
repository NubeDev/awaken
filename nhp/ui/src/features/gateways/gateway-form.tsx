/**
 * Create / edit a `kind:"gateway"` (ADMIN.md §5, DOMAIN-MODEL §gateway): key,
 * name, site (the REQUIRED parent relation — the gate enforces it, verified live),
 * model, host. The site picker lists `kind:"site"` records by their record id
 * (relations are by id, per the WS-03 seed). Cross-tenant site onboarding is the
 * wizard's job (WS-06); here you pick an existing site.
 *
 * status / last_seen are POLLER-OWNED (DOMAIN-MODEL "Status fields are
 * poller-owned"): this form NEVER sends them. On edit the existing content is
 * spread back so the poller's status/last_seen survive the PATCH untouched — the
 * UI just doesn't expose them as inputs.
 */
import { useState } from 'react'
import type { Gateway, GatewayRecord } from '@/api/records'
import { Button } from '@/components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useCreateGateway, useSites, useUpdateGateway } from './hooks'

type GatewayFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Present = edit; absent = create. */
  gateway?: GatewayRecord
}

export function GatewayForm({ open, onOpenChange, gateway }: GatewayFormProps) {
  const editing = gateway !== undefined
  const start = gateway?.content
  const [key, setKey] = useState(start?.key ?? '')
  const [name, setName] = useState(start?.name ?? '')
  const [site, setSite] = useState(start?.site ?? '')
  const [model, setModel] = useState(start?.model ?? '')
  const [host, setHost] = useState(start?.host ?? '')

  const create = useCreateGateway()
  const update = useUpdateGateway()
  const sites = useSites()
  const pending = create.isPending || update.isPending
  // site is required by the gateway collection (gate-enforced, verified live).
  const valid = key.trim() !== '' && name.trim() !== '' && site !== ''

  const save = () => {
    if (editing && gateway) {
      // Spread existing content first so poller-owned status/last_seen survive.
      const content: Gateway = { ...gateway.content, key, name, site, model, host }
      update.mutate(
        { id: gateway.id, content },
        { onSuccess: () => onOpenChange(false) }
      )
      return
    }
    // Create never sets status/last_seen — the poller writes them later.
    const content: Gateway = { kind: 'gateway', key, name, site, model, host }
    create.mutate(content, { onSuccess: () => onOpenChange(false) })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-md'>
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit gateway' : 'New gateway'}</DialogTitle>
          <DialogDescription>
            A field device hosting network ports. Online status and last-seen are
            written by the polling service and shown read-only.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid gap-1'>
            <Label htmlFor='gw-key'>Key</Label>
            <Input
              id='gw-key'
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder='gw-01'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='gw-name'>Name</Label>
            <Input
              id='gw-name'
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder='Basement gateway'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='gw-site'>Site</Label>
            <Select value={site} onValueChange={setSite}>
              <SelectTrigger id='gw-site'>
                <SelectValue placeholder='Select a site' />
              </SelectTrigger>
              <SelectContent>
                {(sites.data ?? []).map((s) => (
                  <SelectItem key={s.id} value={s.id}>
                    {s.content.name} ({s.content.key})
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className='grid gap-4 sm:grid-cols-2'>
            <div className='grid gap-1'>
              <Label htmlFor='gw-model'>Model</Label>
              <Input
                id='gw-model'
                value={model}
                onChange={(e) => setModel(e.target.value)}
                placeholder='EdgeBox RPi'
              />
            </div>
            <div className='grid gap-1'>
              <Label htmlFor='gw-host'>Host</Label>
              <Input
                id='gw-host'
                value={host}
                onChange={(e) => setHost(e.target.value)}
                placeholder='10.0.0.12'
              />
            </div>
          </div>
        </div>

        <DialogFooter>
          <Button variant='ghost' onClick={() => onOpenChange(false)}>
            Cancel
          </Button>
          <Button onClick={save} disabled={!valid || pending}>
            {pending ? 'Saving…' : 'Save'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
