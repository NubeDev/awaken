// Admin · Principals & grants — CRUD over identities and their capability grants,
// driving the control-plane surface (crates/rubix-server/src/http/admin). A
// principal is the substrate identity (subject + namespace + kind + role); grants
// are the gate's capability strings. No domain concept appears here.

import { getRouteApi } from '@tanstack/react-router'
import { useState } from 'react'
import { KeyRound, Plus, Trash2 } from 'lucide-react'
import { useApi } from '../../api/ConnectionContext'
import { useQuery } from '@tanstack/react-query'
import { listGrants } from '../../api/admin'
import { usePrincipals } from '../../hooks/useAdmin'
import { usePrincipalMutations, useGrantMutations } from '../../hooks/useAdminMutations'
import { AdminLayout } from '../../components/admin/AdminLayout'
import { ErrorView, LoadingView, EmptyView } from '../../components/ui/StateView'
import { Button } from '../../components/ui/button'
import { Input } from '../../components/ui/input'
import { Label } from '../../components/ui/label'
import { Badge } from '../../components/ui/badge'
import { Checkbox } from '../../components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../components/ui/table'
import { useToast } from '../../components/ui/toast'
import {
  CAPABILITIES,
  PRINCIPAL_KINDS,
  ROLES,
  type Principal,
  type CreatePrincipalRequest,
} from '../../types/Admin'

const route = getRouteApi('/t/$tenant/admin/principals')

