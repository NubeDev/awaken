/**
 * Read-only view of a principal's capability grants (ADMIN.md §6). Grant mutation
 * (PUT/DELETE .../grants/:capability) exists on the backend but is NOT exposed in
 * the POC UI: which capabilities a role/extension needs is a backend policy
 * concern, and the seed already confers the right grants per role (cast.rs). So
 * here we DISPLAY the grants (honest, no fake mutation) and leave granting to the
 * seed / a later WS. See WS-05.md.
 */
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Badge } from '@/components/ui/badge'
import { useGrants } from './hooks'

export function GrantsDialog({
  subject,
  open,
  onOpenChange,
}: {
  subject: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const grants = useGrants(subject, open)
  const rows = grants.data ?? []
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className='max-w-md'>
        <DialogHeader>
          <DialogTitle>Grants — {subject}</DialogTitle>
          <DialogDescription>
            Capability grants on this principal (read-only). Grants are conferred by
            the tenant seed per role; mutation is out of POC scope.
          </DialogDescription>
        </DialogHeader>
        {grants.isLoading ? (
          <p className='text-muted-foreground text-sm'>Loading…</p>
        ) : rows.length === 0 ? (
          <p className='text-muted-foreground text-sm'>No capability grants.</p>
        ) : (
          <div className='flex flex-wrap gap-2'>
            {rows.map((g) => (
              <Badge key={g.capability} variant='outline'>
                {g.capability}
              </Badge>
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  )
}
