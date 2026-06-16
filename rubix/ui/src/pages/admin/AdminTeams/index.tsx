// Admin · Teams — named groups of principals, and the access that flows to their
// members. Backs the control-plane surface in crates/rubix-server/src/http/admin/
// teams.rs. A team has members (principals) and capability grants; a team grant is
// inherited by every member (the gate's effective-grants union), so granting a
// whole team is how access is assigned at scale rather than principal-by-principal.

import { getRouteApi } from '@tanstack/react-router'
import { useState } from 'react'
import { Users, Plus, Trash2, UserPlus, X } from 'lucide-react'
import { useApi } from '../../../api/ConnectionContext'
import { useQuery } from '@tanstack/react-query'
import { listTeamMembers, listTeamGrants } from '../../../api/admin'
import { useTeams, usePrincipals } from '../../../hooks/useAdmin'
import { useTeamMutations, useTeamDetailMutations } from '../../../hooks/useAdminMutations'
import { usePageHeader } from '../../../components/shell/page-header'
import { ErrorView, LoadingView, EmptyView } from '../../../components/ui/StateView'
import { Button } from '../../../components/ui/button'
import { Input } from '../../../components/ui/input'
import { Label } from '../../../components/ui/label'
import { Badge } from '../../../components/ui/badge'
import { Checkbox } from '../../../components/ui/checkbox'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../../../components/ui/dialog'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '../../../components/ui/select'
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from '../../../components/ui/table'
import { useToast } from '../../../components/ui/toast'
import { CAPABILITIES, type Team } from '../../../types/Admin'

const route = getRouteApi('/t/$tenant/admin/teams')

