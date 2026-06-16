/**
 * Create / edit a `kind:"tenant"` (DOMAIN-MODEL §tenant): key, name, and the
 * rubix `namespace` its data lives under. A tenant is the portfolio root — it has
 * no parent relation. On edit the existing content is spread back so any tags
 * survive the PATCH untouched.
 */
import { useState } from 'react'
import type { Tenant, TenantRecord } from '@/api/records'
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
import { useCreateTenant, useUpdateTenant } from './hooks'

type TenantFormProps = {
  open: boolean
  onOpenChange: (open: boolean) => void
  /** Present = edit; absent = create. */
  tenant?: TenantRecord
}

export function TenantForm({ open, onOpenChange, tenant }: TenantFormProps) {
  const editing = tenant !== undefined
  const start = tenant?.content
  const [key, setKey] = useState(start?.key ?? '')
  const [name, setName] = useState(start?.name ?? '')
  const [namespace, setNamespace] = useState(start?.namespace ?? '')

  const create = useCreateTenant()
  const update = useUpdateTenant()
  const pending = create.isPending || update.isPending
  const valid = key.trim() !== '' && name.trim() !== ''

  const save = () => {
    if (editing && tenant) {
      const content: Tenant = { ...tenant.content, key, name, namespace }
      update.mutate(
        { id: tenant.id, content },
        { onSuccess: () => onOpenChange(false) }
      )
      return
    }
    const content: Tenant = { kind: 'tenant', key, name, namespace }
    create.mutate(content, { onSuccess: () => onOpenChange(false) })
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-md'>
        <DialogHeader>
          <DialogTitle>{editing ? 'Edit tenant' : 'New tenant'}</DialogTitle>
          <DialogDescription>
            The portfolio root that owns sites. The namespace is the rubix
            namespace its data lives under.
          </DialogDescription>
        </DialogHeader>

        <div className='grid gap-4'>
          <div className='grid gap-1'>
            <Label htmlFor='tenant-key'>Key</Label>
            <Input
              id='tenant-key'
              value={key}
              onChange={(e) => setKey(e.target.value)}
              placeholder='acme'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='tenant-name'>Name</Label>
            <Input
              id='tenant-name'
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder='Acme Industries'
            />
          </div>
          <div className='grid gap-1'>
            <Label htmlFor='tenant-namespace'>Namespace</Label>
            <Input
              id='tenant-namespace'
              value={namespace}
              onChange={(e) => setNamespace(e.target.value)}
              placeholder='acme'
            />
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