export function AdminPrincipals() {
  const { tenant } = route.useParams()
  const principals = usePrincipals(tenant)
  const { setRole, remove } = usePrincipalMutations(tenant)
  const { toast } = useToast()

  const [createOpen, setCreateOpen] = useState(false)
  const [selected, setSelected] = useState<Principal | null>(null)

  async function handleRole(subject: string, role: string) {
    try {
      await setRole.mutateAsync({ subject, role })
      toast('Role updated')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Update failed', 'error')
    }
  }

  async function handleDelete(p: Principal) {
    if (!window.confirm(`Delete principal ${p.subject}?`)) return
    try {
      await remove.mutateAsync(p.subject)
      toast('Principal deleted')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Delete failed', 'error')
    }
  }

  return (
    <AdminLayout active="principals">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
              <KeyRound size={20} className="text-muted-foreground" />
            </div>
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight">Principals</h1>
              <div className="text-[13px] text-muted-foreground">
                Identities and their capability grants in {tenant}.
              </div>
            </div>
          </div>
          <Button onClick={() => setCreateOpen(true)} className="gap-1.5">
            <Plus size={16} /> New principal
          </Button>
        </div>

        {principals.isLoading && <LoadingView label="Loading principals…" />}
        {principals.error && <ErrorView error={principals.error} />}
        {principals.data && principals.data.length === 0 && (
          <EmptyView title="No principals" hint="Create one to grant it capabilities." />
        )}

        {principals.data && principals.data.length > 0 && (
          <div className="rounded-xl border border-border bg-card/40">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Subject</TableHead>
                  <TableHead>Kind</TableHead>
                  <TableHead>Role</TableHead>
                  <TableHead>Grants</TableHead>
                  <TableHead />
                </TableRow>
              </TableHeader>
              <TableBody>
                {principals.data.map((p) => (
                  <TableRow key={p.subject}>
                    <TableCell className="mono font-medium">{p.subject}</TableCell>
                    <TableCell>
                      <Badge variant="secondary" className="text-[10px]">
                        {p.kind}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <Select value={p.role} onValueChange={(role) => handleRole(p.subject, role)}>
                        <SelectTrigger className="h-8 w-[120px]">
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {ROLES.map((r) => (
                            <SelectItem key={r} value={r}>
                              {r}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </TableCell>
                    <TableCell>
                      <Button variant="outline" size="sm" onClick={() => setSelected(p)}>
                        Manage
                      </Button>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleDelete(p)}
                        aria-label="Delete principal"
                      >
                        <Trash2 size={14} className="text-destructive" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </div>
        )}
      </div>

      <CreatePrincipalDialog tenant={tenant} open={createOpen} onOpenChange={setCreateOpen} />
      {selected && (
        <GrantsDialog
          tenant={tenant}
          principal={selected}
          open={selected !== null}
          onOpenChange={(o) => !o && setSelected(null)}
        />
      )}
    </AdminLayout>
  )
}

function CreatePrincipalDialog({
  tenant,
  open,
  onOpenChange,
}: {
  tenant: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { create } = usePrincipalMutations(tenant)
  const { toast } = useToast()
  const [form, setForm] = useState<CreatePrincipalRequest>({
    subject: '',
    kind: 'user',
    role: 'viewer',
    secret: '',
  })
  const [mintedSecret, setMintedSecret] = useState<string | null>(null)

  function set<K extends keyof CreatePrincipalRequest>(key: K, value: CreatePrincipalRequest[K]) {
    setForm((f) => ({ ...f, [key]: value }))
  }

  async function handleCreate() {
    try {
      // Empty secret ⇒ omit it so the server mints one and returns it once.
      const body: CreatePrincipalRequest = { ...form, secret: form.secret || undefined }
      const created = await create.mutateAsync(body)
      if (created.secret) {
        setMintedSecret(created.secret)
        toast('Principal created — copy the secret now')
      } else {
        toast('Principal created')
        onOpenChange(false)
      }
      setForm({ subject: '', kind: 'user', role: 'viewer', secret: '' })
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Create failed', 'error')
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) setMintedSecret(null)
        onOpenChange(o)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New principal</DialogTitle>
          <DialogDescription>
            Leave the secret blank to have the server generate one (shown once).
          </DialogDescription>
        </DialogHeader>

        {mintedSecret ? (
          <div className="flex flex-col gap-2">
            <Label>Generated secret — copy it now, it is shown only once</Label>
            <code className="mono select-all break-all rounded-md border border-border bg-card px-3 py-2 text-xs">
              {mintedSecret}
            </code>
          </div>
        ) : (
          <div className="flex flex-col gap-4">
            <div className="flex flex-col gap-2">
              <Label htmlFor="p-subject">Subject</Label>
              <Input
                id="p-subject"
                value={form.subject}
                onChange={(e) => set('subject', e.target.value)}
                placeholder="alice"
                autoComplete="off"
              />
            </div>
            <div className="grid grid-cols-2 gap-3">
              <div className="flex flex-col gap-2">
                <Label>Kind</Label>
                <Select value={form.kind} onValueChange={(v) => set('kind', v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {PRINCIPAL_KINDS.map((k) => (
                      <SelectItem key={k} value={k}>
                        {k}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
              <div className="flex flex-col gap-2">
                <Label>Role</Label>
                <Select value={form.role} onValueChange={(v) => set('role', v)}>
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {ROLES.map((r) => (
                      <SelectItem key={r} value={r}>
                        {r}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              </div>
            </div>
            <div className="flex flex-col gap-2">
              <Label htmlFor="p-secret">Secret (optional)</Label>
              <Input
                id="p-secret"
                value={form.secret}
                onChange={(e) => set('secret', e.target.value)}
                placeholder="leave blank to generate"
                autoComplete="off"
              />
            </div>
          </div>
        )}

        <DialogFooter>
          {mintedSecret ? (
            <Button onClick={() => onOpenChange(false)}>Done</Button>
          ) : (
            <>
              <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={create.isPending}>
                Cancel
              </Button>
              <Button onClick={handleCreate} disabled={create.isPending || !form.subject}>
                {create.isPending ? 'Creating…' : 'Create'}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function GrantsDialog({
  tenant,
  principal,
  open,
  onOpenChange,
}: {
  tenant: string
  principal: Principal
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const api = useApi(tenant)
  const grants = useQuery({
    queryKey: ['grants', tenant, principal.subject],
    queryFn: () => listGrants(api, principal.subject),
  })
  const { grant, revoke } = useGrantMutations(tenant, principal.subject)
  const { toast } = useToast()

  const held = new Set((grants.data ?? []).map((g) => g.capability))

  async function toggle(capability: string, on: boolean) {
    try {
      if (on) await grant.mutateAsync(capability)
      else await revoke.mutateAsync(capability)
      toast(on ? `Granted ${capability}` : `Revoked ${capability}`)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Grant change failed', 'error')
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>
            Grants · <span className="mono">{principal.subject}</span>
          </DialogTitle>
          <DialogDescription>Toggle the capabilities this principal holds.</DialogDescription>
        </DialogHeader>

        {grants.isLoading && <p className="text-sm text-muted-foreground">Loading grants…</p>}
        {grants.error && <ErrorView error={grants.error} />}

        {grants.data && (
          <div className="flex flex-col gap-1.5">
            {CAPABILITIES.map((cap) => {
              const on = held.has(cap)
              return (
                <label
                  key={cap}
                  className="flex cursor-pointer items-center justify-between rounded-lg border border-border bg-card/40 px-3 py-2 text-sm"
                >
                  <span className="mono text-xs">{cap}</span>
                  <Checkbox
                    checked={on}
                    disabled={grant.isPending || revoke.isPending}
                    onCheckedChange={(c) => toggle(cap, c === true)}
                  />
                </label>
              )
            })}
          </div>
        )}

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