export function AdminTeams() {
  const { tenant } = route.useParams()
  const teams = useTeams(tenant)
  const { remove } = useTeamMutations(tenant)
  const { toast } = useToast()

  const [createOpen, setCreateOpen] = useState(false)
  const [selected, setSelected] = useState<Team | null>(null)

  usePageHeader({ crumbs: ['Admin', 'Teams'] })

  async function handleDelete(t: Team) {
    if (!window.confirm(`Delete team ${t.display_name}? Members keep their accounts.`)) return
    try {
      await remove.mutateAsync(t.slug)
      toast('Team deleted')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Delete failed', 'error')
    }
  }

  return (
    <div className="px-6 py-6">
      <div className="mx-auto max-w-[1100px]">
        <div className="mb-5 flex items-center justify-between gap-3">
          <div className="flex items-center gap-3">
            <div className="grid size-11 place-items-center rounded-xl border border-border bg-card">
              <Users size={20} className="text-muted-foreground" />
            </div>
            <div>
              <h1 className="text-[22px] font-semibold tracking-tight">Teams</h1>
              <div className="text-[13px] text-muted-foreground">
                Group people and grant access once — every member inherits it.
              </div>
            </div>
          </div>
          <Button onClick={() => setCreateOpen(true)} className="gap-1.5">
            <Plus size={16} /> New team
          </Button>
        </div>

        {teams.isLoading && <LoadingView label="Loading teams…" />}
        {teams.error && <ErrorView error={teams.error} />}
        {teams.data && teams.data.length === 0 && (
          <EmptyView title="No teams yet" hint="Create a team, add members, then grant it access." />
        )}

        {teams.data && teams.data.length > 0 && (
          <div className="rounded-xl border border-border bg-card/40">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Team</TableHead>
                  <TableHead>Slug</TableHead>
                  <TableHead />
                  <TableHead />
                </TableRow>
              </TableHeader>
              <TableBody>
                {teams.data.map((t) => (
                  <TableRow key={t.slug}>
                    <TableCell className="font-medium">{t.display_name}</TableCell>
                    <TableCell className="mono text-xs text-muted-foreground">{t.slug}</TableCell>
                    <TableCell>
                      <Button variant="outline" size="sm" onClick={() => setSelected(t)}>
                        Manage
                      </Button>
                    </TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => handleDelete(t)}
                        aria-label="Delete team"
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

      <CreateTeamDialog tenant={tenant} open={createOpen} onOpenChange={setCreateOpen} />
      {selected && (
        <ManageTeamDialog
          tenant={tenant}
          team={selected}
          open={selected !== null}
          onOpenChange={(o) => !o && setSelected(null)}
        />
      )}
    </div>
  )
}

// Slugify a display name into a URL-safe key the way the backend expects: lower,
// spaces/punctuation → single hyphens, trimmed. So "Field Engineers" → "field-engineers".
function slugify(input: string): string {
  return input
    .toLowerCase()
    .trim()
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-+|-+$/g, '')
}

function CreateTeamDialog({
  tenant,
  open,
  onOpenChange,
}: {
  tenant: string
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const { create } = useTeamMutations(tenant)
  const { toast } = useToast()
  const [name, setName] = useState('')
  // The slug auto-derives from the name but stays editable once the user touches it.
  const [slug, setSlug] = useState('')
  const [slugEdited, setSlugEdited] = useState(false)
  const effectiveSlug = slugEdited ? slug : slugify(name)

  function reset() {
    setName('')
    setSlug('')
    setSlugEdited(false)
  }

  async function handleCreate() {
    try {
      await create.mutateAsync({ slug: effectiveSlug, display_name: name.trim() || effectiveSlug })
      toast('Team created')
      reset()
      onOpenChange(false)
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Create failed', 'error')
    }
  }

  return (
    <Dialog
      open={open}
      onOpenChange={(o) => {
        if (!o) reset()
        onOpenChange(o)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>New team</DialogTitle>
          <DialogDescription>
            A team groups people. Add members and grant it access after creating.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-4">
          <div className="flex flex-col gap-2">
            <Label htmlFor="t-name">Name</Label>
            <Input
              id="t-name"
              value={name}
              onChange={(e) => setName(e.target.value)}
              placeholder="Field Engineers"
              autoComplete="off"
            />
          </div>
          <div className="flex flex-col gap-2">
            <Label htmlFor="t-slug">Slug</Label>
            <Input
              id="t-slug"
              value={effectiveSlug}
              onChange={(e) => {
                setSlugEdited(true)
                setSlug(slugify(e.target.value))
              }}
              placeholder="field-engineers"
              autoComplete="off"
              className="mono text-xs"
            />
            <p className="text-xs text-muted-foreground">
              The stable key used in grants. Lowercase, hyphenated.
            </p>
          </div>
        </div>

        <DialogFooter>
          <Button variant="ghost" onClick={() => onOpenChange(false)} disabled={create.isPending}>
            Cancel
          </Button>
          <Button onClick={handleCreate} disabled={create.isPending || !effectiveSlug}>
            {create.isPending ? 'Creating…' : 'Create'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

function ManageTeamDialog({
  tenant,
  team,
  open,
  onOpenChange,
}: {
  tenant: string
  team: Team
  open: boolean
  onOpenChange: (open: boolean) => void
}) {
  const api = useApi(tenant)
  const principals = usePrincipals(tenant)
  const members = useQuery({
    queryKey: ['team-members', tenant, team.slug],
    queryFn: () => listTeamMembers(api, team.slug),
  })
  const grants = useQuery({
    queryKey: ['team-grants', tenant, team.slug],
    queryFn: () => listTeamGrants(api, team.slug),
  })
  const { addMember, removeMember, grant, revoke } = useTeamDetailMutations(tenant, team.slug)
  const { toast } = useToast()

  const [toAdd, setToAdd] = useState('')

  const memberSet = new Set((members.data ?? []).map((m) => m.subject))
  // Only user principals not already on the team can be added.
  const addable = (principals.data ?? []).filter(
    (p) => p.kind === 'user' && !memberSet.has(p.subject),
  )
  const heldCaps = new Set((grants.data ?? []).map((g) => g.capability))

  async function handleAdd() {
    if (!toAdd) return
    try {
      await addMember.mutateAsync(toAdd)
      toast('Member added')
      setToAdd('')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Add failed', 'error')
    }
  }

  async function handleRemove(subject: string) {
    try {
      await removeMember.mutateAsync(subject)
      toast('Member removed')
    } catch (e) {
      toast(e instanceof Error ? e.message : 'Remove failed', 'error')
    }
  }

  async function toggleCap(capability: string, on: boolean) {
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
      <DialogContent className="max-w-[640px]">
        <DialogHeader>
          <DialogTitle>{team.display_name}</DialogTitle>
          <DialogDescription>
            Members and the capabilities every member inherits.
          </DialogDescription>
        </DialogHeader>

        <div className="flex flex-col gap-6">
          {/* Members */}
          <section className="flex flex-col gap-3">
            <h3 className="text-sm font-semibold">Members</h3>

            <div className="flex items-center gap-2">
              <Select value={toAdd} onValueChange={setToAdd}>
                <SelectTrigger className="h-9 flex-1">
                  <SelectValue placeholder={addable.length ? 'Add a person…' : 'No one to add'} />
                </SelectTrigger>
                <SelectContent>
                  {addable.map((p) => (
                    <SelectItem key={p.subject} value={p.subject}>
                      {p.subject}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
              <Button
                size="sm"
                className="gap-1.5"
                onClick={handleAdd}
                disabled={!toAdd || addMember.isPending}
              >
                <UserPlus size={14} /> Add
              </Button>
            </div>

            {members.isLoading && <p className="text-sm text-muted-foreground">Loading members…</p>}
            {members.error && <ErrorView error={members.error} />}
            {members.data && members.data.length === 0 && (
              <p className="rounded-lg border border-dashed border-border px-3 py-2 text-sm text-muted-foreground">
                No members yet.
              </p>
            )}
            {members.data && members.data.length > 0 && (
              <div className="flex flex-wrap gap-1.5">
                {members.data.map((m) => (
                  <Badge key={m.subject} variant="secondary" className="gap-1 py-1 pl-2.5 pr-1">
                    <span className="mono text-xs">{m.subject}</span>
                    <button
                      type="button"
                      aria-label={`Remove ${m.subject}`}
                      className="grid size-4 place-items-center rounded hover:bg-muted-foreground/20"
                      onClick={() => handleRemove(m.subject)}
                      disabled={removeMember.isPending}
                    >
                      <X size={11} />
                    </button>
                  </Badge>
                ))}
              </div>
            )}
          </section>

          {/* Capabilities */}
          <section className="flex flex-col gap-3">
            <h3 className="text-sm font-semibold">Team access</h3>
            <p className="text-xs text-muted-foreground">
              Capabilities granted here are inherited by every member.
            </p>
            {grants.isLoading && <p className="text-sm text-muted-foreground">Loading grants…</p>}
            {grants.error && <ErrorView error={grants.error} />}
            {grants.data && (
              <div className="flex flex-col gap-1.5">
                {CAPABILITIES.map((cap) => {
                  const on = heldCaps.has(cap)
                  return (
                    <label
                      key={cap}
                      className="flex cursor-pointer items-center justify-between rounded-lg border border-border bg-card/40 px-3 py-2 text-sm"
                    >
                      <span className="mono text-xs">{cap}</span>
                      <Checkbox
                        checked={on}
                        disabled={grant.isPending || revoke.isPending}
                        onCheckedChange={(c) => toggleCap(cap, c === true)}
                      />
                    </label>
                  )
                })}
              </div>
            )}
          </section>
        </div>

        <DialogFooter>
          <Button onClick={() => onOpenChange(false)}>Done</Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
